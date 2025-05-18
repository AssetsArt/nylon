use nylon_config::runtime::RuntimeConfig;
use nylon_error::NylonError;
use pingora::{
    prelude::{Opt, background_service},
    proxy,
    server::{Server, configuration::ServerConf},
};

use crate::{background_service::NylonBackgroundService, dynamic_certificate::new_tls_settings};

#[derive(Debug, Clone)]
pub struct NylonRuntime {}

impl NylonRuntime {
    /// Create a new server
    ///
    /// # Returns
    ///
    /// * `Result<Server, NylonError>` - The result of the operation
    pub fn new_server() -> Result<Server, NylonError> {
        let config = RuntimeConfig::get()?;

        let opt = Opt {
            daemon: config.pingora.daemon,
            ..Default::default()
        };
        let mut pingora_server =
            Server::new(Some(opt)).map_err(|e| NylonError::PingoraError(e.to_string()))?;
        let mut conf = ServerConf {
            daemon: config.pingora.daemon,
            grace_period_seconds: Some(config.pingora.grace_period_seconds),
            graceful_shutdown_timeout_seconds: Some(
                config.pingora.graceful_shutdown_timeout_seconds,
            ),
            threads: config.pingora.threads,
            ..Default::default()
        };
        let path_to_string = |path: &std::path::PathBuf| -> Option<String> {
            path.to_str().filter(|s| !s.is_empty()).map(String::from)
        };

        if let Some(v) = &config.pingora.error_log {
            conf.error_log = path_to_string(v);
        }
        if let Some(v) = &config.pingora.pid_file {
            conf.pid_file = path_to_string(v).unwrap_or_default();
        }
        if let Some(v) = &config.pingora.upgrade_sock {
            conf.upgrade_sock = path_to_string(v).unwrap_or_default();
        }
        if let Some(v) = &config.pingora.ca_file {
            conf.ca_file = path_to_string(v);
        }

        conf.user = config.pingora.user.clone().filter(|s| !s.is_empty());
        conf.group = config.pingora.group.clone().filter(|s| !s.is_empty());
        conf.work_stealing = config.pingora.work_stealing.unwrap_or(conf.work_stealing);
        if let Some(v) = &config.pingora.upstream_keepalive_pool_size {
            conf.upstream_keepalive_pool_size = *v
        }

        pingora_server.configuration = conf.into();

        let runtime = NylonRuntime {};
        let http_listener = config.http;
        let https_listener = config.https;

        {
            let mut pingora_svc =
                proxy::http_proxy_service(&pingora_server.configuration, runtime.clone());

            if let Some(http_zero_addr) = http_listener.iter().find(|a| a.contains("0.0.0.0")) {
                pingora_svc.add_tcp(http_zero_addr);
                tracing::info!("Proxy server started on http://{}", http_zero_addr);
            } else {
                for addr in &http_listener {
                    pingora_svc.add_tcp(addr);
                    tracing::info!("Proxy server started on http://{}", addr);
                }
            }

            pingora_server.add_service(pingora_svc);
        }

        if !https_listener.is_empty() {
            let mut pingora_svc =
                proxy::http_proxy_service(&pingora_server.configuration, runtime.clone());

            if let Some(https_zero_addr) = https_listener.iter().find(|a| a.contains("0.0.0.0")) {
                pingora_svc.add_tls_with_settings(https_zero_addr, None, new_tls_settings()?);
                tracing::info!("Proxy server started on https://{}", https_zero_addr);
            } else {
                for addr in &https_listener {
                    pingora_svc.add_tls_with_settings(addr, None, new_tls_settings()?);
                    tracing::info!("Proxy server started on https://{}", addr);
                }
            }

            pingora_server.add_service(pingora_svc);
        }

        // background service
        let bg_service = background_service("NylonBackgroundService", NylonBackgroundService {});
        pingora_server.add_service(bg_service);

        Ok(pingora_server)
    }
}
