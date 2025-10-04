use async_trait::async_trait;
use dashmap::DashMap;
use nylon_types::{plugins::FfiPlugin, tls::AcmeConfig};
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
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    // shutdown
                    tracing::info!("Shutting down background service");

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
