mod background_service;
mod context;
mod dynamic_certificate;
mod proxy;
mod runtime;

use nylon_command::Commands;
use nylon_config::{proxy::ProxyConfig, runtime::RuntimeConfig};
use nylon_error::NylonError;
use runtime::NylonRuntime;

fn main() -> Result<(), NylonError> {
    // Initialize the logger.
    tracing_subscriber::fmt::init();

    // command
    let args = nylon_command::parse();
    // println!("{:?}", args);

    match args.command {
        Commands::Service(service) => {
            tracing::debug!("service: {:?}", service);
        }
        Commands::Run { config } => {
            handle_run(config)?;
        }
    }

    Ok(())
}

/// Handle the run command
///
/// # Arguments
///
/// * `path` - The path to the config file
///
/// # Returns
///
/// * `Result<(), NylonError>` - The result of the operation
fn handle_run(path: String) -> Result<(), NylonError> {
    tracing::debug!("[run] path: {:?}", path);
    let config = RuntimeConfig::from_file(&path)?;
    config.store()?;
    // tracing::debug!("[run] config: {:#?}", config);
    tracing::debug!("[run] config: {:#?}", RuntimeConfig::get()?);
    let proxy_config =
        ProxyConfig::from_dir(config.config_dir.to_string_lossy().to_string().as_str())?;
    tracing::debug!("[run] proxy_config: {:#?}", proxy_config);
    NylonRuntime::new_server()
        .expect("Failed to create server")
        .run_forever();
}
