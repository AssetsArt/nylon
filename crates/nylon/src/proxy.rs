use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_plugin::{run_middleware, try_request_filter, try_response_filter};
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};

#[async_trait]
impl ProxyHttp for NylonRuntime {
    type CTX = NylonContext;

    fn new_ctx(&self) -> Self::CTX {
        NylonContext::new()
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let mut res = Response::new(self).await?;
        if let Err(e) = ctx.parse_request(session).await {
            return res
                .status(e.http_status())
                .body_json(e.exception_json())?
                .send(session, ctx)
                .await;
        }
        let (route, params) = match nylon_store::routes::find_route(session) {
            Ok(route) => route,
            Err(e) => {
                return res
                    .status(e.http_status())
                    .body_json(e.exception_json())?
                    .send(session, ctx)
                    .await;
            }
        };

        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| {
                if let Some(name) = m.0.plugin.as_ref() {
                    return try_request_filter(name).is_some();
                } else if m.0.request_filter.is_some() {
                    return true;
                }
                false
            });
        for middleware in middleware_items {
            let plugin_name = match &middleware.0.plugin {
                Some(name) => name,
                None => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        NylonError::ConfigError(format!(
                            "Middleware plugin not found: {:?}",
                            middleware.0.plugin
                        )),
                    ));
                }
            };
            match run_middleware(
                plugin_name,
                &middleware.0.payload,
                &middleware.1,
                ctx,
                session,
                None,
                None,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    return res
                        .status(e.http_status())
                        .body_json(e.exception_json())?
                        .send(session, ctx)
                        .await;
                }
            }
        }

        if route.service.service_type == ServiceType::Plugin {
            let http_context =
                match nylon_sdk::proxy_http::build_http_context(session, &params, ctx).await {
                    Ok(context) => context,
                    Err(e) => {
                        return res
                            .status(e.http_status())
                            .body_json(e.exception_json())?
                            .send(session, ctx)
                            .await;
                    }
                };
            ctx.route = Some(route);
            match nylon_plugin::dispatcher::http_service_dispatch(ctx, &http_context).await {
                Ok(buf) => {
                    return res.dispatcher_to_response(&buf)?.send(session, ctx).await;
                }
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        e.to_string(),
                    ));
                }
            }
            // return Ok(true);
        } else {
            let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
                Ok(backend) => backend,
                Err(e) => {
                    return res
                        .status(e.http_status())
                        .body_json(e.exception_json())?
                        .send(session, ctx)
                        .await;
                }
            };
            ctx.backend = match backend::selection(&http_service, session, ctx) {
                Ok(b) => b,
                Err(e) => {
                    return res
                        .status(e.http_status())
                        .body_json(e.exception_json())?
                        .send(session, ctx)
                        .await;
                }
            };
            // next phase will be handled by upstream_peer
            ctx.route = Some(route);
            Ok(false)
        }
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let peer = match ctx.backend.ext.get::<HttpPeer>() {
            Some(p) => p.clone(),
            None => {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[upstream_peer]",
                    NylonError::ConfigError(format!(
                        "[backend:{}] no peer found",
                        ctx.backend.addr
                    )),
                ));
            }
        };
        Ok(Box::new(peer))
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
        let Some(route) = ctx.route.clone() else {
            return Ok(());
        };
        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| {
                if let Some(name) = m.0.plugin.as_ref() {
                    return try_response_filter(name).is_some();
                } else if m.0.response_filter.is_some() {
                    return true;
                }
                false
            });

        for middleware in middleware_items {
            let plugin_name = match &middleware.0.plugin {
                Some(name) => name,
                None => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        NylonError::ConfigError(format!(
                            "Middleware plugin not found: {:?}",
                            middleware.0.plugin
                        )),
                    ));
                }
            };
            match run_middleware(
                plugin_name,
                &middleware.0.payload,
                &middleware.1,
                ctx,
                session,
                Some(upstream_response),
                None,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[response_filter]",
                        e.to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

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
        // println!("connected_to_upstream");
        // println!("reused: {}", _reused);
        // println!("peer: {:#?}", _peer);
        // println!("fd: {}", _fd);
        // println!("digest: {:#?}", _digest);
        Ok(())
    }

    async fn logging(
        &self,
        _session: &mut Session,
        _e: Option<&pingora::Error>,
        _ctx: &mut Self::CTX,
    ) where
        Self::CTX: Send + Sync,
    {
        // println!("logging");
        // println!("e: {:#?}", _e);
    }
}
