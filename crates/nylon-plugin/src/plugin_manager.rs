use crate::{constants::builtin_plugins, messaging::MessagingPlugin, types::BuiltinPlugin};
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_types::plugins::FfiPlugin;
use std::sync::Arc;

pub struct PluginManager;

pub enum PluginHandle {
    Ffi(Arc<FfiPlugin>),
    Messaging(Arc<MessagingPlugin>),
}

impl PluginManager {
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

    pub fn is_request_filter(name: &str) -> bool {
        matches!(name, builtin_plugins::REQUEST_HEADER_MODIFIER)
    }

    pub fn is_response_filter(name: &str) -> bool {
        matches!(name, builtin_plugins::RESPONSE_HEADER_MODIFIER)
    }

    pub fn get_plugin(name: &str) -> Result<PluginHandle, NylonError> {
        if let Some(plugins) =
            &nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS)
        {
            if let Some(plugin) = plugins.get(name) {
                return Ok(PluginHandle::Ffi(plugin.clone()));
            }
        }

        if let Some(plugins) = &nylon_store::get::<DashMap<String, Arc<MessagingPlugin>>>(
            nylon_store::KEY_MESSAGING_PLUGINS,
        ) {
            if let Some(plugin) = plugins.get(name) {
                return Ok(PluginHandle::Messaging(plugin.clone()));
            }
        }

        Err(NylonError::ConfigError(format!(
            "Plugin '{}' not found",
            name
        )))
    }

    //     /// Get or create a session stream for a plugin
    //     pub fn get_or_create_session_stream(
    //         plugin_name: &str,
    //         ctx: &mut NylonContext,
    //     ) -> Result<SessionStream, NylonError> {
    //         let plugin = Self::get_plugin(plugin_name)?;

    //         Ok(ctx
    //             .session_stream
    //             .entry(plugin_name.to_string())
    //             .or_insert_with(|| SessionStream::new(plugin))
    //             .clone())
    //     }
}
