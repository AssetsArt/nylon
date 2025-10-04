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
    #[serde(rename = "static")]
    Static,
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
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StaticConfig {
    /// Root directory to serve files from
    pub root: String,
    /// Default index file to serve for directories (default: index.html)
    pub index: Option<String>,
    /// Enable SPA fallback: on 404, serve index file instead
    pub spa: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceItem {
    pub name: String,
    pub service_type: ServiceType,
    pub algorithm: Option<Algorithm>,
    pub endpoints: Option<Vec<Endpoint>>,
    pub health_check: Option<HealthCheck>,
    pub plugin: Option<Plugin>,
    #[serde(rename = "static")]
    pub static_conf: Option<StaticConfig>,
}
