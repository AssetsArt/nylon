# WebSocket over NATS Design (Core NATS Only)

## Overview

Extend the NATS transport to support full WebSocket functionality using **Core NATS Queue Groups** only:
- WebSocket upgrade and connection management
- Bidirectional frame streaming (text, binary, control frames)
- Room-based broadcasting via NATS subjects
- Connection lifecycle (open, message, close, error events)
- **No JetStream required** - uses Core NATS Queue Groups per [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough)

## Simplified Architecture: Core NATS Queue Groups

Using NATS Core with Queue Groups provides automatic load balancing without JetStream complexity.

```
┌────────────────────────────────────────────────────────────┐
│                    Nylon (WebSocket Server)                 │
├────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Client connects → WebSocket upgrade (HTTP 101)          │
│  2. Subscribe to reply inbox: _INBOX.{random}               │
│  3. Publish events to: nylon.ws.{plugin}.events             │
│     with reply-to: _INBOX.{random}                          │
│  4. Workers in queue group process & reply                  │
│                                                              │
└────────────────────────────────────────────────────────────┘
                         ↕ Core NATS (Queue Groups)
┌────────────────────────────────────────────────────────────┐
│         NATS Workers (Queue Group: "ws-workers")            │
├────────────────────────────────────────────────────────────┤
│                                                              │
│  Multiple workers subscribe with SAME queue group:          │
│  - nats.subscribe("nylon.ws.{plugin}.events", "ws-workers") │
│  - NATS automatically load-balances to ONE worker           │
│  - Worker processes and replies via msg.Respond()           │
│  - No state shared between workers                          │
│                                                              │
└────────────────────────────────────────────────────────────┘
```

**Subject Layout (Core NATS Only):**
```
# WebSocket events (with queue groups for load balancing)
nylon.ws.{plugin_name}.events                   → Queue group: "ws-workers"
  ↳ Workers auto-balance via NATS queue groups
  ↳ Reply via msg.Respond() to sender's inbox

# Room broadcasting (fan-out to all subscribers)
nats.ws.room.{room_name}                        → All room members subscribe
  ↳ No queue group = all subscribers receive
  ↳ Publish-subscribe pattern

# Per-session replies (Nylon subscribes to unique inbox)
_INBOX.{random_id}                              → Ephemeral reply inbox per session
  ↳ NATS automatically creates reply channels
  ↳ No manual subscription management needed
```

**How Queue Groups Work:**
Per [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough):
- Multiple workers subscribe to same subject with same queue group name
- NATS randomly selects ONE worker to receive each message
- Automatic load balancing, no coordination needed
- Workers can be added/removed dynamically

**Message Protocol:**
```rust
#[derive(Serialize, Deserialize)]
pub struct WebSocketFrame {
    pub session_id: String,
    pub opcode: u8,        // 0x1=text, 0x2=binary, 0x8=close, 0x9=ping, 0xA=pong
    pub payload: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub event: WebSocketEventType,
    pub session_id: String,
    pub connection_id: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub enum WebSocketEventType {
    Open,
    Close { code: u16, reason: String },
    Error { message: String },
}

#[derive(Serialize, Deserialize)]
pub struct RoomBroadcast {
    pub room: String,
    pub message: WebSocketFrame,
    pub exclude_sender: Option<String>,
}
```

### Simplified Implementation: **Core NATS with Queue Groups**

Keep WebSocket handling in Nylon, use Core NATS Queue Groups for load balancing.

```
Client ←→ Nylon (WS Server) ←→ Core NATS ←→ Workers (Queue Group)
          │                                   │
          ├─ Frame parsing                    ├─ on_message(text)
          ├─ Connection mgmt                  ├─ on_binary(data)  
          ├─ Request/Reply pattern            ├─ msg.Respond()
          └─ Room pub/sub                     └─ broadcast(room)
```

**Benefits:**
- ✅ No JetStream = simpler deployment
- ✅ No state management = workers stateless
- ✅ Auto load-balancing via queue groups
- ✅ Dynamic worker scaling (add/remove anytime)
- ✅ Built-in back-pressure handling

### Phase 1: WebSocket Events with Queue Groups

Extend `messaging_methods.rs` to support WebSocket using Core NATS:

