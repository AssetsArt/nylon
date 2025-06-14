use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use pingora::proxy::Session;

#[async_trait]
pub trait NylonContextExt {
    fn new() -> Self;
    async fn parse_request(&mut self, session: &mut Session) -> Result<(), NylonError>;
}

#[async_trait]
impl NylonContextExt for NylonContext {
    fn new() -> Self {
        Self::default()
    }

    async fn parse_request(&mut self, session: &mut Session) -> Result<(), NylonError> {
        self.client_ip = match session.client_addr() {
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
        self.request_body = session.read_request_body().await.unwrap_or_default();
        self.response_body = Some(Bytes::new());
        self.tls = match session.digest() {
            Some(d) => d.ssl_digest.is_some(),
            None => false,
        };

        match session.as_http2() {
            Some(session) => {
                let host = session.req_header().uri.host().unwrap_or("");
                self.host = host.to_string();
            }
            None => {
                let host = match session.req_header().headers.get("Host") {
                    Some(h) => h.to_str().unwrap_or("").split(':').next().unwrap_or(""),
                    None => "",
                };
                self.host = host.to_string();
            }
        }
        Ok(())
    }
}
