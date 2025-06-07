#![allow(clippy::type_complexity)]
use crate::{route::MiddlewareItem, services::ServiceItem, template::Expr};
use pingora::lb::Backend;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Route {
    pub service: ServiceItem,
    pub rewrite: Option<String>,
    pub route_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub path_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
}

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub backend: Backend,
    pub client_ip: String,
    pub route: Option<Route>,
    pub params: Option<HashMap<String, String>>,
    pub request_id: String,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: "127.0.0.1".to_string(),
            route: None,
            params: None,
            request_id: Uuid::now_v7().to_string(),
        }
    }
}
