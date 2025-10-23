use crate::{constants::ffi_symbols, messaging};
use dashmap::DashMap;
use libloading::{Library, Symbol};
use nylon_types::plugins::{
    FfiCloseSessionFn, FfiEventStreamFn, FfiInitializeFn, FfiPlugin, FfiPluginFreeFn,
    FfiRegisterSessionFn, FfiShutdownFn, PluginItem, PluginType,
};
use std::sync::Arc;

pub fn load(plugin: &PluginItem) {
    match plugin.plugin_type {
        PluginType::Ffi => load_ffi_plugin(plugin),
        PluginType::Messaging => {
            if let Err(err) = messaging::register_plugin(plugin) {
                tracing::error!(plugin = %plugin.name, error = %err, "failed to register messaging plugin");
            }
        }
        PluginType::Wasm => {
            tracing::warn!(plugin = %plugin.name, "WASM plugins not implemented");
        }
    }
}

fn load_ffi_plugin(plugin: &PluginItem) {
    let Some(file) = plugin.file.clone() else {
        tracing::error!(plugin = %plugin.name, "FFI plugin missing 'file' field");
        return;
    };
    let lib_store =
        match nylon_store::get::<DashMap<String, Arc<Library>>>(nylon_store::KEY_LIBRARY_FILE) {
            Some(lib) => lib,
            None => {
                let new_lib = DashMap::new();
                nylon_store::insert(nylon_store::KEY_LIBRARY_FILE, new_lib.clone());
                new_lib
            }
        };

    let lib = match lib_store.get(&file) {
        Some(lib) => lib,
        None => {
            let lib = unsafe {
                match Library::new(&file) {
                    Ok(lib) => lib,
                    Err(e) => {
                        eprintln!("Failed to load shared library: {}", e);
                        return;
                    }
                }
            };
            lib_store.insert(file.clone(), Arc::new(lib));
            match lib_store.get(&file) {
                Some(lib) => lib,
                None => {
                    eprintln!("Failed to get loaded library");
                    return;
                }
            }
        }
    };
    let plugin_free = unsafe {
        let symbol: Symbol<FfiPluginFreeFn> = lib
            .get(ffi_symbols::PLUGIN_FREE.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::PLUGIN_FREE);
            });
        std::mem::transmute::<Symbol<FfiPluginFreeFn>, Symbol<'static, FfiPluginFreeFn>>(symbol)
    };
    let register_session = unsafe {
        let symbol: Symbol<FfiRegisterSessionFn> = lib
            .get(ffi_symbols::REGISTER_SESSION.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::REGISTER_SESSION);
            });
        std::mem::transmute::<Symbol<FfiRegisterSessionFn>, Symbol<'static, FfiRegisterSessionFn>>(
            symbol,
        )
    };
    let event_stream = unsafe {
        let symbol: Symbol<FfiEventStreamFn> = lib
            .get(ffi_symbols::EVENT_STREAM.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::EVENT_STREAM);
            });
        std::mem::transmute::<Symbol<FfiEventStreamFn>, Symbol<'static, FfiEventStreamFn>>(symbol)
    };
    let close_session = unsafe {
        let symbol: Symbol<FfiCloseSessionFn> = lib
            .get(ffi_symbols::CLOSE_SESSION.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::CLOSE_SESSION);
            });
        std::mem::transmute::<Symbol<FfiCloseSessionFn>, Symbol<'static, FfiCloseSessionFn>>(symbol)
    };
    let shutdown = unsafe {
        let symbol: Symbol<FfiShutdownFn> = lib
            .get(ffi_symbols::SHUTDOWN.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::SHUTDOWN);
            });
        std::mem::transmute::<Symbol<FfiShutdownFn>, Symbol<'static, FfiShutdownFn>>(symbol)
    };

    let ffi_item = FfiPlugin {
        _lib: lib.clone(),
        plugin_free,
        register_session,
        event_stream,
        close_session,
        shutdown,
    };
    let plugins =
        match nylon_store::get::<DashMap<String, Arc<FfiPlugin>>>(nylon_store::KEY_PLUGINS) {
            Some(plugins) => plugins,
            None => {
                let new_plugins = DashMap::new();
                nylon_store::insert(nylon_store::KEY_PLUGINS, new_plugins.clone());
                new_plugins
            }
        };
    plugins.insert(plugin.name.clone(), Arc::new(ffi_item));
    nylon_store::insert(nylon_store::KEY_PLUGINS, plugins);

    // initialize
    let initialize = unsafe {
        let symbol: Symbol<FfiInitializeFn> = lib
            .get(ffi_symbols::INITIALIZE.as_bytes())
            .unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", ffi_symbols::INITIALIZE);
            });
        std::mem::transmute::<Symbol<FfiInitializeFn>, Symbol<'static, FfiInitializeFn>>(symbol)
    };
    let config = match &plugin.config {
        Some(config) => serde_json::to_string(&config).unwrap_or_default(),
        None => "".to_string(),
    };
    let config_ptr = config.as_ptr();
    let config_len = config.len() as u32;
    unsafe {
        initialize(config_ptr, config_len);
    }
}
