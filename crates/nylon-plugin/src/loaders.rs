use dashmap::DashMap;
use libloading::{Library, Symbol};
use nylon_types::plugins::PluginItem;
use std::{collections::HashMap, sync::Arc};

const FFI_PLUGIN_FREE: &str = "plugin_free";

#[repr(C)]
pub struct FfiOutput {
    pub ptr: *mut u8,
    pub len: usize,
}
type FfiEntryFn = unsafe extern "C" fn(*const u8, usize) -> FfiOutput;
type FfiPluginFreeFn = unsafe extern "C" fn(*mut u8);

#[derive(Debug)]
pub struct FfiPlugin {
    _lib: Arc<Library>,
    pub entry: HashMap<String, Symbol<'static, FfiEntryFn>>,
    pub plugin_free: Symbol<'static, FfiPluginFreeFn>,
}

pub fn load(plugin: &PluginItem) {
    let file = plugin.file.clone();
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
        let symbol: Symbol<FfiPluginFreeFn> =
            lib.get(FFI_PLUGIN_FREE.as_bytes()).unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", FFI_PLUGIN_FREE);
            });
        std::mem::transmute::<Symbol<FfiPluginFreeFn>, Symbol<'static, FfiPluginFreeFn>>(symbol)
    };
    let mut entry_map = HashMap::new();
    for entry in &plugin.entry.clone().unwrap_or_default() {
        let handle = unsafe {
            let symbol: Symbol<FfiEntryFn> = lib.get(entry.as_bytes()).unwrap_or_else(|_| {
                panic!("Failed to load symbol: {}", entry);
            });
            std::mem::transmute::<Symbol<FfiEntryFn>, Symbol<'static, FfiEntryFn>>(symbol)
        };
        entry_map.insert(entry.clone(), handle);
    }
    let ffi_item = FfiPlugin {
        _lib: lib.clone(),
        entry: entry_map,
        plugin_free,
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
}
