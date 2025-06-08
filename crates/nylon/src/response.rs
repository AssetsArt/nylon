use crate::runtime::NylonRuntime;
use bytes::Bytes;
use nylon_sdk::fbs::http_context_generated::nylon_http_context::root_as_nylon_http_context;
use nylon_types::context::NylonContext;
use pingora::{
    ErrorType,
    protocols::http::HttpTask,
    proxy::{ProxyHttp, Session},
};
use serde_json::Value;

pub struct Response<'a> {
    pub body: Option<Bytes>,
    pub proxy: &'a NylonRuntime,
    pub ctx: &'a mut NylonContext,
}

impl<'a> Response<'a> {
    pub async fn new(proxy: &'a NylonRuntime, ctx: &'a mut NylonContext) -> pingora::Result<Self> {
        Ok(Self {
            body: None,
            proxy,
            ctx,
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
        let _ = self.ctx.response_header.set_status(status);
        self
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        let _ = self.ctx.response_header.remove_header(key);
        if let Err(e) = self
            .ctx
            .response_header
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

    pub async fn send(&mut self, session: &mut Session) -> pingora::Result<bool> {
        let mut headers = self.ctx.response_header.clone();
        self.proxy
            .response_filter(session, &mut headers, self.ctx)
            .await?;
        let mut tasks = vec![HttpTask::Header(Box::new(headers), false)];
        let _ = self
            .proxy
            .response_body_filter(session, &mut self.body, false, self.ctx)
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

    pub async fn dispatcher_to_response(
        &mut self,
        session: &mut Session,
        dispatcher: &[u8],
        end: bool,
    ) -> pingora::Result<&mut Self> {
        let http_ctx = match root_as_nylon_http_context(dispatcher) {
            Ok(d) => d,
            Err(e) => {
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[Response]".to_string(),
                    e.to_string(),
                ));
            }
        };

        // set request headers
        let request = http_ctx.request();
        let headers = request.headers();
        for h in headers.iter().flatten() {
            let _ = session.req_header_mut().remove_header(h.key());
            let _ = session
                .req_header_mut()
                .append_header(h.key().to_string(), h.value().to_string());
        }

        // set response status and headers
        for h in self.ctx.response_header.headers.clone() {
            if let Some(key) = h.0 {
                let _ = self.ctx.response_header.remove_header(key.as_str());
            }
        }
        let status = http_ctx.response().status() as u16;
        self.status(status);
        let headers = http_ctx.response().headers();
        for h in headers.iter().flatten() {
            self.header(h.key(), h.value());
        }

        // set response body
        if end {
            let body = http_ctx
                .response()
                .body()
                .unwrap_or_default()
                .bytes()
                .to_vec();
            self.body(Bytes::from(body));
        }
        Ok(self)
    }
}
