use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone)]
pub enum PluginPhase {
    Zero,
    RequestFilter,
    ResponseFilter,
    ResponseBodyFilter,
    Logging,
}

impl PluginPhase {
    pub fn to_u8(self) -> u8 {
        match self {
            PluginPhase::Zero => 0,
            PluginPhase::RequestFilter => 1,
            PluginPhase::ResponseFilter => 2,
            PluginPhase::ResponseBodyFilter => 3,
            PluginPhase::Logging => 4,
        }
    }
}

#[repr(C)]
pub struct FfiBuffer {
    pub sid: u32,
    pub phase: u8,
    pub method: u32,
    pub ptr: *const u8,
    pub len: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PluginType {
    #[serde(rename = "wasm")]
    Wasm,
    #[serde(rename = "ffi")]
    Ffi,
    #[serde(rename = "messaging")]
    Messaging,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LifeCycle {
    pub setup: Option<bool>,
    pub shutdown: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PluginItem {
    pub name: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    pub file: Option<String>,
    pub entry: Option<Vec<String>>,
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub messaging: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub max_inflight: Option<u32>,
    #[serde(default)]
    pub overflow_policy: Option<OverflowPolicy>,
    #[serde(default)]
    pub per_phase: Option<HashMap<MessagingPhase, MessagingPhaseConfig>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverflowPolicy {
    Reject,
    Queue,
    Shed,
}

impl Default for OverflowPolicy {
    fn default() -> Self {
        Self::Queue
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MessagingPhase {
    RequestFilter,
    ResponseFilter,
    ResponseBodyFilter,
    Logging,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MessagingPhaseConfig {
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub on_error: Option<MessagingOnError>,
    #[serde(default)]
    pub retry: Option<RetryPolicyConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessagingOnError {
    Continue,
    End,
    Retry,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RetryPolicyConfig {
    #[serde(default)]
    pub max: Option<u32>,
    #[serde(default)]
    pub backoff_ms_initial: Option<u64>,
    #[serde(default)]
    pub backoff_ms_max: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MessagingTlsConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub ca_file: Option<String>,
    #[serde(default)]
    pub cert_file: Option<String>,
    #[serde(default)]
    pub key_file: Option<String>,
    #[serde(default)]
    pub insecure_skip_verify: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MessagingAuthConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub nkey: Option<String>,
    #[serde(default)]
    pub credentials_file: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct MessagingConfig {
    pub name: String,
    pub servers: Vec<String>,
    #[serde(default)]
    pub subject_prefix: Option<String>,
    #[serde(default)]
    pub request_timeout_ms: Option<u64>,
    #[serde(default)]
    pub max_inflight: Option<u32>,
    #[serde(default)]
    pub overflow_policy: Option<OverflowPolicy>,
    #[serde(default)]
    pub retry: Option<RetryPolicyConfig>,
    #[serde(default)]
    pub tls: Option<MessagingTlsConfig>,
    #[serde(default)]
    pub auth: Option<MessagingAuthConfig>,
    #[serde(default)]
    pub default_headers: Option<HashMap<String, String>>,
}

// FFI Plugin
pub type FfiInitializeFn = unsafe extern "C" fn(*const u8, u32);
pub type FfiPluginFreeFn = unsafe extern "C" fn(*mut u8);
pub type FfiRegisterSessionFn =
    unsafe extern "C" fn(u32, *const u8, u32, extern "C" fn(*const FfiBuffer)) -> bool;
pub type FfiEventStreamFn = unsafe extern "C" fn(*const FfiBuffer);
pub type FfiCloseSessionFn = unsafe extern "C" fn(u32);
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
    pub session_id: u32,
}