```rust
// New WebSocket methods using Core NATS Queue Groups
methods::WEBSOCKET_UPGRADE => {
    // Setup reply inbox for this session
    let reply_inbox = format!("_INBOX.{}", uuid::Uuid::new_v4());
    let mut reply_sub = nats_client.subscribe(&reply_inbox).await?;
    
    // Send upgrade event to worker queue
    let event = WebSocketEvent {
        event: WebSocketEventType::Open,
        session_id: session_stream.session_id.to_string(),
        connection_id: format!("{}", ctx.connection_id),
        metadata: extract_headers(session),
    };
    
    // Publish to queue group subject with reply-to
    let subject = format!("nylon.ws.{}.events", plugin_name);
    nats_client.publish_with_reply(
        &subject,
        &reply_inbox,  // Workers reply to this inbox
        &encode(&event)?
    ).await?;
    
    // Return 101 Switching Protocols
    send_upgrade_response(session).await?;
    
    // Store reply subscription for this session
    ctx.set_ws_reply_sub(session_id, reply_sub);
    
    // Enter streaming mode
    Ok(Some(PluginResult::new_streaming()))
}

methods::WEBSOCKET_ON_MESSAGE_TEXT => {
    // Forward text message to worker queue (ONE worker will handle it)
    let event = WebSocketEvent {
        event: WebSocketEventType::Message,
        session_id: session_stream.session_id.to_string(),
        data: data,
        opcode: 0x1,  // text
    };
    
    // Publish to queue group - NATS picks one worker
    let subject = format!("nylon.ws.{}.events", plugin_name);
    let reply = nats_client.request(&subject, &encode(&event)?, timeout).await?;
    
    // Parse worker response and send to client
    let response: WebSocketResponse = decode(&reply)?;
    if let Some(frames) = response.frames {
        for frame in frames {
            send_ws_frame_to_client(session, &frame).await?;
        }
    }
    
    Ok(None)
}
```

### Phase 2: Room Broadcasting via Core NATS Pub/Sub

Use NATS publish-subscribe (NO queue group = fan-out to all):

```rust
// Nylon joins room for this session
methods::WEBSOCKET_JOIN_ROOM => {
    let room = String::from_utf8_lossy(&data);
    
    // Subscribe to room subject (NO queue group = receive all)
    let room_subject = format!("nylon.ws.room.{}", room);
    let room_sub = nats_client.subscribe(&room_subject).await?;
    
    // Store subscription for this session
    ctx.add_room_subscription(session_id, room.to_string(), room_sub);
    
    // Notify worker (via queue group)
    let event = WebSocketEvent {
        event: WebSocketEventType::JoinRoom,
        session_id: session_stream.session_id.to_string(),
        room: room.to_string(),
    };
    let subject = format!("nylon.ws.{}.events", plugin_name);
    nats_client.publish(&subject, &encode(&event)?).await?;
    
    Ok(None)
}

// Worker broadcasts to room (called by worker via msg.Respond)
// Worker receives request, processes, then publishes to room
pub async fn handle_broadcast_request(msg: &nats::Message) -> Result<()> {
    let request: BroadcastRequest = decode(&msg.data)?;
    
    // Publish to room subject (all Nylon instances with subscribers receive)
    let room_subject = format!("nylon.ws.room.{}", request.room);
    
    let broadcast = RoomBroadcast {
        room: request.room,
        opcode: 0x1,  // text
        payload: request.message,
        sender_id: request.sender_session_id,
    };
    
    nats_client.publish(&room_subject, &encode(&broadcast)?).await?;
    
    // Reply success to requesting worker
    msg.respond(&encode(&BroadcastResponse { success: true })?)?;
    
    Ok(())
}
```

### Phase 3: Request-Reply Loop with Queue Groups

Simplified loop using NATS request-reply pattern:

