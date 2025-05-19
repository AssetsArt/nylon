use std::{collections::HashMap, net::IpAddr};

use nylon_error::NylonError;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct HealthCheck {
    pub enabled: bool,
    pub path: String,
    pub interval: String,
    pub timeout: String,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Endpoint {
    pub ip: String,
    pub port: u16,
    pub weight: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum ServiceType {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "plugin")]
    Plugin,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Algorithm {
    #[serde(rename = "round_robin")]
    RoundRobin,
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "consistent")]
    Consistent,
    #[serde(rename = "weighted")]
    Weighted,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Plugin {
    pub name: String,
    pub entry: String,
    pub payload: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceItem {
    pub name: String,
    pub service_type: ServiceType,
    pub algorithm: Option<Algorithm>,
    pub endpoints: Option<Vec<Endpoint>>,
    pub health_check: Option<HealthCheck>,
    pub plugin: Option<Plugin>,
}

impl Endpoint {
    pub fn is_valid_ip(&self) -> Result<(), NylonError> {
        match self.ip.parse::<IpAddr>() {
            Ok(_) => Ok(()),
            Err(err) => Err(NylonError::ConfigError(format!(
                "Invalid IP address: {}",
                err
            ))),
        }
    }
}

impl HealthCheck {
    pub fn is_valid(&self) -> Result<(), NylonError> {
        if self.interval.is_empty() {
            return Err(NylonError::ConfigError("Interval must be set".to_string()));
        }
        if self.timeout.is_empty() {
            return Err(NylonError::ConfigError("Timeout must be set".to_string()));
        }
        if self.healthy_threshold == 0 {
            return Err(NylonError::ConfigError(
                "Healthy threshold must be set".to_string(),
            ));
        }
        if self.unhealthy_threshold == 0 {
            return Err(NylonError::ConfigError(
                "Unhealthy threshold must be set".to_string(),
            ));
        }
        if self.path.is_empty() {
            return Err(NylonError::ConfigError("Path must be set".to_string()));
        }
        if !self.interval.ends_with("s") {
            return Err(NylonError::ConfigError(
                "Interval must be in the format of [0-9]+[s]".to_string(),
            ));
        }
        if !self.timeout.ends_with("s") {
            return Err(NylonError::ConfigError(
                "Timeout must be in the format of [0-9]+[s]".to_string(),
            ));
        }
        Ok(())
    }
}
