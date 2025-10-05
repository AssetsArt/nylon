use crate::service::ServiceCommands;
use service_manager::*;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::Path;
use tracing::{error, info, warn};

const SERVICE_NAME: &str = "nylon";
const SERVICE_DESCRIPTION: &str = "Nylon - The Extensible Proxy Server";
const DEFAULT_CONFIG_PATH: &str = "/etc/nylon/config.yaml";
const DEFAULT_PROXY_CONFIG_DIR: &str = "/etc/nylon/proxy";
const DEFAULT_ACME_DIR: &str = "/etc/nylon/acme";
const DEFAULT_STATIC_DIR: &str = "/etc/nylon/static";

// Default configuration template
const DEFAULT_CONFIG_YAML: &str = r#"# Nylon Proxy Server Configuration
# Generated automatically during installation

http:
  - 0.0.0.0:8088

https:
  - 0.0.0.0:8443

metrics:
  - 127.0.0.1:6192

config_dir: "/etc/nylon/proxy"
acme: "/etc/nylon/acme"

pingora:
  daemon: false
  grace_period_seconds: 30
  graceful_shutdown_timeout_seconds: 10

# WebSocket adapter configuration (optional)
# websocket:
#   adapter_type: memory  # memory | redis | cluster
#   redis:
#     host: "localhost"
#     port: 6379
#     password: null
#     db: 0
#     key_prefix: "nylon:ws"
"#;

// Default proxy configuration template
const DEFAULT_PROXY_YAML: &str = r#"# Nylon Proxy Configuration
# Edit this file to configure your services and routes

header_selector: x-nylon-proxy

services:
  # Static assets / SPA
  - name: static
    service_type: static
    static:
      root: /etc/nylon/static
      index: index.html
      spa: true

# Host & path routing
routes:
  - route:
      type: host
      value: localhost
    name: app-route
    paths:
      # Static files
      - path:
          - /
          - /{*path}
        service:
          name: static
"#;

// Default static index.html
const DEFAULT_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<title>Welcome to nylon!</title>
</head>
<body>
<h1>Welcome to nylon!</h1>
<p>If you see this page, the nylon proxy server is successfully installed and
working.</p>

<p>For online documentation and support please refer to the repository.<br/>

<p><em>Thank you for using nylon.</em></p>
</body>
</html>
"#;

/// Error type for service handler operations
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to get executable path: {0}")]
    ExecutablePath(String),

    #[error("Service operation failed: {0}")]
    Operation(String),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Get the service manager for the current platform
fn get_service_manager() -> Result<Box<dyn ServiceManager>> {
    native_service_manager().map_err(ServiceError::Io)
}

/// Get the current executable path
fn get_executable_path() -> Result<std::path::PathBuf> {
    env::current_exe().map_err(|e| ServiceError::ExecutablePath(e.to_string()))
}

/// Create default configuration files if they don't exist
fn create_default_config() -> Result<()> {
    // Create main config directory
    let config_dir = Path::new(DEFAULT_CONFIG_PATH).parent().unwrap();
    if !config_dir.exists() {
        info!("Creating config directory: {}", config_dir.display());
        fs::create_dir_all(config_dir)?;
    }

    // Create main config file if it doesn't exist
    if !Path::new(DEFAULT_CONFIG_PATH).exists() {
        info!("Creating default config: {}", DEFAULT_CONFIG_PATH);
        fs::write(DEFAULT_CONFIG_PATH, DEFAULT_CONFIG_YAML)?;
        info!("✓ Default config created");
    } else {
        warn!(
            "Config file already exists, skipping: {}",
            DEFAULT_CONFIG_PATH
        );
    }

    // Create proxy config directory
    if !Path::new(DEFAULT_PROXY_CONFIG_DIR).exists() {
        info!(
            "Creating proxy config directory: {}",
            DEFAULT_PROXY_CONFIG_DIR
        );
        fs::create_dir_all(DEFAULT_PROXY_CONFIG_DIR)?;
    }

    // Create base proxy config file (using base.yaml naming to match examples)
    let base_proxy_path = format!("{}/base.yaml", DEFAULT_PROXY_CONFIG_DIR);
    if !Path::new(&base_proxy_path).exists() {
        info!("Creating base proxy config: {}", base_proxy_path);
        fs::write(&base_proxy_path, DEFAULT_PROXY_YAML)?;
        info!("✓ Base proxy config created");
    }

    // Create static directory for static assets
    if !Path::new(DEFAULT_STATIC_DIR).exists() {
        info!("Creating static directory: {}", DEFAULT_STATIC_DIR);
        fs::create_dir_all(DEFAULT_STATIC_DIR)?;
    }

    // Create index.html for static service
    let index_html_path = format!("{}/index.html", DEFAULT_STATIC_DIR);
    if !Path::new(&index_html_path).exists() {
        info!("Creating static index.html: {}", index_html_path);
        fs::write(&index_html_path, DEFAULT_INDEX_HTML)?;
        info!("✓ Static index.html created");
    }

    // Create ACME directory for certificates
    if !Path::new(DEFAULT_ACME_DIR).exists() {
        info!("Creating ACME directory: {}", DEFAULT_ACME_DIR);
        fs::create_dir_all(DEFAULT_ACME_DIR)?;
    }

    Ok(())
}

