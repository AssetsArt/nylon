use async_trait::async_trait;
use dashmap::DashMap;
use nylon_config::{proxy::ProxyConfigExt, runtime::RuntimeConfig};
use nylon_types::{plugins::FfiPlugin, proxy::ProxyConfig, tls::AcmeConfig};
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};

pub struct NylonBackgroundService;
#[async_trait]
impl BackgroundService for NylonBackgroundService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut period_1d = interval(Duration::from_secs(86400));
        let mut hc_interval = interval(Duration::from_secs(5));
        let signal = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup());
        let mut signal = match signal {
            Ok(signal) => signal,
            Err(e) => {
                error!("Failed to create signal handler: {}", e);
                std::process::exit(1);
            }
        };
        loop {
            tokio::select! {
                _ = signal.recv() => {
                    info!("Received SIGHUP signal - reloading configuration...");
                    if let Err(e) = reload_configuration().await {
                        error!("Failed to reload configuration: {}", e);
                    } else {
                        info!("✓ Configuration reloaded successfully");
                    }
                },
                _ = shutdown.changed() => {
                    // shutdown
                    info!("Shutting down background service");

                    // Shutting down plugins
                    let plugins =
                    match nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS) {
                        Some(plugins) => plugins,
                        None => {
                            let new_plugins = DashMap::new();
                            nylon_store::insert(nylon_store::KEY_PLUGINS, new_plugins.clone());
                            new_plugins
                        }
                    };
                    for plugin in plugins.iter() {
                        unsafe {
                            (plugin.value().shutdown)();
                        }
                    }
                    break;
                },
                _ = hc_interval.tick() => {
                    // periodic health checks for all services
                    nylon_store::lb_backends::run_health_checks_for_all().await;
                },
                _ = period_1d.tick() => {
                    info!("Running daily certificate expiration check");
                    if let Err(e) = check_and_renew_certificates().await {
                        error!("Failed to check/renew certificates: {}", e);
                    }
                }
            }
        }
    }
}

