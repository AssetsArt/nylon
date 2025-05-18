use crate::{
    plugins::PluginItem,
    route::{MiddlewareItem, RouteConfig},
    services::ServiceItem,
    tls::TlsConfig,
    utils::read_dir_recursive,
};
use nylon_error::NylonError;
use serde::Deserialize;
use std::collections::HashMap;

const MAX_DEPTH: u16 = 10;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxyConfig {
    pub services: Option<Vec<ServiceItem>>,
    pub tls: Option<Vec<TlsConfig>>,
    pub header_selector: Option<String>,
    pub routes: Option<Vec<RouteConfig>>,
    pub plugins: Option<Vec<PluginItem>>,
    pub middleware_groups: Option<HashMap<String, Vec<MiddlewareItem>>>,
}

impl ProxyConfig {
    pub fn from_file(path: &str) -> Result<Self, NylonError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| NylonError::ConfigError(e.to_string()))?;
        serde_yml::from_str(&content).map_err(|e| NylonError::ConfigError(e.to_string()))
    }

    pub fn from_dir(dir: &str) -> Result<Self, NylonError> {
        let files = read_dir_recursive(&dir.to_string(), MAX_DEPTH)?;
        let mut config = ProxyConfig::default();
        for file in files {
            let content = std::fs::read_to_string(file)
                .map_err(|e| NylonError::ConfigError(e.to_string()))?;
            let file_config: ProxyConfig = serde_yml::from_str(&content)
                .map_err(|e| NylonError::ConfigError(e.to_string()))?;
            config.merge(file_config);
        }
        Ok(config)
    }
}

impl ProxyConfig {
    fn merge(&mut self, other: Self) {
        // header_selector
        if let Some(new_header_selector) = other.header_selector {
            self.header_selector = Some(new_header_selector);
        }
        if let Some(new_services) = other.services {
            if let Some(services) = self.services.as_mut() {
                services.extend(new_services);
            } else {
                self.services = Some(new_services);
            }
        }
        if let Some(new_tls) = other.tls {
            if let Some(tls) = self.tls.as_mut() {
                tls.extend(new_tls);
            } else {
                self.tls = Some(new_tls);
            }
        }
        if let Some(new_routes) = other.routes {
            if let Some(routes) = self.routes.as_mut() {
                routes.extend(new_routes);
            } else {
                self.routes = Some(new_routes);
            }
        }
        if let Some(new_plugins) = other.plugins {
            if let Some(plugins) = self.plugins.as_mut() {
                plugins.extend(new_plugins);
            } else {
                self.plugins = Some(new_plugins);
            }
        }
        if let Some(new_middleware_groups) = other.middleware_groups {
            if let Some(middleware_groups) = self.middleware_groups.as_mut() {
                middleware_groups.extend(new_middleware_groups);
            } else {
                self.middleware_groups = Some(new_middleware_groups);
            }
        }
    }
}
