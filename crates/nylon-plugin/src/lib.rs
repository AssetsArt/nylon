use crate::stream::PluginSessionStream;
use bytes::Bytes;
use dashmap::DashMap;
use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
use nylon_sdk::fbs::plugin_generated::nylon_plugin::{
    HeaderKeyValue, HeaderKeyValueArgs, NylonHttpHeaders, NylonHttpHeadersArgs,
};
use nylon_types::{
    context::NylonContext,
    plugins::{FfiPlugin, SessionStream},
    route::MiddlewareItem,
    template::{Expr, apply_payload_ast},
};
use pingora::{http::ResponseHeader, protocols::http::HttpTask, proxy::Session};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

pub mod loaders;
mod native;
pub mod stream;

pub enum BuiltinPlugin {
    RequestHeaderModifier,
    ResponseHeaderModifier,
}

pub struct MiddlewareContext {
    pub middleware: MiddlewareItem,
    pub payload: Option<Value>,
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
    pub params: Option<HashMap<String, String>>,
}

fn try_builtin(name: &str) -> Option<BuiltinPlugin> {
    tracing::debug!("Trying builtin plugin: {}", name);
    match name {
        "RequestHeaderModifier" => Some(BuiltinPlugin::RequestHeaderModifier),
        "ResponseHeaderModifier" => Some(BuiltinPlugin::ResponseHeaderModifier),
        _ => None,
    }
}

pub fn try_request_filter(name: &str) -> Option<BuiltinPlugin> {
    match name {
        "RequestHeaderModifier" => Some(BuiltinPlugin::RequestHeaderModifier),
        _ => None,
    }
}

pub fn try_response_filter(name: &str) -> Option<BuiltinPlugin> {
    match name {
        "ResponseHeaderModifier" => Some(BuiltinPlugin::ResponseHeaderModifier),
        _ => None,
    }
}

pub fn get_plugin(name: &str) -> Result<Arc<FfiPlugin>, NylonError> {
    let Some(plugins) =
        &nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS)
    else {
        return Err(NylonError::ConfigError("Plugins not found".to_string()));
    };
    let Some(plugin) = plugins.get(name) else {
        return Err(NylonError::ConfigError("Plugin not found".to_string()));
    };
    Ok(plugin.clone())
}

