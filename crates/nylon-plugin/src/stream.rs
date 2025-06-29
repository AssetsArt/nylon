use crate::loaders::FfiPlugin;
use nylon_error::NylonError;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU32, Ordering},
};
use tokio::sync::mpsc;

type SessionSender = mpsc::UnboundedSender<(u32, Vec<u8>)>;
static ACTIVE_SESSIONS: Lazy<Mutex<HashMap<u32, SessionSender>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(1);

// method
pub const METHOD_NEXT: u32 = 1;
pub const METHOD_GET_PAYLOAD: u32 = 2;

extern "C" fn handle_ffi_event(
    session_id: u32,
    method: u32,
    data_ptr: *const u8,
    len: i32,
) {
    let mut data = Vec::new();
    if len > 0 {
        data = unsafe { std::slice::from_raw_parts(data_ptr, len as usize) }.to_vec();
    }
    let sessions = match ACTIVE_SESSIONS.lock() {
        Ok(sessions) => sessions,
        Err(e) => {
            eprintln!("Failed to lock ACTIVE_SESSIONS: {:?}", e);
            return;
        }
    };
    if let Some(sender) = sessions.get(&session_id) {
        let _ = sender.send((method, data));
    } else {
        eprintln!("Unknown session_id={}", session_id);
    }
}

pub struct SessionStream {
    plugin: Arc<FfiPlugin>,
    session_id: u32,
}

impl SessionStream {
    pub fn new(plugin: Arc<FfiPlugin>) -> Self {
        let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        Self { plugin, session_id }
    }

    pub async fn open(
        &self,
        entry: &str,
    ) -> Result<(u32, mpsc::UnboundedReceiver<(u32, Vec<u8>)>), NylonError> {
        let (tx, rx) = mpsc::unbounded_channel();
        unsafe {
            let ok = (*self.plugin.register_session)(
                self.session_id,
                entry.as_ptr(),
                entry.len() as i32,
                handle_ffi_event,
            );
            if !ok {
                return Err(NylonError::ConfigError(
                    "Failed to register session".to_string(),
                ));
            }
        }
        ACTIVE_SESSIONS
            .lock()
            .map_err(|e| {
                NylonError::ConfigError(format!("Failed to lock ACTIVE_SESSIONS: {:?}", e))
            })?
            .insert(self.session_id, tx);
        Ok((self.session_id, rx))
    }

    pub async fn event_stream(
        &self,
        method: u32,
        data: &[u8],
    ) -> Result<(), NylonError> {
        unsafe {
            (*self.plugin.event_stream)(
                self.session_id,
                method,
                data.as_ptr(),
                data.len() as i32,
            );
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<(), NylonError> {
        close_session(self.plugin.clone(), self.session_id).await
    }
}

pub async fn close_session(plugin: Arc<FfiPlugin>, session_id: u32) -> Result<(), NylonError> {
    unsafe {
        (*plugin.close_session)(session_id);
    }
    ACTIVE_SESSIONS
        .lock()
        .map_err(|e| NylonError::ConfigError(format!("Failed to lock ACTIVE_SESSIONS: {:?}", e)))?
        .remove(&session_id);
    Ok(())
}
