use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{run_middleware, stream::PluginSessionStream, types::MiddlewareContext};
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};
use std::time::Duration;
use tracing::{error, info};
use std::sync::atomic::Ordering;

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
        Err(_) => None,
    }
}

async fn process_middleware<T>(
    proxy: &T,
    phase: u8,
    ctx: &mut NylonContext,
    session: &mut Session,
) -> pingora::Result<bool>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
    // Collect all middleware items from route and path levels
    let route_opt = ctx.route.read().map_err(|_| pingora::Error::because(
        ErrorType::InternalError,
        "[middleware]",
        NylonError::InternalServerError("lock poisoned".into()),
    ))?.clone();
    let Some(route) = &route_opt else {
        return Ok(false);
    };
    let path_middleware = &route.path_middleware;
    let middleware_items = route
        .route_middleware
        .iter()
        .flatten()
        .chain(path_middleware.iter().flatten());

    // Process each middleware item
    for middleware in middleware_items.cloned().collect::<Vec<_>>() {
        // debug!("Processing middleware: {:?}", middleware.0.plugin);

        match run_middleware(
            proxy,
            phase,
            &MiddlewareContext {
                middleware: middleware.0.clone(),
                payload: middleware.0.payload.clone(),
                payload_ast: middleware.1.clone(),
            },
            ctx,
            session,
        )
        .await
        {
            Ok((http_end, _)) if http_end => {
                return Ok(true);
            }
            Ok((_, stream_end)) if stream_end => {
                return Ok(true);
            }
            Ok(_) => {
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
        NylonContext::default()
    }

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
        let host_owned = res
            .ctx
            .host
            .read()
            .map_err(|_| pingora::Error::because(ErrorType::InternalError, "[proxy]", "host lock".to_string()))?
            .clone();
        let tls = res.ctx.tls.load(Ordering::Relaxed);
        if let Some(redirect_url) = process_tls_redirect(&host_owned, tls) {
            info!("Redirecting to TLS: {}", redirect_url);
            res.redirect(redirect_url);
            return res.send(session).await;
        }

        // Store route and params in context
        {
            let mut r = res.ctx.route.write().map_err(|_| pingora::Error::because(ErrorType::InternalError, "[proxy]", "route lock".to_string()))?;
            *r = Some(route.clone());
        }
        {
            let mut p = res.ctx.params.write().map_err(|_| pingora::Error::because(ErrorType::InternalError, "[proxy]", "params lock".to_string()))?;
            *p = Some(params.clone());
        }

        // Process middleware
        match process_middleware(self, 1, res.ctx, session).await {
            Ok(true) => return Ok(true),
            Ok(false) => {}
            Err(e) => {
                let nylon_error = NylonError::InternalServerError(e.to_string());
                return handle_error_response(&mut res, session, nylon_error).await;
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            if let Some(plugin) = &route.service.plugin {
                match nylon_plugin::session_stream(
                    self,
                    plugin.name.as_str(),
                    1,
                    plugin.entry.as_str(),
                    res.ctx,
                    session,
                    &plugin.payload,
                    &None,
                )
                .await
                {
                    Ok(_result) => {
                        // Plugin service handled the request lifecycle (HTTP or stream)
                        return Ok(true);
                    }
                    Err(e) => {
                        return handle_error_response(&mut res, session, e).await;
                    }
                }
            } else {
                let err =
                    NylonError::ConfigError("Plugin service missing 'plugin' config".to_string());
                return handle_error_response(&mut res, session, err).await;
            }
        }

        // Handle regular HTTP service type only
        if route.service.service_type == ServiceType::Http {
            let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
                Ok(backend) => backend,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            };

            // Get backend selection
            let selected_backend = match backend::selection(&http_service, session, res.ctx) {
                Ok(b) => b,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            };

            {
                let mut b = res.ctx.backend.write().map_err(|_| pingora::Error::because(ErrorType::InternalError, "[proxy]", "backend lock".to_string()))?;
                *b = selected_backend;
            }
        }

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let backend_guard = ctx
            .backend
            .read()
            .map_err(|_| pingora::Error::because(ErrorType::InternalError, "[upstream_peer]", "backend lock".to_string()))?;
        let peer = backend_guard
            .ext
            .get::<HttpPeer>()
            .ok_or_else(|| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[upstream_peer]",
                    NylonError::ConfigError(format!("[backend] no peer found")),
                )
            })?;
        Ok(Box::new(peer.clone()))
    }

    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // Process middleware
        let _ = process_middleware(self, 2, ctx, session).await;

        // Add response headers
        for (key, value) in ctx
            .add_response_header
            .read()
            .map_err(|_| pingora::Error::because(ErrorType::InternalError, "[response_filter]", "add_header lock".to_string()))?
            .iter()
        {
            let _ = upstream_response.append_header(key.to_ascii_lowercase(), value);
        }

        // Remove response headers
        for key in ctx
            .remove_response_header
            .read()
            .map_err(|_| pingora::Error::because(ErrorType::InternalError, "[response_filter]", "remove_header lock".to_string()))?
            .iter()
        {
            let key = key.to_ascii_lowercase();
            let _ = upstream_response.remove_header(&key);
        }

        // Set response status if modified
        upstream_response.set_status(ctx.set_response_status.load(Ordering::Relaxed))?;

        Ok(())
    }

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
        {
            let mut buf = ctx
                .set_response_body
                .write()
                .map_err(|_| pingora::Error::because(ErrorType::InternalError, "[body_filter]", "set_response_body lock".to_string()))?;
            if !buf.is_empty() {
                if let Some(old_body) = body {
                    let mut rs_body = old_body.to_vec();
                    rs_body.extend_from_slice(&buf);
                    buf.clear();
                    *body = Some(Bytes::from(rs_body));
                } else {
                    let rs_body = Bytes::from(buf.clone());
                    buf.clear();
                    *body = Some(rs_body);
                }
            }
        }
        Ok(None)
    }

    async fn logging(
        &self,
        _session: &mut Session,
        _e: Option<&pingora::Error>,
        ctx: &mut Self::CTX,
    ) where
        Self::CTX: Send + Sync,
    {
        let streams = ctx
            .session_stream
            .read()
            .map(|m| m.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for stream in streams {
            let _ = stream.close().await;
        }
    }
}
