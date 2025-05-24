use nylon_error::NylonError;
use nylon_types::{context::NylonContext, template::Expr};
use pingora::proxy::Session;
use serde_json::Value;
use std::collections::HashMap;

mod native;

enum BuiltinPlugin {
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

pub fn run_middleware(
    plugin_name: &str,
    payload: &Option<Value>,
    payload_ast: &Option<HashMap<String, Vec<Expr>>>,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> Result<(), NylonError> {
    match try_builtin(plugin_name) {
        Some(BuiltinPlugin::RequestHeaderModifier) => {
            tracing::debug!("Running request header modifier plugin: {}", plugin_name);
            tracing::debug!("Payload: {:#?}", payload);
            native::header_modifier::request(ctx, session, payload, payload_ast)?;
        }
        Some(BuiltinPlugin::ResponseHeaderModifier) => {
            todo!("response header modifier");
        }
        _ => {
            // fallback ไป external plugin (WASM, FFI)
            todo!("external plugin");
        }
    }
    Ok(())
}
