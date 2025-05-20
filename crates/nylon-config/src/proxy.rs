use crate::{
    services::{EndpointExt, HealthCheckExt},
    utils::read_dir_recursive,
};
use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_store::http_discovery::store_backends;
use nylon_types::{
    proxy::ProxyConfig,
    services::{ServiceItem, ServiceType},
};

const MAX_DEPTH: u16 = 10;

#[async_trait]
pub trait ProxyConfigExt {
    fn merge(&mut self, other: ProxyConfig);
    fn validate(&self) -> Result<(), NylonError>;
    async fn store(&self) -> Result<(), NylonError>;
    fn from_file(path: &str) -> Result<ProxyConfig, NylonError>;
    fn from_dir(dir: &str) -> Result<ProxyConfig, NylonError>;
}

#[async_trait]
impl ProxyConfigExt for ProxyConfig {
    fn from_file(path: &str) -> Result<Self, NylonError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| NylonError::ConfigError(e.to_string()))?;
        serde_yml::from_str(&content).map_err(|e| NylonError::ConfigError(e.to_string()))
    }

    fn from_dir(dir: &str) -> Result<Self, NylonError> {
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

    fn validate(&self) -> Result<(), NylonError> {
        // check if services are unique
        let mut seen = std::collections::HashSet::new();
        for service in self.services.iter().flatten() {
            if !seen.insert(service.name.clone()) {
                return Err(NylonError::ConfigError(
                    "Service names must be unique".to_string(),
                ));
            }
        }
        // check if routes are unique
        let mut seen = std::collections::HashSet::new();
        for route in self.routes.iter().flatten() {
            if !seen.insert(route.name.clone()) {
                return Err(NylonError::ConfigError(
                    "Route names must be unique".to_string(),
                ));
            }
        }
        // check if tls are unique
        let mut seen = std::collections::HashSet::new();
        for tls in self.tls.iter().flatten() {
            if !seen.insert(tls.name.clone()) {
                return Err(NylonError::ConfigError(
                    "TLS names must be unique".to_string(),
                ));
            }
        }
        // check if plugins are unique
        let mut seen = std::collections::HashSet::new();
        for plugin in self.plugins.iter().flatten() {
            if !seen.insert(plugin.name.clone()) {
                return Err(NylonError::ConfigError(
                    "Plugin names must be unique".to_string(),
                ));
            }
        }
        // check if middleware groups are unique
        let mut seen = std::collections::HashSet::new();
        for (name, _) in self.middleware_groups.iter().flatten() {
            if !seen.insert(name.clone()) {
                return Err(NylonError::ConfigError(
                    "Middleware group names must be unique".to_string(),
                ));
            }
        }
        // validate http service
        for service in self.services.iter().flatten() {
            if service.service_type == ServiceType::Http {
                // check if host is set
                if service.endpoints.is_none() {
                    return Err(NylonError::ConfigError(
                        "HTTP service must have at least one endpoint".to_string(),
                    ));
                }
                for endpoint in service.endpoints.iter().flatten() {
                    endpoint.is_valid_ip()?;
                    if endpoint.port == 0 {
                        return Err(NylonError::ConfigError(
                            "Endpoint port must be set".to_string(),
                        ));
                    }
                }
                if let Some(health_check) = &service.health_check {
                    health_check.is_valid()?;
                }
            } else if service.service_type == ServiceType::Plugin {
                if service.plugin.is_none() {
                    return Err(NylonError::ConfigError(
                        "Plugin service must have a plugin".to_string(),
                    ));
                }
                if let Some(plugin) = &service.plugin {
                    if plugin.name.is_empty() {
                        return Err(NylonError::ConfigError(
                            "Plugin name must be set".to_string(),
                        ));
                    }
                    if plugin.entry.is_empty() {
                        return Err(NylonError::ConfigError(
                            "Plugin entry must be set".to_string(),
                        ));
                    }
                    // check if plugin exists
                    if !self.plugins.iter().flatten().any(|p| p.name == plugin.name) {
                        return Err(NylonError::ConfigError(format!(
                            "Plugin {} does not exist",
                            plugin.name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    async fn store(&self) -> Result<(), NylonError> {
        // validate
        self.validate()?;

        let services = self
            .services
            .iter()
            .flatten()
            .collect::<Vec<&ServiceItem>>();
        store_backends(services).await?;

        Ok(())
    }
}
