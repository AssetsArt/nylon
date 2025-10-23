use nylon_types::transport::{PluginTransport, TraceMeta, TransportEvent, TransportInvoke, TransportResult};
use std::time::Instant;

/// Messaging-based transport implementation (NATS)
pub struct MessagingTransport {
    /// Buffer for outbound events (to be flushed to NATS reply)
    outbox: Vec<TransportEvent>,
    trace: TraceMeta,
    // TODO: Add NATS client, subjects, etc.
    // client: Arc<NatsClient>,
    // request_subject: String,
    // reply_subject: String,
}

impl MessagingTransport {
    pub fn new(trace: TraceMeta) -> Self {
        Self {
            outbox: Vec::new(),
            trace,
        }
    }
    
    /// Get buffered events to flush
    #[allow(dead_code)]
    pub fn take_outbox(&mut self) -> Vec<TransportEvent> {
        std::mem::take(&mut self.outbox)
    }
}

impl PluginTransport for MessagingTransport {
    fn send_event(&mut self, ev: TransportEvent) -> TransportResult<()> {
        // Buffer event; execute_session will flush to NATS reply
        self.outbox.push(ev);
        Ok(())
    }

    fn try_recv_invoke(&mut self, _deadline: Instant) -> TransportResult<Option<TransportInvoke>> {
        // Phase 1 skeleton: return None
        // Real implementation will poll NATS request-reply here
        Ok(None)
    }

    fn trace_meta(&self) -> &TraceMeta {
        &self.trace
    }
}

