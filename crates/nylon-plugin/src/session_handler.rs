//! Session handling and stream management for plugins

use crate::{constants::methods, stream::PluginSessionStream, types::PluginResult};
use bytes::Bytes;
use http::{HeaderMap, HeaderValue};
use nylon_error::NylonError;
use nylon_sdk::fbs::plugin_generated::nylon_plugin::{
    HeaderKeyValue, HeaderKeyValueArgs, NylonHttpHeaders, NylonHttpHeadersArgs,
};
use nylon_types::{context::NylonContext, plugins::SessionStream};
use pingora::{http::ResponseHeader, protocols::http::HttpTask, proxy::Session};

/// Handles session stream operations for plugins
pub struct SessionHandler;

impl SessionHandler {
    /// Process a method from the plugin session stream
    pub async fn process_method(
        method: usize,
        data: Vec<u8>,
        ctx: &mut NylonContext,
        session: &mut Session,
        session_stream: &SessionStream,
    ) -> Result<Option<PluginResult>, NylonError> {
        match method {
            // Control methods
            methods::GET_PAYLOAD => {
                Self::handle_get_payload(session_stream, &data).await?;
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
                Self::handle_set_response_stream_header(ctx, session).await?;
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
        session_stream: &SessionStream,
        payload: &[u8],
    ) -> Result<(), NylonError> {
        session_stream
            .event_stream(methods::GET_PAYLOAD, payload)
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

    async fn handle_set_response_stream_header(
        ctx: &mut NylonContext,
        session: &mut Session,
    ) -> Result<(), NylonError> {
        let mut headers = ResponseHeader::build(ctx.set_response_status, None)
            .map_err(|e| NylonError::ConfigError(format!("Invalid headers: {}", e)))?;

        // Add headers
        for (key, value) in ctx.add_response_header.iter() {
            let _ = headers.append_header(key.to_ascii_lowercase(), value);
        }

        // Remove headers
        for key in ctx.remove_response_header.iter() {
            let key = key.to_ascii_lowercase();
            let _ = headers.remove_header(&key);
        }

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
            .event_stream(methods::READ_RESPONSE_FULL_BODY, &ctx.set_response_body)
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
            .event_stream(methods::READ_REQUEST_FULL_BODY, &ctx.request_body)
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
                    .event_stream(methods::READ_REQUEST_HEADER, value.as_bytes())
                    .await?;
            }
        } else {
            session_stream
                .event_stream(methods::READ_REQUEST_HEADER, &[])
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
            .event_stream(methods::READ_REQUEST_HEADERS, headers)
            .await
    }
}
