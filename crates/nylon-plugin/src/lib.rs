use crate::{
    plugin_manager::PluginManager,
    stream::{PluginSessionStream, get_rx},
    types::{BuiltinPlugin, MiddlewareContext},
};
use nylon_error::NylonError;
use nylon_types::{context::NylonContext, plugins::SessionStream};
use pingora::proxy::Session;

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
    ctx: &mut NylonContext,
) -> Result<(), NylonError> {
    let plugin = PluginManager::get_plugin(plugin_name)?;
    let ss = SessionStream::new(plugin, ctx.session_id);
    if ctx.session_id == 0 {
        // open session
        let session_id = ss.open(entry).await?;
        ctx.session_id = session_id;
    }

    // call entry
    ss.event_stream(1, 0, b"test").await?;

    // loop rx
    let rx = get_rx(ctx.session_id).await?;
    let mut rx_guard = rx.lock().await;

    loop {
        tokio::select! {
            Some((method, data)) = rx_guard.recv() => {
                // if let Some(result) = SessionHandler::process_method(
                //     method,
                //     data,
                //     ctx,
                //     session,
                //     &session_stream,
                // ).await? {
                //     return Ok(result);
                // }
                println!("method: {:?}, data: {:?}", method, data);
                todo!("dddd")
            }
        }
    }
}

pub async fn run_middleware(
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(bool, bool), NylonError> {
    let (middleware, payload, payload_ast, _) = (
        &middleware_context.middleware,
        &middleware_context.payload,
        &middleware_context.payload_ast,
        &middleware_context.params,
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
            /*
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
            */
            // if let Some(request_filter) = &middleware.request_filter {
            //     let result =
            //         session_stream(plugin_name, request_filter, &payload, params, ctx, session)
            //             .await?;
            //     return Ok((result.http_end, result.stream_end));
            // } else if let Some(_response_filter) = &middleware.response_filter {
            //     // todo!("response filter");
            // } else if let Some(_response_body_filter) = &middleware.response_body_filter {
            //     // todo!("response body filter");
            // } else if let Some(_logging) = &middleware.logging {
            //     // todo!("logging");
            // }

            // println!("plugin_name: {}", plugin_name);
            // println!("entry: {}", entry);
            let result = session_stream(plugin_name, entry, ctx).await?;
            println!("result: {:?}", result);
            Ok((false, false))
        }
    }
}
