pub mod lb_backends;
pub mod redis_adapter;
pub mod routes;
pub mod tls;
pub mod websocket_adapter;
pub mod websockets;

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::any::Any;

// default values
pub const DEFAULT_HEADER_SELECTOR: &str = "x-nylon-proxy";

// constants
pub const KEY_RUNTIME_CONFIG: &str = "runtime_config";
pub const KEY_CONFIG_PATH: &str = "config_path";
pub const KEY_COMMAND_SOCKET_PATH: &str = "/tmp/_nylon.sock";
pub const KEY_LB_BACKENDS: &str = "lb_backends";
pub const KEY_ROUTES: &str = "routes";
pub const KEY_TLS_ROUTES: &str = "tls_routes";
pub const KEY_ROUTES_MATCHIT: &str = "routes_matchit";
pub const KEY_HEADER_SELECTOR: &str = "header_selector";
pub const KEY_LIBRARY_FILE: &str = "library_file";
pub const KEY_PLUGINS: &str = "plugins";
pub const KEY_MESSAGING_PLUGINS: &str = "messaging_plugins";
pub const KEY_TLS: &str = "tls";
pub const KEY_MESSAGING_CONFIG: &str = "messaging_config";
pub const KEY_ACME_CERTS: &str = "acme_certs";
pub const KEY_ACME_CONFIG: &str = "acme_config";
pub const KEY_ACME_METRICS: &str = "acme_metrics";

// storage for global variables
static GLOBAL_STORE: Lazy<DashMap<String, Box<dyn Any + Send + Sync>>> = Lazy::new(DashMap::new);

pub fn insert<T: Any + Send + Sync + 'static>(key: &str, value: T) {
    GLOBAL_STORE.insert(key.to_string(), Box::new(value));
}

pub fn get<T: Any + Clone + Send + Sync + 'static>(key: &str) -> Option<T> {
    let entry = GLOBAL_STORE.get(key)?;
    let any_ref = entry.downcast_ref::<T>()?;
    Some(any_ref.clone())
}
