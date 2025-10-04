#![allow(clippy::type_complexity)]

use crate::{plugins::SessionStream, route::MiddlewareItem, services::ServiceItem, template::Expr};
use pingora::lb::Backend;
use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
};

#[derive(Debug, Clone)]
pub struct Route {
    pub service: ServiceItem,
    pub rewrite: Option<String>,
    pub route_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub path_middleware: Option<Vec<(MiddlewareItem, Option<HashMap<String, Vec<Expr>>>)>>,
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
}

#[derive(Debug)]
pub struct NylonContext {
    pub backend: RwLock<Backend>,
    pub client_ip: RwLock<String>,
    pub route: RwLock<Option<Route>>,
    pub params: RwLock<Option<HashMap<String, String>>>,
    pub host: RwLock<String>,
    pub tls: AtomicBool,
    pub session_ids: RwLock<HashMap<String, u32>>,
    pub session_stream: RwLock<HashMap<String, SessionStream>>,
    pub add_response_header: RwLock<HashMap<String, String>>,
    pub remove_response_header: RwLock<Vec<String>>,
    pub set_response_status: AtomicU16,
    pub set_response_body: RwLock<Vec<u8>>,
    pub read_body: AtomicBool,
    pub request_body: RwLock<Vec<u8>>,
    // Caches per request to avoid repeated parsing
    pub cached_query: RwLock<Option<HashMap<String, String>>>,
    pub cached_cookies: RwLock<Option<HashMap<String, String>>>,
}

impl Default for NylonContext {
    fn default() -> Self {
        Self {
            // Backend and routing
            backend: RwLock::new(
                Backend::new("127.0.0.1:80").expect("Unable to create default backend"),
            ),
            client_ip: RwLock::new("127.0.0.1".to_string()),
            route: RwLock::new(None),
            params: RwLock::new(None),
            host: RwLock::new("".to_string()),
            tls: AtomicBool::new(false),
            session_ids: RwLock::new(HashMap::new()),
            session_stream: RwLock::new(HashMap::new()),

            // Response modifications
            add_response_header: RwLock::new(HashMap::new()),
            remove_response_header: RwLock::new(Vec::new()),
            set_response_status: AtomicU16::new(200),
            set_response_body: RwLock::new(Vec::new()),

            // Request modifications
            read_body: AtomicBool::new(false),
            request_body: RwLock::new(Vec::new()),

            // Request caches
            cached_query: RwLock::new(None),
            cached_cookies: RwLock::new(None),
        }
    }
}

impl Clone for NylonContext {
    fn clone(&self) -> Self {
        Self {
            backend: RwLock::new(self.backend.read().expect("lock").clone()),
            client_ip: RwLock::new(self.client_ip.read().expect("lock").clone()),
            route: RwLock::new(self.route.read().expect("lock").clone()),
            params: RwLock::new(self.params.read().expect("lock").clone()),
            host: RwLock::new(self.host.read().expect("lock").clone()),
            tls: AtomicBool::new(self.tls.load(Ordering::Relaxed)),
            session_ids: RwLock::new(self.session_ids.read().expect("lock").clone()),
            session_stream: RwLock::new(self.session_stream.read().expect("lock").clone()),
            add_response_header: RwLock::new(
                self.add_response_header.read().expect("lock").clone(),
            ),
            remove_response_header: RwLock::new(
                self.remove_response_header.read().expect("lock").clone(),
            ),
            set_response_status: AtomicU16::new(self.set_response_status.load(Ordering::Relaxed)),
            set_response_body: RwLock::new(self.set_response_body.read().expect("lock").clone()),
            read_body: AtomicBool::new(self.read_body.load(Ordering::Relaxed)),
            request_body: RwLock::new(self.request_body.read().expect("lock").clone()),
            cached_query: RwLock::new(self.cached_query.read().expect("lock").clone()),
            cached_cookies: RwLock::new(self.cached_cookies.read().expect("lock").clone()),
        }
    }
}
