use crate::{backend, context::NylonContext, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
use pingora::{
    ErrorType,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};
use serde_json::json;

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
        let mut res = Response::new(self, ctx, session).await?;
        let (route, _) = match nylon_store::routes::find_route(res.session) {
            Ok(route) => route,
            Err(e) => {
                return res
                    .status(500)
                    .body_json(json!({
                        "error": "INTERNAL_ERROR",
                        "message": e.to_string(),
                    }))?
                    .send()
                    .await;
            }
        };
        let http_service = match nylon_store::lb_backends::get(&route.service).await {
            Ok(backend) => backend,
            Err(e) => {
                return res
                    .status(500)
                    .body_json(json!({
                        "error": "CONFIG_ERROR",
                        "message": e.to_string(),
                    }))?
                    .send()
                    .await;
            }
        };
        let ip = match res.session.client_addr() {
            Some(ip) => match ip.as_inet() {
                Some(ip) => ip.ip().to_string(),
                None => {
                    return res
                        .status(400)
                        .body_json(json!({
                            "error": "CLIENT_ERROR",
                            "message": "Unable to get client IP",
                        }))?
                        .send()
                        .await;
                }
            },
            None => {
                return res
                    .status(400)
                    .body_json(json!({
                        "error": "CLIENT_ERROR",
                        "message": "Unable to get client IP",
                    }))?
                    .send()
                    .await;
            }
        };
        let mut selection_key = ip.to_string();
        if let Some(header_value) = res.session.req_header().headers.get("x-forwarded-for") {
            let value = header_value.to_str().unwrap_or_default();
            selection_key.push_str(value);
        }
        ctx.backend = match backend::selection(selection_key.as_str(), &http_service) {
            Ok(b) => b,
            Err(e) => {
                return res
                    .status(500)
                    .body_json(json!({
                        "error": "CONFIG_ERROR",
                        "message": e.to_string(),
                    }))?
                    .send()
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
