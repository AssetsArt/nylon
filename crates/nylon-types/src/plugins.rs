use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PluginType {
    #[serde(rename = "wasm")]
    Wasm,
    #[serde(rename = "ffi")]
    Ffi,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LifeCycle {
    pub setup: Option<bool>,
    pub shutdown: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginItem {
    pub name: String,
    pub file: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub entry: Option<Vec<String>>,
    pub config: Option<serde_json::Value>,
}

// FFI Plugin
pub type FfiInitializeFn = unsafe extern "C" fn(*const u8, usize);
pub type FfiPluginFreeFn = unsafe extern "C" fn(*mut u8);
pub type FfiRegisterSessionFn = unsafe extern "C" fn(
    usize,
    *const u8,
    usize,
    extern "C" fn(usize, usize, *const u8, usize),
) -> bool;
pub type FfiEventStreamFn = unsafe extern "C" fn(usize, usize, *const u8, usize);
pub type FfiCloseSessionFn = unsafe extern "C" fn(usize);
pub type FfiShutdownFn = unsafe extern "C" fn();

#[derive(Debug)]
pub struct FfiPlugin {
    pub _lib: Arc<Library>,
    pub plugin_free: Symbol<'static, FfiPluginFreeFn>,
    pub register_session: Symbol<'static, FfiRegisterSessionFn>,
    pub event_stream: Symbol<'static, FfiEventStreamFn>,
    pub close_session: Symbol<'static, FfiCloseSessionFn>,
    pub shutdown: Symbol<'static, FfiShutdownFn>,
}

// Plugin Session Stream
#[derive(Debug, Clone)]
pub struct SessionStream {
    pub plugin: Arc<FfiPlugin>,
    pub session_id: usize,
}
