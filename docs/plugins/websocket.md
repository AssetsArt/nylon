# WebSocket Support

Build real-time applications with WebSocket support in Nylon plugins.

## WebSocket Upgrade

Upgrade HTTP connection to WebSocket in `RequestFilter` phase:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
        OnOpen: func(ws *sdk.WebSocketConn) {
            fmt.Println("WebSocket connected")
        },
        OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
            fmt.Printf("Received: %s\n", msg)
            ws.SendText("Echo: " + msg)
        },
        OnClose: func(ws *sdk.WebSocketConn) {
            fmt.Println("WebSocket closed")
        },
    })
    
    if err != nil {
        fmt.Println("Upgrade failed:", err)
        ctx.Next()  // Fallback to HTTP
    }
})
```

## WebSocket Callbacks

### OnOpen

Called when WebSocket connection is established:

```go
OnOpen: func(ws *sdk.WebSocketConn) {
    fmt.Println("[WebSocket] Client connected")
    ws.SendText("Welcome!")
}
```

### OnMessageText

Handle text messages:

```go
OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
    fmt.Printf("[WebSocket] Received text: %s\n", msg)
    
    // Echo back
    ws.SendText("You said: " + msg)
}
```

### OnMessageBinary

Handle binary messages:

```go
OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
    fmt.Printf("[WebSocket] Received %d bytes\n", len(data))
    
    // Echo back
    ws.SendBinary(data)
}
```

### OnClose

Called when connection closes:

```go
OnClose: func(ws *sdk.WebSocketConn) {
    fmt.Println("[WebSocket] Client disconnected")
    
    // Cleanup resources
}
```

### OnError

Handle errors:

```go
OnError: func(ws *sdk.WebSocketConn, err string) {
    fmt.Printf("[WebSocket] Error: %s\n", err)
}
```

## WebSocket Methods

### SendText(message string)

Send text message to client:

```go
ws.SendText("Hello, client!")
```

### SendBinary(data []byte)

Send binary message to client:

```go
data := []byte{0x01, 0x02, 0x03}
ws.SendBinary(data)
```

### Close()

Close WebSocket connection:

```go
ws.Close()
```

## Room Support

### JoinRoom(roomName string)

Join a broadcast room:

```go
OnOpen: func(ws *sdk.WebSocketConn) {
    ws.JoinRoom("lobby")
    fmt.Println("Joined lobby")
}
```

### LeaveRoom(roomName string)

Leave a room:

```go
OnClose: func(ws *sdk.WebSocketConn) {
    ws.LeaveRoom("lobby")
}
```

### BroadcastText(roomName, message string)

Broadcast text to all clients in room:

```go
OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
    // Broadcast to all in lobby
    ws.BroadcastText("lobby", msg)
}
```

### BroadcastBinary(roomName string, data []byte)

Broadcast binary to all clients in room:

```go
OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
    ws.BroadcastBinary("lobby", data)
}
```

## Examples

### Chat Server

```go
plugin.AddPhaseHandler("chat", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnOpen: func(ws *sdk.WebSocketConn) {
                fmt.Println("[Chat] User joined")
                
                // Join default room
                ws.JoinRoom("general")
                
                // Notify others
                ws.BroadcastText("general", "A user joined the chat")
                
                // Send welcome message
                ws.SendText("Welcome to the chat!")
            },
            
            OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
                fmt.Printf("[Chat] Message: %s\n", msg)
                
                // Broadcast to all in room
                ws.BroadcastText("general", msg)
            },
            
            OnClose: func(ws *sdk.WebSocketConn) {
                fmt.Println("[Chat] User left")
                
                // Leave room
                ws.LeaveRoom("general")
                
                // Notify others
                ws.BroadcastText("general", "A user left the chat")
            },
            
            OnError: func(ws *sdk.WebSocketConn, err string) {
                fmt.Printf("[Chat] Error: %s\n", err)
            },
        })
        
        if err != nil {
            res := ctx.Response()
            res.SetStatus(400)
            res.BodyText("WebSocket upgrade failed")
            ctx.RemoveResponseHeader("Content-Length")
            ctx.SetResponseHeader("Transfer-Encoding", "chunked")
            ctx.End()
            return
        }
    })
})
```

### Echo Server

```go
plugin.AddPhaseHandler("echo", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
                ws.SendText("Echo: " + msg)
            },
            
            OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
                ws.SendBinary(data)
            },
        })
    })
})
```

### Multi-Room Chat

```go
var rooms = map[string]bool{
    "general": true,
    "tech": true,
    "random": true,
}

plugin.AddPhaseHandler("multi-chat", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        currentRoom := "general"
        
        ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnOpen: func(ws *sdk.WebSocketConn) {
                ws.JoinRoom(currentRoom)
                ws.SendText("Joined " + currentRoom)
            },
            
            OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
                // Check for room commands
                if strings.HasPrefix(msg, "/join ") {
                    newRoom := strings.TrimPrefix(msg, "/join ")
                    
                    if !rooms[newRoom] {
                        ws.SendText("Room not found")
                        return
                    }
                    
                    // Leave current room
                    ws.LeaveRoom(currentRoom)
                    
                    // Join new room
                    currentRoom = newRoom
                    ws.JoinRoom(currentRoom)
                    ws.SendText("Joined " + currentRoom)
                    return
                }
                
                // Broadcast message
                ws.BroadcastText(currentRoom, msg)
            },
            
            OnClose: func(ws *sdk.WebSocketConn) {
                ws.LeaveRoom(currentRoom)
            },
        })
    })
})
```

### JSON-based Communication

```go
type Message struct {
    Type string `json:"type"`
    Data string `json:"data"`
}

