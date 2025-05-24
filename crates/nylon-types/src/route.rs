use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Clone)]
pub struct MiddlewareItem {
    pub group: Option<String>,
    pub plugin: Option<String>,
    pub request_filter: Option<String>,
    pub response_filter: Option<String>,
    pub response_body_filter: Option<String>,
    pub logging: Option<String>,
    pub payload: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RouteConfig {
    pub route: RouteMatcher,
    pub name: String,
    pub tls: Option<TlsRoute>,
    pub middleware: Option<Vec<MiddlewareItem>>,
    pub paths: Vec<PathConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RouteMatcher {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TlsRoute {
    pub name: String,
    pub redirect: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PathConfig {
    pub path: Value,
    pub service: ServiceRef,
    pub middleware: Option<Vec<MiddlewareItem>>,
    pub methods: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceRef {
    pub name: String,
    pub rewrite: Option<String>,
}
