#![allow(clippy::type_complexity)]
use crate::{route::MiddlewareItem, services::ServiceType, template::Expr};
use pingora::lb::Backend;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Route {
    pub service: String,
    pub service_type: ServiceType,
    pub rewrite: Option<String>,
    pub route_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub path_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
}

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub headers: HashMap<String, String>,
    pub backend: Backend,
    pub client_ip: String,
    pub route: Option<Route>,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            headers: HashMap::new(),
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: "127.0.0.1".to_string(),
            route: None,
        }
    }
}
