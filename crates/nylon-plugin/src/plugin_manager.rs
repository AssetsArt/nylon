//! Plugin management and builtin plugin handling

use crate::{constants::builtin_plugins, types::BuiltinPlugin};
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_types::{
    context::NylonContext,
    plugins::{FfiPlugin, SessionStream},
};
use std::sync::Arc;
use crate::stream::PluginSessionStream;

/// Manages plugins and provides access to builtin plugins
pub struct PluginManager;

impl PluginManager {
    /// Try to get a builtin plugin by name
    pub fn try_builtin(name: &str) -> Option<BuiltinPlugin> {
        // tracing::debug!("Trying builtin plugin: {}", name);
        match name {
            builtin_plugins::REQUEST_HEADER_MODIFIER => Some(BuiltinPlugin::RequestHeaderModifier),
            builtin_plugins::RESPONSE_HEADER_MODIFIER => {
                Some(BuiltinPlugin::ResponseHeaderModifier)
            }
            _ => None,
        }
    }

    /// Check if a plugin name is a request filter
    pub fn is_request_filter(name: &str) -> bool {
        matches!(name, builtin_plugins::REQUEST_HEADER_MODIFIER)
    }

    /// Check if a plugin name is a response filter
    pub fn is_response_filter(name: &str) -> bool {
        matches!(name, builtin_plugins::RESPONSE_HEADER_MODIFIER)
    }

    /// Get a plugin from the store by name
    pub fn get_plugin(name: &str) -> Result<Arc<FfiPlugin>, NylonError> {
        let Some(plugins) =
            &nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS)
        else {
            return Err(NylonError::ConfigError("Plugins not found".to_string()));
        };

        let Some(plugin) = plugins.get(name) else {
            return Err(NylonError::ConfigError(format!(
                "Plugin '{}' not found",
                name
            )));
        };

        Ok(plugin.clone())
    }

    /// Get or create a session stream for a plugin
    pub fn get_or_create_session_stream(
        plugin_name: &str,
        ctx: &mut NylonContext,
    ) -> Result<SessionStream, NylonError> {
        let plugin = Self::get_plugin(plugin_name)?;

        Ok(ctx
            .session_stream
            .entry(plugin_name.to_string())
            .or_insert_with(|| {
                SessionStream::new(plugin)
            })
            .clone())
    }
}
