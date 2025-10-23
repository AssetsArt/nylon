use bytes::Bytes;
use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
use nylon_messaging::MessagingTransport;
use nylon_types::context::NylonContext;
use nylon_types::plugins::PluginPhase;
use nylon_types::template::{Expr, apply_payload_ast};
use nylon_types::transport::{TransportEvent, TransportInvoke};
use pingora::proxy::{ProxyHttp, Session};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use tokio::task;

use crate::cache;
use crate::constants::methods;
use crate::session_handler::SessionHandler;
use crate::transport_handler::TransportSessionHandler;
use crate::types::PluginResult;

/// Process a method invoke from messaging transport (NATS)
/// Unlike FFI, messaging doesn't have a SessionStream, so we adapt the handlers
pub async fn process_messaging_method<T>(
    proxy: &T,
    handler: &mut TransportSessionHandler<MessagingTransport>,
    inv: TransportInvoke,
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<serde_json::Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    response_body: &Option<Bytes>,
) -> Result<Option<PluginResult>, NylonError>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
    let method = inv.method;
    let data = inv.data;

    match method {
        // Control methods
        methods::NEXT => Ok(Some(PluginResult::default())),
        methods::END => Ok(Some(PluginResult::new(true, false))),

        // Response write methods (modify state, no return data needed)
        methods::SET_RESPONSE_HEADER => {
            SessionHandler::handle_set_response_header(&data, ctx).await?;
            Ok(None)
        }
        methods::REMOVE_RESPONSE_HEADER => {
            SessionHandler::handle_remove_response_header(&data, ctx).await?;
            Ok(None)
        }
        methods::SET_RESPONSE_STATUS => {
            SessionHandler::handle_set_response_status(&data, ctx).await?;
            Ok(None)
        }
        methods::SET_RESPONSE_FULL_BODY => {
            SessionHandler::handle_set_response_full_body(data, ctx).await?;
            Ok(None)
        }
        methods::SET_RESPONSE_STREAM_HEADER => {
            SessionHandler::handle_set_response_stream_header(proxy, ctx, session).await?;
            Ok(None)
        }
        methods::SET_RESPONSE_STREAM_DATA => {
            SessionHandler::handle_set_response_stream_data(data, session).await?;
            Ok(None)
        }
        methods::SET_RESPONSE_STREAM_END => {
            SessionHandler::handle_set_response_stream_end(session).await?;
            Ok(Some(PluginResult::new(false, true)))
        }

        // Read methods - send response data back through the messaging transport
        methods::GET_PAYLOAD => {
            let mut payload_bytes = Vec::new();

            if let Some(mut payload_value) = payload.clone() {
                if let Some(ast) = payload_ast {
                    {
                        let headers = session.req_header_mut();
                        apply_payload_ast(&mut payload_value, ast, headers, ctx);
                    }
                }

                payload_bytes = serde_json::to_vec(&payload_value).map_err(|e| {
                    NylonError::RuntimeError(format!("Failed to serialize payload: {}", e))
                })?;
            }

            send_transport_event(handler, method, payload_bytes)?;
            Ok(None)
        }
        methods::READ_RESPONSE_FULL_BODY => {
            let mut body = ctx
                .set_response_body
                .read()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                .clone();

            if let Some(response) = response_body {
                body.extend_from_slice(response.as_ref());
            }

            send_transport_event(handler, method, body)?;
            Ok(None)
        }
        methods::READ_REQUEST_FULL_BODY
        | methods::READ_REQUEST_HEADER
        | methods::READ_REQUEST_HEADERS
        | methods::READ_REQUEST_URL
        | methods::READ_REQUEST_PATH
        | methods::READ_REQUEST_QUERY
        | methods::READ_REQUEST_PARAMS
        | methods::READ_REQUEST_HOST
        | methods::READ_REQUEST_CLIENT_IP
        | methods::READ_REQUEST_METHOD
        | methods::READ_RESPONSE_STATUS
        | methods::READ_REQUEST_BYTES
        | methods::READ_RESPONSE_BYTES
        | methods::READ_REQUEST_TIMESTAMP
        | methods::READ_RESPONSE_HEADERS
        | methods::READ_RESPONSE_DURATION
        | methods::READ_RESPONSE_ERROR => {
            match method {
                methods::READ_REQUEST_FULL_BODY => {
                    if !session.is_body_empty() && !ctx.read_body.load(Ordering::Relaxed) {
                        ctx.read_body.store(true, Ordering::Relaxed);
                        session.enable_retry_buffering();

                        while let Ok(Some(chunk)) = session.read_request_body().await {
                            ctx.request_body
                                .write()
                                .map_err(|_| {
                                    NylonError::InternalServerError("lock poisoned".into())
                                })?
                                .extend_from_slice(chunk.as_ref());
                        }
                    }

                    let body = ctx
                        .request_body
                        .read()
                        .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                        .clone();

                    send_transport_event(handler, method, body)?;
                }
                methods::READ_REQUEST_HEADER => {
                    let response_bytes = if !data.is_empty() {
                        let header_key = String::from_utf8_lossy(&data).to_string();
                        let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
                            Some(h2) => &h2.req_header().headers,
                            None => &session.req_header().headers,
                        };

                        headers
                            .get(&header_key)
                            .map(|value| value.as_bytes().to_vec())
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    send_transport_event(handler, method, response_bytes)?;
                }
                methods::READ_REQUEST_HEADERS => {
                    let headers: &HeaderMap<HeaderValue> = match session.as_http2() {
                        Some(h2) => &h2.req_header().headers,
                        None => &session.req_header().headers,
                    };

                    let headers_vec: Vec<(String, String)> = headers
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().to_string(),
                                v.to_str().unwrap_or_default().to_string(),
                            )
                        })
                        .collect();

                    let serialized = cache::build_headers_flatbuffer(&headers_vec);
                    send_transport_event(handler, method, serialized)?;
                }
                methods::READ_REQUEST_URL => {
                    let is_tls = ctx.tls.load(Ordering::Relaxed);
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
                    send_transport_event(handler, method, full_url.into_bytes())?;
                }
                methods::READ_REQUEST_PATH => {
                    let path = match session.as_http2() {
                        Some(h2) => h2.req_header().uri.path(),
                        None => session.req_header().uri.path(),
                    };
                    send_transport_event(handler, method, path.as_bytes().to_vec())?;
                }
                methods::READ_REQUEST_QUERY => {
                    let query = match session.as_http2() {
                        Some(h2) => h2.req_header().uri.query().unwrap_or(""),
                        None => session.req_header().uri.query().unwrap_or(""),
                    };
                    send_transport_event(handler, method, query.as_bytes().to_vec())?;
                }
                methods::READ_REQUEST_PARAMS => {
                    let params_json = {
                        let params = ctx
                            .params
                            .read()
                            .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
                        serde_json::to_vec(&*params).map_err(|e| {
                            NylonError::InternalServerError(format!("serialize error: {}", e))
                        })?
                    };
                    send_transport_event(handler, method, params_json)?;
                }
                methods::READ_REQUEST_HOST => {
                    let host = ctx
                        .host
                        .read()
                        .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                        .clone();
                    send_transport_event(handler, method, host.into_bytes())?;
                }
                methods::READ_REQUEST_CLIENT_IP => {
                    let client_ip = ctx
                        .client_ip
                        .read()
                        .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?
                        .clone();
                    send_transport_event(handler, method, client_ip.into_bytes())?;
                }
                methods::READ_REQUEST_METHOD => {
                    let method_name = match session.as_http2() {
                        Some(h2) => h2.req_header().method.as_str(),
                        None => session.req_header().method.as_str(),
                    };
                    send_transport_event(handler, method, method_name.as_bytes().to_vec())?;
                }
                methods::READ_RESPONSE_STATUS => {
                    let status = ctx.set_response_status.load(Ordering::Relaxed).to_string();
                    send_transport_event(handler, method, status.into_bytes())?;
                }
                methods::READ_REQUEST_BYTES => {
                    let bytes: i64 = session
                        .req_header()
                        .headers
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s: &str| s.parse::<i64>().ok())
                        .unwrap_or(0);
                    send_transport_event(handler, method, bytes.to_string().into_bytes())?;
                }
                methods::READ_RESPONSE_BYTES => {
                    let mut bytes: i64 = ctx
                        .set_response_body
                        .read()
                        .map(|body| body.len() as i64)
                        .unwrap_or(0);

                    if let Some(body) = response_body {
                        bytes += body.len() as i64;
                    }

                    send_transport_event(handler, method, bytes.to_string().into_bytes())?;
                }
                methods::READ_REQUEST_TIMESTAMP => {
                    let timestamp = ctx.request_timestamp.load(Ordering::Relaxed);
                    send_transport_event(handler, method, timestamp.to_string().into_bytes())?;
                }
                methods::READ_RESPONSE_HEADERS => {
                    let headers_map = ctx
                        .add_response_header
                        .read()
                        .map(|h| h.clone())
                        .unwrap_or_default();

                    let headers_vec: Vec<(String, String)> = headers_map
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();

                    let serialized = cache::build_headers_flatbuffer(&headers_vec);
                    send_transport_event(handler, method, serialized)?;
                }
                methods::READ_RESPONSE_DURATION => {
                    let start_time = ctx.request_timestamp.load(Ordering::Relaxed);
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let duration = now.saturating_sub(start_time);

                    send_transport_event(handler, method, duration.to_string().into_bytes())?;
                }
                methods::READ_RESPONSE_ERROR => {
                    let error_msg = ctx
                        .error_message
                        .read()
                        .map(|e| e.clone().unwrap_or_default())
                        .unwrap_or_default();
                    send_transport_event(handler, method, error_msg.into_bytes())?;
                }
                _ => {}
            }

            Ok(None)
        }

        // WebSocket methods - not supported in messaging transport
        methods::WEBSOCKET_UPGRADE
        | methods::WEBSOCKET_SEND_TEXT
        | methods::WEBSOCKET_SEND_BINARY
        | methods::WEBSOCKET_CLOSE
        | methods::WEBSOCKET_JOIN_ROOM
        | methods::WEBSOCKET_LEAVE_ROOM
        | methods::WEBSOCKET_BROADCAST_ROOM_TEXT
        | methods::WEBSOCKET_BROADCAST_ROOM_BINARY => {
            tracing::warn!(
                method,
                "WebSocket method not supported in messaging transport"
            );
            Err(NylonError::ConfigError(format!(
                "WebSocket method {} not supported in messaging transport",
                method
            )))
        }

        // Unknown method
        _ => Err(NylonError::ConfigError(format!(
            "Invalid method: {}",
            method
        ))),
    }
}

