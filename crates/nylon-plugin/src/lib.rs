use nylon_error::NylonError;
use nylon_sdk::fbs::{
    dispatcher_generated::nylon_dispatcher::root_as_nylon_dispatcher,
    http_context_generated::nylon_http_context::root_as_nylon_http_context,
};
use nylon_types::{context::NylonContext, route::MiddlewareItem, template::Expr};
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

pub struct MiddlewareContext {
    pub middleware: MiddlewareItem,
    pub payload: Option<Value>,
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
    pub params: Option<HashMap<String, String>>,
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
    middleware_context: &MiddlewareContext,
    ctx: &mut NylonContext,
    session: &mut Session,
    upstream_response: Option<&mut ResponseHeader>,
) -> Result<(bool, Vec<u8>), NylonError> {
    let (middleware, payload, payload_ast, params) = (
        &middleware_context.middleware,
        &middleware_context.payload,
        &middleware_context.payload_ast,
        &middleware_context.params,
    );
    let Some(plugin_name) = &middleware.plugin else {
        return Ok((false, vec![]));
    };
    match try_builtin(plugin_name.as_str()) {
        Some(BuiltinPlugin::RequestHeaderModifier) => {
            native::header_modifier::request(ctx, session, payload, payload_ast)?;
            Ok((false, vec![]))
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
            Ok((false, vec![]))
        }
        _ => {
            if let Some(request_filter) = &middleware.request_filter {
                let http_context =
                    nylon_sdk::proxy_http::build_http_context(session, params.clone(), ctx).await?;
                let dispatcher = dispatcher::http_service_dispatch(
                    ctx,
                    Some(plugin_name.as_str()),
                    Some(request_filter),
                    &http_context,
                )
                .await?;
                let dispatcher = root_as_nylon_dispatcher(&dispatcher)
                    .map_err(|e| NylonError::ConfigError(format!("Invalid dispatcher: {}", e)))?;
                let http_end = dispatcher.http_end();
                return Ok((http_end, dispatcher.data().bytes().to_vec()));
            } else if let (Some(response_filter), Some(upstream_response)) =
                (&middleware.response_filter, upstream_response)
            {
                let http_context =
                    nylon_sdk::proxy_http::build_http_context(session, params.clone(), ctx).await?;
                let dispatcher = dispatcher::http_service_dispatch(
                    ctx,
                    Some(plugin_name.as_str()),
                    Some(response_filter),
                    &http_context,
                )
                .await?;
                let dispatcher = root_as_nylon_dispatcher(&dispatcher)
                    .map_err(|e| NylonError::ConfigError(format!("Invalid dispatcher: {}", e)))?;
                let http_ctx = match root_as_nylon_http_context(dispatcher.data().bytes()) {
                    Ok(d) => d,
                    Err(e) => {
                        return Err(NylonError::ConfigError(format!(
                            "Invalid http context: {}",
                            e
                        )));
                    }
                };
                let response = http_ctx.response();
                let headers = response.headers();
                for h in headers.iter().flatten() {
                    // println!("header: {:?}, {:?}", h.key(), h.value());
                    let _ =
                        upstream_response.append_header(h.key().to_string(), h.value().to_string());
                }
            } else if let Some(_response_body_filter) = &middleware.response_body_filter {
                todo!("response_body_filter");
            } else if let Some(_logging) = &middleware.logging {
                todo!("logging");
            }

            Ok((false, vec![]))
        }
    }
}
