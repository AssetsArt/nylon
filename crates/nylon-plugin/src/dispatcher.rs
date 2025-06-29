use crate::{loaders::FfiPlugin, stream};
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
    plugin: Option<&str>,
    entry: Option<&str>,
    dispatch_data: &[u8],
    payload: &Option<Vec<u8>>,
) -> Result<Vec<u8>, NylonError> {
    let request_id = &ctx.request_id;
    let (plugin_name, entry) = match plugin {
        Some(p) => (
            p,
            match entry {
                Some(e) => e,
                None => return Err(NylonError::ConfigError("Entry not found".to_string())),
            },
        ),
        None => {
            let Some(route) = &ctx.route else {
                return Err(NylonError::ConfigError("Route not found".to_string()));
            };
            let Some(plugin) = &route.service.plugin else {
                return Err(NylonError::ConfigError("Plugin not found".to_string()));
            };
            (plugin.name.as_str(), plugin.entry.as_ref())
        }
    };
    dispatch(
        request_id,
        plugin_name,
        entry,
        dispatch_data,
        payload,
        &ctx.plugin_store,
    )
    .await
}

pub async fn dispatch(
    request_id: &str,
    plugin_name: &str,
    entry_name: &str,
    dispatcher_data: &[u8],
    payload: &Option<Vec<u8>>,
    store: &Option<Vec<u8>>,
) -> Result<Vec<u8>, NylonError> {
    let plugin = get_plugin(plugin_name)?;
    let plugin_free = plugin.plugin_free.clone();
    let plugin_name_str = plugin_name.to_string();

    let mut fbs = flatbuffers::FlatBufferBuilder::new();
    let request_id = fbs.create_string(request_id);
    let name = fbs.create_string(plugin_name);
    let entry = fbs.create_string(entry_name);
    let data_vec = fbs.create_vector(dispatcher_data);
    let payload_vec = fbs.create_vector(payload.as_ref().unwrap_or(&vec![]));
    let store_vec = fbs.create_vector(store.as_ref().unwrap_or(&vec![]));
    let dispatcher = NylonDispatcher::create(
        &mut fbs,
        &NylonDispatcherArgs {
            http_end: false,
            request_id: Some(request_id),
            name: Some(name),
            entry: Some(entry),
            data: Some(data_vec),
            payload: Some(payload_vec),
            store: Some(store_vec),
        },
    );
    fbs.finish(dispatcher, None);
    /*
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
    */

    // let (session_id, mut rx) = stream::open_session_stream().await?;

    // // Spawn task to consume
    // tokio::spawn(async move {
    //     while let Some(chunk) = rx.recv().await {
    //         println!("Rust received chunk: {:?}", String::from_utf8_lossy(&chunk));
    //     }
    // });

    // // Send data to Go
    // for i in 0..3 {
    //     let msg = format!("Rust msg {}", i);
    //     stream::send_to_stream(session_id, msg.as_bytes()).await;
    //     tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // }

    // stream::close_session_stream(session_id).await;

    let session_stream = stream::SessionStream::new(plugin.clone());
    let (_session_id, mut rx) = session_stream.open(entry_name).await?;

    while let Some((method, _)) = rx.recv().await {
        // TODO: handle event
        if method == stream::METHOD_GET_PAYLOAD {
            let payload = payload.as_ref().unwrap_or(&vec![]).clone();
            session_stream
                .event_stream(method, &payload)
                .await?;
        }
        if method == stream::METHOD_NEXT {
            break;
        }
        println!("method: {:?}", method);
    }

    // tokio::task::spawn_blocking(move || {
    //     let ctx_dispatcher = fbs.finished_data();
    //     let ptr = ctx_dispatcher.as_ptr();
    //     let len = ctx_dispatcher.len();
    //     let output = unsafe { entry_fn(ptr, len) };
    //     if output.ptr.is_null() {
    //         return Err(NylonError::ConfigError(
    //             "Plugin entry function returned null".to_string(),
    //         ));
    //     }
    //     let ptr = output.ptr;
    //     let copied = unsafe { std::slice::from_raw_parts(ptr, output.len) }.to_vec();
    //     std::panic::catch_unwind(move || unsafe {
    //         (plugin_free)(ptr);
    //     })
    //     .unwrap_or_else(|e| {
    //         eprintln!(
    //             "plugin_free panic for plugin '{}': {:?}",
    //             plugin_name_str, e
    //         );
    //     });
    //     Ok(copied)
    // })
    // .await
    // .map_err(|e| {
    //     NylonError::ConfigError(format!(
    //         "Plugin {} entry {} panicked: {:?}",
    //         plugin_name, entry_name, e
    //     ))
    // })?

    Ok(vec![])
}
