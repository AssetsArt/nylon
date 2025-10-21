use crate::{backend, context::NylonContextExt, response::Response, runtime::NylonRuntime};
use async_trait::async_trait;
use bytes::Bytes;
use nylon_error::NylonError;
use nylon_plugin::{
    run_middleware,
    stream::PluginSessionStream,
    types::{MiddlewareContext, PluginResult},
};
use nylon_types::{context::NylonContext, plugins::PluginPhase, services::ServiceType};
use pingora::{
    ErrorType,
    http::ResponseHeader,
    prelude::HttpPeer,
    proxy::{ProxyHttp, Session},
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{debug, error, info};

async fn handle_error_response<'a>(
    res: &'a mut Response<'a>,
    session: &'a mut Session,
    error: impl Into<NylonError>,
) -> pingora::Result<bool> {
    let error = error.into();
    error!("Request error: {}", error);

    res.status(error.http_status())
        .body_json(error.exception_json())?
        .send(session)
        .await
}

/// Handle ACME HTTP-01 challenge requests
async fn handle_acme_challenge<'a>(
    res: &'a mut Response<'a>,
    session: &'a mut Session,
    path: &str,
) -> pingora::Result<bool> {
    // แยก token จาก path: /.well-known/acme-challenge/{token}
    let token = path.trim_start_matches("/.well-known/acme-challenge/");

    // ดึง host จาก request
    let host = session
        .req_header()
        .headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    // ตัดพอร์ตออกถ้ามีใน Host header
    let host_name = host.split(':').next().unwrap_or(host);

    // ตรวจสอบว่า host ถูกกำหนดไว้ใน ACME config
    let allowed = nylon_store::get::<HashMap<String, nylon_types::tls::AcmeConfig>>(
        nylon_store::KEY_ACME_CONFIG,
    )
    .map(|m| m.contains_key(host_name))
    .unwrap_or(false);
    if !allowed {
        error!("ACME challenge host not configured: {}", host_name);
        return res
            .status(404)
            .body_json(serde_json::json!({
                "error": "Host not configured for ACME"
            }))?
            .send(session)
            .await;
    }

    // ดึง acme_dir จาก runtime config
    let acme_dir = match nylon_config::runtime::RuntimeConfig::get() {
        Ok(config) => config.acme.to_string_lossy().to_string(),
        Err(_) => ".acme".to_string(),
    };

    // ดึง challenge response
    match nylon_tls::AcmeClient::load_challenge_token(&acme_dir, host_name, token) {
        Ok(key_auth) => {
            debug!("ACME challenge response for {}: {}", host_name, token);
            res.status(200);
            {
                let mut headers = res.ctx.add_response_header.write().expect("lock");
                headers.insert("Content-Type".to_string(), "text/plain".to_string());
            }
            res.body(Bytes::from(key_auth.as_bytes().to_vec()));
            res.send(session).await
        }
        Err(e) => {
            error!(
                "ACME challenge token not found for {} / {}: {}",
                host_name, token, e
            );
            res.status(404)
                .body_json(serde_json::json!({
                    "error": "Challenge token not found"
                }))?
                .send(session)
                .await
        }
    }
}

fn process_tls_redirect(host: &str, tls: bool) -> Option<String> {
    if tls {
        return None;
    }

    match nylon_store::routes::get_tls_route(host) {
        Ok(Some(redirect)) => {
            let redirect = redirect
                .replace("${host}", host)
                .replace("http://", "")
                .replace("https://", "");

            Some(format!("https://{}", redirect))
        }
        Ok(None) => None,
        Err(_) => None,
    }
}

async fn process_middleware<T>(
    proxy: &T,
    phase: PluginPhase,
    ctx: &mut NylonContext,
    session: &mut Session,
    response_body: &Option<Bytes>,
    error: Option<&pingora::Error>,
) -> pingora::Result<PluginResult>
where
    T: ProxyHttp + Send + Sync,
    <T as ProxyHttp>::CTX: Send + Sync + From<NylonContext>,
{
    // Store error message if present
    if let Some(err) = error
        && let Ok(mut error_msg) = ctx.error_message.write()
    {
        *error_msg = Some(err.to_string());
    }
    // Collect all middleware items from route and path levels
    let route_opt = ctx
        .route
        .read()
        .map_err(|_| {
            pingora::Error::because(
                ErrorType::InternalError,
                "[middleware]",
                NylonError::InternalServerError("lock poisoned".into()),
            )
        })?
        .clone();
    let Some(route) = &route_opt else {
        return Ok(PluginResult::default());
    };
    let path_middleware = &route.path_middleware;
    let middleware_items = route
        .route_middleware
        .iter()
        .flatten()
        .chain(path_middleware.iter().flatten());

    // Process each middleware item
    for middleware in middleware_items.cloned().collect::<Vec<_>>() {
        // debug!("Processing middleware: {:?}", middleware.0.plugin);

        match run_middleware(
            proxy,
            &phase,
            &MiddlewareContext {
                middleware: middleware.0.clone(),
                payload: middleware.0.payload.clone(),
                payload_ast: middleware.1.clone(),
            },
            ctx,
            session,
            response_body,
        )
        .await
        {
            Ok((http_end, _)) if http_end => {
                return Ok(PluginResult::new(true, false));
            }
            Ok((_, stream_end)) if stream_end => {
                return Ok(PluginResult::new(false, true));
            }
            Ok(_) => {
                continue;
            }
            Err(e) => {
                error!("Middleware error: {}", e);
                return Err(pingora::Error::because(
                    ErrorType::InternalError,
                    "[middleware]",
                    e,
                ));
            }
        }
    }

    Ok(PluginResult::default())
}

