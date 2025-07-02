use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{MiddlewareContext, run_middleware, try_request_filter};
use nylon_types::{context::NylonContext, services::ServiceType};
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

        // acme request
        // todo: handle acme request

        // check if tls route
        if let Ok(Some(redirect)) = nylon_store::routes::get_tls_route(&res.ctx.host) {
            if !res.ctx.tls {
                // todo: handle tls route
                // println!("tls route redirect: {}", redirect);
                // ${host}
                let redirect = redirect.replace("${host}", &res.ctx.host);
                let redirect = redirect.replace("http://", "");
                let redirect = redirect.replace("https://", "");

                res.redirect(format!("https://{}", redirect));
                return res.send(session).await;
            }
        }

        // clone route and params
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

        // let session_stream = stream::SessionStream::new(plugin.clone());
        for middleware in middleware_items {
            match run_middleware(
                &MiddlewareContext {
                    middleware: middleware.0.clone(),
                    payload: middleware.0.payload.clone(),
                    payload_ast: middleware.1.clone(),
                    params: Some(params.clone()),
                },
                res.ctx,
                session,
            )
            .await
            {
                Ok((http_end, _)) if http_end => {
                    return res.send(session).await;
                }
                Ok((_, stream_end)) if stream_end => {
                    return Ok(true);
                }
                Ok(_) => continue,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            // todo: handle plugin service type
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
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        for (key, value) in ctx.add_response_header.iter() {
            let _ = upstream_response.append_header(key.to_ascii_lowercase(), value);
        }
        for key in ctx.remove_response_header.iter() {
            let key = key.to_ascii_lowercase();
            let _ = upstream_response.remove_header(&key);
        }
        upstream_response.set_status(ctx.set_response_status)?;
        Ok(())
    }

    /// Similar to [Self::response_filter()] but for response body chunks
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
