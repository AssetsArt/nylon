use crate::runtime::NylonRuntime;
use bytes::Bytes;
use nylon_types::context::NylonContext;
use pingora::{
    ErrorType,
    http::ResponseHeader,
    protocols::http::HttpTask,
    proxy::{ProxyHttp, Session},
};
use serde_json::Value;

pub struct Response<'a> {
    pub headers: ResponseHeader,
    pub body: Option<Bytes>,
    pub proxy: &'a NylonRuntime,
}

impl<'a> Response<'a> {
    pub async fn new(proxy: &'a NylonRuntime) -> pingora::Result<Self> {
        Ok(Self {
            headers: match ResponseHeader::build(200, None) {
                Ok(h) => h,
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[Response]".to_string(),
                        e.to_string(),
                    ));
                }
            },
            body: None,
            proxy,
        })
    }

    pub fn _redirect_https(
        &mut self,
        host: String,
        path: String,
        port: Option<String>,
    ) -> &mut Self {
        self.status(301);
        let port_str = port.unwrap_or_default();
        let location = format!("https://{}{}{}", host, port_str, path);
        self.header("Location", &location);
        self.header("Content-Length", "0");
        self
    }

    pub fn status(&mut self, status: u16) -> &mut Self {
        let _ = self.headers.set_status(status);
        self
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        if let Err(e) = self
            .headers
            .append_header(key.to_string(), value.to_string())
        {
            tracing::error!("Error adding header: {:?}", e);
        }
        self
    }

    pub fn body(&mut self, body: Bytes) -> &mut Self {
        let body_len = body.len();
        self.body = Some(body);
        self.header("Content-Length", &body_len.to_string());
        self
    }

    pub fn body_json(&mut self, body: Value) -> pingora::Result<&mut Self> {
        let body_bytes = match serde_json::to_vec(&body) {
            Ok(b) => b,
            Err(e) => {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[Response]".to_string(),
                    e.to_string(),
                ));
            }
        };
        self.body(Bytes::from(body_bytes));
        self.header("Content-Type", "application/json");
        Ok(self)
    }

    pub async fn send(
        &mut self,
        session: &mut Session,
        ctx: &mut NylonContext,
    ) -> pingora::Result<bool> {
        self.proxy
            .response_filter(session, &mut self.headers, ctx)
            .await?;
        let mut tasks = vec![HttpTask::Header(Box::new(self.headers.clone()), false)];
        let _ = self
            .proxy
            .response_body_filter(session, &mut self.body, false, ctx)
            .is_ok();
        if let Some(body) = self.body.clone() {
            tasks.push(HttpTask::Body(Some(body), false));
        } else {
            tasks.push(HttpTask::Body(None, false));
        }
        tasks.push(HttpTask::Done);

        if let Err(e) = session.response_duplex_vec(tasks).await {
            tracing::error!("Error sending response: {:?}", e);
            return Err(pingora::Error::because(
                ErrorType::InternalError,
                "[Response]".to_string(),
                e.to_string(),
            ));
        }
        Ok(true)
    }
}
