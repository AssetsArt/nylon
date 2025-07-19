pub mod constants;
pub mod loaders;
mod native;
pub mod plugin_manager;
pub mod session_handler;
pub mod stream;
pub mod types;

use crate::{
    plugin_manager::PluginManager,
    session_handler::SessionHandler,
    stream::{PluginSessionStream, get_rx},
    types::{BuiltinPlugin, MiddlewareContext, PluginResult},
};
use nylon_error::NylonError;
use nylon_types::{context::NylonContext, plugins::SessionStream, template::Expr};
use pingora::proxy::Session;
use std::collections::HashMap;

/// Execute a session stream for a plugin
pub async fn session_stream(
    plugin_name: &str,
    phase: u8,
    entry: &str,
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<serde_json::Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
) -> Result<PluginResult, NylonError> {
    let plugin = PluginManager::get_plugin(plugin_name)?;
    let session_stream = SessionStream::new(plugin, ctx.session_id);
    if ctx.session_id == 0 {
        // open session
        let session_id = session_stream.open(entry).await?;
        ctx.session_id = session_id;
    }
    // loop rx
    let rx = get_rx(ctx.session_id).await?;
    let mut rx_guard = rx.lock().await;

    // add session stream to context
    ctx.session_stream
        .insert(plugin_name.to_string(), session_stream.clone());

    // call phase
    let session_stream_clone = session_stream.clone();
    tokio::spawn(async move {
        let _ = session_stream_clone.event_stream(phase, 0, b"").await;
    });

    loop {
        // wait for method
        tokio::select! {
            Some((method, data)) = rx_guard.recv() => {
                if let Some(result) = SessionHandler::process_method(
                    method,
                    data,
                    ctx,
                    session,
                    &session_stream,
                    payload,
                    payload_ast,
                ).await? {
                    return Ok(result);
                }
            }
        }
    }
}

pub async fn run_middleware(
    phase: u8,
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(bool, bool), NylonError> {
    let (middleware, payload, payload_ast) = (
        &middleware_context.middleware,
        &middleware_context.payload,
        &middleware_context.payload_ast,
    );
    let (Some(plugin_name), Some(entry)) = (&middleware.plugin, &middleware.entry) else {
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
            let result = session_stream(
                plugin_name,
                phase,
                entry,
                ctx,
                session,
                &payload,
                &payload_ast,
            )
            .await?;
            Ok((result.http_end, result.stream_end))
        }
    }
}
