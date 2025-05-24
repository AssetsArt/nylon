use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_plugin::run_middleware;
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
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
        let (route, _) = match nylon_store::routes::find_route(session) {
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
            .chain(route.path_middleware.iter().flatten());
        tracing::debug!("Request filter: {:#?}", middleware_items);
        for middleware in middleware_items {
            let plugin_name = match &middleware.plugin {
                Some(name) => name,
                None => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        NylonError::ConfigError(format!(
                            "Middleware plugin not found: {:?}",
                            middleware.plugin
                        )),
                    ));
                }
            };
            match run_middleware(plugin_name, &middleware.payload, ctx, session) {
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

        if route.service_type == ServiceType::Plugin {
            return Ok(true);
        } else {
            let http_service = match nylon_store::lb_backends::get(&route.service).await {
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
}
