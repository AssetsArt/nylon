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
use pingora::proxy::{ProxyHttp, Session};
use std::collections::HashMap;

/// Execute a session stream for a plugin
pub async fn session_stream<T>(
    proxy: &T,
    plugin_name: &str,
    phase: u8,
    entry: &str,
    ctx: &mut NylonContext,
    session: &mut Session,
    payload: &Option<serde_json::Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
) -> Result<PluginResult, NylonError>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
    let plugin = PluginManager::get_plugin(plugin_name)?;
    let key = format!("{}-{}", plugin_name, entry);
    let mut session_id = ctx.session_ids.get(&key).unwrap_or(&0).clone();
    let session_stream;
    if let Some(ss) = ctx.session_stream.get(&key) {
        session_stream = ss.clone();
    } else {
        let ss = SessionStream::new(plugin, session_id);
        ctx.session_stream.insert(key.clone(), ss.clone());
        session_stream = ss;
    }
    if session_id == 0 {
        // open session
        let new_session_id = session_stream.open(entry).await?;
        session_id = new_session_id;
        ctx.session_ids.insert(key.clone(), new_session_id);
    }
    let rx_arc = match get_rx(session_id.clone()) {
        Ok(rx) => rx,
        Err(_) => {
            let session_stream_clone = session_stream.clone();
            tokio::spawn(async move {
                let _ = session_stream_clone.event_stream(phase, 0, b"").await;
            });
            return Ok(PluginResult {
                http_end: false,
                stream_end: false,
            });
        }
    };
    let mut rx = match rx_arc.try_lock() {
        Ok(rx) => rx,
        Err(_) => {
            let session_stream_clone = session_stream.clone();
            tokio::spawn(async move {
                let _ = session_stream_clone.event_stream(phase, 0, b"").await;
            });
            return Ok(PluginResult {
                http_end: false,
                stream_end: false,
            });
        }
    };

    let session_stream_clone = session_stream.clone();
    tokio::spawn(async move {
        let _ = session_stream_clone.event_stream(phase, 0, b"").await;
    });
    loop {
        tokio::select! {
            Some((method, data)) = rx.recv() => {
                if let Some(result) = SessionHandler::process_method(
                    proxy,
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

pub async fn run_middleware<T>(
    proxy: &T,
    phase: u8,
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(bool, bool), NylonError>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
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
                proxy,
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
