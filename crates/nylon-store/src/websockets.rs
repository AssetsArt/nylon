use crate::websocket_adapter::{MemoryAdapter, WebSocketAdapter};
use dashmap::DashMap;
use nylon_error::NylonError;
use nylon_types::websocket::{
    AdapterType, WebSocketAdapterConfig, WebSocketConnection, WebSocketEvent, WebSocketMessage,
};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

// WebSocket related constants

// RFC 6455 GUID used to compute Sec-WebSocket-Accept
pub const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// Global WebSocket adapter instance
static WEBSOCKET_ADAPTER: Lazy<RwLock<Option<Arc<dyn WebSocketAdapter>>>> =
    Lazy::new(|| RwLock::new(None));

// Local connection senders to push messages to active sessions
static LOCAL_SENDERS: Lazy<DashMap<String, UnboundedSender<WebSocketMessage>>> =
    Lazy::new(|| DashMap::new());

/// Initialize WebSocket adapter with configuration
pub async fn initialize_adapter(config: Option<WebSocketAdapterConfig>) -> Result<(), NylonError> {
    let adapter: Arc<dyn WebSocketAdapter> = match config {
        Some(config) => match config.adapter_type {
            AdapterType::Memory => Arc::new(MemoryAdapter::new()) as Arc<dyn WebSocketAdapter>,
            AdapterType::Redis => {
                let redis_config = config.redis.ok_or_else(|| {
                    NylonError::ConfigError(
                        "Redis configuration required for Redis adapter".to_string(),
                    )
                })?;

                use crate::redis_adapter::RedisAdapter;
                Arc::new(RedisAdapter::new(redis_config).await?) as Arc<dyn WebSocketAdapter>
            }
            AdapterType::Cluster => {
                // For now, cluster uses Redis adapter
                let redis_config = config.redis.ok_or_else(|| {
                    NylonError::ConfigError(
                        "Redis configuration required for Cluster adapter".to_string(),
                    )
                })?;

                use crate::redis_adapter::RedisAdapter;
                Arc::new(RedisAdapter::new(redis_config).await?) as Arc<dyn WebSocketAdapter>
            }
        },
        None => Arc::new(MemoryAdapter::new()) as Arc<dyn WebSocketAdapter>,
    };

    let mut global_adapter = WEBSOCKET_ADAPTER.write().await;
    // Start cluster event dispatcher if adapter provides receiver
    if let Some(mut rx) = adapter.get_event_receiver() {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    WebSocketEvent::SendToConnection {
                        connection_id,
                        message,
                        ..
                    } => {
                        if let Some(sender) = LOCAL_SENDERS.get(&connection_id) {
                            let _ = sender.send(message);
                        }
                    }
                    WebSocketEvent::BroadcastToRoom {
                        room,
                        message,
                        exclude_connection,
                        ..
                    } => {
                        if let Ok(connections) = get_room_connections(&room).await {
                            for cid in connections {
                                if let Some(exclude) = &exclude_connection {
                                    if &cid == exclude {
                                        continue;
                                    }
                                }
                                if let Some(sender) = LOCAL_SENDERS.get(&cid) {
                                    let _ = sender.send(message.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }
    *global_adapter = Some(adapter);

    Ok(())
}

/// Get the global WebSocket adapter
pub async fn get_adapter() -> Result<Arc<dyn WebSocketAdapter>, NylonError> {
    let adapter_guard = WEBSOCKET_ADAPTER.read().await;
    adapter_guard
        .as_ref()
        .ok_or_else(|| NylonError::ConfigError("WebSocket adapter not initialized".to_string()))
        .map(|adapter| adapter.clone())
}

/// Add a WebSocket connection
pub async fn add_connection(connection: WebSocketConnection) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter.add_connection(connection).await
}

/// Remove a WebSocket connection
pub async fn remove_connection(connection_id: &str) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter.remove_connection(connection_id).await
}

/// Join a connection to a room
pub async fn join_room(connection_id: &str, room: &str) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter.join_room(connection_id, room).await
}

/// Leave a connection from a room
pub async fn leave_room(connection_id: &str, room: &str) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter.leave_room(connection_id, room).await
}

/// Broadcast message to all connections in a room
pub async fn broadcast_to_room(
    room: &str,
    message: WebSocketMessage,
    exclude_connection: Option<&str>,
) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter
        .broadcast_to_room(room, message, exclude_connection)
        .await
}

/// Send message to a specific connection
pub async fn send_to_connection(
    connection_id: &str,
    message: WebSocketMessage,
) -> Result<(), NylonError> {
    let adapter = get_adapter().await?;
    adapter.send_to_connection(connection_id, message).await
}

/// Get all connections in a room
pub async fn get_room_connections(room: &str) -> Result<Vec<String>, NylonError> {
    let adapter = get_adapter().await?;
    adapter.get_room_connections(room).await
}

/// Get all rooms for a connection
pub async fn get_connection_rooms(connection_id: &str) -> Result<Vec<String>, NylonError> {
    let adapter = get_adapter().await?;
    adapter.get_connection_rooms(connection_id).await
}

/// Register a local sender for a connection to receive cluster messages
pub fn register_local_sender(connection_id: String, sender: UnboundedSender<WebSocketMessage>) {
    LOCAL_SENDERS.insert(connection_id, sender);
}

/// Unregister a local sender when a connection closes
pub fn unregister_local_sender(connection_id: &str) {
    LOCAL_SENDERS.remove(connection_id);
}

/// Get current node id from adapter
pub async fn get_node_id() -> Result<String, NylonError> {
    let adapter = get_adapter().await?;
    Ok(adapter.get_node_id())
}
