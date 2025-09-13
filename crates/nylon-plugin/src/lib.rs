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
use crate::constants::methods;
use bytes::Bytes;
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
    // WebSocket read/relay state
    let mut ws_active = false;
    let mut read_buf: Vec<u8> = Vec::with_capacity(4096);

    fn build_ws_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(2 + payload.len() + 8);
        frame.push(0x80 | (opcode & 0x0F));
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

    loop {
        if !ws_active {
            if let Some((method, data)) = rx.recv().await {
                if method == methods::WEBSOCKET_UPGRADE {
                    ws_active = true;
                }
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
            } else {
                // channel closed
                return Ok(PluginResult::default());
            }
            continue;
        }

        tokio::select! {
            // Plugin -> server events
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
            // Client -> server frames
            Ok(Some(chunk)) = session.read_request_body() => {
                read_buf.extend_from_slice(&chunk);
                // parse frames in read_buf
                loop {
                    if read_buf.len() < 2 { break; }
                    let b0 = read_buf[0];
                    let b1 = read_buf[1];
                    let fin = (b0 & 0x80) != 0;
                    let opcode = b0 & 0x0F;
                    let masked = (b1 & 0x80) != 0;
                    let mut idx = 2usize;
                    let mut payload_len: usize = (b1 & 0x7F) as usize;
                    if payload_len == 126 {
                        if read_buf.len() < idx + 2 { break; }
                        payload_len = u16::from_be_bytes([read_buf[idx], read_buf[idx+1]]) as usize;
                        idx += 2;
                    } else if payload_len == 127 {
                        if read_buf.len() < idx + 8 { break; }
                        payload_len = u64::from_be_bytes([
                            read_buf[idx],read_buf[idx+1],read_buf[idx+2],read_buf[idx+3],
                            read_buf[idx+4],read_buf[idx+5],read_buf[idx+6],read_buf[idx+7]
                        ]) as usize;
                        idx += 8;
                    }
                    let mut mask_key = [0u8;4];
                    if masked {
                        if read_buf.len() < idx + 4 { break; }
                        mask_key.copy_from_slice(&read_buf[idx..idx+4]);
                        idx += 4;
                    }
                    if read_buf.len() < idx + payload_len { break; }
                    let mut payload = read_buf[idx..idx+payload_len].to_vec();
                    if masked {
                        for i in 0..payload_len { payload[i] ^= mask_key[i % 4]; }
                    }
                    // remove frame from buffer
                    let remove_len = idx + payload_len;
                    read_buf.drain(0..remove_len);

                    // handle opcodes
                    match opcode {
                        0x1 => { // text
                            session_stream.event_stream(0, methods::WEBSOCKET_ON_MESSAGE_TEXT, &payload).await?;
                        }
                        0x2 => { // binary
                            session_stream.event_stream(0, methods::WEBSOCKET_ON_MESSAGE_BINARY, &payload).await?;
                        }
                        0x8 => { // close
                            let frame = build_ws_frame(0x8, &payload);
                            let _ = session.response_duplex_vec(vec![pingora::protocols::http::HttpTask::Body(Some(Bytes::from(frame)), false), pingora::protocols::http::HttpTask::Done]).await;
                            let _ = session_stream.event_stream(0, methods::WEBSOCKET_ON_CLOSE, &[]).await;
                            return Ok(PluginResult::new(false, true));
                        }
                        0x9 => { // ping -> pong
                            let frame = build_ws_frame(0xA, &payload);
                            let _ = session.response_duplex_vec(vec![pingora::protocols::http::HttpTask::Body(Some(Bytes::from(frame)), false)]).await;
                        }
                        0xA => { /* pong: ignore */ }
                        _ => { /* ignore */ }
                    }
                    if !fin { /* fragmentation not supported in stub */ }
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
