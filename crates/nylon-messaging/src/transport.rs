use crate::{NatsClient, encode_request, decode_response, PROTOCOL_VERSION};
use nylon_types::transport::{PluginTransport, TraceMeta, TransportEvent, TransportInvoke, TransportResult};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use async_nats::Subscriber;
use futures::StreamExt;

/// Messaging-based transport implementation (NATS)
pub struct MessagingTransport {
    /// Buffer for outbound events (to be flushed to NATS reply)
    outbox: Vec<TransportEvent>,
    trace: TraceMeta,
    client: Arc<NatsClient>,
    request_subject: String,
    reply_subscription: Arc<Mutex<Option<Subscriber>>>,
    session_id: u32,
    #[allow(dead_code)]
    plugin_name: String,
}

impl MessagingTransport {
    pub fn new(
        trace: TraceMeta,
        client: Arc<NatsClient>,
        request_subject: String,
        session_id: u32,
        plugin_name: String,
    ) -> Self {
        Self {
            outbox: Vec::new(),
            trace,
            client,
            request_subject,
            reply_subscription: Arc::new(Mutex::new(None)),
            session_id,
            plugin_name,
        }
    }
    
    /// Setup reply subscription for receiving invoke responses
    pub async fn setup_reply_subscription(&self, reply_subject: String) -> TransportResult<()> {
        let subscriber = self
            .client
            .client()
            .subscribe(reply_subject)
            .await
            .map_err(|e| format!("Failed to subscribe to reply subject: {}", e))?;
        
        let mut guard = self.reply_subscription.lock().await;
        *guard = Some(subscriber);
        Ok(())
    }
    
    /// Flush buffered events to NATS by sending them as requests
    pub async fn flush_events(&mut self) -> TransportResult<()> {
        if self.outbox.is_empty() {
            return Ok(());
        }
        
        let events = std::mem::take(&mut self.outbox);
        
        for event in events {
            // Convert TransportEvent to PluginRequest
            let request = crate::protocol::PluginRequest {
                version: PROTOCOL_VERSION,
                request_id: self.trace.request_id.as_ref()
                    .and_then(|s| s.parse::<u128>().ok())
                    .unwrap_or(0),
                session_id: self.session_id,
                phase: event.phase,
                method: event.method,
                data: event.data,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                headers: None,
            };
            
            let payload = encode_request(&request)
                .map_err(|e| format!("Failed to encode request: {}", e))?;
            
            // Fire-and-forget publish (events don't expect replies)
            self.client
                .publish(&self.request_subject, &payload, None)
                .await
                .map_err(|e| format!("Failed to publish event: {}", e))?;
        }
        
        Ok(())
    }
    
    /// Get buffered events count
    pub fn pending_events(&self) -> usize {
        self.outbox.len()
    }
}

impl PluginTransport for MessagingTransport {
    fn send_event(&mut self, ev: TransportEvent) -> TransportResult<()> {
        // Buffer event; caller should call flush_events() periodically
        self.outbox.push(ev);
        Ok(())
    }

    fn try_recv_invoke(&mut self, deadline: Instant) -> TransportResult<Option<TransportInvoke>> {
        // Non-blocking poll of NATS reply subscription
        let subscription = self.reply_subscription.clone();
        let duration = deadline.saturating_duration_since(Instant::now());
        
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut guard = subscription.lock().await;
                let sub = match guard.as_mut() {
                    Some(s) => s,
                    None => return Ok(None), // No subscription setup yet
                };
                
                // Try to receive with timeout
                let timeout_result = tokio::time::timeout(duration, sub.next()).await;
                
                match timeout_result {
                    Ok(Some(msg)) => {
                        // Decode response
                        let response = decode_response(&msg.payload)
                            .map_err(|e| format!("Failed to decode response: {}", e))?;
                        
                        // Check for method invoke
                        if let Some(method) = response.method {
                            Ok(Some(TransportInvoke {
                                method,
                                data: response.data,
                                request_id: Some(response.request_id.to_string()),
                            }))
                        } else {
                            // Response without method, skip
                            Ok(None)
                        }
                    }
                    Ok(None) => Ok(None), // Stream ended
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

