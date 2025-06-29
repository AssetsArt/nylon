#![allow(clippy::type_complexity)]
use crate::{plugins::SessionStream, route::MiddlewareItem, services::ServiceItem, template::Expr};
use pingora::lb::Backend;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Route {
    pub service: ServiceItem,
    pub rewrite: Option<String>,
    pub route_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub path_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
}

#[derive(Debug, Clone)]
pub struct NylonContext {
    pub add_response_header: HashMap<String, String>,
    pub remove_response_header: Vec<String>,
    pub set_response_status: u16,

    pub backend: Backend,
    pub client_ip: String,
    pub route: Option<Route>,
    pub params: Option<HashMap<String, String>>,
    pub request_id: String,
    pub plugin_store: Option<Vec<u8>>,
    pub host: String,
    pub tls: bool,
    pub session_stream: HashMap<String, SessionStream>,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            backend: Backend::new("127.0.0.1:80").expect("Unable to create backend"),
            client_ip: "127.0.0.1".to_string(),
            route: None,
            params: None,
            request_id: Uuid::now_v7().to_string(),
            add_response_header: HashMap::new(),
            remove_response_header: Vec::new(),
            plugin_store: None,
            host: "".to_string(),
            tls: false,
            set_response_status: 200,
            session_stream: HashMap::new(),
        }
    }
}
