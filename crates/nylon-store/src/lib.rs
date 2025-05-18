use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::any::Any;

// constants
pub const KEY_RUNTIME_CONFIG: &str = "runtime_config";
pub const KEY_COMMAND_SOCKET_PATH: &str = "/tmp/_nylon.sock";

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