/// Handle service commands
pub fn handle_service_command(command: ServiceCommands) -> Result<()> {
    match command {
        ServiceCommands::Install => install_service(),
        ServiceCommands::Uninstall => uninstall_service(),
        ServiceCommands::Start => start_service(),
        ServiceCommands::Stop => stop_service(),
        ServiceCommands::Restart => restart_service(),
        ServiceCommands::Status => status_service(),
        ServiceCommands::Reload => reload_service(),
    }
}

/// Install the service
fn install_service() -> Result<()> {
    info!("Installing {} service...", SERVICE_NAME);

    // Create default configuration files
    info!("Setting up configuration files...");
    if let Err(e) = create_default_config() {
        warn!("Failed to create default config: {}", e);
        warn!("You may need to create the configuration files manually");
    }

    let manager = get_service_manager()?;
    let exe_path = get_executable_path()?;

    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();

    // Create custom systemd service content with reload support
    let service_contents = create_systemd_service_content(&exe_path);

    let service = ServiceInstallCtx {
        label: label.clone(),
        program: exe_path,
        args: vec![
            OsString::from("run"),
            OsString::from("-c"),
            OsString::from(DEFAULT_CONFIG_PATH),
        ],
        contents: service_contents,
        username: None,
        working_directory: None,
        environment: None,
        autostart: true,
        disable_restart_on_failure: false,
    };

    manager.install(service)?;
    info!("✓ Service installed successfully");
    info!("  Description: {}", SERVICE_DESCRIPTION);
    info!("");
    info!("Configuration files:");
    info!("  • Main config: {}", DEFAULT_CONFIG_PATH);
    info!("  • Proxy configs: {}", DEFAULT_PROXY_CONFIG_DIR);
    info!("  • Static files: {}", DEFAULT_STATIC_DIR);
    info!("  • ACME certs: {}", DEFAULT_ACME_DIR);
    info!("");
    info!("Service features:");
    info!("  • Reload config without restart: systemctl reload nylon");
    info!("  • Auto-restart on failure");
    info!("");
    info!("Next steps:");
    info!(
        "  1. Edit your proxy config: {}/base.yaml",
        DEFAULT_PROXY_CONFIG_DIR
    );
    info!("  2. Start the service: nylon service start");
    info!("  3. Visit http://localhost:8088");
    info!("  4. Reload config anytime: systemctl reload nylon");

    Ok(())
}

/// Create custom systemd service content
#[cfg(target_os = "linux")]
fn create_systemd_service_content(exe_path: &std::path::Path) -> Option<String> {
    let exe_path_str = exe_path.to_string_lossy();
    Some(format!(
        r#"[Unit]
Description={}
After=network.target

[Service]
Type=simple
ExecStart={} run -c {}
ExecStop=/usr/bin/pkill -9 {}
ExecReload=/usr/bin/pkill -HUP {}
Restart=on-failure
RestartSec=1
KillMode=process

[Install]
WantedBy=multi-user.target
"#,
        SERVICE_DESCRIPTION,
        exe_path_str,
        DEFAULT_CONFIG_PATH,
        SERVICE_NAME,
        SERVICE_NAME
    ))
}

