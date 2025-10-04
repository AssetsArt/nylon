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

    pub fn redirect(&mut self, redirect: String) -> &mut Self {
        self.status(301);
        {
            let mut headers = self.ctx.add_response_header.write().expect("lock");
            headers.insert("Location".to_string(), redirect);
            headers.insert("Content-Length".to_string(), "0".to_string());
        }
        self
    }

    pub fn status(&mut self, status: u16) -> &mut Self {
        self.ctx.set_response_status.store(status, std::sync::atomic::Ordering::Relaxed);
        self
    }

    pub fn body(&mut self, body: Bytes) -> &mut Self {
        let body_len = body.len();
        self.body = Some(body);
        {
            let mut headers = self.ctx.add_response_header.write().expect("lock");
            headers.insert("Content-Length".to_string(), body_len.to_string());
        }
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
        // self.header("Content-Type", "application/json");
        Ok(self)
    }

    pub async fn send(&mut self, session: &mut Session) -> pingora::Result<bool> {
        let status = self.ctx.set_response_status.load(std::sync::atomic::Ordering::Relaxed);
        let mut headers = ResponseHeader::build(status, None)?;
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
        // println!("tasks: {:?}", tasks);
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
