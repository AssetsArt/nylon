use crate::loaders::FfiPlugin;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_types::context::NylonContext;
use std::sync::Arc;

fn get_plugin(name: &str) -> Result<Arc<FfiPlugin>, NylonError> {
    let Some(plugins) =
        &nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS)
    else {
        return Err(NylonError::ConfigError("Plugins not found".to_string()));
    };
    let Some(plugin) = plugins.get(name) else {
        return Err(NylonError::ConfigError("Plugin not found".to_string()));
    };
    Ok(plugin.clone())
}

pub async fn http_dispatch(ctx: &mut NylonContext, dispatch: &[u8]) -> Result<Vec<u8>, NylonError> {
    let Some(route) = &ctx.route else {
        return Err(NylonError::ConfigError("Route not found".to_string()));
    };
    let Some(plugin) = &route.service.plugin else {
        return Err(NylonError::ConfigError("Plugin not found".to_string()));
    };
    let entry = &plugin.entry;
    let plugin_name = &plugin.name;
    let plugin = get_plugin(plugin_name)?;

    let Some(entry_fn) = plugin.entry.get(entry) else {
        return Err(NylonError::ConfigError(format!(
            "Plugin {} entry {} not found",
            plugin_name, entry
        )));
    };
    let ptr = dispatch.as_ptr();
    let len = dispatch.len();
    let plugin_free = plugin.plugin_free.clone();
    std::panic::catch_unwind(move || {
        let output = unsafe { entry_fn(ptr, len) };
        if output.ptr.is_null() {
            return Err(NylonError::ConfigError(
                "Plugin entry function returned null".to_string(),
            ));
        }
        let ptr = output.ptr;
        let copied = unsafe { std::slice::from_raw_parts(ptr, output.len) }.to_vec();
        std::panic::catch_unwind(move || unsafe {
            (plugin_free)(ptr);
        })
        .unwrap_or_else(|e| {
            eprintln!("plugin_free panic for plugin '{}': {:?}", plugin_name, e);
        });
        Ok(copied)
    })
    .unwrap_or_else(|e| {
        Err(NylonError::ConfigError(format!(
            "Plugin {} entry {} panicked: {:?}",
            plugin_name, entry, e
        )))
    })
}
