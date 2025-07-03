//! HTTP Proxy Implementation
//!
//! This module contains the core HTTP proxy functionality for Nylon,
//! implementing the Pingora ProxyHttp trait to handle incoming requests,
//! apply middleware filters, and route traffic to upstream services.

use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{plugin_manager::PluginManager, run_middleware, types::MiddlewareContext};
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Helper function to handle error responses consistently
///
/// # Arguments
///
/// * `res` - The response object to modify
/// * `session` - The HTTP session
/// * `error` - The error to convert to an HTTP response
///
/// # Returns
///
/// * `pingora::Result<bool>` - Whether the request should end
async fn handle_error_response<'a>(
    res: &'a mut Response<'a>,
    session: &'a mut Session,
    error: impl Into<NylonError>,
) -> pingora::Result<bool> {
    let error = error.into();
    error!("Request error: {}", error);

    res.status(error.http_status())
        .body_json(error.exception_json())?
        .send(session)
        .await
}

/// Helper function to process TLS redirects
///
/// # Arguments
///
/// * `host` - The request host
/// * `tls` - Whether the request is already using TLS
///
/// # Returns
///
/// * `Option<String>` - The redirect URL if a redirect is needed
fn process_tls_redirect(host: &str, tls: bool) -> Option<String> {
    if tls {
        return None;
    }

    match nylon_store::routes::get_tls_route(host) {
        Ok(Some(redirect)) => {
            let redirect = redirect
                .replace("${host}", host)
                .replace("http://", "")
                .replace("https://", "");

            Some(format!("https://{}", redirect))
        }
        Ok(None) => None,
        Err(e) => {
            warn!("Failed to get TLS route for host {}: {}", host, e);
            None
        }
    }
}

/// Helper function to process middleware for a route
///
/// # Arguments
///
/// * `route` - The matched route
/// * `params` - Route parameters
/// * `ctx` - The request context
/// * `session` - The HTTP session
///
/// # Returns
///
/// * `pingora::Result<bool>` - Whether the request should end
async fn process_middleware(
    route: &nylon_types::context::Route,
    params: &std::collections::HashMap<String, String>,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> pingora::Result<bool> {
    // Collect all middleware items from route and path levels
    let middleware_items = route
        .route_middleware
        .iter()
        .flatten()
        .chain(route.path_middleware.iter().flatten())
        .filter(|m| {
            m.0.request_filter.is_some()
                || m.0
                    .plugin
                    .as_ref()
                    .map(|name| PluginManager::is_request_filter(name))
                    .unwrap_or(false)
        });

    // Process each middleware item
    for middleware in middleware_items {
        // debug!("Processing middleware: {:?}", middleware.0.plugin);

        match run_middleware(
            &MiddlewareContext {
                middleware: middleware.0.clone(),
                payload: middleware.0.payload.clone(),
                payload_ast: middleware.1.clone(),
                params: Some(params.clone()),
            },
            ctx,
            session,
        )
        .await
        {
            Ok((http_end, _)) if http_end => {
                info!("Middleware ended HTTP request");
                return Ok(true);
            }
            Ok((_, stream_end)) if stream_end => {
                info!("Middleware ended stream");
                return Ok(true);
            }
            Ok(_) => {
                // debug!("Middleware completed, continuing");
                continue;
            }
            Err(e) => {
                error!("Middleware error: {}", e);
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[middleware]",
                    e,
                ));
            }
        }
    }

    Ok(false)
}

#[async_trait]
impl ProxyHttp for NylonRuntime {
    type CTX = NylonContext;

    fn new_ctx(&self) -> Self::CTX {
        NylonContext::new()
    }

    /// Handles incoming HTTP requests and applies middleware filters
    ///
    /// This method is called for each incoming HTTP request and performs:
    /// 1. Request parsing and validation
    /// 2. Route matching
    /// 3. TLS redirect processing
    /// 4. Middleware execution
    /// 5. Backend selection
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let mut res = Response::new(self, ctx).await?;

        // Parse request and handle errors
        if let Err(e) = res.ctx.parse_request(session).await {
            return handle_error_response(&mut res, session, e).await;
        }

        // Find matching route
        let (route, params) = match nylon_store::routes::find_route(session) {
            Ok(route) => route,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        // Handle ACME requests (TODO: implement ACME challenge handling)
        // TODO: handle acme request

        // Check for TLS redirect
        if let Some(redirect_url) = process_tls_redirect(&res.ctx.host, res.ctx.tls) {
            info!("Redirecting to TLS: {}", redirect_url);
            res.redirect(redirect_url);
            return res.send(session).await;
        }

        // Store route and params in context
        res.ctx.route = Some(route.clone());
        res.ctx.params = Some(params.clone());

        // Process middleware
        match process_middleware(&route, &params, res.ctx, session).await {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(e) => {
                let nylon_error = NylonError::InternalServerError(e.to_string());
                return handle_error_response(&mut res, session, nylon_error).await;
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            // TODO: implement plugin service type handling
            warn!("Plugin service type not yet implemented");
        }

        // Handle regular service type
        let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
            Ok(backend) => backend,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        // Get backend selection
        let selected_backend = match backend::selection(&http_service, session, res.ctx) {
            Ok(b) => b,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        res.ctx.backend = selected_backend;

        Ok(false)
    }

    /// Selects the upstream peer for the request
    ///
    /// # Arguments
    ///
    /// * `_session` - The HTTP session (unused)
    /// * `ctx` - The request context containing backend information
    ///
    /// # Returns
    ///
    /// * `pingora::Result<Box<HttpPeer>>` - The selected upstream peer
    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let peer = ctx.backend.ext.get::<HttpPeer>().ok_or_else(|| {
            pingora::Error::because(
                ErrorType::InternalError,
                "[upstream_peer]",
                NylonError::ConfigError(format!("[backend:{}] no peer found", ctx.backend.addr)),
            )
        })?;
        Ok(Box::new(peer.clone()))
    }

    /// Processes response filters for the request
    ///
    /// This method modifies the upstream response headers based on
    /// the context modifications made by middleware.
    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // Add response headers
        for (key, value) in ctx.add_response_header.iter() {
            let _ = upstream_response.append_header(key.to_ascii_lowercase(), value);
        }

        // Remove response headers
        for key in ctx.remove_response_header.iter() {
            let key = key.to_ascii_lowercase();
            let _ = upstream_response.remove_header(&key);
        }

        // Set response status if modified
        upstream_response.set_status(ctx.set_response_status)?;

        Ok(())
    }

    /// Processes response body filters
    ///
    /// This method modifies the response body based on context modifications.
    fn response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        if !ctx.set_response_body.is_empty() {
            if let Some(old_body) = body {
                let mut rs_body = old_body.to_vec();
                rs_body.extend_from_slice(&ctx.set_response_body);
                ctx.set_response_body.clear();
                *body = Some(Bytes::from(rs_body));
            } else {
                let rs_body = Bytes::from(ctx.set_response_body.to_vec());
                ctx.set_response_body.clear();
                *body = Some(rs_body);
            }
        }
        Ok(None)
    }
}
