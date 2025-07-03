//! Nylon Plugin System
//!
//! This module provides a flexible and extensible plugin system for the Nylon proxy server.
//! It supports both built-in plugins and dynamically loaded external plugins.
//!
//! # Architecture
//!
//! The plugin system is organized into several modules:
//!
//! - **constants**: Defines all constants used throughout the plugin system
//! - **types**: Core types and structures for plugin operations
//! - **plugin_manager**: Plugin management and discovery
//! - **session_handler**: Session stream management and method processing
//! - **stream**: Stream management and FFI communication
//! - **loaders**: Dynamic library loading and symbol resolution
//! - **native**: Built-in native plugins
//!
//! # Examples
//!
//! ```rust
//! use nylon_plugin::{run_middleware, MiddlewareContext};
//!
//! // Create middleware context
//! let context = MiddlewareContext {
//!     middleware: middleware_item,
//!     payload: Some(payload_value),
//!     payload_ast: Some(ast),
//!     params: Some(params),
//! };
//!
//! // Run middleware
//! let result = run_middleware(&context, &mut ctx, &mut session).await?;
//! ```

use crate::{
    plugin_manager::PluginManager,
    session_handler::SessionHandler,
    stream::PluginSessionStream,
    types::{BuiltinPlugin, MiddlewareContext, PluginResult},
};
use nylon_error::NylonError;
use nylon_types::{context::NylonContext, template::apply_payload_ast};
use pingora::proxy::Session;

use std::collections::HashMap;

pub mod constants;
pub mod loaders;
mod native;
pub mod plugin_manager;
pub mod session_handler;
pub mod stream;
pub mod types;

/// Execute a session stream for a plugin
pub async fn session_stream(
    plugin_name: &str,
    entry: &str,
    _payload: &Option<Vec<u8>>,
    _params: &Option<HashMap<String, String>>,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<PluginResult, NylonError> {
    let session_stream = PluginManager::get_or_create_session_stream(plugin_name, ctx)?;
    let (_session_id, mut rx) = session_stream.open(entry).await?;

    loop {
        tokio::select! {
            Some((method, data)) = rx.recv() => {
                if let Some(result) = SessionHandler::process_method(
                    method,
                    data,
                    ctx,
                    session,
                    &session_stream,
                ).await? {
                    return Ok(result);
                }
            }
        }
    }
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
    match PluginManager::try_builtin(plugin_name.as_str()) {
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
                let result =
                    session_stream(plugin_name, request_filter, &payload, params, ctx, session)
                        .await?;
                return Ok((result.http_end, result.stream_end));
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
