//! Nylon Runtime Server Implementation
//!
//! This module contains the core runtime functionality for the Nylon proxy server,
//! including server initialization, configuration, and service management.

use crate::{background_service::NylonBackgroundService, dynamic_certificate::new_tls_settings};
use nylon_config::runtime::RuntimeConfig;
use nylon_error::NylonError;
use pingora::{
    prelude::{Opt, background_service},
    proxy,
    server::{Server, configuration::ServerConf},
};
use tracing::info;

/// Nylon runtime server instance
#[derive(Debug, Clone)]
pub struct NylonRuntime {}

impl NylonRuntime {
    /// Create a new Nylon server instance
    ///
    /// This method initializes the Pingora server with Nylon-specific configuration
    /// including HTTP/HTTPS listeners, TLS settings, and background services.
    ///
    /// # Returns
    ///
    /// * `Result<Server, NylonError>` - The configured server instance or an error
    pub fn new_server() -> Result<Server, NylonError> {
        let config = RuntimeConfig::get()?;
        info!("Initializing Nylon server with configuration");

        // Create Pingora server with basic options
        let opt = Opt {
            daemon: config.pingora.daemon,
            ..Default::default()
        };

        let mut pingora_server =
            Server::new(Some(opt)).map_err(|e| NylonError::PingoraError(e.to_string()))?;

        // Configure server settings
        let conf = create_server_config(&config)?;
        pingora_server.configuration = conf.into();

        let runtime = NylonRuntime {};

        // Add HTTP service
        add_http_service(&mut pingora_server, &config, &runtime)?;

        // Add HTTPS service if configured
        if !config.https.is_empty() {
            add_https_service(&mut pingora_server, &config, &runtime)?;
        }

        // Add background service
        let bg_service = background_service("NylonBackgroundService", NylonBackgroundService {});
        pingora_server.add_service(bg_service);

        info!("Nylon server initialization completed successfully");
        Ok(pingora_server)
    }
}

/// Create server configuration from runtime config
///
/// # Arguments
///
/// * `config` - The runtime configuration
///
/// # Returns
///
/// * `Result<ServerConf, NylonError>` - The server configuration
fn create_server_config(config: &RuntimeConfig) -> Result<ServerConf, NylonError> {
    let mut conf = ServerConf {
        daemon: config.pingora.daemon,
        grace_period_seconds: Some(config.pingora.grace_period_seconds),
        graceful_shutdown_timeout_seconds: Some(config.pingora.graceful_shutdown_timeout_seconds),
        threads: config.pingora.threads,
        ..Default::default()
    };

    // Helper function to convert PathBuf to Option<String>
    let path_to_string = |path: &std::path::PathBuf| -> Option<String> {
        path.to_str().filter(|s| !s.is_empty()).map(String::from)
    };

    // Set optional configuration values
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

    // Set user and group if provided
    conf.user = config.pingora.user.clone().filter(|s| !s.is_empty());
    conf.group = config.pingora.group.clone().filter(|s| !s.is_empty());

    // Set work stealing if configured
    conf.work_stealing = config.pingora.work_stealing.unwrap_or(conf.work_stealing);

    // Set upstream keepalive pool size if configured
    if let Some(v) = &config.pingora.upstream_keepalive_pool_size {
        conf.upstream_keepalive_pool_size = *v
    }

    Ok(conf)
}

/// Add HTTP service to the server
///
/// # Arguments
///
/// * `server` - The Pingora server instance
/// * `config` - The runtime configuration
/// * `runtime` - The Nylon runtime instance
///
/// # Returns
///
/// * `Result<(), NylonError>` - Success or error
fn add_http_service(
    server: &mut Server,
    config: &RuntimeConfig,
    runtime: &NylonRuntime,
) -> Result<(), NylonError> {
    let mut pingora_svc = proxy::http_proxy_service(&server.configuration, runtime.clone());

    // Find and add zero address first (for binding to all interfaces)
    if let Some(http_zero_addr) = config.http.iter().find(|a| a.contains("0.0.0.0")) {
        pingora_svc.add_tcp(http_zero_addr);
        info!("HTTP proxy server started on http://{}", http_zero_addr);
    } else {
        // Add all configured HTTP addresses
        for addr in &config.http {
            pingora_svc.add_tcp(addr);
            info!("HTTP proxy server started on http://{}", addr);
        }
    }

    server.add_service(pingora_svc);
    Ok(())
}

/// Add HTTPS service to the server
///
/// # Arguments
///
/// * `server` - The Pingora server instance
/// * `config` - The runtime configuration
/// * `runtime` - The Nylon runtime instance
///
/// # Returns
///
/// * `Result<(), NylonError>` - Success or error
fn add_https_service(
    server: &mut Server,
    config: &RuntimeConfig,
    runtime: &NylonRuntime,
) -> Result<(), NylonError> {
    let mut pingora_svc = proxy::http_proxy_service(&server.configuration, runtime.clone());

    // Create TLS settings
    let tls_settings = new_tls_settings()?;

    // Find and add zero address first (for binding to all interfaces)
    if let Some(https_zero_addr) = config.https.iter().find(|a| a.contains("0.0.0.0")) {
        pingora_svc.add_tls_with_settings(https_zero_addr, None, tls_settings);
        info!("HTTPS proxy server started on https://{}", https_zero_addr);
    } else {
        // Add all configured HTTPS addresses
        for addr in &config.https {
            let tls_settings = new_tls_settings()?;
            pingora_svc.add_tls_with_settings(addr, None, tls_settings);
            info!("HTTPS proxy server started on https://{}", addr);
        }
    }

    server.add_service(pingora_svc);
    Ok(())
}
