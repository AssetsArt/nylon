use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
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
        Ok(false)
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