```rust
pub async fn execute_websocket_session<T>(
    proxy: &T,
    plugin: Arc<MessagingPlugin>,
    session: &mut Session,
    ctx: &mut NylonContext,
    // ... other params
) -> Result<PluginResult, NylonError> {
    let session_id = generate_session_id();
    let event_subject = format!("nylon.ws.{}.events", plugin.name());
    
    // Subscribe to rooms (pub/sub, no queue group)
    let mut room_subscriptions: HashMap<String, Subscription> = HashMap::new();
    
    // Send WebSocket upgrade event (request-reply to queue group)
    let open_event = WebSocketEvent {
        event: WebSocketEventType::Open,
        session_id: session_id.clone(),
        metadata: extract_ws_headers(session),
    };
    
    // Request to queue group, one worker responds
    let response = nats_client.request(
        &event_subject,
        &encode(&open_event)?,
        Duration::from_secs(5)
    ).await?;
    
    let open_response: WebSocketResponse = decode(&response)?;
    
    // Send 101 Switching Protocols to client
    send_upgrade_response(session).await?;
    
    // WebSocket frame buffer
    let mut read_buf = Vec::with_capacity(4096);
    
    loop {
        tokio::select! {
            // Client → Worker (request-reply per message)
            result = session.read_request_body() => {
                match result {
                    Ok(Some(chunk)) => {
                        read_buf.extend_from_slice(&chunk);
                        
                        // Parse WebSocket frames
                        while let Some(frame) = parse_ws_frame(&mut read_buf)? {
                            let event = WebSocketEvent {
                                event: WebSocketEventType::Message,
                                session_id: session_id.clone(),
                                opcode: frame.opcode,
                                data: frame.payload,
                            };
                            
                            // Request to queue group (ONE worker handles)
                            let response = nats_client.request(
                                &event_subject,
                                &encode(&event)?,
                                Duration::from_millis(5000)
                            ).await?;
                            
                            // Parse response and send frames back to client
                            let ws_response: WebSocketResponse = decode(&response)?;
                            if let Some(frames) = ws_response.frames {
                                for out_frame in frames {
                                    let ws_frame = build_ws_frame(
                                        out_frame.opcode,
                                        &out_frame.payload
                                    );
                                    session.response_duplex_vec(vec![
                                        HttpTask::Body(Some(Bytes::from(ws_frame)), false)
                                    ]).await?;
                                }
                            }
                            
                            // Handle room join/leave
                            if let Some(rooms) = ws_response.join_rooms {
                                for room in rooms {
                                    let room_sub = nats_client.subscribe(
                                        &format!("nylon.ws.room.{}", room)
                                    ).await?;
                                    room_subscriptions.insert(room, room_sub);
                                }
                            }
                        }
                    }
                    Ok(None) | Err(_) => {
                        // Client disconnected - notify worker
                        let close_event = WebSocketEvent {
                            event: WebSocketEventType::Close,
                            session_id: session_id.clone(),
                            code: 1000,
                            reason: "Client disconnected".to_string(),
                        };
                        let _ = nats_client.publish(&event_subject, &encode(&close_event)?).await;
                        return Ok(PluginResult::new(false, true));
                    }
                }
            }
            
            // Room broadcasts (fan-out from other sessions)
            broadcast = receive_room_broadcast(&mut room_subscriptions) => {
                if let Some((room, broadcast)) = broadcast {
                    // Skip if this is our own message
                    if broadcast.sender_id != session_id {
                        let ws_frame = build_ws_frame(
                            broadcast.opcode,
                            &broadcast.payload
                        );
                        session.response_duplex_vec(vec![
                            HttpTask::Body(Some(Bytes::from(ws_frame)), false)
                        ]).await?;
                    }
                }
            }
        }
    }
}

async fn receive_room_broadcast(
    subs: &mut HashMap<String, Subscription>
) -> Option<(String, RoomBroadcast)> {
    // Poll all room subscriptions
    for (room, sub) in subs.iter_mut() {
        if let Ok(Some(msg)) = tokio::time::timeout(
            Duration::from_millis(1),
            sub.next()
        ).await {
            if let Ok(broadcast) = decode::<RoomBroadcast>(&msg.payload) {
                return Some((room.clone(), broadcast));
            }
        }
    }
    None
}
```

## Go SDK Changes (Queue Group Worker)

```go
// sdk/go/sdk/transport_nats_ws.go
type NatsWebSocketTransport struct {
    conn        *nats.Conn
    queueGroup  string  // e.g. "ws-workers"
    pluginName  string
    handlers    map[WebSocketEventType]EventHandler
}

// Initialize subscribes to event subject with queue group
func (t *NatsWebSocketTransport) Start() error {
    subject := fmt.Sprintf("nylon.ws.%s.events", t.pluginName)
    
    // Subscribe with queue group - NATS load balances automatically
    // Per https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough
    _, err := t.conn.QueueSubscribe(subject, t.queueGroup, func(msg *nats.Msg) {
        var event WebSocketEvent
        if err := msgpack.Unmarshal(msg.Data, &event); err != nil {
            msg.Respond([]byte(fmt.Sprintf(`{"error": "%s"}`, err)))
            return
        }
        
        // Process event based on type
        response := t.handleEvent(&event)
        
        // Reply to requesting Nylon instance
        data, _ := msgpack.Marshal(response)
        msg.Respond(data)
    })
    
    return err
}

func (t *NatsWebSocketTransport) handleEvent(event *WebSocketEvent) *WebSocketResponse {
    handler, exists := t.handlers[event.Event]
    if !exists {
        return &WebSocketResponse{Error: "No handler"}
    }
    
    ctx := &Context{
        SessionID: event.SessionID,
        Event:     event,
    }
    
    // Call user handler
    result := handler(ctx)
    
    return &WebSocketResponse{
        Frames:     ctx.OutgoingFrames,
        JoinRooms:  ctx.RoomsToJoin,
        LeaveRooms: ctx.RoomsToLeave,
    }
}

// User-facing API
func (p *NatsPlugin) OnWebSocketOpen(handler func(ctx *Context)) {
    p.transport.handlers[EventOpen] = handler
}

func (p *NatsPlugin) OnWebSocketMessage(handler func(ctx *Context, text string)) {
    p.transport.handlers[EventMessage] = func(ctx *Context) error {
        return handler(ctx, string(ctx.Event.Data))
    }
}

// Worker sends frame back to client (adds to response)
func (ctx *Context) Send(text string) {
    ctx.OutgoingFrames = append(ctx.OutgoingFrames, WebSocketFrame{
        Opcode:  0x1,  // text
        Payload: []byte(text),
    })
}

// Worker joins room (Nylon will subscribe)
func (ctx *Context) JoinRoom(room string) {
    ctx.RoomsToJoin = append(ctx.RoomsToJoin, room)
}

// Worker broadcasts to room (publishes to room subject)
func (ctx *Context) Broadcast(room string, message string) error {
    broadcast := RoomBroadcast{
        Room:      room,
        Opcode:    0x1,
        Payload:   []byte(message),
        SenderID:  ctx.SessionID,
    }
    
    subject := fmt.Sprintf("nylon.ws.room.%s", room)
    data, _ := msgpack.Marshal(broadcast)
    
    return ctx.natsConn.Publish(subject, data)
}
```