/// For non-Linux platforms, no custom content
#[cfg(not(target_os = "linux"))]
fn create_systemd_service_content(_exe_path: &std::path::Path) -> Option<String> {
    None
}

/// Uninstall the service
fn uninstall_service() -> Result<()> {
    info!("Uninstalling {} service...", SERVICE_NAME);

    let manager = get_service_manager()?;
    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();

    // Try to stop the service first
    let _ = manager.stop(ServiceStopCtx {
        label: label.clone(),
    });

    manager.uninstall(ServiceUninstallCtx {
        label: label.clone(),
    })?;

    info!("✓ Service uninstalled successfully");

    Ok(())
}

/// Start the service
fn start_service() -> Result<()> {
    info!("Starting {} service...", SERVICE_NAME);

    let manager = get_service_manager()?;
    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();

    manager.start(ServiceStartCtx {
        label: label.clone(),
    })?;

    info!("✓ Service started successfully");

    Ok(())
}

/// Stop the service
fn stop_service() -> Result<()> {
    info!("Stopping {} service...", SERVICE_NAME);

    let manager = get_service_manager()?;
    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();

    manager.stop(ServiceStopCtx {
        label: label.clone(),
    })?;

    info!("✓ Service stopped successfully");

    Ok(())
}

/// Restart the service
fn restart_service() -> Result<()> {
    info!("Restarting {} service...", SERVICE_NAME);

    // Stop and start the service
    #[cfg(unix)]
    {
        use std::process::Command;
        // pkill -9 nylon
        let output = Command::new("pkill")
            .args(["-9", SERVICE_NAME])
            .output()?;

        if !output.status.success() {
            error!("Failed to stop service: {}", output.status);
            return Err(ServiceError::Operation("Failed to stop service".to_string()));
        }

        // wait a moment for clean shutdown
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    #[cfg(windows)]
    {
        // On Windows, we restart the service
        info!("Restart is not supported on Windows, restarting service instead...");
        stop_service()?;
    }


    // start
    match start_service() {
        Ok(_) => {
            println!("Service is running");
        }
        Err(e) => {
            error!("Failed to start service: {}", e);
            return Err(e);
        }
    }

    info!("✓ Service restarted successfully");

    Ok(())
}

/// Get service status
fn status_service() -> Result<()> {
    let manager = get_service_manager()?;
    let label: ServiceLabel = SERVICE_NAME.parse().unwrap();

    match manager.status(ServiceStatusCtx {
        label: label.clone(),
    }) {
        Ok(status) => {
            info!("Service Status:");
            info!("  Name: {}", SERVICE_NAME);
            info!("  Description: {}", SERVICE_DESCRIPTION);

            match status {
                ServiceStatus::Running => {
                    info!("  State: ✓ Running");
                }
                ServiceStatus::Stopped(reason) => {
                    info!("  State: ✗ Stopped");
                    if let Some(reason_str) = reason {
                        info!("  Reason: {}", reason_str);
                    }
                }
                ServiceStatus::NotInstalled => {
                    info!("  State: ✗ Not Installed");
                    info!("  Tip: Run 'nylon service install' to install the service");
                }
            }
        }
        Err(e) => {
            error!("Failed to get service status: {}", e);
            return Err(ServiceError::Io(e));
        }
    }

    Ok(())
}

/// Reload the service configuration
fn reload_service() -> Result<()> {
    info!("Reloading {} service configuration...", SERVICE_NAME);

    // For reload, we need to send a signal to the running process
    // This is platform-specific and might require additional implementation

    #[cfg(unix)]
    {
        use std::process::Command;

        // Try to send SIGHUP to the service
        let output = Command::new("pkill")
            .args(["-HUP", SERVICE_NAME])
            .output()?;

        if output.status.success() {
            info!("✓ Service configuration reloaded successfully");
        } else {
            error!("Failed to reload service configuration");
            return Err(ServiceError::Operation(
                "Failed to send reload signal".to_string(),
            ));
        }
    }

    #[cfg(windows)]
    {
        // On Windows, we restart the service
        info!("Reload is not supported on Windows, restarting service instead...");
        restart_service()?;
    }

    Ok(())
}