/// Check if a method is supported in messaging transport
pub fn is_method_supported(method: u32) -> bool {
    matches!(
        method,
        methods::NEXT
            | methods::END
            | methods::SET_RESPONSE_HEADER
            | methods::REMOVE_RESPONSE_HEADER
            | methods::SET_RESPONSE_STATUS
            | methods::SET_RESPONSE_FULL_BODY
            | methods::SET_RESPONSE_STREAM_HEADER
            | methods::SET_RESPONSE_STREAM_DATA
            | methods::SET_RESPONSE_STREAM_END
            | methods::GET_PAYLOAD
            | methods::READ_RESPONSE_FULL_BODY
            | methods::READ_REQUEST_FULL_BODY
            | methods::READ_REQUEST_HEADER
            | methods::READ_REQUEST_HEADERS
            | methods::READ_REQUEST_URL
            | methods::READ_REQUEST_PATH
            | methods::READ_REQUEST_QUERY
            | methods::READ_REQUEST_PARAMS
            | methods::READ_REQUEST_HOST
            | methods::READ_REQUEST_CLIENT_IP
            | methods::READ_REQUEST_METHOD
            | methods::READ_RESPONSE_STATUS
            | methods::READ_REQUEST_BYTES
            | methods::READ_RESPONSE_BYTES
            | methods::READ_REQUEST_TIMESTAMP
            | methods::READ_RESPONSE_HEADERS
            | methods::READ_RESPONSE_DURATION
            | methods::READ_RESPONSE_ERROR
    )
}

fn send_transport_event(
    handler: &mut TransportSessionHandler<MessagingTransport>,
    method: u32,
    data: Vec<u8>,
) -> Result<(), NylonError> {
    handler
        .send_event(TransportEvent {
            phase: PluginPhase::Zero.to_u8(),
            method,
            data,
        })
        .map_err(|e| {
            NylonError::RuntimeError(format!("Failed to send messaging transport event: {}", e))
        })?;

    let flush_result = task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(async { handler.transport_mut().flush_events().await })
    });

    flush_result.map_err(|e| {
        NylonError::RuntimeError(format!("Failed to flush messaging transport events: {}", e))
    })
}
