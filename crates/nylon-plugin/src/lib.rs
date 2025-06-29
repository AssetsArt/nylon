use crate::stream::PluginSessionStream;
use bytes::Bytes;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_types::{
    context::NylonContext,
    plugins::{FfiPlugin, SessionStream},
    route::MiddlewareItem,
    template::{Expr, apply_payload_ast},
};
use pingora::proxy::Session;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

pub mod dispatcher;
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

pub async fn run_middleware(
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<bool, NylonError> {
    let (middleware, payload, payload_ast, params) = (
        &middleware_context.middleware,
        &middleware_context.payload,
        &middleware_context.payload_ast,
        &middleware_context.params,
    );
    let Some(plugin_name) = &middleware.plugin else {
        return Ok(false);
    };
    match try_builtin(plugin_name.as_str()) {
        Some(BuiltinPlugin::RequestHeaderModifier) => {
            native::header_modifier::request(ctx, session, payload, payload_ast)?;
            Ok(false)
        }
        Some(BuiltinPlugin::ResponseHeaderModifier) => {
            native::header_modifier::response(ctx, session, payload, payload_ast)?;
            Ok(false)
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
                // try to get plugin
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
                let (_session_id, mut rx) = session_stream.open(request_filter).await?;
                while let Some((method, _)) = rx.recv().await {
                    if method == stream::METHOD_GET_PAYLOAD {
                        let payload = payload.as_ref().unwrap_or(&vec![]).clone();
                        session_stream.event_stream(method, &payload).await?;
                    } else if method == stream::METHOD_NEXT {
                        break;
                    } else {
                        return Err(NylonError::ConfigError(format!(
                            "Invalid method: {}",
                            method
                        )));
                    }
                }
                return Ok(false);
            } else if let Some(_response_filter) = &middleware.response_filter {
                // let http_context =
                //     nylon_sdk::proxy_http::build_http_context(session, ctx, params.clone())?;
                // let dispatcher = dispatcher::http_service_dispatch(
                //     ctx,
                //     Some(plugin_name.as_str()),
                //     Some(response_filter),
                //     &http_context,
                //     &payload,
                // )?;
                // let dispatcher = root_as_nylon_dispatcher(&dispatcher)
                //     .map_err(|e| NylonError::ConfigError(format!("Invalid dispatcher: {}", e)))?;
                // ctx.plugin_store = Some(dispatcher.store().unwrap_or_default().bytes().to_vec());
                // let http_ctx = match root_as_nylon_http_context(dispatcher.data().bytes()) {
                //     Ok(d) => d,
                //     Err(e) => {
                //         return Err(NylonError::ConfigError(format!(
                //             "Invalid http context: {}",
                //             e
                //         )));
                //     }
                // };

                // // clear all headers
                // for h in ctx.response_header.headers.clone() {
                //     if let Some(key) = h.0 {
                //         let _ = ctx.response_header.remove_header(key.as_str());
                //     }
                // }

                // let response = http_ctx.response();
                // let headers = response.headers();
                // for h in headers.iter().flatten() {
                //     let _ = ctx
                //         .response_header
                //         .append_header(h.key().to_string(), h.value().to_string());
                // }
                // let status = response.status();
                // let _ = ctx.response_header.set_status(status as u16);
            } else if let Some(_response_body_filter) = &middleware.response_body_filter {
                // // println!("response body filter {:?}", ctx.response_body);
                // let http_context =
                //     nylon_sdk::proxy_http::build_http_context(session, ctx, params.clone())?;
                // let dispatcher = dispatcher::http_service_dispatch(
                //     ctx,
                //     Some(plugin_name.as_str()),
                //     Some(response_body_filter),
                //     &http_context,
                //     &payload,
                // )?;
                // let dispatcher = root_as_nylon_dispatcher(&dispatcher)
                //     .map_err(|e| NylonError::ConfigError(format!("Invalid dispatcher: {}", e)))?;
                // ctx.plugin_store = Some(dispatcher.store().unwrap_or_default().bytes().to_vec());
                // let http_ctx = match root_as_nylon_http_context(dispatcher.data().bytes()) {
                //     Ok(d) => d,
                //     Err(e) => {
                //         return Err(NylonError::ConfigError(format!(
                //             "Invalid http context: {}",
                //             e
                //         )));
                //     }
                // };
                // let response = http_ctx.response();
                // let body = response.body().unwrap_or_default();
                // ctx.response_body = Some(Bytes::from(body.bytes().to_vec()));
            } else if let Some(_logging) = &middleware.logging {
                // todo!("logging");
            }

            Ok(false)
        }
    }
}
