use nylon_error::NylonError;
use nylon_types::services::{Endpoint, HealthCheck};
use std::net::IpAddr;

pub trait EndpointExt {
    fn is_valid_ip(&self) -> Result<(), NylonError>;
}

pub trait HealthCheckExt {
    fn is_valid(&self) -> Result<(), NylonError>;
}

impl EndpointExt for Endpoint {
    fn is_valid_ip(&self) -> Result<(), NylonError> {
        match self.ip.parse::<IpAddr>() {
            Ok(_) => Ok(()),
            Err(err) => Err(NylonError::ConfigError(format!(
                "Invalid IP address: {}",
                err
            ))),
        }
    }
}

impl HealthCheckExt for HealthCheck {
    fn is_valid(&self) -> Result<(), NylonError> {
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
