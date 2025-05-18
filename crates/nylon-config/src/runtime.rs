use nylon_error::NylonError;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

fn default_config_dir() -> PathBuf {
    PathBuf::from("/etc/nylon/config")
}

fn default_daemon() -> bool {
    true
}

fn default_threads() -> usize {
    let cpus = num_cpus::get();
    let reserved = if cpus >= 6 {
        2
    } else if cpus > 1 {
        1
    } else {
        0
    };
    (cpus - reserved).clamp(1, 16)
}

fn default_grace_period() -> u64 {
    60
}

fn default_shutdown_timeout() -> u64 {
    10
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeConfig {
    /// HTTP listening addresses
    #[serde(default)]
    pub http: Vec<String>,

    /// HTTPS listening addresses
    #[serde(default)]
    pub https: Vec<String>,

    /// Prometheus metrics addresses
    #[serde(default)]
    pub metrics: Vec<String>,

    /// Path to directory containing service and route definitions
    #[serde(default = "default_config_dir")]
    pub config_dir: PathBuf,

    /// Pingora runtime configuration
    #[serde(default)]
    pub pingora: PingoraConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PingoraConfig {
    /// Run in daemon mode
    #[serde(default = "default_daemon")]
    pub daemon: bool,

    /// Number of worker threads
    #[serde(default = "default_threads")]
    pub threads: usize,

    /// Grace period for in-flight connections
    #[serde(default = "default_grace_period")]
    pub grace_period_seconds: u64,

    /// Maximum wait time before forced shutdown
    #[serde(default = "default_shutdown_timeout")]
    pub graceful_shutdown_timeout_seconds: u64,

    /// Max number of upstream keepalive connections
    #[serde(default)]
    pub upstream_keepalive_pool_size: Option<usize>,

    /// Enable work stealing between threads
    #[serde(default)]
    pub work_stealing: Option<bool>,

    /// File path for error logging
    #[serde(default)]
    pub error_log: Option<PathBuf>,

    /// File path for PID file
    #[serde(default)]
    pub pid_file: Option<PathBuf>,

    /// Socket path for zero-downtime upgrade
    #[serde(default)]
    pub upgrade_sock: Option<PathBuf>,

    /// User to drop privileges to
    #[serde(default)]
    pub user: Option<String>,

    /// Group to drop privileges to
    #[serde(default)]
    pub group: Option<String>,

    /// Path to trusted CA certificates
    #[serde(default)]
    pub ca_file: Option<PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            http: vec![],
            https: vec![],
            metrics: vec![],
            config_dir: default_config_dir(),
            pingora: PingoraConfig::default(),
        }
    }
}

impl Default for PingoraConfig {
    fn default() -> Self {
        Self {
            daemon: default_daemon(),
            threads: default_threads(),
            grace_period_seconds: default_grace_period(),
            graceful_shutdown_timeout_seconds: default_shutdown_timeout(),
            upstream_keepalive_pool_size: None,
            work_stealing: None,
            error_log: None,
            pid_file: None,
            upgrade_sock: None,
            user: None,
            group: None,
            ca_file: None,
        }
    }
}

impl FromStr for RuntimeConfig {
    type Err = NylonError;

    /// Parse the runtime config from a string
    ///
    /// # Arguments
    ///
    /// * `s` - The string to parse
    ///
    /// # Returns
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yml::from_str(s).map_err(|e| NylonError::ConfigError(e.to_string()))
    }
}

impl RuntimeConfig {
    /// Load the runtime config from a file
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the config file
    ///
    /// # Returns
    ///
    /// * `Result<Self, NylonError>` - The result of the operation
    pub fn from_file(path: &str) -> Result<Self, NylonError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| NylonError::ConfigError(e.to_string()))?;
        Self::from_str(&content)
    }

    /// Store the runtime config in the store
    ///
    /// # Returns
    ///
    /// * `Result<(), NylonError>` - The result of the operation
    pub fn store(&self) -> Result<(), NylonError> {
        nylon_store::insert(nylon_store::KEY_RUNTIME_CONFIG, self.clone());
        Ok(())
    }

    /// Get the runtime config from the store
    ///
    /// # Returns
    ///
    /// * `Result<Self, NylonError>` - The result of the operation
    pub fn get() -> Result<Self, NylonError> {
        nylon_store::get(nylon_store::KEY_RUNTIME_CONFIG).ok_or(NylonError::ConfigError(
            "Runtime config not found".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let yaml = r#"
http:
  - "127.0.0.1:80"
  - "10.10.0.3:80"
https:
  - "127.0.0.1:443"
  - "10.10.0.3:443"
metrics:
  - "10.10.0.3:6192"
config_dir: /etc/nylon/config
debug: true
pingora:
  daemon: true
  threads: 6
  grace_period_seconds: 60
  graceful_shutdown_timeout_seconds: 10
"#;

        let config = RuntimeConfig::from_str(yaml).unwrap();
        assert_eq!(config.http.len(), 2);
        assert_eq!(config.https.len(), 2);
        assert_eq!(config.metrics.len(), 1);
        assert_eq!(config.config_dir.to_str().unwrap(), "/etc/nylon/config");
        assert!(config.pingora.daemon);
        assert_eq!(config.pingora.threads, 6);
    }
}
