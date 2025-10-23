use crate::{constants::methods, stream::PluginSessionStream, types::PluginResult};
use base64::Engine;
use bytes::Bytes;
use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
use nylon_sdk::fbs::plugin_generated::nylon_plugin::HeaderKeyValue;
use nylon_types::plugins::PluginPhase;
use nylon_types::websocket::WebSocketMessage;
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
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use tokio::sync::mpsc;

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
    pub async fn process_method<T>(
        proxy: &T,
        method: u32,
        data: Vec<u8>,
        ctx: &mut NylonContext,
        session: &mut Session,
        session_stream: &SessionStream,
        payload: &Option<serde_json::Value>,
        payload_ast: &Option<HashMap<String, Vec<Expr>>>,
        response_body: &Option<Bytes>,
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
                Self::handle_read_response_full_body(session_stream, ctx, response_body).await?;
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
            methods::READ_REQUEST_URL => {
                Self::handle_read_request_url(session_stream, session, ctx).await?;
                Ok(None)
            }
            methods::READ_REQUEST_PATH => {
                Self::handle_read_request_path(session_stream, session).await?;
                Ok(None)
            }
            methods::READ_REQUEST_QUERY => {
                Self::handle_read_request_query(session_stream, session).await?;
                Ok(None)
            }
            methods::READ_REQUEST_PARAMS => {
                Self::handle_read_request_params(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_REQUEST_HOST => {
                Self::handle_read_request_host(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_REQUEST_CLIENT_IP => {
                Self::handle_read_request_client_ip(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_REQUEST_METHOD => {
                Self::handle_read_request_method(session_stream, session).await?;
                Ok(None)
            }
            methods::READ_RESPONSE_STATUS => {
                Self::handle_read_response_status(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_REQUEST_BYTES => {
                Self::handle_read_request_bytes(session_stream, session).await?;
                Ok(None)
            }
            methods::READ_RESPONSE_BYTES => {
                Self::handle_read_response_bytes(session_stream, ctx, response_body).await?;
                Ok(None)
            }
            methods::READ_REQUEST_TIMESTAMP => {
                Self::handle_read_request_timestamp(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_RESPONSE_HEADERS => {
                Self::handle_read_response_headers(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_RESPONSE_DURATION => {
                Self::handle_read_response_duration(session_stream, ctx).await?;
                Ok(None)
            }
            methods::READ_RESPONSE_ERROR => {
                Self::handle_read_response_error(session_stream, ctx).await?;
                Ok(None)
            }

            // WebSocket control methods (temporary stub to simulate events)
            methods::WEBSOCKET_UPGRADE => {
                // Perform WebSocket handshake (101)
                let headers = session.req_header();
                let key = headers
                    .headers
                    .get("sec-websocket-key")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                if key.is_empty() {
                    // Fallback text response if no key
                    let mut headers = ResponseHeader::build(400u16, None)
                        .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
                    let _ = headers.append_header("content-type", "text/plain");
                    let tasks = vec![
                        HttpTask::Header(Box::new(headers), false),
                        HttpTask::Body(
                            Some(Bytes::from_static(b"Missing Sec-WebSocket-Key")),
                            false,
                        ),
                        HttpTask::Done,
                    ];
                    session.response_duplex_vec(tasks).await.map_err(|e| {
                        NylonError::ConfigError(format!("Error sending response: {}", e))
                    })?;
                    return Ok(Some(PluginResult::new(true, false)));
                }

                // Compute Sec-WebSocket-Accept
                let mut hasher = Sha1::new();
                hasher.update(key.as_bytes());
                hasher.update(nylon_store::websockets::WEBSOCKET_GUID.as_bytes());
                let accept_key =
                    base64::engine::general_purpose::STANDARD.encode(hasher.finalize());

                let mut resp = ResponseHeader::build(101u16, None)
                    .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
                let _ = resp.append_header("upgrade", "websocket");
                let _ = resp.append_header("connection", "Upgrade");
                let _ = resp.append_header("sec-websocket-accept", &accept_key);

                session
                    .response_duplex_vec(vec![HttpTask::Header(Box::new(resp), false)])
                    .await
                    .map_err(|e| {
                        NylonError::ConfigError(format!("Error sending response: {}", e))
                    })?;

                // Register connection in adapter and start local dispatcher
                let connection_id = format!(
                    "{}:{}",
                    nylon_store::websockets::get_node_id()
                        .await
                        .unwrap_or_else(|_| "node".into()),
                    session_stream.session_id
                );
                let connection = nylon_types::websocket::WebSocketConnection {
                    id: connection_id.clone(),
                    session_id: session_stream.session_id,
                    rooms: vec![],
                    node_id: nylon_store::websockets::get_node_id()
                        .await
                        .unwrap_or_default(),
                    connected_at: chrono::Utc::now().timestamp() as u64,
                    metadata: HashMap::new(),
                };
                let _ = nylon_store::websockets::add_connection(connection).await;

                // local rx for cluster events
                let (tx, rx): (
                    mpsc::UnboundedSender<nylon_types::websocket::WebSocketMessage>,
                    mpsc::UnboundedReceiver<nylon_types::websocket::WebSocketMessage>,
                ) = mpsc::unbounded_channel();
                nylon_store::websockets::register_local_sender(connection_id.clone(), tx);
                // store ws rx per session for use in outer event loop if needed
                let _ = crate::stream::set_ws_rx(session_stream.session_id, rx).await;

                // Notify plugin side that WebSocket connection is established immediately
                let _ = session_stream
                    .event_stream(PluginPhase::Zero, methods::WEBSOCKET_ON_OPEN, &[])
                    .await;

                // Spawn task to forward cluster messages to client frames
                // NOTE: actual forwarding is handled in lib.rs select loop via get_ws_rx

                // Keep session open (wait for future events)
                Ok(None)
            }
            methods::WEBSOCKET_SEND_TEXT => {
                // Send a text frame to client
                let frame = Self::build_ws_frame(0x1, &data);
                let tasks = vec![HttpTask::Body(Some(Bytes::from(frame)), false)];
                session.response_duplex_vec(tasks).await.map_err(|e| {
                    NylonError::ConfigError(format!("Error sending WS text: {}", e))
                })?;
                Ok(None)
            }
            methods::WEBSOCKET_SEND_BINARY => {
                // Send a binary frame to client
                let frame = Self::build_ws_frame(0x2, &data);
                let tasks = vec![HttpTask::Body(Some(Bytes::from(frame)), false)];
                session.response_duplex_vec(tasks).await.map_err(|e| {
                    NylonError::ConfigError(format!("Error sending WS binary: {}", e))
                })?;
                Ok(None)
            }
            methods::WEBSOCKET_CLOSE => {
                // Send close frame to client
                let frame = Self::build_ws_frame(0x8, &[]);
                let tasks = vec![
                    HttpTask::Body(Some(Bytes::from(frame)), false),
                    HttpTask::Done,
                ];
                session.response_duplex_vec(tasks).await.map_err(|e| {
                    NylonError::ConfigError(format!("Error sending WS close: {}", e))
                })?;

                // Notify plugin that connection is closing
                // Spawn task to ensure event is sent before connection cleanup
                tokio::spawn({
                    let session_stream = session_stream.clone();
                    async move {
                        let _ = session_stream
                            .event_stream(PluginPhase::Zero, methods::WEBSOCKET_ON_CLOSE, &[])
                            .await;
                    }
                });

                // Cleanup adapter registration
                let conn_id = format!(
                    "{}:{}",
                    nylon_store::websockets::get_node_id()
                        .await
                        .unwrap_or_default(),
                    session_stream.session_id
                );
                nylon_store::websockets::unregister_local_sender(&conn_id);
                tokio::spawn(async move {
                    let _ = nylon_store::websockets::remove_connection(&conn_id).await;
                });

                // End the session
                Ok(Some(PluginResult::new(false, true)))
            }

            // WebSocket room operations
            methods::WEBSOCKET_JOIN_ROOM => {
                let room = String::from_utf8_lossy(&data).to_string();
                if !room.is_empty() {
                    let conn_id = format!(
                        "{}:{}",
                        nylon_store::websockets::get_node_id()
                            .await
                            .unwrap_or_default(),
                        session_stream.session_id
                    );
                    let _ = nylon_store::websockets::join_room(&conn_id, &room).await;
                }
                Ok(None)
            }
            methods::WEBSOCKET_LEAVE_ROOM => {
                let room = String::from_utf8_lossy(&data).to_string();
                if !room.is_empty() {
                    let conn_id = format!(
                        "{}:{}",
                        nylon_store::websockets::get_node_id()
                            .await
                            .unwrap_or_default(),
                        session_stream.session_id
                    );
                    let _ = nylon_store::websockets::leave_room(&conn_id, &room).await;
                }
                Ok(None)
            }
            methods::WEBSOCKET_BROADCAST_ROOM_TEXT => {
                if let Some((room, payload)) = Self::split_room_payload(&data) {
                    let message = String::from_utf8_lossy(&payload).to_string();
                    let _ = nylon_store::websockets::broadcast_to_room(
                        &room,
                        WebSocketMessage::Text(message),
                        None,
                    )
                    .await;
                }
                Ok(None)
            }
            methods::WEBSOCKET_BROADCAST_ROOM_BINARY => {
                if let Some((room, payload)) = Self::split_room_payload(&data) {
                    let _ = nylon_store::websockets::broadcast_to_room(
                        &room,
                        WebSocketMessage::Binary(payload),
                        None,
                    )
                    .await;
                }
                Ok(None)
            }

            // Unknown method
            _ => Err(NylonError::ConfigError(format!(
                "Invalid method: {}",
                method
            ))),
        }
    }

    /// Split room and payload using a NUL (0x00) delimiter: [room_bytes, 0x00, payload_bytes]
    fn split_room_payload(data: &[u8]) -> Option<(String, Vec<u8>)> {
        if let Some(pos) = data.iter().position(|b| *b == 0) {
            let room = String::from_utf8_lossy(&data[..pos]).to_string();
            let payload = data[pos + 1..].to_vec();
            if !room.is_empty() {
                return Some((room, payload));
            }
        }
        None
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
        let payload_slice = payload.as_deref().unwrap_or_default();
        session_stream
            .event_stream(PluginPhase::Zero, methods::GET_PAYLOAD, payload_slice)
            .await
    }

    pub async fn handle_set_response_header(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        let headers = flatbuffers::root::<HeaderKeyValue>(data)
            .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;
        ctx.add_response_header
            .write()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
            .insert(headers.key().to_string(), headers.value().to_string());
        Ok(())
    }

    pub async fn handle_remove_response_header(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        let header_key = String::from_utf8_lossy(data).to_string();
        ctx.remove_response_header
            .write()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
            .push(header_key);
        Ok(())
    }

    pub async fn handle_set_response_status(
        data: &[u8],
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        if data.len() >= 2 {
            let status = u16::from_be_bytes([data[0], data[1]]);
            ctx.set_response_status
                .store(status, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    pub async fn handle_set_response_full_body(
        data: Vec<u8>,
        ctx: &mut NylonContext,
    ) -> Result<(), NylonError> {
        *ctx.set_response_body
            .write()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))? = data;
        Ok(())
    }

    pub async fn handle_set_response_stream_header<T>(
        proxy: &T,
        ctx: &mut NylonContext,
        session: &mut Session,
    ) -> Result<(), NylonError>
    where
        T: ProxyHttp + Send + Sync,
        <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
    {
        let mut headers = ResponseHeader::build(
            ctx.set_response_status
                .load(std::sync::atomic::Ordering::Relaxed),
            None,
        )
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

    pub async fn handle_set_response_stream_data(
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

    pub async fn handle_set_response_stream_end(session: &mut Session) -> Result<(), NylonError> {
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
        response_body: &Option<Bytes>,
    ) -> Result<(), NylonError> {
        let mut body = {
            ctx.set_response_body
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                .clone()
        };
        if let Some(response_body) = response_body {
            body.extend_from_slice(response_body.as_ref());
        }
        session_stream
            .event_stream(PluginPhase::Zero, methods::READ_RESPONSE_FULL_BODY, &body)
            .await
    }

    async fn handle_read_request_full_body(
        session_stream: &SessionStream,
        ctx: &mut NylonContext,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        if !session.is_body_empty() && !ctx.read_body.load(std::sync::atomic::Ordering::Relaxed) {
            ctx.read_body
                .store(true, std::sync::atomic::Ordering::Relaxed);
            session.enable_retry_buffering();
            while let Ok(Some(data)) = session.read_request_body().await {
                ctx.request_body
                    .write()
                    .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                    .extend_from_slice(&data);
            }
        }
        let req_body = {
            ctx.request_body
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                .clone()
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_FULL_BODY,
                &req_body,
            )
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
                    .event_stream(
                        PluginPhase::Zero,
                        methods::READ_REQUEST_HEADER,
                        value.as_bytes(),
                    )
                    .await?;
            }
        } else {
            session_stream
                .event_stream(PluginPhase::Zero, methods::READ_REQUEST_HEADER, &[])
                .await?;
        }
        Ok(())
    }

    async fn handle_read_request_headers(
        session_stream: &SessionStream,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
            Some(h2) => &h2.req_header().headers,
            None => &session.req_header().headers,
        };

        // Convert to Vec for caching
        let headers_vec: Vec<(String, String)> = headers
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();

        // Use cached FlatBuffer serialization
        let serialized = crate::cache::build_headers_flatbuffer(&headers_vec);

        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_HEADERS,
                &serialized,
            )
            .await
    }

    async fn handle_read_request_url(
        session_stream: &SessionStream,
        session: &mut Session,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        // Build full URL: scheme://host[:port]/path?query
        let is_tls = ctx.tls.load(std::sync::atomic::Ordering::Relaxed);
        let scheme = if is_tls { "https" } else { "http" };
        let port = ctx
            .port
            .read()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
            .clone();
        let host = ctx
            .host
            .read()
            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
            .clone();

        let host_part = if !port.is_empty() && !["80", "443"].contains(&port.as_str()) {
            format!("{}:{}", host, port)
        } else {
            host
        };

        let uri = match session.as_http2() {
            Some(h2) => &h2.req_header().uri,
            None => &session.req_header().uri,
        };

        let path_and_query = uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or(uri.path());

        let full_url = format!("{}://{}{}", scheme, host_part, path_and_query);

        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_URL,
                full_url.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_path(
        session_stream: &SessionStream,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let path = match session.as_http2() {
            Some(h2) => h2.req_header().uri.path(),
            None => session.req_header().uri.path(),
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_PATH,
                path.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_query(
        session_stream: &SessionStream,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let query = match session.as_http2() {
            Some(h2) => h2.req_header().uri.query().unwrap_or(""),
            None => session.req_header().uri.query().unwrap_or(""),
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_QUERY,
                query.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_params(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let params_json = {
            let params = ctx
                .params
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
            serde_json::to_vec(&*params)
                .map_err(|e| NylonError::InternalServerError(format!("serialize error: {}", e)))?
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_PARAMS,
                &params_json,
            )
            .await
    }

    async fn handle_read_request_host(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let host = {
            ctx.host
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                .clone()
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_HOST,
                host.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_client_ip(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let client_ip = {
            ctx.client_ip
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                .clone()
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_CLIENT_IP,
                client_ip.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_method(
        session_stream: &SessionStream,
        session: &Session,
    ) -> Result<(), NylonError> {
        let method = match session.as_http2() {
            Some(h2) => h2.req_header().method.as_str(),
            None => session.req_header().method.as_str(),
        };
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_METHOD,
                method.as_bytes(),
            )
            .await
    }

    async fn handle_read_response_status(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let status = ctx
            .set_response_status
            .load(std::sync::atomic::Ordering::Relaxed);
        let status_str = status.to_string();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_RESPONSE_STATUS,
                status_str.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_bytes(
        session_stream: &SessionStream,
        session: &Session,
    ) -> Result<(), NylonError> {
        // Try to get Content-Length from request headers
        let bytes: i64 = session
            .req_header()
            .headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s: &str| s.parse::<i64>().ok())
            .unwrap_or(0);
        let bytes_str = bytes.to_string();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_BYTES,
                bytes_str.as_bytes(),
            )
            .await
    }

    async fn handle_read_response_bytes(
        session_stream: &SessionStream,
        ctx: &NylonContext,
        response_body: &Option<Bytes>,
    ) -> Result<(), NylonError> {
        // Try to get response body length from context
        let mut bytes: i64 = ctx
            .set_response_body
            .read()
            .map(|body| body.len() as i64)
            .unwrap_or(0);

        if let Some(response_body) = response_body {
            bytes += response_body.len() as i64;
        }

        let bytes_str = bytes.to_string();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_RESPONSE_BYTES,
                bytes_str.as_bytes(),
            )
            .await
    }

    async fn handle_read_request_timestamp(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let timestamp = ctx
            .request_timestamp
            .load(std::sync::atomic::Ordering::Relaxed);
        let timestamp_str = timestamp.to_string();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_REQUEST_TIMESTAMP,
                timestamp_str.as_bytes(),
            )
            .await
    }

    async fn handle_read_response_headers(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        // Get response headers from context
        let headers_map = ctx
            .add_response_header
            .read()
            .map(|h| h.clone())
            .unwrap_or_default();

        // Convert to Vec for caching
        let headers_vec: Vec<(String, String)> = headers_map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Use cached FlatBuffer serialization
        let serialized = crate::cache::build_headers_flatbuffer(&headers_vec);

        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_RESPONSE_HEADERS,
                &serialized,
            )
            .await
    }

    async fn handle_read_response_duration(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let start_time = ctx
            .request_timestamp
            .load(std::sync::atomic::Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let duration = now.saturating_sub(start_time);
        let duration_str = duration.to_string();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_RESPONSE_DURATION,
                duration_str.as_bytes(),
            )
            .await
    }

    async fn handle_read_response_error(
        session_stream: &SessionStream,
        ctx: &NylonContext,
    ) -> Result<(), NylonError> {
        let error_msg = ctx
            .error_message
            .read()
            .map(|e| e.clone().unwrap_or_default())
            .unwrap_or_default();
        session_stream
            .event_stream(
                PluginPhase::Zero,
                methods::READ_RESPONSE_ERROR,
                error_msg.as_bytes(),
            )
            .await
    }
}
