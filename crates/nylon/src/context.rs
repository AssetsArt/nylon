use std::collections::HashMap;

use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use pingora::{lb::Backend, proxy::Session};

#[async_trait]
pub trait NylonContextExt {
    fn new() -> Self;
    async fn parse_request(&mut self, session: &mut Session) -> Result<(), NylonError>;
}

#[async_trait]
impl NylonContextExt for NylonContext {
    fn new() -> Self {
        Self {
            headers: HashMap::new(),
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: String::new(),
            request_id: None,
        }
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

        let headers = &session.req_header().headers;
        for (name, value) in headers.iter() {
            if let Ok(value_str) = value.to_str() {
                self.headers.insert(name.to_string(), value_str.to_string());
            }
        }

        Ok(())
    }
}
