use bytes::Bytes;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use nylon_types::template::Expr;
use nylon_types::transport::TransportInvoke;
use pingora::proxy::{ProxyHttp, Session};
use std::collections::HashMap;

use crate::constants::methods;
use crate::session_handler::SessionHandler;
use crate::types::PluginResult;

/// Process a method invoke from messaging transport (NATS)
/// Unlike FFI, messaging doesn't have a SessionStream, so we adapt the handlers
pub async fn process_messaging_method<T>(
    proxy: &T,
    inv: TransportInvoke,
    ctx: &mut NylonContext,
    session: &mut Session,
    _payload: &Option<serde_json::Value>,
    _payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    _response_body: &Option<Bytes>,
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

        // Read methods - these need SessionStream to send responses back
        // For messaging, we'll need to send the response data back through NATS
        methods::GET_PAYLOAD
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
        | methods::READ_RESPONSE_ERROR => {
            // TODO: Implement read methods for messaging
            // These need to send response data back through NATS
            tracing::warn!(
                method,
                "Read method not yet implemented for messaging transport"
            );
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
    )
}

