use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{MiddlewareContext, run_middleware, try_request_filter, try_response_filter};
use nylon_sdk::fbs::dispatcher_generated::nylon_dispatcher::root_as_nylon_dispatcher;
use nylon_types::{context::NylonContext, services::ServiceType, template::apply_payload_ast};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};
use std::time::Duration;

/// Helper function to handle error responses consistently
async fn handle_error_response<'a>(
    res: &'a mut Response<'a>,
    session: &'a mut Session,
    error: impl Into<NylonError>,
) -> pingora::Result<bool> {
    let error = error.into();
    res.status(error.http_status())
        .body_json(error.exception_json())?
        .send(session)
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
        let mut res = Response::new(self, ctx).await?;
        // let ctx = res.ctx;

        // Parse request and handle errors
        if let Err(e) = res.ctx.parse_request(session).await {
            return handle_error_response(&mut res, session, e).await;
        }

        // Find matching route
        let (route, params) = match nylon_store::routes::find_route(session) {
            Ok(route) => route,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        res.ctx.route = Some(route.clone());
        res.ctx.params = Some(params.clone());

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
            // println!("middleware: {:#?}", middleware);
            match run_middleware(
                &MiddlewareContext {
                    middleware: middleware.0.clone(),
                    payload: middleware.0.payload.clone(),
                    payload_ast: middleware.1.clone(),
                    params: Some(params.clone()),
                },
                res.ctx,
                session,
            ) {
                Ok((http_end, dispatcher)) if http_end => {
                    return res
                        .dispatcher_to_response(session, &dispatcher, http_end)
                        .await?
                        .send(session)
                        .await;
                }
                Ok((http_end, dispatcher)) if !dispatcher.is_empty() => {
                    res.dispatcher_to_response(session, &dispatcher, http_end)
                        .await?;
                    continue;
                }
                Ok((_, _)) => continue,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            let http_context = match nylon_sdk::proxy_http::build_http_context(
                session,
                res.ctx,
                Some(params.clone()),
            ) {
                Ok(context) => context,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            };
            let headers = session.req_header_mut();
            let payload = match (route.service.plugin, route.payload_ast) {
                (Some(plugin), Some(payload_ast)) => {
                    let Some(payload) = plugin.payload else {
                        return Err(pingora::Error::because(
                            ErrorType::InternalError,
                            "[plugin_service_dispatch]",
                            "Plugin payload not found",
                        ));
                    };
                    let mut payload = payload.clone();
                    apply_payload_ast(&mut payload, &payload_ast, headers, res.ctx);
                    serde_json::to_vec(&payload).ok()
                }
                _ => None,
            };
            match nylon_plugin::dispatcher::http_service_dispatch(
                res.ctx,
                None,
                None,
                &http_context,
                &payload,
            ) {
                Ok(buf) => {
                    let dispatcher = root_as_nylon_dispatcher(&buf).map_err(|e| {
                        pingora::Error::because(
                            ErrorType::InternalError,
                            "[plugin_service_dispatch]",
                            e.to_string(),
                        )
                    })?;
                    res.ctx.plugin_store =
                        Some(dispatcher.store().unwrap_or_default().bytes().to_vec());
                    return res
                        .dispatcher_to_response(session, dispatcher.data().bytes(), true)
                        .await?
                        .send(session)
                        .await;
                }
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[plugin_service_dispatch]",
                        e.to_string(),
                    ));
                }
            }
        }

        // Handle regular service type
        let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
            Ok(backend) => backend,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        res.ctx.backend = match backend::selection(&http_service, session, res.ctx) {
            Ok(b) => b,
            Err(e) => return handle_error_response(&mut res, session, e).await,
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

        // set response header
        let _ = ctx
            .response_header
            .set_status(upstream_response.status.as_u16());
        for h in upstream_response.headers.clone() {
            if let Some(key) = h.0 {
                let _ = ctx.response_header.remove_header(key.as_str());
                let _ = ctx.response_header.append_header(key, h.1);
            }
        }

        // clear all headers in upstream_response
        for h in upstream_response.headers.clone() {
            if let Some(key) = h.0 {
                let _ = upstream_response.remove_header(key.as_str());
            }
        }

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
                    params: ctx.params.clone(),
                },
                ctx,
                session,
            ) {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[response_filter]",
                    e.to_string(),
                ));
            }
        }

        // set response header to upstream_response
        let _ = upstream_response.set_status(ctx.response_header.status.as_u16());
        for h in ctx.response_header.headers.clone() {
            if let Some(key) = h.0 {
                let _ = upstream_response.remove_header(key.as_str());
                let _ = upstream_response.append_header(key, h.1);
            }
        }

        Ok(())
    }

    /// Similar to [Self::response_filter()] but for response body chunks
    fn response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        ctx.response_body = body.clone();
        let Some(route) = ctx.route.clone() else {
            return Ok(None);
        };
        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| m.0.response_body_filter.is_some());

        for middleware in middleware_items {
            if let Err(e) = run_middleware(
                &MiddlewareContext {
                    middleware: middleware.0.clone(),
                    payload: middleware.0.payload.clone(),
                    payload_ast: middleware.1.clone(),
                    params: ctx.params.clone(),
                },
                ctx,
                session,
            ) {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[response_body_filter]",
                    e.to_string(),
                ));
            }
        }
        *body = ctx.response_body.clone();
        Ok(None)
    }

    // Called when connected to upstream server
    // async fn connected_to_upstream(
    //     &self,
    //     _session: &mut Session,
    //     _reused: bool,
    //     _peer: &HttpPeer,
    //     #[cfg(unix)] _fd: std::os::unix::io::RawFd,
    //     #[cfg(windows)] _sock: std::os::windows::io::RawSocket,
    //     _digest: Option<&pingora::protocols::Digest>,
    //     _ctx: &mut Self::CTX,
    // ) -> pingora::Result<()>
    // where
    //     Self::CTX: Send + Sync,
    // {
    //     Ok(())
    // }

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
