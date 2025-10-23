use std::time::Instant;

/// Transport result type
pub type TransportResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

/// Trace metadata for distributed tracing
#[derive(Clone, Debug, Default)]
pub struct TraceMeta {
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
}

/// Event sent to plugin (from Nylon to plugin)
#[derive(Debug, Clone)]
pub struct TransportEvent {
    pub phase: u8,
    pub method: u32,
    pub data: Vec<u8>,
}

/// Invoke received from plugin (from plugin to Nylon)
#[derive(Debug, Clone)]
pub struct TransportInvoke {
    pub method: u32,
    pub data: Vec<u8>,
    pub request_id: Option<String>,
}

impl TransportInvoke {
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

/// Abstract transport for plugin communication
pub trait PluginTransport: Send {
    /// Send an event to the plugin
    fn send_event(&mut self, ev: TransportEvent) -> TransportResult<()>;
    
    /// Try to receive an invoke from the plugin (non-blocking with deadline)
    /// Returns None if deadline is reached without receiving anything
    fn try_recv_invoke(&mut self, deadline: Instant) -> TransportResult<Option<TransportInvoke>>;
    
    /// Get trace metadata
    fn trace_meta(&self) -> &TraceMeta;
}

