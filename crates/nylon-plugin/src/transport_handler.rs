use nylon_types::transport::{PluginTransport, TransportEvent, TransportInvoke, TransportResult};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Transport-based session handler with request deduplication
pub struct TransportSessionHandler<T: PluginTransport> {
    transport: T,
    seen_request_ids: HashSet<String>,
}

impl<T: PluginTransport> TransportSessionHandler<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            seen_request_ids: HashSet::new(),
        }
    }

    /// Send start phase event
    pub fn start_phase(&mut self, phase: u8) -> TransportResult<()> {
        let ev = TransportEvent {
            phase,
            method: 0,
            data: Vec::new(),
        };
        self.transport.send_event(ev)?;
        Ok(())
    }

    /// Process loop with timeout and deduplication
    pub fn process_loop(&mut self, timeout_ms: u64) -> TransportResult<PluginLoopResult> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        
        loop {
            let maybe = self.transport.try_recv_invoke(deadline)?;
            
            if let Some(inv) = maybe {
                // Deduplicate by request_id
                if let Some(id) = inv.request_id() {
                    if !self.seen_request_ids.insert(id.to_string()) {
                        // Already processed, skip
                        continue;
                    }
                }
                
                // Return invoke to caller for processing
                return Ok(PluginLoopResult::Invoke(inv));
            } else {
                // Timeout reached
                return Ok(PluginLoopResult::Timeout);
            }
        }
    }

    /// Send event back to plugin
    pub fn send_event(&mut self, ev: TransportEvent) -> TransportResult<()> {
        self.transport.send_event(ev)
    }

    /// Get transport reference
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

/// Result of process loop
#[derive(Debug)]
pub enum PluginLoopResult {
    Invoke(TransportInvoke),
    Timeout,
}

