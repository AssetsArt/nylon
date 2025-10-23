use crate::{
    runtime::RuntimeConfig,
    services::{EndpointExt, HealthCheckExt},
    utils::read_dir_recursive,
};
use async_trait::async_trait;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_plugin::loaders;
use nylon_store as store;
use nylon_types::{
    plugins::{MessagingConfig, PluginType},
    proxy::ProxyConfig,
    route::RouteConfig,
    services::{ServiceItem, ServiceType},
    tls::TlsConfig,
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
        serde_yaml_ng::from_str(&content).map_err(|e| NylonError::ConfigError(e.to_string()))
    }

    fn from_dir(dir: &str) -> Result<Self, NylonError> {
        let files = read_dir_recursive(&dir.to_string(), MAX_DEPTH)?;
        let mut config = ProxyConfig::default();
        for file in files {
            let content = std::fs::read_to_string(file)
                .map_err(|e| NylonError::ConfigError(e.to_string()))?;
            let file_config: ProxyConfig = serde_yaml_ng::from_str(&content)
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
        if let Some(new_messaging) = other.messaging {
            if let Some(messaging) = self.messaging.as_mut() {
                messaging.extend(new_messaging);
            } else {
                self.messaging = Some(new_messaging);
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
                return Err(NylonError::ConfigError(format!(
                    "Service name {} is not unique",
                    service.name
                )));
            }
        }
        // check if routes are unique
        let mut seen = std::collections::HashSet::new();
        for route in self.routes.iter().flatten() {
            if !seen.insert(route.name.clone()) {
                return Err(NylonError::ConfigError(format!(
                    "Route name {} is not unique",
                    route.name
                )));
            }
        }
        // check if tls are unique
        let mut seen = std::collections::HashSet::new();
        for tls in self.tls.iter().flatten() {
            for domain in tls.domains.iter() {
                if !seen.insert(domain.clone()) {
                    return Err(NylonError::ConfigError(format!(
                        "TLS domain {} is not unique",
                        domain
                    )));
                }
            }
        }
        // check if plugins are unique
        let mut seen = std::collections::HashSet::new();
        for plugin in self.plugins.iter().flatten() {
            if !seen.insert(plugin.name.clone()) {
                return Err(NylonError::ConfigError(format!(
                    "Plugin name {} is not unique",
                    plugin.name
                )));
            }
        }
        if let Some(messaging) = &self.messaging {
            let mut seen = std::collections::HashSet::new();
            for config in messaging {
                if !seen.insert(config.name.clone()) {
                    return Err(NylonError::ConfigError(format!(
                        "Messaging config name {} is not unique",
                        config.name
                    )));
                }
                if config.servers.is_empty() {
                    return Err(NylonError::ConfigError(format!(
                        "Messaging config {} must specify at least one server",
                        config.name
                    )));
                }
            }
        }
        // check if middleware groups are unique
        let mut seen = std::collections::HashSet::new();
        for (name, _) in self.middleware_groups.iter().flatten() {
            if !seen.insert(name.clone()) {
                return Err(NylonError::ConfigError(format!(
                    "Middleware group name {} is not unique",
                    name
                )));
            }
        }
        // validate http service
        for service in self.services.iter().flatten() {
            if service.service_type == ServiceType::Http {
                // check if host is set
                if service.endpoints.is_none() {
                    return Err(NylonError::ConfigError(format!(
                        "HTTP service {} must have at least one endpoint",
                        service.name
                    )));
                }
                for endpoint in service.endpoints.iter().flatten() {
                    endpoint.is_valid_ip()?;
                    if endpoint.port == 0 {
                        return Err(NylonError::ConfigError(format!(
                            "Endpoint port must be set for {:?}",
                            endpoint
                        )));
                    }
                }
                if let Some(health_check) = &service.health_check {
                    health_check.is_valid()?;
                }
            } else if service.service_type == ServiceType::Plugin {
                if service.plugin.is_none() {
                    return Err(NylonError::ConfigError(format!(
                        "Plugin service {} must have a plugin",
                        service.name
                    )));
                }
                if let Some(plugin) = &service.plugin {
                    if plugin.name.is_empty() {
                        return Err(NylonError::ConfigError(format!(
                            "Plugin name must be set for {}",
                            plugin.name
                        )));
                    }
                    if plugin.entry.is_empty() {
                        return Err(NylonError::ConfigError(format!(
                            "Plugin entry must be set for {}",
                            plugin.name
                        )));
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
        if let Some(plugins) = &self.plugins {
            for plugin in plugins {
                match plugin.plugin_type {
                    PluginType::Ffi => {
                        if plugin.file.as_ref().map(|f| f.is_empty()).unwrap_or(true) {
                            return Err(NylonError::ConfigError(format!(
                                "FFI plugin {} requires a 'file' path",
                                plugin.name
                            )));
                        }
                    }
                    PluginType::Messaging => {
                        let Some(messaging_name) = plugin
                            .messaging
                            .as_ref()
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                        else {
                            return Err(NylonError::ConfigError(format!(
                                "Messaging plugin {} must set 'messaging' reference",
                                plugin.name
                            )));
                        };

                        if let Some(messaging_configs) = &self.messaging {
                            if !messaging_configs
                                .iter()
                                .any(|cfg| cfg.name == messaging_name)
                            {
                                return Err(NylonError::ConfigError(format!(
                                    "Messaging config '{}' referenced by plugin {} not found",
                                    messaging_name, plugin.name
                                )));
                            }
                        } else {
                            return Err(NylonError::ConfigError(format!(
                                "Messaging plugin {} references '{}' but no messaging configs defined",
                                plugin.name, messaging_name
                            )));
                        }
                    }
                    PluginType::Wasm => {
                        return Err(NylonError::ConfigError(
                            "WASM plugins are not supported yet".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    async fn store(&self) -> Result<(), NylonError> {
        // validate
        self.validate()?;

        // store tls (with acme_dir from runtime config)
        let acme_dir = if let Ok(runtime_config) = RuntimeConfig::get() {
            Some(runtime_config.acme.to_string_lossy().to_string())
        } else {
            None
        };
        store::tls::store(
            self.tls.iter().flatten().collect::<Vec<&TlsConfig>>(),
            acme_dir,
        )?;

        // store lb backends
        let services = self
            .services
            .iter()
            .flatten()
            .collect::<Vec<&ServiceItem>>();
        store::lb_backends::store(&services).await?;

        // store routes
        store::routes::store(
            self.routes.iter().flatten().collect::<Vec<&RouteConfig>>(),
            &services,
            &self.middleware_groups,
        )?;

        // store header selector
        store::insert(
            store::KEY_HEADER_SELECTOR,
            self.header_selector
                .clone()
                .unwrap_or(store::DEFAULT_HEADER_SELECTOR.to_string()),
        );

        // store messaging configs for runtime lookup
        let messaging_store: DashMap<String, MessagingConfig> = DashMap::new();
        if let Some(configs) = &self.messaging {
            for config in configs {
                messaging_store.insert(config.name.clone(), config.clone());
            }
        }
        store::insert(store::KEY_MESSAGING_CONFIG, messaging_store);

        // register plugins
        for plugin in self.plugins.iter().flatten() {
            loaders::load(plugin);
        }

        Ok(())
    }
}
