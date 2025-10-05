//! Nylon - The Extensible Proxy Server
//!
//! This is the main entry point for the Nylon proxy server application.
//! It handles command-line argument parsing and initializes the server runtime.

mod backend;
mod background_service;
mod context;
mod dynamic_certificate;
mod proxy;
mod response;
mod runtime;

use nylon_command::Commands;
use nylon_config::{proxy::ProxyConfigExt, runtime::RuntimeConfig};
use nylon_error::NylonError;
use nylon_types::proxy::ProxyConfig;
use runtime::NylonRuntime;
use tracing::{error, info, warn};

/// Main entry point for the Nylon proxy server
fn main() {
    // Initialize logging with appropriate level
    tracing_subscriber::fmt::init();

    info!("Starting Nylon proxy server...");

    // Parse command line arguments
    let args = nylon_command::parse();

    // Handle different commands
    if let Err(e) = handle_commands(args.command) {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}

/// Handle different command types
///
/// # Arguments
///
/// * `args` - Parsed command line arguments
///
/// # Returns
///
/// * `Result<(), NylonError>` - The result of the operation
fn handle_commands(args: Commands) -> Result<(), NylonError> {
    match args {
        Commands::Service(service) => {
            info!("Service command received: {:?}", service);
            nylon_command::handle_service_command(service)
                .map_err(|e| NylonError::RuntimeError(format!("Service command failed: {}", e)))?;
            Ok(())
        }
        Commands::Run { config } => handle_run_command(config),
    }
}

/// Handle the run command
///
/// # Arguments
///
/// * `config_path` - The path to the configuration file
///
/// # Returns
///
/// * `Result<(), NylonError>` - The result of the operation
fn handle_run_command(config_path: String) -> Result<(), NylonError> {
    info!("Loading configuration from: {}", config_path);

    // Load and validate runtime configuration
    let config = RuntimeConfig::from_file(&config_path)?;
    config.store()?;

    info!("Runtime configuration loaded successfully");
    tracing::debug!("Runtime config: {:#?}", RuntimeConfig::get()?);

    // Load proxy configuration
    let proxy_config =
        ProxyConfig::from_dir(config.config_dir.to_string_lossy().to_string().as_str())?;
    tracing::debug!("Proxy config: {:#?}", proxy_config);

    // Create and run the server
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| NylonError::RuntimeError(format!("Failed to create Tokio runtime: {}", e)))?;

    rt.block_on(async {
        proxy_config.store().await?;

        // Initialize WebSocket adapter
        let runtime_config = RuntimeConfig::get()?;
        nylon_store::websockets::initialize_adapter(runtime_config.websocket).await?;

        // Initialize ACME metrics
        let acme_metrics = nylon_tls::AcmeMetrics::new();
        nylon_store::insert(nylon_store::KEY_ACME_METRICS, acme_metrics);

        // Initialize ACME certificates
        if let Err(e) = initialize_acme_certificates().await {
            error!("Failed to initialize ACME certificates: {}", e);
        }

        // wating signal HUP for reload config
        // loop {
        //     info!("Reloading runtime config...");
        //     let config = match RuntimeConfig::from_file(&config_path) {
        //         Ok(config) => config,
        //         Err(e) => {
        //             error!("Failed to load runtime config: {}", e);
        //             continue;
        //         }
        //     };
        //     match config.store() {
        //         Ok(_) => {
        //             info!("Runtime config stored successfully");
        //         }
        //         Err(e) => {
        //             error!("Failed to store runtime config: {}", e);
        //         }
        //     }

        //     // store
        //     info!("Storing proxy config...");
        //     match proxy_config.store().await {
        //         Ok(_) => {
        //             info!("Proxy config stored successfully");
        //         }
        //         Err(e) => {
        //             error!("Failed to store proxy config: {}", e);
        //         }
        //     }
        // }

        Ok::<(), NylonError>(())
    })?;

    info!("Starting Nylon runtime server...");
    NylonRuntime::new_server()
        .map_err(|e| NylonError::RuntimeError(format!("Failed to create server: {}", e)))?
        .run_forever();
}

/// Initialize ACME certificates สำหรับ domains ที่ใช้ ACME
async fn initialize_acme_certificates() -> Result<(), NylonError> {
    use nylon_types::tls::AcmeConfig;
    use std::collections::HashMap;

    info!("Initializing ACME certificates...");

    // ดึง ACME configs
    let acme_configs =
        match nylon_store::get::<HashMap<String, AcmeConfig>>(nylon_store::KEY_ACME_CONFIG) {
            Some(configs) if !configs.is_empty() => configs,
            _ => {
                info!("No ACME domains configured");
                return Ok(());
            }
        };

    info!("Found {} domains configured for ACME", acme_configs.len());

    for (domain, acme_config) in acme_configs.iter() {
        let acme_dir = acme_config.acme_dir.as_deref().unwrap_or(".acme");

        // ตรวจสอบว่ามี certificate อยู่แล้วหรือไม่ (พร้อม chain ถ้ามี)
        match nylon_tls::AcmeClient::load_certificate_with_chain(acme_dir, domain) {
            Ok((cert, key, chain)) => {
                // มี certificate อยู่แล้ว ตรวจสอบว่ายังใช้งานได้หรือไม่
                match nylon_tls::CertificateInfo::new(domain.clone(), cert, key, chain) {
                    Ok(cert_info) => {
                        if cert_info.is_expired() {
                            info!(
                                "Certificate for {} is expired, issuing new certificate...",
                                domain
                            );
                            issue_new_certificate(domain, acme_config).await?;
                        } else {
                            info!(
                                "Using existing certificate for {}, expires in {} days",
                                domain,
                                cert_info.days_until_expiry()
                            );
                            // เก็บ certificate info ใน store
                            nylon_store::tls::store_acme_cert(cert_info)?;
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse existing certificate for {}: {}", domain, e);
                        issue_new_certificate(domain, acme_config).await?;
                    }
                }
            }
            Err(_) => {
                // ไม่มี certificate ต้องออกใหม่
                info!(
                    "No existing certificate for {}, issuing new certificate...",
                    domain
                );
                issue_new_certificate(domain, acme_config).await?;
            }
        }
    }

    info!("ACME certificates initialization completed");
    Ok(())
}

/// ออก certificate ใหม่สำหรับ domain
async fn issue_new_certificate(
    domain: &str,
    acme_config: &nylon_types::tls::AcmeConfig,
) -> Result<(), NylonError> {
    let result = async {
        let mut client = nylon_tls::AcmeClient::new(acme_config).await?;
        let (cert, key, chain) = client.issue_certificate(domain).await?;

        let cert_info = nylon_tls::CertificateInfo::new(domain.to_string(), cert, key, chain)?;

        info!(
            "Certificate issued successfully for {}, expires at: {}",
            domain, cert_info.expires_at
        );

        nylon_store::tls::store_acme_cert(cert_info.clone())?;

        // Update metrics
        if let Some(metrics) =
            nylon_store::get::<nylon_tls::AcmeMetrics>(nylon_store::KEY_ACME_METRICS)
        {
            metrics.record_issuance_success(domain);
            metrics.update_days_until_expiry(domain, cert_info.days_until_expiry());
        }

        Ok::<(), NylonError>(())
    }
    .await;

    if let Err(e) = &result {
        // Record failure in metrics
        if let Some(metrics) =
            nylon_store::get::<nylon_tls::AcmeMetrics>(nylon_store::KEY_ACME_METRICS)
        {
            metrics.record_issuance_failure(domain);
        }
        return Err(e.clone());
    }

    result
}
