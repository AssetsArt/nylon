use nylon_error::NylonError;
use pingora::{lb::Backend, proxy::Session};

pub struct NylonContext {
    pub backend: Backend,
    pub client_ip: String,
}

impl NylonContext {
    pub fn new() -> Self {
        Self {
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: String::new(),
        }
    }
}

impl NylonContext {
    pub async fn parse_request(&mut self, session: &mut Session) -> Result<(), NylonError> {
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
        Ok(())
    }
}
