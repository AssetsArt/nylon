use crate::{
    plugins::{MessagingConfig, PluginItem},
    route::{MiddlewareItem, RouteConfig},
    services::ServiceItem,
    tls::TlsConfig,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProxyConfig {
    pub services: Option<Vec<ServiceItem>>,
    pub tls: Option<Vec<TlsConfig>>,
    pub header_selector: Option<String>,
    pub routes: Option<Vec<RouteConfig>>,
    pub plugins: Option<Vec<PluginItem>>,
    pub messaging: Option<Vec<MessagingConfig>>,
    pub middleware_groups: Option<HashMap<String, Vec<MiddlewareItem>>>,
}
