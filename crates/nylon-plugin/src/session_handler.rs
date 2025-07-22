use crate::{constants::methods, stream::PluginSessionStream, types::PluginResult};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
use nylon_sdk::fbs::plugin_generated::nylon_plugin::{
    HeaderKeyValue, HeaderKeyValueArgs, NylonHttpHeaders, NylonHttpHeadersArgs,
};
use nylon_types::{
    context::NylonContext,
    plugins::SessionStream,
    template::{Expr, apply_payload_ast},
};
use pingora::{
    http::ResponseHeader,
    protocols::http::HttpTask,
    proxy::{ProxyHttp, Session},
};
use std::collections::HashMap;

/// Handles session stream operations for plugins
pub struct SessionHandler;

impl SessionHandler {
    /// Process a method from the plugin session stream
    pub async fn process_method<'a, T>(
        proxy: &T,
        method: u32,
        data: Vec<u8>,
        ctx: &mut NylonContext,
        session: &mut Session,
        session_stream: &SessionStream,
        payload: &Option<serde_json::Value>,
        payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    ) -> Result<Option<PluginResult>, NylonError>
    where
        T: ProxyHttp + Send + Sync,
        <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
    {
        // println!("method: {}, sid: {}", method, session_stream.session_id);
        match method {
            // Control methods
            methods::GET_PAYLOAD => {
                Self::handle_get_payload(ctx, session, session_stream, payload, payload_ast)
                    .await?;
                Ok(None)
            }
            methods::NEXT => Ok(Some(PluginResult::default())),
            methods::END => Ok(Some(PluginResult::new(true, false))),

            // Response methods
            methods::SET_RESPONSE_HEADER => {
                Self::handle_set_response_header(&data, ctx).await?;
                Ok(None)
            }
            methods::REMOVE_RESPONSE_HEADER => {
                Self::handle_remove_response_header(&data, ctx).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STATUS => {
                Self::handle_set_response_status(&data, ctx).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_FULL_BODY => {
                Self::handle_set_response_full_body(&data, ctx).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STREAM_HEADER => {
                Self::handle_set_response_stream_header(proxy, ctx, session).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STREAM_DATA => {
                Self::handle_set_response_stream_data(&data, session).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STREAM_END => {
                Self::handle_set_response_stream_end(session).await?;
                Ok(Some(PluginResult::new(false, true)))
            }
            methods::READ_RESPONSE_FULL_BODY => {
                Self::handle_read_response_full_body(session_stream, ctx).await?;
                Ok(None)
            }

            // Request methods
            methods::READ_REQUEST_FULL_BODY => {
                Self::handle_read_request_full_body(session_stream, ctx, session).await?;
                Ok(None)
            }
            methods::READ_REQUEST_HEADER => {
                Self::handle_read_request_header(&data, session_stream, session).await?;
                Ok(None)
            }
            methods::READ_REQUEST_HEADERS => {
                Self::handle_read_request_headers(session_stream, session).await?;
                Ok(None)
            }

            // Unknown method
            _ => Err(NylonError::ConfigError(format!(
                "Invalid method: {}",
                method
            ))),
        }
    }

    async fn handle_get_payload(
        ctx: &mut NylonContext,
        session: &mut Session,
        session_stream: &SessionStream,
        payload: &Option<serde_json::Value>,
        payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    ) -> Result<(), NylonError> {
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
        let payload_slice = payload.as_ref().map(|p| p.as_slice()).unwrap_or_default();
        session_stream
            .event_stream(0, methods::GET_PAYLOAD, payload_slice)
            .await
    }

    async fn handle_set_response_header(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        let headers = flatbuffers::root::<HeaderKeyValue>(data)
            .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
        ctx.add_response_header
            .insert(headers.key().to_string(), headers.value().to_string());
        Ok(())
    }

    async fn handle_remove_response_header(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        let header_key = String::from_utf8_lossy(data).to_string();
        ctx.remove_response_header.push(header_key);
        Ok(())
    }

    async fn handle_set_response_status(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        if data.len() >= 2 {
            let status = u16::from_be_bytes([data[0], data[1]]);
            ctx.set_response_status = status;
        }
        Ok(())
    }

    async fn handle_set_response_full_body(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        ctx.set_response_body = data.to_vec();
        Ok(())
    }

    async fn handle_set_response_stream_header<'a, T>(
        proxy: &T,
        ctx: &'a mut NylonContext,
        session: &mut Session,
    ) -> Result<(), NylonError>
    where
        T: ProxyHttp + Send + Sync,
        <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
    {
        let mut headers = ResponseHeader::build(ctx.set_response_status, None)
            .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;

        let mut proxy_ctx: <T as ProxyHttp>::CTX = ctx.clone().into();
        proxy
            .response_filter(session, &mut headers, &mut proxy_ctx)
            .await
            .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;

        let tasks = vec![HttpTask::Header(Box::new(headers), false)];
        session
            .response_duplex_vec(tasks)
            .await
            .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;
        Ok(())
    }

    async fn handle_set_response_stream_data(
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let tasks = vec![HttpTask::Body(Some(Bytes::from(data.to_vec())), false)];
        session
            .response_duplex_vec(tasks)
            .await
            .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;
        Ok(())
    }

    async fn handle_set_response_stream_end(session: &mut Session) -> Result<(), NylonError> {
        let tasks = vec![HttpTask::Done];
        session
            .response_duplex_vec(tasks)
            .await
            .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;
        Ok(())
    }

    async fn handle_read_response_full_body(
        session_stream: &SessionStream,
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        session_stream
            .event_stream(0, methods::READ_RESPONSE_FULL_BODY, &ctx.set_response_body)
            .await
    }

    async fn handle_read_request_full_body(
        session_stream: &SessionStream,
        ctx: &mut NylonContext,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        if !session.is_body_empty() && !ctx.read_body {
            ctx.read_body = true;
            session.enable_retry_buffering();
            while let Ok(Some(data)) = session.read_request_body().await {
                ctx.request_body.extend_from_slice(&data);
            }
        }
        session_stream
            .event_stream(0, methods::READ_REQUEST_FULL_BODY, &ctx.request_body)
            .await
    }

    async fn handle_read_request_header(
        data: &[u8],
        session_stream: &SessionStream,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        if !data.is_empty() {
            let read_key = String::from_utf8_lossy(data).to_string();
            let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
                Some(h2) => &h2.req_header().headers,
                None => &session.req_header().headers,
            };
            if let Some(value) = headers.get(&read_key) {
                session_stream
                    .event_stream(0, methods::READ_REQUEST_HEADER, value.as_bytes())
                    .await?;
            }
        } else {
            session_stream
                .event_stream(0, methods::READ_REQUEST_HEADER, &[])
                .await?;
        }
        Ok(())
    }

    async fn handle_read_request_headers(
        session_stream: &SessionStream,
        session: &mut Session,
    ) -> Result<(), NylonError> {
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
        session_stream
            .event_stream(0, methods::READ_REQUEST_HEADERS, headers)
            .await
    }
}
