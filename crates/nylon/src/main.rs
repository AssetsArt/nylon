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
use tracing::{error, info, Level};

/// Main entry point for the Nylon proxy server
fn main() {
    // Initialize logging with appropriate level
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

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
            // TODO: Implement service command handling
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
    let proxy_config = ProxyConfig::from_dir(
        config.config_dir.to_string_lossy().to_string().as_str()
    )?;
    tracing::debug!("Proxy config: {:#?}", proxy_config);
    
    // Create and run the server
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| NylonError::RuntimeError(format!("Failed to create Tokio runtime: {}", e)))?;
    
    rt.block_on(proxy_config.store())?;
    
    info!("Starting Nylon runtime server...");
    NylonRuntime::new_server()
        .map_err(|e| NylonError::RuntimeError(format!("Failed to create server: {}", e)))?
        .run_forever();
}
