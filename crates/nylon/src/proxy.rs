use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{run_middleware, try_request_filter, try_response_filter};
use nylon_sdk::fbs::{
    dispatcher_generated::nylon_dispatcher::{NylonDispatcher, NylonDispatcherArgs},
    http_context_generated::nylon_http_context::{
        NylonHttpContext, NylonHttpContextArgs, NylonHttpRequest, NylonHttpRequestArgs,
        NylonHttpResponse, NylonHttpResponseArgs,
    },
};
use nylon_types::{context::NylonContext, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
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

        // build http context
        let mut fbs = flatbuffers::FlatBufferBuilder::new();
        let request = &NylonHttpRequestArgs {
            method: Some(fbs.create_string("GET")),
            path: Some(fbs.create_string("/")),
            query: None,
            headers: None,
            body: None,
        };
        let req_offset = NylonHttpRequest::create(&mut fbs, request);
        let response = &NylonHttpResponseArgs {
            status: 200,
            headers: None,
            body: None,
        };
        let resp_offset = NylonHttpResponse::create(&mut fbs, response);
        let dispatcher_args = &NylonHttpContextArgs {
            request: Some(req_offset),
            response: Some(resp_offset),
        };
        let dispatcher = NylonHttpContext::create(&mut fbs, dispatcher_args);
        fbs.finish(dispatcher, None);
        let dispatcher_data = fbs.finished_data();

        // build ctx dispatcher
        let mut fbs = flatbuffers::FlatBufferBuilder::new();
        let request_id = fbs.create_string(&ctx.request_id);
        let data_vec = fbs.create_vector(dispatcher_data);
        let dispatcher = NylonDispatcher::create(
            &mut fbs,
            &NylonDispatcherArgs {
                http_end: false,
                request_id: Some(request_id),
                name: None,
                entry: None,
                data: Some(data_vec),
            },
        );
        fbs.finish(dispatcher, None);
        let ctx_dispatcher = fbs.finished_data();
        let current_buf = ctx_dispatcher.to_vec();

        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| {
                if let Some(name) = m.0.plugin.as_ref() {
                    return try_request_filter(name).is_some();
                } else if m.0.request_filter.is_some() {
                    return true;
                }
                false
            });
        for middleware in middleware_items {
            let plugin_name = match &middleware.0.plugin {
                Some(name) => name,
                None => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        NylonError::ConfigError(format!(
                            "Middleware plugin not found: {:?}",
                            middleware.0.plugin
                        )),
                    ));
                }
            };
            match run_middleware(
                plugin_name,
                &middleware.0.payload,
                &middleware.1,
                ctx,
                session,
                None,
                None,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    return res
                        .status(e.http_status())
                        .body_json(e.exception_json())?
                        .send(session, ctx)
                        .await;
                }
            }
        }

        if route.service.service_type == ServiceType::Plugin {
            ctx.route = Some(route);
            match nylon_plugin::dispatcher::http_dispatch(ctx, &current_buf).await {
                Ok(buf) => {
                    use nylon_sdk::fbs::dispatcher_generated::nylon_dispatcher::root_as_nylon_dispatcher;
                    use nylon_sdk::fbs::http_context_generated::nylon_http_context::root_as_nylon_http_context;
                    let dispatcher =
                        root_as_nylon_dispatcher(buf.as_slice()).expect("invalid dispatcher");

                    println!("http end: {}", dispatcher.http_end());

                    let http_ctx = root_as_nylon_http_context(dispatcher.data().bytes())
                        .expect("invalid http context");
                    let body = http_ctx.response().body().unwrap().bytes().to_vec();
                    let b = Bytes::from(body);
                    return res
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(b)
                        .send(session, ctx)
                        .await;
                }
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        e.to_string(),
                    ));
                }
            }
            // return Ok(true);
        } else {
            let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
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
            // next phase will be handled by upstream_peer
            ctx.route = Some(route);
            Ok(false)
        }
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

    async fn response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        let Some(route) = ctx.route.clone() else {
            return Ok(());
        };
        let middleware_items = route
            .route_middleware
            .iter()
            .flatten()
            .chain(route.path_middleware.iter().flatten())
            .filter(|m| {
                if let Some(name) = m.0.plugin.as_ref() {
                    return try_response_filter(name).is_some();
                } else if m.0.response_filter.is_some() {
                    return true;
                }
                false
            });

        for middleware in middleware_items {
            let plugin_name = match &middleware.0.plugin {
                Some(name) => name,
                None => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[request_filter]",
                        NylonError::ConfigError(format!(
                            "Middleware plugin not found: {:?}",
                            middleware.0.plugin
                        )),
                    ));
                }
            };
            match run_middleware(
                plugin_name,
                &middleware.0.payload,
                &middleware.1,
                ctx,
                session,
                Some(upstream_response),
                None,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    return Err(pingora::Error::because(
                        ErrorType::InternalError,
                        "[response_filter]",
                        e.to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    async fn connected_to_upstream(
        &self,
        _session: &mut Session,
        _reused: bool,
        _peer: &HttpPeer,
        #[cfg(unix)] _fd: std::os::unix::io::RawFd,
        #[cfg(windows)] _sock: std::os::windows::io::RawSocket,
        _digest: Option<&pingora::protocols::Digest>,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<()>
    where
        Self::CTX: Send + Sync,
    {
        // println!("connected_to_upstream");
        // println!("reused: {}", _reused);
        // println!("peer: {:#?}", _peer);
        // println!("fd: {}", _fd);
        // println!("digest: {:#?}", _digest);
        Ok(())
    }

    async fn logging(
        &self,
        _session: &mut Session,
        _e: Option<&pingora::Error>,
        _ctx: &mut Self::CTX,
    ) where
        Self::CTX: Send + Sync,
    {
        // println!("logging");
        // println!("e: {:#?}", _e);
    }
}
