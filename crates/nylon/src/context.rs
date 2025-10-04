use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use pingora::proxy::Session;
use std::sync::atomic::Ordering;

#[async_trait]
pub trait NylonContextExt {
    async fn parse_request(&self, session: &mut Session) -> Result<(), NylonError>;
}

#[async_trait]
impl NylonContextExt for NylonContext {
    async fn parse_request(&self, session: &mut Session) -> Result<(), NylonError> {
        {
            let mut client_ip = self
                .client_ip
                .write()
                .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
            *client_ip = match session.client_addr() {
                Some(ip) => match ip.as_inet() {
                    Some(ip) => ip.ip().to_string(),
                    None => {
                        return Err(NylonError::HttpException(
                            400,
                            "BAD_REQUEST",
                            "Unable to get client IP",
                        ));
                    }
                },
                None => {
                    return Err(NylonError::HttpException(
                        400,
                        "BAD_REQUEST",
                        "Unable to get client IP",
                    ));
                }
            };
        }
        let is_tls = match session.digest() {
            Some(d) => d.ssl_digest.is_some(),
            None => false,
        };
        self.tls.store(is_tls, Ordering::Relaxed);
        // reset per-request caches
        {
            if let Ok(mut q) = self.cached_query.write() {
                *q = None;
            }
            if let Ok(mut c) = self.cached_cookies.write() {
                *c = None;
            }
        }
        match session.as_http2() {
            Some(session) => {
                let host = session.req_header().uri.host().unwrap_or("");
                let mut h = self
                    .host
                    .write()
                    .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
                *h = host.to_string();
            }
            None => {
                let host = match session.req_header().headers.get("Host") {
                    Some(h) => h.to_str().unwrap_or("").split(':').next().unwrap_or(""),
                    None => "",
                };
                let mut h = self
                    .host
                    .write()
                    .map_err(|_| NylonError::InternalServerError("lock poisoned".into()))?;
                *h = host.to_string();
            }
        }
        Ok(())
    }
}
