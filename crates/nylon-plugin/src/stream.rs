use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::plugins::{FfiPlugin, SessionStream};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::mpsc;

type SessionSender = mpsc::UnboundedSender<(usize, Vec<u8>)>;
static ACTIVE_SESSIONS: Lazy<Mutex<HashMap<usize, SessionSender>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static NEXT_SESSION_ID: AtomicUsize = AtomicUsize::new(1);

// method
pub const METHOD_NEXT: usize = 1;
pub const METHOD_END: usize = 2;
pub const METHOD_GET_PAYLOAD: usize = 3;
// response
pub const METHOD_SET_RESPONSE_HEADER: usize = 100;
pub const METHOD_REMOVE_RESPONSE_HEADER: usize = 101;
pub const METHOD_SET_RESPONSE_STATUS: usize = 102;
pub const METHOD_SET_RESPONSE_FULL_BODY: usize = 103;
pub const METHOD_SET_RESPONSE_STREAM_DATA: usize = 104;
pub const METHOD_SET_RESPONSE_STREAM_END: usize = 105;
pub const METHOD_SET_RESPONSE_STREAM_HEADER: usize = 106;
pub const METHOD_READ_RESPONSE_FULL_BODY: usize = 107;
// request
pub const METHOD_READ_REQUEST_FULL_BODY: usize = 200;

extern "C" fn handle_ffi_event(session_id: usize, method: usize, data_ptr: *const u8, len: usize) {
    let mut data = Vec::new();
    if len > 0 {
        data = unsafe { std::slice::from_raw_parts(data_ptr, len) }.to_vec();
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

#[async_trait]
pub trait PluginSessionStream {
    fn new(plugin: Arc<FfiPlugin>) -> Self;
    async fn open(
        &self,
        entry: &str,
    ) -> Result<(usize, mpsc::UnboundedReceiver<(usize, Vec<u8>)>), NylonError>;
    async fn event_stream(&self, method: usize, data: &[u8]) -> Result<(), NylonError>;
    async fn close(&self) -> Result<(), NylonError>;
}

#[async_trait]
impl PluginSessionStream for SessionStream {
    fn new(plugin: Arc<FfiPlugin>) -> Self {
        let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        Self { plugin, session_id }
    }

    async fn open(
        &self,
        entry: &str,
    ) -> Result<(usize, mpsc::UnboundedReceiver<(usize, Vec<u8>)>), NylonError> {
        let (tx, rx) = mpsc::unbounded_channel();
        ACTIVE_SESSIONS
            .lock()
            .map_err(|e| {
                NylonError::ConfigError(format!("Failed to lock ACTIVE_SESSIONS: {:?}", e))
            })?
            .insert(self.session_id, tx);
        unsafe {
            let ok = (*self.plugin.register_session)(
                self.session_id,
                entry.as_ptr(),
                entry.len(),
                handle_ffi_event,
            );
            if !ok {
                return Err(NylonError::ConfigError(
                    "Failed to register session".to_string(),
                ));
            }
        }
        Ok((self.session_id, rx))
    }

    async fn event_stream(&self, method: usize, data: &[u8]) -> Result<(), NylonError> {
        unsafe {
            (*self.plugin.event_stream)(self.session_id, method, data.as_ptr(), data.len());
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), NylonError> {
        close_session(self.plugin.clone(), self.session_id).await
    }
}

pub async fn close_session(plugin: Arc<FfiPlugin>, session_id: usize) -> Result<(), NylonError> {
    unsafe {
        (*plugin.close_session)(session_id);
    }
    ACTIVE_SESSIONS
        .lock()
        .map_err(|e| NylonError::ConfigError(format!("Failed to lock ACTIVE_SESSIONS: {:?}", e)))?
        .remove(&session_id);
    Ok(())
}
