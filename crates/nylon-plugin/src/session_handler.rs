//! Session handling and stream management for plugins

use std::collections::HashMap;

use crate::{constants::methods, stream::PluginSessionStream, types::PluginResult};
// use bytes::Bytes;
// use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
// use nylon_sdk::fbs::plugin_generated::nylon_plugin::{
//     HeaderKeyValue, HeaderKeyValueArgs, NylonHttpHeaders, NylonHttpHeadersArgs,
// };
use nylon_types::{
    context::NylonContext,
    plugins::SessionStream,
    template::{Expr, apply_payload_ast},
};
use pingora::proxy::Session;
// use pingora::{http::ResponseHeader, protocols::http::HttpTask, proxy::Session};
// use serde_json::json;

/// Handles session stream operations for plugins
pub struct SessionHandler;

impl SessionHandler {
    /// Process a method from the plugin session stream
    pub async fn process_method(
        method: u32,
        _data: Vec<u8>,
        ctx: &mut NylonContext,
        session: &mut Session,
        session_stream: &SessionStream,
        payload: &Option<serde_json::Value>,
        payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    ) -> Result<Option<PluginResult>, NylonError> {
        match method {
            // Control methods
            methods::GET_PAYLOAD => {
                Self::handle_get_payload(session_stream, session, payload, payload_ast, ctx)
                    .await?;
                Ok(None)
            }
            methods::NEXT => Ok(Some(PluginResult::default())),
            methods::END => Ok(Some(PluginResult::new(true, false))),

            // Response methods
            // methods::SET_RESPONSE_HEADER => {
            //     Self::handle_set_response_header(&data, ctx).await?;
            //     Ok(None)
            // }
            // methods::REMOVE_RESPONSE_HEADER => {
            //     Self::handle_remove_response_header(&data, ctx).await?;
            //     Ok(None)
            // }
            // methods::SET_RESPONSE_STATUS => {
            //     Self::handle_set_response_status(&data, ctx).await?;
            //     Ok(None)
            // }
            // methods::SET_RESPONSE_FULL_BODY => {
            //     Self::handle_set_response_full_body(&data, ctx).await?;
            //     Ok(None)
            // }
            // methods::SET_RESPONSE_STREAM_HEADER => {
            //     Self::handle_set_response_stream_header(ctx, session).await?;
            //     Ok(None)
            // }
            // methods::SET_RESPONSE_STREAM_DATA => {
            //     Self::handle_set_response_stream_data(&data, session).await?;
            //     Ok(None)
            // }
            // methods::SET_RESPONSE_STREAM_END => {
            //     Self::handle_set_response_stream_end(session).await?;
            //     Ok(Some(PluginResult::new(false, true)))
            // }
            // methods::READ_RESPONSE_FULL_BODY => {
            //     Self::handle_read_response_full_body(session_stream, ctx).await?;
            //     Ok(None)
            // }

            // // Request methods
            // methods::READ_REQUEST_FULL_BODY => {
            //     Self::handle_read_request_full_body(session_stream, ctx, session).await?;
            //     Ok(None)
            // }
            // methods::READ_REQUEST_HEADER => {
            //     Self::handle_read_request_header(&data, session_stream, session).await?;
            //     Ok(None)
            // }
            // methods::READ_REQUEST_HEADERS => {
            //     Self::handle_read_request_headers(session_stream, session).await?;
            //     Ok(None)
            // }

            // Unknown method
            _ => Err(NylonError::ConfigError(format!(
                "Invalid method: {}",
                method
            ))),
        }
    }

    async fn handle_get_payload(
        session_stream: &SessionStream,
        session: &mut Session,
        payload: &Option<serde_json::Value>,
        payload_ast: &Option<HashMap<String, Vec<Expr>>>,
        ctx: &mut NylonContext,
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
}
