use nylon_types::plugins::{PluginPhase, SessionStream};
use nylon_types::transport::{PluginTransport, TraceMeta, TransportEvent, TransportInvoke, TransportResult};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use tokio::time;

use crate::stream::PluginSessionStream;

/// FFI-based transport implementation
pub struct FfiTransport {
    session_stream: SessionStream,
    rx: Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>,
    trace: TraceMeta,
}

impl FfiTransport {
    pub fn new(
        session_stream: SessionStream,
        rx: Arc<Mutex<UnboundedReceiver<(u32, Vec<u8>)>>>,
        trace: TraceMeta,
    ) -> Self {
        Self {
            session_stream,
            rx,
            trace,
        }
    }
}

impl PluginTransport for FfiTransport {
    fn send_event(&mut self, ev: TransportEvent) -> TransportResult<()> {
        let phase = match ev.phase {
            0 => PluginPhase::Zero,
            1 => PluginPhase::RequestFilter,
            2 => PluginPhase::ResponseFilter,
            3 => PluginPhase::ResponseBodyFilter,
            4 => PluginPhase::Logging,
            _ => return Err(format!("Invalid phase: {}", ev.phase).into()),
        };
        
        // Spawn async task to send event (FFI session_stream.event_stream is async)
        let session_stream = self.session_stream.clone();
        let method = ev.method;
        let data = ev.data;
        tokio::spawn(async move {
            let _ = session_stream.event_stream(phase, method, &data).await;
        });
        
        Ok(())
    }

    fn try_recv_invoke(&mut self, deadline: Instant) -> TransportResult<Option<TransportInvoke>> {
        let rx = self.rx.clone();
        let duration = deadline.saturating_duration_since(Instant::now());
        
        // Block on async receive with timeout
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let timeout_result = time::timeout(duration, async {
                    let mut rx_guard = rx.lock().await;
                    rx_guard.recv().await
                })
                .await;
                
                match timeout_result {
                    Ok(Some((method, data))) => Ok(Some(TransportInvoke {
                        method,
                        data,
                        request_id: None,
                    })),
                    Ok(None) => Ok(None), // Channel closed
                    Err(_) => Ok(None),   // Timeout
                }
            })
        });
        
        result
    }

    fn trace_meta(&self) -> &TraceMeta {
        &self.trace
    }
}