**Example Worker:**
```go
plugin := nylon.NewNatsPlugin(nylon.Config{
    Name:       "chat-worker",
    QueueGroup: "chat-workers",  // Multiple workers share this queue
})

plugin.OnWebSocketOpen(func(ctx *nylon.Context) {
    fmt.Printf("Client connected: %s\n", ctx.SessionID)
    ctx.Send("Welcome to chat!")
})

plugin.OnWebSocketMessage(func(ctx *nylon.Context, text string) {
    fmt.Printf("Received: %s\n", text)
    
    // Join room
    if strings.HasPrefix(text, "/join ") {
        room := strings.TrimPrefix(text, "/join ")
        ctx.JoinRoom(room)
        ctx.Send(fmt.Sprintf("Joined room: %s", room))
    } else {
        // Broadcast to all in room
        ctx.Broadcast("general", text)
    }
})

plugin.Start()  // Subscribes to nylon.ws.chat.events with queue group
```

## Performance Considerations (Core NATS Only)

1. **Latency**: NATS Core adds ~0.5-1ms per request-reply hop
2. **Throughput**: NATS Core handles 10M+ msg/s
3. **Backpressure**: Built-in via request-reply timeout
4. **Memory**: No persistence = minimal memory footprint
5. **Scalability**: Queue groups auto-balance across workers
6. **No State**: Workers are completely stateless

## Trade-offs

| Aspect | FFI Direct | NATS Core Queue Groups |
|--------|-----------|------------------------|
| Latency | ~100μs | ~0.5-1ms per hop |
| Scalability | Single node | Horizontal (queue groups) |
| Complexity | Medium | **Low** (no JetStream) |
| State | In-process | **Stateless workers** |
| Failover | No | Yes (auto-rebalance) |
| Deployment | Simple binary | NATS + Workers |
| Dependencies | None | NATS server only |

## Migration Path

1. ✅ **Phase 1**: Keep FFI for WebSocket, NATS for non-WS (current)
2. 🚧 **Phase 2**: Implement WebSocket events with Core NATS Queue Groups
3. ⏳ **Phase 3**: Room broadcasting via pub/sub (no queue group)
4. ⏳ **Phase 4**: Production validation and benchmarking

## Conclusion

**Final Architecture: Core NATS with Queue Groups**

Per [NATS Queue Groups documentation](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough):

✅ **Recommended Approach:**
- Use **Core NATS Queue Groups** for load balancing (no JetStream needed)
- Nylon handles WebSocket protocol (frames, handshake, connection lifecycle)
- Workers receive **high-level events** via request-reply pattern
- One worker per message (automatic via queue groups)
- Room broadcasting via **pub/sub** (no queue group = fan-out to all)
- Completely **stateless workers** - no session state management

✅ **Benefits:**
- Simple deployment (just NATS Core server)
- Auto load-balancing built-in
- Workers can be added/removed dynamically
- No persistence layer needed
- Minimal latency overhead (~0.5-1ms)

✅ **Perfect for:**
- Horizontally scaled plugin workers
- Stateless request processing
- Multi-room chat/broadcast features
- Zero-downtime deployments (rolling updates)

This is the simplest possible architecture while maintaining full WebSocket functionality!

