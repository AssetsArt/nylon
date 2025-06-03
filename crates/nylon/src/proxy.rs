use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_plugin::{MiddlewareContext, run_middleware, try_request_filter, try_response_filter};
use nylon_sdk::fbs::dispatcher_generated::nylon_dispatcher::root_as_nylon_dispatcher;
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};

/// Helper function to handle error responses consistently
async fn handle_error_response<'a>(
    res: &'a mut Response<'a>,
    session: &'a mut Session,
    ctx: &'a mut NylonContext,
    error: impl Into<NylonError>,
) -> pingora::Result<bool> {
    let error = error.into();
    res.status(error.http_status())
        .body_json(error.exception_json())?
        .send(session, ctx)
        .await
}

#[async_trait]
impl ProxyHttp for NylonRuntime {
    type CTX = NylonContext;

    fn new_ctx(&self) -> Self::CTX {
        NylonContext::new()
    }

    /// Handles incoming HTTP requests and applies middleware filters
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let mut res = Response::new(self).await?;

        // Parse request and handle errors
        if let Err(e) = ctx.parse_request(session).await {
            return handle_error_response(&mut res, session, ctx, e).await;
        }

        // Find matching route
        let (route, params) = match nylon_store::routes::find_route(session) {
            Ok(route) => route,
            Err(e) => return handle_error_response(&mut res, session, ctx, e).await,
        };

        ctx.route = Some(route.clone());

        // Process middleware
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
                        .map(|name| try_request_filter(name).is_some())
                        .unwrap_or(false)
            });

        for middleware in middleware_items {
            match run_middleware(
                &MiddlewareContext {
                    middleware: middleware.0.clone(),
                    payload: middleware.0.payload.clone(),
                    payload_ast: middleware.1.clone(),
                    params: Some(params.clone()),
                },
                ctx,
                session,
                None,
            )
            .await
            {
                Ok((http_end, dispatcher)) if http_end => {
                    return res
                        .dispatcher_to_response(&dispatcher)?
                        .send(session, ctx)
                        .await;
                }
                Ok(_) => continue,
                Err(e) => return handle_error_response(&mut res, session, ctx, e).await,
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            let http_context =
                match nylon_sdk::proxy_http::build_http_context(session, Some(params.clone()), ctx)
                    .await
                {
                    Ok(context) => context,
                    Err(e) => return handle_error_response(&mut res, session, ctx, e).await,
                };

            match nylon_plugin::dispatcher::http_service_dispatch(ctx, None, None, &http_context)
                .await
            {
                Ok(buf) => {
                    let dispatcher = root_as_nylon_dispatcher(&buf).map_err(|e| {
                        pingora::Error::because(
                            ErrorType::InternalError,
                            "[request_filter]",
                            e.to_string(),
                        )
                    })?;
                    return res
                        .dispatcher_to_response(dispatcher.data().bytes())?
                        .send(session, ctx)
                        .await;
                }
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        e.to_string(),
                    ));
                }
            }
        }

        // Handle regular service type
        let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
            Ok(backend) => backend,
            Err(e) => return handle_error_response(&mut res, session, ctx, e).await,
        };

        ctx.backend = match backend::selection(&http_service, session, ctx) {
            Ok(b) => b,
            Err(e) => return handle_error_response(&mut res, session, ctx, e).await,
        };

        Ok(false)
    }

    /// Selects the upstream peer for the request
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
    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let Some(route) = ctx.route.clone() else {
            return Ok(());
        };

        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| {
                m.0.response_filter.is_some()
                    || m.0
                        .plugin
                        .as_ref()
                        .map(|name| try_response_filter(name).is_some())
                        .unwrap_or(false)
            });

        for middleware in middleware_items {
            if let Err(e) = run_middleware(
                &MiddlewareContext {
                    middleware: middleware.0.clone(),
                    payload: middleware.0.payload.clone(),
                    payload_ast: middleware.1.clone(),
                    params: None,
                },
                ctx,
                session,
                Some(upstream_response),
            )
            .await
            {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[response_filter]",
                    e.to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Called when connected to upstream server
    async fn connected_to_upstream(
        &self,
        _session: &mut Session,
        _reused: bool,
        _peer: &HttpPeer,
        #[cfg(unix)] _fd: std::os::unix::io::RawFd,
        #[cfg(windows)] _sock: std::os::windows::io::RawSocket,
        _digest: Option<&pingora::protocols::Digest>,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        Ok(())
    }

    /// Handles request logging
    async fn logging(
        &self,
        _session: &mut Session,
        _e: Option<&pingora::Error>,
        _ctx: &mut Self::CTX,
    ) where
        Self::CTX: Send + Sync,
    {
        // Logging implementation
    }
}