/// ตรวจสอบและ renew certificates ที่กำลังจะหมดอายุ
async fn check_and_renew_certificates() -> Result<(), nylon_error::NylonError> {
    let certificates = nylon_store::tls::get_all_certificates();

    if certificates.is_empty() {
        info!("No ACME certificates to check");
        return Ok(());
    }

    info!("Checking {} ACME certificates", certificates.len());

    for cert_info in certificates {
        let days_until_expiry = cert_info.days_until_expiry();

        info!(
            "Certificate for {}: expires in {} days",
            cert_info.domain, days_until_expiry
        );

        // Update metrics with current expiry info
        if let Some(metrics) =
            nylon_store::get::<nylon_tls::AcmeMetrics>(nylon_store::KEY_ACME_METRICS)
        {
            metrics.update_days_until_expiry(&cert_info.domain, days_until_expiry);
        }

        if cert_info.is_expired() {
            error!(
                "Certificate for {} is expired! Renewing immediately...",
                cert_info.domain
            );
            renew_certificate(&cert_info.domain).await?;
        } else if cert_info.needs_renewal() {
            warn!(
                "Certificate for {} needs renewal (expires in {} days)",
                cert_info.domain,
                cert_info.days_until_expiry()
            );
            // Add small jitter to avoid burst renewals
            let jitter_ms = fastrand::u64(..2000);
            sleep(Duration::from_millis(jitter_ms)).await;
            // Try renew with simple backoff on transient errors
            let mut attempts = 0u8;
            let max_attempts = 3u8;
            let mut backoff_ms = 1000u64;
            loop {
                attempts += 1;
                match renew_certificate(&cert_info.domain).await {
                    Ok(_) => break,
                    Err(e) if attempts < max_attempts => {
                        warn!(
                            "Renew failed for {} (attempt {}/{}): {}. Retrying in {}ms",
                            cert_info.domain, attempts, max_attempts, e, backoff_ms
                        );
                        sleep(Duration::from_millis(backoff_ms)).await;
                        backoff_ms *= 2;
                    }
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Renew certificate สำหรับ domain
async fn renew_certificate(domain: &str) -> Result<(), nylon_error::NylonError> {
    info!("Renewing certificate for domain: {}", domain);

    let result = async {
        // ดึง ACME config สำหรับ domain นี้
        let acme_configs =
            nylon_store::get::<HashMap<String, AcmeConfig>>(nylon_store::KEY_ACME_CONFIG)
                .ok_or_else(|| {
                    nylon_error::NylonError::ConfigError("ACME config not found".to_string())
                })?;

        let acme_config = acme_configs.get(domain).ok_or_else(|| {
            nylon_error::NylonError::ConfigError(format!(
                "ACME config not found for domain: {}",
                domain
            ))
        })?;

        // สร้าง ACME client
        let mut client = nylon_tls::AcmeClient::new(acme_config).await?;

        // ออก certificate ใหม่
        let (cert, key, chain) = client.issue_certificate(domain).await?;

        // สร้าง CertificateInfo
        let cert_info = nylon_tls::CertificateInfo::new(domain.to_string(), cert, key, chain)?;

        info!(
            "Certificate renewed successfully for {}, expires at: {}",
            domain, cert_info.expires_at
        );

        // เก็บ certificate ใหม่
        nylon_store::tls::store_acme_cert(cert_info.clone())?;

        // Update metrics
        if let Some(metrics) =
            nylon_store::get::<nylon_tls::AcmeMetrics>(nylon_store::KEY_ACME_METRICS)
        {
            metrics.record_renewal_success(domain);
            metrics.update_days_until_expiry(domain, cert_info.days_until_expiry());
        }

        Ok::<(), nylon_error::NylonError>(())
    }
    .await;

    if let Err(e) = &result {
        // Record failure in metrics
        if let Some(metrics) =
            nylon_store::get::<nylon_tls::AcmeMetrics>(nylon_store::KEY_ACME_METRICS)
        {
            metrics.record_renewal_failure(domain);
        }
        return Err(e.clone());
    }

    result
}

/// Reload configuration from file
async fn reload_configuration() -> Result<(), nylon_error::NylonError> {
    info!("Starting configuration reload...");

    // Get stored config path
    let config_path = nylon_store::get::<String>(nylon_store::KEY_CONFIG_PATH)
        .ok_or_else(|| nylon_error::NylonError::ConfigError("Config path not found".to_string()))?;

    info!("Loading runtime configuration from: {}", config_path);

    // Load and validate runtime configuration
    let runtime_config = RuntimeConfig::from_file(&config_path)?;

    // Store new runtime config
    runtime_config.store()?;
    info!("✓ Runtime configuration updated");

    // Load proxy configuration from config_dir
    let proxy_config = ProxyConfig::from_dir(
        runtime_config
            .config_dir
            .to_string_lossy()
            .to_string()
            .as_str(),
    )?;

    // Store new proxy config
    proxy_config.store().await?;
    info!("✓ Proxy configuration updated");

    // Reload ACME certificates if needed
    if let Err(e) = reload_acme_certificates().await {
        warn!("Failed to reload ACME certificates: {}", e);
    }

    Ok(())
}

/// Reload ACME certificates configuration
async fn reload_acme_certificates() -> Result<(), nylon_error::NylonError> {
    use nylon_types::tls::AcmeConfig;

    info!("Reloading ACME certificates...");

    // Get ACME configs
    let acme_configs =
        match nylon_store::get::<HashMap<String, AcmeConfig>>(nylon_store::KEY_ACME_CONFIG) {
            Some(configs) if !configs.is_empty() => configs,
            _ => {
                info!("No ACME domains configured after reload");
                return Ok(());
            }
        };

    info!(
        "Found {} domains configured for ACME after reload",
        acme_configs.len()
    );

    // Check each domain's certificate
    for (domain, acme_config) in acme_configs.iter() {
        let acme_dir = acme_config.acme_dir.as_deref().unwrap_or(".acme");

        info!("Checking certificate for domain: {}", domain);

        // Check if certificate exists and is valid
        match nylon_tls::AcmeClient::load_certificate_with_chain(acme_dir, domain) {
            Ok((cert, key, chain)) => {
                match nylon_tls::CertificateInfo::new(domain.clone(), cert, key, chain) {
                    Ok(cert_info) => {
                        if cert_info.is_expired() {
                            info!(
                                "Certificate for {} is expired, issuing new certificate...",
                                domain
                            );
                            renew_certificate(domain).await?;
                        } else {
                            info!(
                                "Certificate for {} is still valid, expires in {} days",
                                domain,
                                cert_info.days_until_expiry()
                            );
                            // Update in store
                            nylon_store::tls::store_acme_cert(cert_info)?;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse certificate for {}: {}", domain, e);
                        info!("Issuing new certificate for {}...", domain);
                        renew_certificate(domain).await?;
                    }
                }
            }
            Err(_) => {
                // No certificate found - issue a new one for the new domain
                info!(
                    "No certificate found for {} after reload, issuing new certificate...",
                    domain
                );
                if let Err(e) = renew_certificate(domain).await {
                    error!("Failed to issue certificate for {}: {}", domain, e);
                    // Don't return error, continue with other domains
                }
            }
        }
    }

    info!("ACME certificates reload completed");
    Ok(())
}
