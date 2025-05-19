use serde::{Deserialize, Serialize};

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
    pub life_cycle: Option<LifeCycle>,
}
