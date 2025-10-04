use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// WebSocket adapter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketAdapterConfig {
    pub adapter_type: AdapterType,
    pub redis: Option<RedisAdapterConfig>,
    pub cluster: Option<ClusterAdapterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AdapterType {
    #[serde(rename = "memory")]
    Memory,
    #[serde(rename = "redis")]
    Redis,
    #[serde(rename = "cluster")]
    Cluster,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisAdapterConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: Option<u8>,
    pub key_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterAdapterConfig {
    pub nodes: Vec<String>,
    pub key_prefix: Option<String>,
}

/// WebSocket connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnection {
    pub id: String,
    pub session_id: u32,
    pub rooms: Vec<String>,
    pub node_id: String,
    pub connected_at: u64,
    pub metadata: HashMap<String, String>,
}

/// WebSocket room information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketRoom {
    pub name: String,
    pub connections: Vec<String>,
    pub created_at: u64,
    pub metadata: HashMap<String, String>,
}

/// WebSocket event types for cluster communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketEvent {
    /// Connection joined a room
    JoinRoom {
        connection_id: String,
        room: String,
        node_id: String,
    },
    /// Connection left a room
    LeaveRoom {
        connection_id: String,
        room: String,
        node_id: String,
    },
    /// Connection disconnected
    Disconnect {
        connection_id: String,
        node_id: String,
    },
    /// Broadcast message to room
    BroadcastToRoom {
        room: String,
        message: WebSocketMessage,
        exclude_connection: Option<String>,
        sender_node_id: String,
    },
    /// Send message to specific connection
    SendToConnection {
        connection_id: String,
        message: WebSocketMessage,
        sender_node_id: String,
    },
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Close { code: u16, reason: String },
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}

/// Adapter event sender type
pub type AdapterEventSender = mpsc::UnboundedSender<WebSocketEvent>;

/// Adapter event receiver type  
pub type AdapterEventReceiver = mpsc::UnboundedReceiver<WebSocketEvent>;
