#![allow(clippy::not_unsafe_ptr_arg_deref)]

use async_trait::async_trait;
use dashmap::DashMap;
use libc::{c_void, free};
use nylon_error::NylonError;
use nylon_types::plugins::{FfiBuffer, FfiPlugin, PluginPhase, SessionStream};
use nylon_types::websocket::WebSocketMessage;
use once_cell::sync::Lazy;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::UnboundedReceiver as UnboundedWsReceiver;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing::{debug, trace};

// Active sessions
type SessionSender = mpsc::UnboundedSender<(u32, Vec<u8>)>;

#[derive(Clone)]
struct SessionResources {
    sender: SessionSender,
    plugin: Arc<FfiPlugin>,
}

static ACTIVE_SESSIONS: Lazy<DashMap<u32, SessionResources>> = Lazy::new(DashMap::new);
static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);
static SESSION_RX: Lazy<DashMap<u32, Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>>> =
    Lazy::new(DashMap::new);

// WS message receivers per session for cluster/local adapter dispatch
static SESSION_WS_RX: Lazy<DashMap<u32, Arc<Mutex<UnboundedWsReceiver<WebSocketMessage>>>>> =
    Lazy::new(DashMap::new);

#[unsafe(no_mangle)]
pub extern "C" fn handle_ffi_event(data: *const FfiBuffer) {
    let ffi = unsafe { &*data };
    let session_id = ffi.sid;
    let method = ffi.method;
    // println!("handle_ffi_event: session_id={}, method={}, phase={}", session_id, method, ffi.phase);
    // let phase = ffi.phase;
    let len = ffi.len as usize;
    let ptr = ffi.ptr;
    trace!(
        "handle_ffi_event: session_id={}, method={}",
        session_id, method
    );
    // Clone sender first to minimize guard lifetime
    if let Some(sender_entry) = ACTIVE_SESSIONS.get(&session_id) {
        let resources = sender_entry.value().clone();
        drop(sender_entry);
        if ptr.is_null() {
            trace!("handle_ffi_event: null payload");
            // println!("handle_ffi_event: null payload");
            let _ = resources.sender.send((method, Vec::new()));
            return;
        }

        unsafe {
            // Fast copy from raw pointer without creating uninitialized values
            let slice = std::slice::from_raw_parts(ptr, len);
            let buf = slice.to_vec();
            // println!("handle_ffi_event: session_id={}, method={}, phase={}", session_id, method, ffi.phase);
            // println!("handle_ffi_event: buf={:?}", buf);
            // println!("handle_ffi_event: buf len={}", buf.len());
            // println!("handle_ffi_event: buf as string={}", String::from_utf8_lossy(&buf));
            if let Err(_e) = resources.sender.send((method, buf)) {
                debug!("send error: {:?}", session_id);
            }
            (*resources.plugin.plugin_free)(ptr as *mut u8);
        }
    } else {
        trace!("handle_ffi_event: no active session for sid={}", session_id);
        if !ptr.is_null() {
            unsafe {
                free(ptr as *mut c_void);
            }
        }
    }
}

// === SessionStream trait ===
#[async_trait]
pub trait PluginSessionStream {
    fn new(plugin: Arc<FfiPlugin>, session_id: u32) -> Self;
    async fn open(&self, entry: &str) -> Result<u32, NylonError>;
    async fn event_stream(
        &self,
        phase: PluginPhase,
        method: u32,
        data: &[u8],
    ) -> Result<(), NylonError>;
    async fn close(&self) -> Result<(), NylonError>;
}

#[async_trait]
impl PluginSessionStream for SessionStream {
    fn new(plugin: Arc<FfiPlugin>, session_id: u32) -> Self {
        if session_id == 0 {
            let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
            Self { plugin, session_id }
        } else {
            Self { plugin, session_id }
        }
    }

    async fn open(&self, entry: &str) -> Result<u32, NylonError> {
        let (tx, rx) = mpsc::unbounded_channel();
        ACTIVE_SESSIONS.insert(
            self.session_id,
            SessionResources {
                sender: tx.clone(),
                plugin: self.plugin.clone(),
            },
        );

        unsafe {
            let ok = (*self.plugin.register_session)(
                self.session_id,
                entry.as_ptr(),
                entry.len() as u32,
                handle_ffi_event,
            );
            if !ok {
                ACTIVE_SESSIONS.remove(&self.session_id);
                return Err(NylonError::ConfigError(
                    "Failed to register session".to_string(),
                ));
            }
        }
        SESSION_RX.insert(self.session_id, Arc::new(Mutex::new(rx)));
        Ok(self.session_id)
    }

    async fn event_stream(
        &self,
        phase: PluginPhase,
        method: u32,
        data: &[u8],
    ) -> Result<(), NylonError> {
        let ffi_buffer = &FfiBuffer {
            sid: self.session_id,
            phase: phase.to_u8(),
            method,
            ptr: data.as_ptr(),
            len: data.len() as u64,
        };
        unsafe {
            (*self.plugin.event_stream)(ffi_buffer);
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), NylonError> {
        let _ = close_session(self.plugin.clone(), self.session_id).await?;
        Ok(())
    }
}

pub async fn close_session(plugin: Arc<FfiPlugin>, session_id: u32) -> Result<(), NylonError> {
    unsafe {
        (*plugin.close_session)(session_id);
    }
    ACTIVE_SESSIONS.remove(&session_id);
    SESSION_RX.remove(&session_id);
    SESSION_WS_RX.remove(&session_id);
    Ok(())
}

pub fn get_rx(
    session_id: u32,
) -> Result<Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>, NylonError> {
    SESSION_RX
        .get(&session_id)
        .map(|entry| Arc::clone(entry.value()))
        .ok_or_else(|| NylonError::ConfigError(format!("Session {} not found", session_id)))
}

pub async fn set_ws_rx(
    session_id: u32,
    rx: UnboundedWsReceiver<WebSocketMessage>,
) -> Result<(), NylonError> {
    SESSION_WS_RX.insert(session_id, Arc::new(Mutex::new(rx)));
    Ok(())
}

pub fn get_ws_rx(
    session_id: u32,
) -> Result<Arc<Mutex<UnboundedWsReceiver<WebSocketMessage>>>, NylonError> {
    SESSION_WS_RX
        .get(&session_id)
        .map(|entry| Arc::clone(entry.value()))
        .ok_or_else(|| NylonError::ConfigError(format!("WS Session {} not found", session_id)))
}
