#![allow(clippy::type_complexity)]

use crate::{plugins::SessionStream, route::MiddlewareItem, services::ServiceItem, template::Expr};
use pingora::lb::Backend;
use std::collections::HashMap;

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
    pub backend: Backend,
    pub client_ip: String,
    pub route: Option<Route>,
    pub params: Option<HashMap<String, String>>,
    pub host: String,
    pub tls: bool,
    pub session_ids: HashMap<String, u32>,
    pub session_stream: HashMap<String, SessionStream>,
    pub add_response_header: HashMap<String, String>,
    pub remove_response_header: Vec<String>,
    pub set_response_status: u16,
    pub set_response_body: Vec<u8>,
    pub read_body: bool,
    pub request_body: Vec<u8>,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            // Backend and routing
            backend: Backend::new("127.0.0.1:80").expect("Unable to create default backend"),
            client_ip: "127.0.0.1".to_string(),
            route: None,
            params: None,
            host: "".to_string(),
            tls: false,
            session_ids: HashMap::new(),
            session_stream: HashMap::new(),

            // Response modifications
            add_response_header: HashMap::new(),
            remove_response_header: Vec::new(),
            set_response_status: 200,
            set_response_body: Vec::new(),

            // Request modifications
            read_body: false,
            request_body: Vec::new(),
        }
    }
}
