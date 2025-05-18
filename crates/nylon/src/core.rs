use crate::{context::NylonContext, runtime::NylonRuntime};
use async_trait::async_trait;
use nylon_error::NylonError;
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
