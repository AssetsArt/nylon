use async_trait::async_trait;
use chrono;
use nylon_error::NylonError;
use nylon_types::websocket::{
    AdapterEventReceiver, AdapterEventSender, WebSocketConnection, WebSocketEvent,
    WebSocketMessage, WebSocketRoom,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

/// WebSocket adapter trait for cluster support
#[async_trait]
pub trait WebSocketAdapter: Send + Sync {
    /// Add a connection to the adapter
    async fn add_connection(&self, connection: WebSocketConnection) -> Result<(), NylonError>;

    /// Remove a connection from the adapter
    async fn remove_connection(&self, connection_id: &str) -> Result<(), NylonError>;

    /// Join a connection to a room
    async fn join_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError>;

    /// Leave a connection from a room
    async fn leave_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError>;

    /// Get all connections in a room
    async fn get_room_connections(&self, room: &str) -> Result<Vec<String>, NylonError>;

    /// Get all rooms for a connection
    async fn get_connection_rooms(&self, connection_id: &str) -> Result<Vec<String>, NylonError>;

    /// Broadcast message to all connections in a room
    async fn broadcast_to_room(
        &self,
        room: &str,
        message: WebSocketMessage,
        exclude_connection: Option<&str>,
    ) -> Result<(), NylonError>;

    /// Send message to a specific connection
    async fn send_to_connection(
        &self,
        connection_id: &str,
        message: WebSocketMessage,
    ) -> Result<(), NylonError>;

    /// Get connection info
    async fn get_connection(
        &self,
        connection_id: &str,
    ) -> Result<Option<WebSocketConnection>, NylonError>;

    /// Get room info
    async fn get_room(&self, room: &str) -> Result<Option<WebSocketRoom>, NylonError>;

    /// Get event receiver for cluster events
    fn get_event_receiver(&self) -> Option<AdapterEventReceiver>;

    /// Get node ID
    fn get_node_id(&self) -> String;
}

/// In-memory WebSocket adapter (default)
pub struct MemoryAdapter {
    connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    rooms: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    connection_rooms: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    node_id: String,
    event_sender: Option<AdapterEventSender>,
    #[allow(dead_code)]
    event_receiver: Mutex<Option<AdapterEventReceiver>>,
}

impl MemoryAdapter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            rooms: Arc::new(RwLock::new(HashMap::new())),
            connection_rooms: Arc::new(RwLock::new(HashMap::new())),
            node_id: Uuid::new_v4().to_string(),
            event_sender: Some(tx),
            event_receiver: Mutex::new(Some(rx)),
        }
    }
}

impl Default for MemoryAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WebSocketAdapter for MemoryAdapter {
    async fn add_connection(&self, connection: WebSocketConnection) -> Result<(), NylonError> {
        let mut connections = self.connections.write().await;
        connections.insert(connection.id.clone(), connection);
        Ok(())
    }

    async fn remove_connection(&self, connection_id: &str) -> Result<(), NylonError> {
        // Remove from connections
        let mut connections = self.connections.write().await;
        connections.remove(connection_id);

        // Remove from all rooms
        let mut connection_rooms = self.connection_rooms.write().await;
        if let Some(rooms) = connection_rooms.remove(connection_id) {
            let mut room_connections = self.rooms.write().await;
            for room in rooms {
                if let Some(room_conns) = room_connections.get_mut(&room) {
                    room_conns.remove(connection_id);
                    if room_conns.is_empty() {
                        room_connections.remove(&room);
                    }
                }
            }
        }

        Ok(())
    }

    async fn join_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError> {
        // Add to room
        let mut rooms = self.rooms.write().await;
        rooms
            .entry(room.to_string())
            .or_insert_with(HashSet::new)
            .insert(connection_id.to_string());

        // Add to connection rooms
        let mut connection_rooms = self.connection_rooms.write().await;
        connection_rooms
            .entry(connection_id.to_string())
            .or_insert_with(HashSet::new)
            .insert(room.to_string());

        Ok(())
    }

    async fn leave_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError> {
        // Remove from room
        let mut rooms = self.rooms.write().await;
        if let Some(room_conns) = rooms.get_mut(room) {
            room_conns.remove(connection_id);
            if room_conns.is_empty() {
                rooms.remove(room);
            }
        }

        // Remove from connection rooms
        let mut connection_rooms = self.connection_rooms.write().await;
        if let Some(conn_rooms) = connection_rooms.get_mut(connection_id) {
            conn_rooms.remove(room);
            if conn_rooms.is_empty() {
                connection_rooms.remove(connection_id);
            }
        }

        Ok(())
    }

    async fn get_room_connections(&self, room: &str) -> Result<Vec<String>, NylonError> {
        let rooms = self.rooms.read().await;
        Ok(rooms
            .get(room)
            .map(|conns| conns.iter().cloned().collect())
            .unwrap_or_default())
    }

    async fn get_connection_rooms(&self, connection_id: &str) -> Result<Vec<String>, NylonError> {
        let connection_rooms = self.connection_rooms.read().await;
        Ok(connection_rooms
            .get(connection_id)
            .map(|rooms| rooms.iter().cloned().collect())
            .unwrap_or_default())
    }

    async fn broadcast_to_room(
        &self,
        room: &str,
        message: WebSocketMessage,
        exclude_connection: Option<&str>,
    ) -> Result<(), NylonError> {
        let connections = self.get_room_connections(room).await?;

        for connection_id in connections {
            if let Some(exclude) = exclude_connection
                && connection_id == exclude
            {
                continue;
            }

            // In memory adapter, we just emit event
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(WebSocketEvent::SendToConnection {
                    connection_id,
                    message: message.clone(),
                    sender_node_id: self.node_id.clone(),
                });
            }
        }

        Ok(())
    }

    async fn send_to_connection(
        &self,
        connection_id: &str,
        message: WebSocketMessage,
    ) -> Result<(), NylonError> {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(WebSocketEvent::SendToConnection {
                connection_id: connection_id.to_string(),
                message,
                sender_node_id: self.node_id.clone(),
            });
        }
        Ok(())
    }

    async fn get_connection(
        &self,
        connection_id: &str,
    ) -> Result<Option<WebSocketConnection>, NylonError> {
        let connections = self.connections.read().await;
        Ok(connections.get(connection_id).cloned())
    }

    async fn get_room(&self, room: &str) -> Result<Option<WebSocketRoom>, NylonError> {
        let rooms = self.rooms.read().await;
        if let Some(connections) = rooms.get(room) {
            Ok(Some(WebSocketRoom {
                name: room.to_string(),
                connections: connections.iter().cloned().collect(),
                created_at: chrono::Utc::now().timestamp() as u64,
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    fn get_event_receiver(&self) -> Option<AdapterEventReceiver> {
        // Provide the receiver once to the caller (single-consumer model)
        let mut guard = self.event_receiver.lock().ok()?;
        guard.take()
    }

    fn get_node_id(&self) -> String {
        self.node_id.clone()
    }
}
