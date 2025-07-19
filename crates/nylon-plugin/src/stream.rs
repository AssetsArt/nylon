#![allow(clippy::not_unsafe_ptr_arg_deref)]

use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::plugins::{FfiBuffer, FfiPlugin, SessionStream};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering},
    },
};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tracing::debug;

// Active sessions
type SessionSender = mpsc::UnboundedSender<(u32, Vec<u8>)>;

static ACTIVE_SESSIONS: Lazy<RwLock<HashMap<u32, SessionSender>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));
static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);
static SESSION_RX: Lazy<Arc<Mutex<HashMap<u32, Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[unsafe(no_mangle)]
pub extern "C" fn handle_ffi_event(session_id: u32, method: u32, data: *const FfiBuffer) {
    debug!(
        "handle_ffi_event: session_id={}, method={}",
        session_id, method
    );

    if data.is_null() {
        debug!("handle_ffi_event: data is null (no payload)");
        if let Ok(sessions) = ACTIVE_SESSIONS.read() {
            if let Some(sender) = sessions.get(&(session_id)) {
                let _ = sender.send((method, Vec::new()));
            }
        }
        return;
    }

    unsafe {
        let ffi = &*data;
        let len = ffi.len as usize;
        let ptr = ffi.ptr;

        if ptr.is_null() || len == 0 {
            debug!("handle_ffi_event: empty payload");
            if let Ok(sessions) = ACTIVE_SESSIONS.read() {
                if let Some(sender) = sessions.get(&(session_id)) {
                    let _ = sender.send((method, Vec::new()));
                }
            }
            return;
        }

        let mut buf = Vec::with_capacity(len);
        buf.extend_from_slice(std::slice::from_raw_parts(ptr, len));

        if let Ok(sessions) = ACTIVE_SESSIONS.read() {
            if let Some(sender) = sessions.get(&(session_id)) {
                if sender.send((method, buf)).is_err() {
                    // Consumed
                }
            }
        }
    }
}

// === SessionStream trait ===
#[async_trait]
pub trait PluginSessionStream {
    fn new(plugin: Arc<FfiPlugin>, session_id: u32) -> Self;
    async fn open(&self, entry: &str) -> Result<u32, NylonError>;
    async fn event_stream(&self, phase: u8, method: u32, data: &[u8]) -> Result<(), NylonError>;
    async fn close(&self) -> Result<(), NylonError>;
}

#[async_trait]
impl PluginSessionStream for SessionStream {
    fn new(plugin: Arc<FfiPlugin>, session_id: u32) -> Self {
        // let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        // Self { plugin, session_id }
        if session_id == 0 {
            let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
            Self { plugin, session_id }
        } else {
            Self { plugin, session_id }
        }
    }

    async fn open(&self, entry: &str) -> Result<u32, NylonError> {
        let (tx, rx) = mpsc::unbounded_channel();

        {
            let mut sessions = ACTIVE_SESSIONS.write().map_err(|e| {
                NylonError::ConfigError(format!("Failed to lock ACTIVE_SESSIONS: {:?}", e))
            })?;
            sessions.insert(self.session_id, tx);
        }

        unsafe {
            let ok = (*self.plugin.register_session)(
                self.session_id,
                entry.as_ptr(),
                entry.len() as u32,
                handle_ffi_event,
            );
            if !ok {
                if let Ok(mut sessions) = ACTIVE_SESSIONS.write() {
                    sessions.remove(&self.session_id);
                }
                return Err(NylonError::ConfigError(
                    "Failed to register session".to_string(),
                ));
            }
        }
        {
            let mut sessions = SESSION_RX.lock().await;
            sessions.insert(self.session_id, Arc::new(Mutex::new(rx)));
        }
        Ok(self.session_id)
    }

    async fn event_stream(&self, phase: u8, method: u32, data: &[u8]) -> Result<(), NylonError> {
        println!(
            "event_stream: phase={}, method={}, data={:?}",
            phase, method, data
        );
        unsafe {
            (*self.plugin.event_stream)(&FfiBuffer {
                sid: self.session_id,
                phase,
                method,
                ptr: data.as_ptr(),
                len: data.len() as u32,
                capacity: data.len() as u32,
            });
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), NylonError> {
        close_session(self.plugin.clone(), self.session_id).await
    }
}

pub async fn close_session(plugin: Arc<FfiPlugin>, session_id: u32) -> Result<(), NylonError> {
    unsafe {
        (*plugin.close_session)(session_id);
    }

    if let Ok(mut sessions) = ACTIVE_SESSIONS.write() {
        sessions.remove(&session_id);
    }
    Ok(())
}

pub async fn get_rx(
    session_id: u32,
) -> Result<Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>, NylonError> {
    let sessions = SESSION_RX.lock().await;

    sessions
        .get(&session_id)
        .cloned()
        .ok_or_else(|| NylonError::ConfigError(format!("Session {} not found", session_id)))
}

pub async fn remove_rx(session_id: u32) -> Result<(), NylonError> {
    let mut sessions = SESSION_RX.lock().await;

    sessions
        .remove(&session_id)
        .ok_or_else(|| NylonError::ConfigError(format!("Session {} not found", session_id)))?;

    Ok(())
}
