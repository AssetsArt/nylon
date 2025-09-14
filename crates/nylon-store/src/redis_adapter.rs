use async_trait::async_trait;
use nylon_error::NylonError;
use nylon_types::websocket::{
    WebSocketConnection, WebSocketRoom, WebSocketEvent, WebSocketMessage, 
    AdapterEventSender, AdapterEventReceiver, RedisAdapterConfig
};
use redis::{Client, AsyncCommands, cmd};
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::StreamExt;
use uuid::Uuid;
use super::websocket_adapter::WebSocketAdapter;

/// Redis-based WebSocket adapter for cluster support
pub struct RedisAdapter {
    client: Arc<Client>,
    config: RedisAdapterConfig,
    node_id: String,
    event_sender: AdapterEventSender,
    #[allow(dead_code)]
    event_receiver: std::sync::Mutex<Option<AdapterEventReceiver>>,
    local_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
}

impl RedisAdapter {
    pub async fn new(config: RedisAdapterConfig) -> Result<Self, NylonError> {
        let redis_url = if let Some(password) = &config.password {
            format!("redis://:{}@{}:{}/{}", 
                password, 
                config.host, 
                config.port, 
                config.db.unwrap_or(0)
            )
        } else {
            format!("redis://{}:{}/{}", 
                config.host, 
                config.port, 
                config.db.unwrap_or(0)
            )
        };
        
        let client = Client::open(redis_url)
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
        
        // Test connection
        let mut conn = client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection test failed: {}", e)))?;
        
        let _: String = cmd("PING").query_async(&mut conn).await
            .map_err(|e| NylonError::ConfigError(format!("Redis ping failed: {}", e)))?;
        
        let (tx, rx) = mpsc::unbounded_channel();
        let node_id = Uuid::new_v4().to_string();
        
        let adapter = Self {
            client: Arc::new(client),
            config,
            node_id: node_id.clone(),
            event_sender: tx,
            event_receiver: std::sync::Mutex::new(Some(rx)),
            local_connections: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Start Redis pub/sub listener
        adapter.start_pubsub_listener().await?;
        // Start heartbeat and janitor tasks
        adapter.start_heartbeat().await?;
        adapter.start_janitor().await?;
        
        Ok(adapter)
    }
    
    async fn start_pubsub_listener(&self) -> Result<(), NylonError> {
        let client = self.client.clone();
        let event_sender = self.event_sender.clone();
        let channel_name = format!("{}:events", self.get_key_prefix());
        
        tokio::spawn(async move {
            loop {
                match client.get_async_connection().await {
                    Ok(conn) => {
                        let mut pubsub = conn.into_pubsub();
                        if let Err(e) = pubsub.subscribe(&channel_name).await {
                            eprintln!("Redis subscribe error: {}", e);
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            continue;
                        }
                        
                        let mut stream = pubsub.on_message();
                        while let Some(msg) = stream.next().await {
                            if let Ok(payload) = msg.get_payload::<String>() {
                                if let Ok(event) = serde_json::from_str::<WebSocketEvent>(&payload) {
                                    let _ = event_sender.send(event);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Redis connection error in pubsub: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    fn get_key_prefix(&self) -> String {
        self.config.key_prefix.clone().unwrap_or_else(|| "nylon:ws".to_string())
    }

    fn node_key(&self, node_id: &str) -> String {
        format!("{}:nodes:{}", self.get_key_prefix(), node_id)
    }

    fn node_connections_key(&self, node_id: &str) -> String {
        format!("{}:node_connections:{}", self.get_key_prefix(), node_id)
    }
    
    #[allow(dead_code)]
    async fn get_connection(&self) -> Result<redis::aio::Connection, NylonError> {
        self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))
    }
    
    async fn publish_event(&self, event: WebSocketEvent) -> Result<(), NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        let channel = format!("{}:events", self.get_key_prefix());
        let payload = serde_json::to_string(&event)
            .map_err(|e| NylonError::ConfigError(format!("Event serialization error: {}", e)))?;
            
        let _: i32 = conn.publish(&channel, payload).await
            .map_err(|e| NylonError::ConfigError(format!("Redis publish error: {}", e)))?;
            
        Ok(())
    }

    async fn start_heartbeat(&self) -> Result<(), NylonError> {
        let client = self.client.clone();
        let node_key = self.node_key(&self.node_id);
        tokio::spawn(async move {
            loop {
                match client.get_async_connection().await {
                    Ok(mut conn) => {
                        let _ : redis::RedisResult<()> = cmd("SET")
                            .arg(&node_key)
                            .arg("1")
                            .arg("EX")
                            .arg(30)
                            .query_async(&mut conn)
                            .await;
                    }
                    Err(_e) => {
                        // ignore, retry later
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });
        Ok(())
    }

    async fn start_janitor(&self) -> Result<(), NylonError> {
        let client = self.client.clone();
        let prefix = self.get_key_prefix();
        tokio::spawn(async move {
            let scan_pattern = format!("{}:node_connections:*", prefix);
            loop {
                if let Ok(mut conn) = client.get_async_connection().await {
                    let mut cursor: u64 = 0;
                    loop {
                        let res: redis::RedisResult<(u64, Vec<String>)> = cmd("SCAN")
                            .arg(cursor)
                            .arg("MATCH").arg(&scan_pattern)
                            .arg("COUNT").arg(200)
                            .query_async(&mut conn)
                            .await;
                        let (next, keys) = match res { Ok(v) => v, Err(_) => break };
                        for key in keys {
                            // extract node_id
                            let node_id = key.split(':').last().unwrap_or("").to_string();
                            let node_key = format!("{}:nodes:{}", prefix, node_id);
                            let exists: redis::RedisResult<i32> = cmd("EXISTS").arg(&node_key).query_async(&mut conn).await;
                            if let Ok(0) = exists {
                                // stale node, clean its connections
                                if let Ok(connections) = redis::cmd("SMEMBERS").arg(&key).query_async::<_, Vec<String>>(&mut conn).await {
                                    for connection_id in connections {
                                        // remove from rooms
                                        let conn_rooms_key = format!("{}:connection_rooms:{}", prefix, connection_id);
                                        if let Ok(rooms) = redis::cmd("SMEMBERS").arg(&conn_rooms_key).query_async::<_, Vec<String>>(&mut conn).await {
                                            for room in rooms {
                                                let room_key = format!("{}:rooms:{}", prefix, room);
                                                let _ : redis::RedisResult<()> = cmd("SREM").arg(&room_key).arg(&connection_id).query_async(&mut conn).await;
                                                // delete room key if empty
                                                if let Ok(0) = cmd("SCARD").arg(&room_key).query_async::<_, i32>(&mut conn).await { let _ : redis::RedisResult<()> = cmd("DEL").arg(&room_key).query_async(&mut conn).await; }
                                            }
                                        }
                                        // delete connection keys
                                        let key_conn = format!("{}:connections:{}", prefix, connection_id);
                                        let _ : redis::RedisResult<()> = cmd("DEL").arg(&key_conn).query_async(&mut conn).await;
                                        let _ : redis::RedisResult<()> = cmd("DEL").arg(&conn_rooms_key).query_async(&mut conn).await;
                                        // remove from node set
                                        let node_conns_key = format!("{}:node_connections:{}", prefix, node_id);
                                        let _ : redis::RedisResult<()> = cmd("SREM").arg(&node_conns_key).arg(&connection_id).query_async(&mut conn).await;
                                    }
                                }
                                // delete empty node set
                                let node_conns_key = format!("{}:node_connections:{}", prefix, node_id);
                                let _ : redis::RedisResult<()> = cmd("DEL").arg(&node_conns_key).query_async(&mut conn).await;
                            }
                        }
                        cursor = next;
                        if cursor == 0 { break; }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        });
        Ok(())
    }
}

#[async_trait]
impl WebSocketAdapter for RedisAdapter {
    async fn add_connection(&self, connection: WebSocketConnection) -> Result<(), NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        let key = format!("{}:connections:{}", self.get_key_prefix(), connection.id);
        let value = serde_json::to_string(&connection)
            .map_err(|e| NylonError::ConfigError(format!("Connection serialization error: {}", e)))?;
            
        let _: () = conn.set(&key, value).await
            .map_err(|e| NylonError::ConfigError(format!("Redis set error: {}", e)))?;

        // add to node connections set for janitor cleanup
        let node_conns_key = self.node_connections_key(&self.node_id);
        let _: () = cmd("SADD").arg(&node_conns_key).arg(&connection.id).query_async(&mut conn).await
            .map_err(|e| NylonError::ConfigError(format!("Redis sadd error: {}", e)))?;
            
        // Store locally for quick access
        let mut local_connections = self.local_connections.write().await;
        local_connections.insert(connection.id.clone(), connection);
        
        Ok(())
    }
    
    async fn remove_connection(&self, connection_id: &str) -> Result<(), NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
        
        // Get connection rooms first
        let rooms = self.get_connection_rooms(connection_id).await?;
        
        // Remove from all rooms
        for room in rooms {
            self.leave_room(connection_id, &room).await?;
        }
        
        // Remove connection
        let key_conn = format!("{}:connections:{}", self.get_key_prefix(), connection_id);
        let key_conn_rooms = format!("{}:connection_rooms:{}", self.get_key_prefix(), connection_id);
        let _: () = conn.del(&key_conn).await
            .map_err(|e| NylonError::ConfigError(format!("Redis del error: {}", e)))?;
        let _: () = conn.del(&key_conn_rooms).await
            .map_err(|e| NylonError::ConfigError(format!("Redis del error: {}", e)))?;
        // remove from node connections set
        let node_conns_key = self.node_connections_key(&self.node_id);
        let _ : () = cmd("SREM").arg(&node_conns_key).arg(connection_id).query_async(&mut conn).await
            .map_err(|e| NylonError::ConfigError(format!("Redis srem error: {}", e)))?;

        // Remove from local cache
        let mut local_connections = self.local_connections.write().await;
        local_connections.remove(connection_id);
        
        // Publish disconnect event
        self.publish_event(WebSocketEvent::Disconnect {
            connection_id: connection_id.to_string(),
            node_id: self.node_id.clone(),
        }).await?;
        
        Ok(())
    }
    
    async fn join_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        // Add to room set
        let room_key = format!("{}:rooms:{}", self.get_key_prefix(), room);
        let _: () = conn.sadd(&room_key, connection_id).await
            .map_err(|e| NylonError::ConfigError(format!("Redis sadd error: {}", e)))?;
            
        // Add to connection rooms set
        let conn_rooms_key = format!("{}:connection_rooms:{}", self.get_key_prefix(), connection_id);
        let _: () = conn.sadd(&conn_rooms_key, room).await
            .map_err(|e| NylonError::ConfigError(format!("Redis sadd error: {}", e)))?;
            
        // Publish join event
        self.publish_event(WebSocketEvent::JoinRoom {
            connection_id: connection_id.to_string(),
            room: room.to_string(),
            node_id: self.node_id.clone(),
        }).await?;
        
        Ok(())
    }
    
    async fn leave_room(&self, connection_id: &str, room: &str) -> Result<(), NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        // Remove from room set
        let room_key = format!("{}:rooms:{}", self.get_key_prefix(), room);
        let _: () = conn.srem(&room_key, connection_id).await
            .map_err(|e| NylonError::ConfigError(format!("Redis srem error: {}", e)))?;
            
        // Remove from connection rooms set
        let conn_rooms_key = format!("{}:connection_rooms:{}", self.get_key_prefix(), connection_id);
        let _: () = conn.srem(&conn_rooms_key, room).await
            .map_err(|e| NylonError::ConfigError(format!("Redis srem error: {}", e)))?;
            
        // Publish leave event
        self.publish_event(WebSocketEvent::LeaveRoom {
            connection_id: connection_id.to_string(),
            room: room.to_string(),
            node_id: self.node_id.clone(),
        }).await?;
        
        // If room becomes empty, optionally delete room key to avoid stale sets
        let remaining: i32 = conn.scard(&room_key).await
            .map_err(|e| NylonError::ConfigError(format!("Redis scard error: {}", e)))?;
        if remaining == 0 {
            let _: () = conn.del(&room_key).await
                .map_err(|e| NylonError::ConfigError(format!("Redis del error: {}", e)))?;
        }

        Ok(())
    }
    
    async fn get_room_connections(&self, room: &str) -> Result<Vec<String>, NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        let room_key = format!("{}:rooms:{}", self.get_key_prefix(), room);
        let connections: Vec<String> = conn.smembers(&room_key).await
            .map_err(|e| NylonError::ConfigError(format!("Redis smembers error: {}", e)))?;
            
        Ok(connections)
    }
    
    async fn get_connection_rooms(&self, connection_id: &str) -> Result<Vec<String>, NylonError> {
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        let conn_rooms_key = format!("{}:connection_rooms:{}", self.get_key_prefix(), connection_id);
        let rooms: Vec<String> = conn.smembers(&conn_rooms_key).await
            .map_err(|e| NylonError::ConfigError(format!("Redis smembers error: {}", e)))?;
            
        Ok(rooms)
    }
    
    async fn broadcast_to_room(
        &self, 
        room: &str, 
        message: WebSocketMessage,
        exclude_connection: Option<&str>
    ) -> Result<(), NylonError> {
        self.publish_event(WebSocketEvent::BroadcastToRoom {
            room: room.to_string(),
            message,
            exclude_connection: exclude_connection.map(|s| s.to_string()),
            sender_node_id: self.node_id.clone(),
        }).await
    }
    
    async fn send_to_connection(
        &self, 
        connection_id: &str, 
        message: WebSocketMessage
    ) -> Result<(), NylonError> {
        self.publish_event(WebSocketEvent::SendToConnection {
            connection_id: connection_id.to_string(),
            message,
            sender_node_id: self.node_id.clone(),
        }).await
    }
    
    async fn get_connection(&self, connection_id: &str) -> Result<Option<WebSocketConnection>, NylonError> {
        // Check local cache first
        {
            let local_connections = self.local_connections.read().await;
            if let Some(connection) = local_connections.get(connection_id) {
                return Ok(Some(connection.clone()));
            }
        }
        
        // Fallback to Redis
        let mut conn = self.client.get_async_connection().await
            .map_err(|e| NylonError::ConfigError(format!("Redis connection error: {}", e)))?;
            
        let key = format!("{}:connections:{}", self.get_key_prefix(), connection_id);
        let value: Option<String> = conn.get(&key).await
            .map_err(|e| NylonError::ConfigError(format!("Redis get error: {}", e)))?;
            
        if let Some(value) = value {
            let connection = serde_json::from_str(&value)
                .map_err(|e| NylonError::ConfigError(format!("Connection deserialization error: {}", e)))?;
            Ok(Some(connection))
        } else {
            Ok(None)
        }
    }
    
    async fn get_room(&self, room: &str) -> Result<Option<WebSocketRoom>, NylonError> {
        let connections = self.get_room_connections(room).await?;
        
        if connections.is_empty() {
            Ok(None)
        } else {
            Ok(Some(WebSocketRoom {
                name: room.to_string(),
                connections,
                created_at: chrono::Utc::now().timestamp() as u64,
                metadata: HashMap::new(),
            }))
        }
    }
    
    fn get_event_receiver(&self) -> Option<AdapterEventReceiver> {
        // provide receiver once
        let mut guard = self.event_receiver.lock().ok()?;
        guard.take()
    }
    
    fn get_node_id(&self) -> String {
        self.node_id.clone()
    }
}
