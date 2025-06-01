use nylon_error::NylonError;
use nylon_types::{context::NylonContext, template::Expr};
use pingora::{http::ResponseHeader, proxy::Session};
use serde_json::Value;
use std::collections::HashMap;

pub mod dispatcher;
pub mod loaders;
mod native;

pub enum BuiltinPlugin {
    RequestHeaderModifier,
    ResponseHeaderModifier,
}

fn try_builtin(name: &str) -> Option<BuiltinPlugin> {
    tracing::debug!("Trying builtin plugin: {}", name);
    match name {
        "RequestHeaderModifier" => Some(BuiltinPlugin::RequestHeaderModifier),
        "ResponseHeaderModifier" => Some(BuiltinPlugin::ResponseHeaderModifier),
        _ => None,
    }
}

pub fn try_request_filter(name: &str) -> Option<BuiltinPlugin> {
    match name {
        "RequestHeaderModifier" => Some(BuiltinPlugin::RequestHeaderModifier),
        _ => None,
    }
}

pub fn try_response_filter(name: &str) -> Option<BuiltinPlugin> {
    match name {
        "ResponseHeaderModifier" => Some(BuiltinPlugin::ResponseHeaderModifier),
        _ => None,
    }
}

pub async fn run_middleware(
    plugin_name: &str,
    payload: &Option<Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    ctx: &mut NylonContext,
    session: &mut Session,
    upstream_response: Option<&mut ResponseHeader>,
    _dispatcher: Option<&mut [u8]>,
) -> Result<(), NylonError> {
    match try_builtin(plugin_name) {
        Some(BuiltinPlugin::RequestHeaderModifier) => {
            native::header_modifier::request(ctx, session, payload, payload_ast)?;
        }
        Some(BuiltinPlugin::ResponseHeaderModifier) => {
            if let Some(upstream_response) = upstream_response {
                native::header_modifier::response(
                    ctx,
                    session,
                    upstream_response,
                    payload,
                    payload_ast,
                )?;
            }
        }
        _ => {
            // fallback ไป external plugin (WASM, FFI)
            todo!("external plugin");
        }
    }
    Ok(())
}