plugin.AddPhaseHandler("json-ws", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
                var message Message
                if err := json.Unmarshal([]byte(msg), &message); err != nil {
                    ws.SendText("Invalid JSON")
                    return
                }
                
                switch message.Type {
                case "echo":
                    response := Message{
                        Type: "echo",
                        Data: message.Data,
                    }
                    data, _ := json.Marshal(response)
                    ws.SendText(string(data))
                    
                case "broadcast":
                    ws.BroadcastText("lobby", message.Data)
                    
                default:
                    ws.SendText("Unknown message type")
                }
            },
        })
    })
})
```

### Authentication

```go
plugin.AddPhaseHandler("auth-ws", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        req := ctx.Request()
        
        // Check token in query parameter
        query := req.Query()
        params, _ := url.ParseQuery(query)
        token := params.Get("token")
        
        if !validateToken(token) {
            res := ctx.Response()
            res.SetStatus(401)
            res.BodyText("Unauthorized")
            ctx.RemoveResponseHeader("Content-Length")
            ctx.SetResponseHeader("Transfer-Encoding", "chunked")
            ctx.End()
            return
        }
        
        ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnOpen: func(ws *sdk.WebSocketConn) {
                ws.SendText("Authenticated")
            },
            
            OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
                // Handle authenticated messages
                ws.BroadcastText("authenticated-room", msg)
            },
        })
    })
})
```

### Rate Limiting

```go
var connections = make(map[string]int)
var mu sync.Mutex

plugin.AddPhaseHandler("rate-limited-ws", func(phase *sdk.PhaseHandler) {
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
        req := ctx.Request()
        clientIP := req.ClientIP()
        
        mu.Lock()
        count := connections[clientIP]
        if count >= 5 {
            mu.Unlock()
            res := ctx.Response()
            res.SetStatus(429)
            res.BodyText("Too many connections")
            ctx.RemoveResponseHeader("Content-Length")
            ctx.SetResponseHeader("Transfer-Encoding", "chunked")
            ctx.End()
            return
        }
        connections[clientIP]++
        mu.Unlock()
        
        ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
            OnOpen: func(ws *sdk.WebSocketConn) {
                ws.SendText("Connected")
            },
            
            OnClose: func(ws *sdk.WebSocketConn) {
                mu.Lock()
                connections[clientIP]--
                mu.Unlock()
            },
        })
    })
})
```

## Configuration

### Service Configuration

```yaml
services:
  - name: websocket
    service_type: plugin
    plugin:
      name: ws-plugin
      entry: "ws"

plugins:
  - name: ws-plugin
    type: ffi
    file: ./websocket.so

routes:
  - route:
      type: host
      value: ws.example.com
    name: websocket
    paths:
      - path: /ws
        service:
          name: websocket
```

### WebSocket Adapter

For multi-server deployments, configure a WebSocket adapter:

```yaml
# Redis adapter (shared state)
websocket:
  adapter_type: redis
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"

# Memory adapter (single server)
websocket:
  adapter_type: memory
```

## Client Example

### JavaScript Client

```html
<!DOCTYPE html>
<html>
<head>
    <title>WebSocket Chat</title>
</head>
<body>
    <div id="messages"></div>
    <input id="input" type="text" placeholder="Type message...">
    <button id="send">Send</button>
    
    <script>
        const ws = new WebSocket('ws://localhost:8080/ws');
        
        ws.onopen = () => {
            console.log('Connected');
        };
        
        ws.onmessage = (event) => {
            const messages = document.getElementById('messages');
            messages.innerHTML += '<div>' + event.data + '</div>';
        };
        
        ws.onclose = () => {
            console.log('Disconnected');
        };
        
        ws.onerror = (error) => {
            console.error('Error:', error);
        };
        
        document.getElementById('send').onclick = () => {
            const input = document.getElementById('input');
            ws.send(input.value);
            input.value = '';
        };
    </script>
</body>
</html>
```

## Best Practices

### 1. Always Handle Errors

```go
err := ctx.WebSocketUpgrade(callbacks)
if err != nil {
    res := ctx.Response()
    res.SetStatus(400)
    res.BodyText("Upgrade failed")
    ctx.RemoveResponseHeader("Content-Length")
    ctx.SetResponseHeader("Transfer-Encoding", "chunked")
    ctx.End()
    return
}
```

### 2. Clean Up Resources

```go
OnClose: func(ws *sdk.WebSocketConn) {
    ws.LeaveRoom("lobby")
    // Clean up other resources
}
```

### 3. Validate Input

```go
OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
    if len(msg) > 1000 {
        ws.SendText("Message too long")
        return
    }
    // Process message
}
```

### 4. Use Rooms for Broadcasting

```go
// ✅ Good - Efficient broadcasting
ws.BroadcastText("lobby", msg)

// ❌ Bad - Manual iteration
for _, conn := range allConnections {
    conn.SendText(msg)
}
```

### 5. Handle Reconnections

```go
OnOpen: func(ws *sdk.WebSocketConn) {
    // Store connection ID
    connID := generateID()
    
    // Restore session if needed
    ws.SendText("Connected with ID: " + connID)
}
```

## See Also

- [Examples](/examples/websocket) - WebSocket examples
- [Go SDK](/plugins/go-sdk) - Complete SDK reference
- [Configuration](/core/configuration) - WebSocket configuration

