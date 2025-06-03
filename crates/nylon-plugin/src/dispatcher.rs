use crate::loaders::FfiPlugin;
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_sdk::fbs::dispatcher_generated::nylon_dispatcher::{
    NylonDispatcher, NylonDispatcherArgs,
};
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

pub async fn http_service_dispatch(
    ctx: &mut NylonContext,
    dispatch_data: &[u8],
) -> Result<Vec<u8>, NylonError> {
    let request_id = &ctx.request_id;
    let Some(route) = &ctx.route else {
        return Err(NylonError::ConfigError("Route not found".to_string()));
    };
    let Some(plugin) = &route.service.plugin else {
        return Err(NylonError::ConfigError("Plugin not found".to_string()));
    };
    let entry = &plugin.entry;
    let plugin_name = &plugin.name;
    dispatch(request_id, plugin_name, entry, dispatch_data).await
}

pub async fn dispatch(
    request_id: &str,
    plugin_name: &str,
    entry_name: &str,
    dispatcher_data: &[u8],
) -> Result<Vec<u8>, NylonError> {
    let plugin = get_plugin(plugin_name)?;
    let Some(entry_fn) = plugin.entry.get(entry_name) else {
        return Err(NylonError::ConfigError(format!(
            "Plugin {} entry {} not found",
            plugin_name, entry_name
        )));
    };
    let plugin_free = plugin.plugin_free.clone();

    let mut fbs = flatbuffers::FlatBufferBuilder::new();
    let request_id = fbs.create_string(request_id);
    let name = fbs.create_string(plugin_name);
    let entry = fbs.create_string(entry_name);
    let data_vec = fbs.create_vector(dispatcher_data);
    let dispatcher = NylonDispatcher::create(
        &mut fbs,
        &NylonDispatcherArgs {
            http_end: false,
            request_id: Some(request_id),
            name: Some(name),
            entry: Some(entry),
            data: Some(data_vec),
        },
    );
    fbs.finish(dispatcher, None);
    let ctx_dispatcher = fbs.finished_data();
    let ptr = ctx_dispatcher.as_ptr();
    let len = ctx_dispatcher.len();
    std::panic::catch_unwind(move || {
        // println!("dispatch: {:?}", ctx_dispatcher);
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
            plugin_name, entry_name, e
        )))
    })
}
