use crate::{constants::methods, stream::PluginSessionStream, types::PluginResult};
use bytes::Bytes;
use sha1::{Digest, Sha1};
use base64::Engine;
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
    fn build_ws_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(2 + payload.len() + 8);
        // FIN=1 RSV=0 and opcode
        frame.push(0x80 | (opcode & 0x0F));
        // Server to client frames are not masked
        let len = payload.len();
        if len <= 125 {
            frame.push(len as u8);
        } else if len <= 65535 {
            frame.push(126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(127);
            frame.extend_from_slice(&(len as u64).to_be_bytes());
        }
        frame.extend_from_slice(payload);
        frame
    }
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
                Self::handle_set_response_full_body(data, ctx).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STREAM_HEADER => {
                Self::handle_set_response_stream_header(proxy, ctx, session).await?;
                Ok(None)
            }
            methods::SET_RESPONSE_STREAM_DATA => {
                Self::handle_set_response_stream_data(data, session).await?;
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

            // WebSocket control methods (temporary stub to simulate events)
            methods::WEBSOCKET_UPGRADE => {
                // Perform WebSocket handshake (101)
                let headers = session.req_header();
                let key = headers.headers.get("sec-websocket-key").and_then(|v| v.to_str().ok()).unwrap_or("");
                if key.is_empty() {
                    // Fallback text response if no key
                    let mut headers = ResponseHeader::build(400u16, None)
                        .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
                    let _ = headers.append_header("content-type", "text/plain");
                    let tasks = vec![
                        HttpTask::Header(Box::new(headers), false),
                        HttpTask::Body(Some(Bytes::from_static(b"Missing Sec-WebSocket-Key")), false),
                        HttpTask::Done,
                    ];
                    session
                        .response_duplex_vec(tasks)
                        .await
                        .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;
                    return Ok(Some(PluginResult::new(true, false)));
                }

                // Compute Sec-WebSocket-Accept
                let mut hasher = Sha1::new();
                hasher.update(key.as_bytes());
                hasher.update(nylon_store::websockets::WEBSOCKET_GUID.as_bytes());
                let accept_key = base64::engine::general_purpose::STANDARD.encode(hasher.finalize());

                let mut resp = ResponseHeader::build(101u16, None)
                    .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
                let _ = resp.append_header("upgrade", "websocket");
                let _ = resp.append_header("connection", "Upgrade");
                let _ = resp.append_header("sec-websocket-accept", &accept_key);

                session
                    .response_duplex_vec(vec![HttpTask::Header(Box::new(resp), false)])
                    .await
                    .map_err(|e| NylonError::ConfigError(format!("Error sending response: {}", e)))?;

                // Notify plugin side
                session_stream
                    .event_stream(0, methods::WEBSOCKET_ON_OPEN, &[])
                    .await?;

                // Keep session open (wait for future events)
                Ok(None)
            }
            methods::WEBSOCKET_SEND_TEXT => {
                // Send a text frame to client
                let frame = Self::build_ws_frame(0x1, &data);
                let tasks = vec![HttpTask::Body(Some(Bytes::from(frame)), false)];
                session
                    .response_duplex_vec(tasks)
                    .await
                    .map_err(|e| NylonError::ConfigError(format!("Error sending WS text: {}", e)))?;
                Ok(None)
            }
            methods::WEBSOCKET_SEND_BINARY => {
                // Send a binary frame to client
                let frame = Self::build_ws_frame(0x2, &data);
                let tasks = vec![HttpTask::Body(Some(Bytes::from(frame)), false)];
                session
                    .response_duplex_vec(tasks)
                    .await
                    .map_err(|e| NylonError::ConfigError(format!("Error sending WS binary: {}", e)))?;
                Ok(None)
            }
            methods::WEBSOCKET_CLOSE => {
                let frame = Self::build_ws_frame(0x8, &[]);
                let tasks = vec![
                    HttpTask::Body(Some(Bytes::from(frame)), false),
                    HttpTask::Done,
                ];
                session
                    .response_duplex_vec(tasks)
                    .await
                    .map_err(|e| NylonError::ConfigError(format!("Error sending WS close: {}", e)))?;
                // notify plugin
                let _ = session_stream
                    .event_stream(0, methods::WEBSOCKET_ON_CLOSE, &[])
                    .await;
                Ok(Some(PluginResult::new(false, true)))
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
        data: Vec<u8>,
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        ctx.set_response_body = data;
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
        data: Vec<u8>,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let tasks = vec![HttpTask::Body(Some(Bytes::from(data)), false)];
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