pub async fn session_stream(
    plugin_name: &str,
    entry: &str,
    payload: &Option<Vec<u8>>,
    _params: &Option<HashMap<String, String>>,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(bool, bool), NylonError> {
    let mut http_end = false;
    let mut stream_end = false;
    let session_stream = match ctx.session_stream.get(plugin_name) {
        Some(session_stream) => session_stream,
        None => {
            let plugin = get_plugin(plugin_name)?;
            let session_stream = SessionStream::new(plugin.clone());
            ctx.session_stream
                .insert(plugin_name.to_string(), session_stream);
            match ctx.session_stream.get(plugin_name) {
                Some(session_stream) => session_stream,
                None => {
                    return Err(NylonError::ConfigError(
                        "Failed to get session stream".to_string(),
                    ));
                }
            }
        }
    };
    let (_session_id, mut rx) = session_stream.open(entry).await?;
    loop {
        tokio::select! {
            Some((method, data)) = rx.recv() => {
                if method == stream::METHOD_GET_PAYLOAD {
                    let payload = payload.as_ref().unwrap_or(&vec![]).clone();
                    session_stream.event_stream(method, &payload).await?;
                } else if method == stream::METHOD_NEXT {
                    break;
                } else if method == stream::METHOD_END {
                    http_end = true;
                    break;
                }
                // response
                else if method == stream::METHOD_SET_RESPONSE_HEADER {
                    let headers = flatbuffers::root::<HeaderKeyValue>(&data).map_err(|e| {
                        NylonError::ConfigError(format!("Invalid headers: {}", e))
                    })?;
                    ctx.add_response_header.insert(headers.key().to_string(), headers.value().to_string());
                } else if method == stream::METHOD_REMOVE_RESPONSE_HEADER {
                    let header_key = String::from_utf8_lossy(&data).to_string();
                    ctx.remove_response_header.push(header_key);
                } else if method == stream::METHOD_SET_RESPONSE_STATUS {
                    let status = u16::from_be_bytes([data[0], data[1]]);
                    ctx.set_response_status = status;
                } else if method == stream::METHOD_SET_RESPONSE_FULL_BODY {
                    ctx.set_response_body = data.to_vec();
                } else if method == stream::METHOD_SET_RESPONSE_STREAM_HEADER {
                    let mut headers = ResponseHeader::build(ctx.set_response_status, None).map_err(|e| {
                        NylonError::ConfigError(format!("Invalid headers: {}", e))
                    })?;
                    for (key, value) in ctx.add_response_header.iter() {
                        let _ = headers.append_header(key.to_ascii_lowercase(), value);
                    }
                    for key in ctx.remove_response_header.iter() {
                        let key = key.to_ascii_lowercase();
                        let _ = headers.remove_header(&key);
                    }
                    let tasks = vec![HttpTask::Header(Box::new(headers), false)];
                    session.response_duplex_vec(tasks).await.map_err(|e| {
                        NylonError::ConfigError(format!("Error sending response: {}", e))
                    })?;
                } else if method == stream::METHOD_SET_RESPONSE_STREAM_DATA {
                    let tasks = vec![HttpTask::Body(Some(Bytes::from(data)), false)];
                    session.response_duplex_vec(tasks).await.map_err(|e| {
                        NylonError::ConfigError(format!("Error sending response: {}", e))
                    })?;
                } else if method == stream::METHOD_SET_RESPONSE_STREAM_END {
                    let tasks = vec![HttpTask::Done];
                    session.response_duplex_vec(tasks).await.map_err(|e| {
                        NylonError::ConfigError(format!("Error sending response: {}", e))
                    })?;
                    stream_end = true;
                    break;
                } else if method == stream::METHOD_READ_RESPONSE_FULL_BODY {
                    session_stream.event_stream(method, &ctx.set_response_body).await?;
                }
                // request
                else if method == stream::METHOD_READ_REQUEST_FULL_BODY {
                    if !session.is_body_empty() {
                        while let Ok(Some(data)) = session.read_request_body().await {
                            ctx.request_body.extend_from_slice(&data);
                        }
                    }
                    session_stream.event_stream(method, &ctx.request_body).await?;
                } else if method == stream::METHOD_READ_REQUEST_HEADER {
                    if !data.is_empty() {
                        let read_key = String::from_utf8_lossy(&data).to_string();
                        let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
                            Some(h2) => &h2.req_header().headers,
                            None => &session.req_header().headers,
                        };
                        let value = headers.get(&read_key);
                        if let Some(value) = value {
                            session_stream.event_stream(method, value.as_bytes()).await?;
                        }
                    } else {
                        session_stream.event_stream(method, &[]).await?;
                    }
                } else if method == stream::METHOD_READ_REQUEST_HEADERS {
                    let mut fbs = flatbuffers::FlatBufferBuilder::new();
                    let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
                        Some(h2) => &h2.req_header().headers,
                        None => &session.req_header().headers,
                    };
                    let headers_vec = headers
                    .iter()
                    .map(|(k, v)| {
                        let key = fbs.create_string(k.as_str());
                        let value = fbs.create_string(v.to_str().unwrap_or_default());
                        HeaderKeyValue::create(
                            &mut fbs,
                            &HeaderKeyValueArgs {
                                key: Some(key),
                                value: Some(value),
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                let headers_vec = fbs.create_vector(&headers_vec);
                let headers = NylonHttpHeaders::create(
                    &mut fbs,
                    &NylonHttpHeadersArgs {
                        headers: Some(headers_vec),
                    },
                );
                fbs.finish(headers, None);
                let headers = fbs.finished_data();
                session_stream.event_stream(method, headers).await?;
                }
                // unknown method
                else {
                    return Err(NylonError::ConfigError(format!(
                        "Invalid method: {}",
                        method
                    )));
                }
            }
        }
    }

    Ok((http_end, stream_end))
}

pub async fn run_middleware(
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(bool, bool), NylonError> {
    let (middleware, payload, payload_ast, params) = (
        &middleware_context.middleware,
        &middleware_context.payload,
        &middleware_context.payload_ast,
        &middleware_context.params,
    );
    let Some(plugin_name) = &middleware.plugin else {
        return Ok((false, false));
    };
    match try_builtin(plugin_name.as_str()) {
        Some(BuiltinPlugin::RequestHeaderModifier) => {
            native::header_modifier::request(ctx, session, payload, payload_ast)?;
            Ok((false, false))
        }
        Some(BuiltinPlugin::ResponseHeaderModifier) => {
            native::header_modifier::response(ctx, session, payload, payload_ast)?;
            Ok((false, false))
        }
        _ => {
            let headers = session.req_header_mut();
            let payload: Option<Vec<u8>> = match payload.as_ref() {
                Some(payload) => {
                    let mut payload = payload.clone();
                    if let Some(payload_ast) = payload_ast {
                        apply_payload_ast(&mut payload, payload_ast, headers, ctx);
                    }
                    serde_json::to_vec(&payload).ok()
                }
                None => None,
            };
            if let Some(request_filter) = &middleware.request_filter {
                // return session_stream(plugin_name, request_filter, &payload, params, ctx, session)
                //     .await;
                return session_stream(plugin_name, request_filter, &payload, params, ctx, session)
                    .await;
            } else if let Some(_response_filter) = &middleware.response_filter {
                // todo!("response filter");
            } else if let Some(_response_body_filter) = &middleware.response_body_filter {
                // todo!("response body filter");
            } else if let Some(_logging) = &middleware.logging {
                // todo!("logging");
            }

            Ok((false, false))
        }
    }
}