#[async_trait]
impl ProxyHttp for NylonRuntime {
    type CTX = NylonContext;

    fn new_ctx(&self) -> Self::CTX {
        let ctx = NylonContext::default();
        // Set request timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        ctx.request_timestamp
            .store(timestamp, std::sync::atomic::Ordering::Relaxed);
        ctx
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let mut res = Response::new(self, ctx).await?;

        // Parse request and handle errors
        if let Err(e) = res.ctx.parse_request(session).await {
            return handle_error_response(&mut res, session, e).await;
        }

        // Handle ACME HTTP-01 challenge requests BEFORE route matching
        let req_path = session.req_header().uri.path().to_string();
        if req_path.starts_with("/.well-known/acme-challenge/") {
            debug!("ACME challenge request: {}", req_path);
            return handle_acme_challenge(&mut res, session, &req_path).await;
        }

        // Find matching route
        let (route, params) = match nylon_store::routes::find_route(session) {
            Ok(route) => route,
            Err(e) => return handle_error_response(&mut res, session, e).await,
        };

        // Check for TLS redirect
        let host_owned = res
            .ctx
            .host
            .read()
            .map_err(|_| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[proxy]",
                    "host lock".to_string(),
                )
            })?
            .clone();
        let tls = res.ctx.tls.load(Ordering::Relaxed);
        if let Some(redirect_url) = process_tls_redirect(&host_owned, tls) {
            info!("Redirecting to TLS: {}", redirect_url);
            res.redirect(redirect_url);
            return res.send(session).await;
        }

        // Store route and params in context
        {
            let mut r = res.ctx.route.write().map_err(|_| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[proxy]",
                    "route lock".to_string(),
                )
            })?;
            *r = Some(route.clone());
        }
        {
            let mut p = res.ctx.params.write().map_err(|_| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[proxy]",
                    "params lock".to_string(),
                )
            })?;
            *p = Some(params.clone());
        }

        // Process middleware
        match process_middleware(
            self,
            PluginPhase::RequestFilter,
            res.ctx,
            session,
            &None,
            None,
        )
        .await
        {
            Ok(result) => {
                if result.http_end {
                    return res.send(session).await;
                } else if result.stream_end {
                    return Ok(true);
                }
            }
            Err(e) => {
                let nylon_error = NylonError::InternalServerError(e.to_string());
                return handle_error_response(&mut res, session, nylon_error).await;
            }
        }

        // Handle plugin service type
        if route.service.service_type == ServiceType::Plugin {
            if let Some(plugin) = &route.service.plugin {
                match nylon_plugin::session_stream(
                    self,
                    plugin.name.as_str(),
                    PluginPhase::RequestFilter,
                    plugin.entry.as_str(),
                    res.ctx,
                    session,
                    &plugin.payload,
                    &None,
                    &None,
                )
                .await
                {
                    Ok(result) => {
                        if result.http_end {
                            return res.send(session).await;
                        } else if result.stream_end {
                            return Ok(true);
                        }
                        return Ok(false);
                    }
                    Err(e) => {
                        return handle_error_response(&mut res, session, e).await;
                    }
                }
            } else {
                let err =
                    NylonError::ConfigError("Plugin service missing 'plugin' config".to_string());
                return handle_error_response(&mut res, session, err).await;
            }
        }

        // Handle regular HTTP service type only
        if route.service.service_type == ServiceType::Http {
            let http_service = match nylon_store::lb_backends::get(&route.service.name).await {
                Ok(backend) => backend,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            };

            // Get backend selection
            let selected_backend = match backend::selection(&http_service, session, res.ctx) {
                Ok(b) => b,
                Err(e) => return handle_error_response(&mut res, session, e).await,
            };

            {
                let mut b = res.ctx.backend.write().map_err(|_| {
                    pingora::Error::because(
                        ErrorType::InternalError,
                        "[proxy]",
                        "backend lock".to_string(),
                    )
                })?;
                *b = selected_backend;
            }
        }

        // Handle static file service type (serve from disk, optional SPA fallback)
        if route.service.service_type == ServiceType::Static {
            let Some(conf) = &route.service.static_conf else {
                let err =
                    NylonError::ConfigError("Static service missing 'static' config".to_string());
                return handle_error_response(&mut res, session, err).await;
            };

            // Build requested path
            let uri_path = if session.is_http2() {
                session
                    .as_http2()
                    .map(|s| s.req_header().uri.path().to_string())
                    .unwrap_or_else(|| "/".to_string())
            } else {
                session.req_header().uri.path().to_string()
            };

            let rewrite_prefix = route.rewrite.clone().unwrap_or_default();
            let rel_path = if !rewrite_prefix.is_empty() && uri_path.starts_with(&rewrite_prefix) {
                uri_path[rewrite_prefix.len()..].to_string()
            } else {
                uri_path.clone()
            };

            // Security: Prevent directory traversal (e.g., /static/../secret)
            if rel_path.split('/').any(|seg| seg == "..") {
                let err = NylonError::HttpException(403, "FORBIDDEN", "Invalid path");
                return handle_error_response(&mut res, session, err).await;
            }

            let root = PathBuf::from(&conf.root);
            let mut file_path = root.join(rel_path.trim_start_matches('/'));

            // If path is a directory or ends with slash, append index file
            let index_name = conf
                .index
                .clone()
                .unwrap_or_else(|| "index.html".to_string());
            if uri_path.ends_with('/')
                || fs::metadata(&file_path)
                    .map(|m| m.is_dir())
                    .unwrap_or(false)
            {
                file_path = file_path.join(&index_name);
            }

            // Try to read file
            debug!("[static] file_path: {}", file_path.display());
            match fs::read(&file_path) {
                Ok(bytes) => {
                    let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
                    {
                        let mut headers = res.ctx.add_response_header.write().expect("lock");
                        headers.insert("Content-Type".to_string(), mime.to_string());
                    }
                    res.status(200).body(Bytes::from(bytes));
                    return res.send(session).await;
                }
                Err(_) => {
                    // If SPA enabled, serve index.html from root
                    if conf.spa.unwrap_or(false) {
                        let spa_index = root.join(&index_name);
                        match fs::read(&spa_index) {
                            Ok(bytes) => {
                                let mime =
                                    mime_guess::from_path(&spa_index).first_or_octet_stream();
                                {
                                    let mut headers =
                                        res.ctx.add_response_header.write().expect("lock");
                                    headers.insert("Content-Type".to_string(), mime.to_string());
                                }
                                res.status(200).body(Bytes::from(bytes));
                                return res.send(session).await;
                            }
                            Err(_) => {
                                let err =
                                    NylonError::HttpException(404, "NOT_FOUND", "File not found");
                                return handle_error_response(&mut res, session, err).await;
                            }
                        }
                    } else {
                        let err = NylonError::HttpException(404, "NOT_FOUND", "File not found");
                        return handle_error_response(&mut res, session, err).await;
                    }
                }
            }
        }

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let backend_guard = ctx.backend.read().map_err(|_| {
            pingora::Error::because(
                ErrorType::InternalError,
                "[upstream_peer]",
                "backend lock".to_string(),
            )
        })?;
        let peer = backend_guard.ext.get::<HttpPeer>().ok_or_else(|| {
            pingora::Error::because(
                ErrorType::InternalError,
                "[upstream_peer]",
                NylonError::ConfigError("[backend] no peer found".to_string()),
            )
        })?;
        Ok(Box::new(peer.clone()))
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
        // Process middleware
        let _ =
            process_middleware(self, PluginPhase::ResponseFilter, ctx, session, &None, None).await;

        // Add response headers
        for (key, value) in ctx
            .add_response_header
            .read()
            .map_err(|_| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[response_filter]",
                    "add_header lock".to_string(),
                )
            })?
            .iter()
        {
            let _ = upstream_response.append_header(key.to_ascii_lowercase(), value);
        }

        // Remove response headers
        for key in ctx
            .remove_response_header
            .read()
            .map_err(|_| {
                pingora::Error::because(
                    ErrorType::InternalError,
                    "[response_filter]",
                    "remove_header lock".to_string(),
                )
            })?
            .iter()
        {
            let key = key.to_ascii_lowercase();
            let _ = upstream_response.remove_header(&key);
        }

        // Set response status if modified
        upstream_response.set_status(ctx.set_response_status.load(Ordering::Relaxed))?;

        Ok(())
    }

    fn response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        // Process middleware for response_body_filter phase
        let _ = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                process_middleware(
                    self,
                    PluginPhase::ResponseBodyFilter,
                    ctx,
                    session,
                    body,
                    None,
                )
                .await
            })
        });

        let buf = ctx.set_response_body.write().map_err(|_| {
            pingora::Error::because(
                ErrorType::InternalError,
                "[body_filter]",
                "set_response_body lock".to_string(),
            )
        })?;

        if !buf.is_empty() {
            *body = Some(Bytes::from(buf.clone()));
        }
        Ok(None)
    }

    async fn logging(&self, session: &mut Session, e: Option<&pingora::Error>, ctx: &mut Self::CTX)
    where
        Self::CTX: Send + Sync,
    {
        // Process middleware for logging phase
        let _ = process_middleware(self, PluginPhase::Logging, ctx, session, &None, e).await;

        let streams = ctx
            .session_stream
            .read()
            .map(|m| m.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for stream in streams {
            let _ = stream.close().await;
        }

        if let Ok(mut sessions) = ctx.session_stream.write() {
            sessions.clear();
        }
        if let Ok(mut ids) = ctx.session_ids.write() {
            ids.clear();
        }
    }
}
