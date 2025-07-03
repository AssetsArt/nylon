//! Types and structures used throughout the plugin system

use nylon_types::{route::MiddlewareItem, template::Expr};
use serde_json::Value;
use std::collections::HashMap;

/// Built-in plugins that are available by default
#[derive(Debug, Clone, PartialEq)]
pub enum BuiltinPlugin {
    RequestHeaderModifier,
    ResponseHeaderModifier,
}

/// Context for middleware execution
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    pub middleware: MiddlewareItem,
    pub payload: Option<Value>,
    pub payload_ast: Option<HashMap<String, Vec<Expr>>>,
    pub params: Option<HashMap<String, String>>,
}

/// Result of plugin execution
#[derive(Debug, Clone, Default)]
pub struct PluginResult {
    pub http_end: bool,
    pub stream_end: bool,
}

impl PluginResult {
    /// Create a new plugin result
    pub fn new(http_end: bool, stream_end: bool) -> Self {
        Self {
            http_end,
            stream_end,
        }
    }
}

/// Plugin execution context
#[derive(Debug)]
pub struct PluginExecutionContext<'a> {
    pub plugin_name: &'a str,
    pub entry: &'a str,
    pub payload: &'a Option<Vec<u8>>,
    pub params: &'a Option<HashMap<String, String>>,
}
