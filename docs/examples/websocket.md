# WebSocket Proxy

Build real-time applications with WebSocket support.

## Simple Echo Server

```go
package main

import "C"
import (
	"fmt"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	plugin.AddPhaseHandler("echo", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
				OnOpen: func(ws *sdk.WebSocketConn) {
					fmt.Println("[Echo] Client connected")
					ws.SendText("Welcome to Echo Server!")
				},
				
				OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
					fmt.Printf("[Echo] Received: %s\n", msg)
					ws.SendText("Echo: " + msg)
				},
				
				OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
					fmt.Printf("[Echo] Received %d bytes\n", len(data))
					ws.SendBinary(data)
				},
				
				OnClose: func(ws *sdk.WebSocketConn) {
					fmt.Println("[Echo] Client disconnected")
				},
				
				OnError: func(ws *sdk.WebSocketConn, err string) {
					fmt.Printf("[Echo] Error: %s\n", err)
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
}
```

**Build:**
```bash
go build -buildmode=plugin -o echo.so plugin.go
```

**Configuration:**
```yaml
plugins:
  - name: ws
    type: ffi
    file: ./echo.so

services:
  - name: websocket
    service_type: plugin
    plugin:
      name: ws
      entry: "echo"

routes:
  - route:
      type: host
      value: localhost
    name: websocket
    paths:
      - path: /ws
        service:
          name: websocket
```

**Client:**
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
    console.log('Connected');
    ws.send('Hello!');
};

ws.onmessage = (event) => {
    console.log('Received:', event.data);
};
```

## Chat Room

```go
plugin.AddPhaseHandler("chat", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
			OnOpen: func(ws *sdk.WebSocketConn) {
				// Join lobby
				ws.JoinRoom("lobby")
				
				// Notify others
				ws.BroadcastText("lobby", "A user joined the chat")
				
				// Welcome message
				ws.SendText("Welcome! You are in the lobby.")
			},
			
			OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
				// Broadcast to everyone in lobby
				ws.BroadcastText("lobby", msg)
			},
			
			OnClose: func(ws *sdk.WebSocketConn) {
				// Leave room
				ws.LeaveRoom("lobby")
				
				// Notify others
				ws.BroadcastText("lobby", "A user left the chat")
			},
		})
	})
})
```

## Multi-Room Chat

```go
plugin.AddPhaseHandler("multi-room", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		currentRoom := "lobby"
		
		ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
			OnOpen: func(ws *sdk.WebSocketConn) {
				ws.JoinRoom(currentRoom)
				ws.SendText("Joined " + currentRoom)
				ws.SendText("Commands: /join <room>, /leave, /rooms")
			},
			
			OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
				// Handle commands
				if strings.HasPrefix(msg, "/join ") {
					newRoom := strings.TrimPrefix(msg, "/join ")
					
					// Leave current room
					ws.LeaveRoom(currentRoom)
					
					// Join new room
					currentRoom = newRoom
					ws.JoinRoom(currentRoom)
					
					ws.SendText("Joined " + currentRoom)
					ws.BroadcastText(currentRoom, "A user joined")
					return
				}
				
				if msg == "/leave" {
					ws.LeaveRoom(currentRoom)
					currentRoom = ""
					ws.SendText("Left room")
					return
				}
				
				if msg == "/rooms" {
					ws.SendText("Current room: " + currentRoom)
					return
				}
				
				// Broadcast message
				if currentRoom != "" {
					ws.BroadcastText(currentRoom, msg)
				}
			},
			
			OnClose: func(ws *sdk.WebSocketConn) {
				if currentRoom != "" {
					ws.LeaveRoom(currentRoom)
				}
			},
		})
	})
})
```

## JSON-based Protocol

```go
type Message struct {
	Type string                 `json:"type"`
	Data map[string]interface{} `json:"data"`
}

plugin.AddPhaseHandler("json-ws", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
			OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
				var message Message
				if err := json.Unmarshal([]byte(msg), &message); err != nil {
					ws.SendText(`{"type":"error","data":{"message":"Invalid JSON"}}`)
					return
				}
				
				switch message.Type {
				case "join":
					room := message.Data["room"].(string)
					ws.JoinRoom(room)
					
					response := Message{
						Type: "joined",
						Data: map[string]interface{}{
							"room": room,
						},
					}
					data, _ := json.Marshal(response)
					ws.SendText(string(data))
					
				case "message":
					room := message.Data["room"].(string)
					text := message.Data["text"].(string)
					
					broadcast := Message{
						Type: "message",
						Data: map[string]interface{}{
							"room": room,
							"text": text,
							"timestamp": time.Now().Unix(),
						},
					}
					data, _ := json.Marshal(broadcast)
					ws.BroadcastText(room, string(data))
					
				default:
					ws.SendText(`{"type":"error","data":{"message":"Unknown message type"}}`)
				}
			},
		})
	})
})
```

## Authentication

```go
plugin.AddPhaseHandler("auth-ws", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		
		// Check token in query
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
		
		// Extract user info
		userID := getUserIDFromToken(token)
		
		ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
			OnOpen: func(ws *sdk.WebSocketConn) {
				// Join user-specific room
				ws.JoinRoom("user:" + userID)
				ws.SendText(fmt.Sprintf("Authenticated as %s", userID))
			},
			
			OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
				// User is authenticated, handle message
				ws.BroadcastText("authenticated", fmt.Sprintf("%s: %s", userID, msg))
			},
			
			OnClose: func(ws *sdk.WebSocketConn) {
				ws.LeaveRoom("user:" + userID)
			},
		})
	})
})
```

## Complete Example

**plugin.go:**
```go
package main

import "C"
import (
	"encoding/json"
	"fmt"
	"strings"
	"time"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

type ChatMessage struct {
	Type      string `json:"type"`
	User      string `json:"user,omitempty"`
	Room      string `json:"room,omitempty"`
	Message   string `json:"message,omitempty"`
	Timestamp int64  `json:"timestamp"`
}

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	plugin.AddPhaseHandler("chat", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
			// Get username from query
			query := req.Query()
			params, _ := url.ParseQuery(query)
			username := params.Get("username")
			
			if username == "" {
				res := ctx.Response()
				res.SetStatus(400)
				res.BodyText("Username required")
				ctx.RemoveResponseHeader("Content-Length")
				ctx.SetResponseHeader("Transfer-Encoding", "chunked")
				ctx.End()
				return
			}
			
			currentRoom := "lobby"
			
			ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
				OnOpen: func(ws *sdk.WebSocketConn) {
					ws.JoinRoom(currentRoom)
					
					// Send welcome
					welcome := ChatMessage{
						Type:      "system",
						Message:   fmt.Sprintf("Welcome %s! You are in %s", username, currentRoom),
						Timestamp: time.Now().Unix(),
					}
					data, _ := json.Marshal(welcome)
					ws.SendText(string(data))
					
					// Notify others
					joined := ChatMessage{
						Type:      "join",
						User:      username,
						Room:      currentRoom,
						Timestamp: time.Now().Unix(),
					}
					data, _ = json.Marshal(joined)
					ws.BroadcastText(currentRoom, string(data))
				},
				
				OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
					// Handle /join command
					if strings.HasPrefix(msg, "/join ") {
						newRoom := strings.TrimPrefix(msg, "/join ")
						
						// Leave current room
						left := ChatMessage{
							Type:      "leave",
							User:      username,
							Room:      currentRoom,
							Timestamp: time.Now().Unix(),
						}
						data, _ := json.Marshal(left)
						ws.BroadcastText(currentRoom, string(data))
						ws.LeaveRoom(currentRoom)
						
						// Join new room
						currentRoom = newRoom
						ws.JoinRoom(currentRoom)
						
						joined := ChatMessage{
							Type:      "join",
							User:      username,
							Room:      currentRoom,
							Timestamp: time.Now().Unix(),
						}
						data, _ = json.Marshal(joined)
						ws.BroadcastText(currentRoom, string(data))
						
						// Confirm to user
						confirm := ChatMessage{
							Type:      "system",
							Message:   "Joined " + currentRoom,
							Timestamp: time.Now().Unix(),
						}
						data, _ = json.Marshal(confirm)
						ws.SendText(string(data))
						return
					}
					
					// Regular message
					message := ChatMessage{
						Type:      "message",
						User:      username,
						Room:      currentRoom,
						Message:   msg,
						Timestamp: time.Now().Unix(),
					}
					data, _ := json.Marshal(message)
					ws.BroadcastText(currentRoom, string(data))
				},
				
				OnClose: func(ws *sdk.WebSocketConn) {
					// Notify others
					left := ChatMessage{
						Type:      "leave",
						User:      username,
						Room:      currentRoom,
						Timestamp: time.Now().Unix(),
					}
					data, _ := json.Marshal(left)
					ws.BroadcastText(currentRoom, string(data))
					
					ws.LeaveRoom(currentRoom)
				},
				
				OnError: func(ws *sdk.WebSocketConn, err string) {
					fmt.Printf("[Chat] Error for %s: %s\n", username, err)
				},
			})
		})
	})
}
```

**Client (HTML):**
```html
<!DOCTYPE html>
<html>
<head>
    <title>Chat</title>
    <style>
        #messages { height: 400px; overflow-y: scroll; border: 1px solid #ccc; padding: 10px; }
        .message { margin: 5px 0; }
        .system { color: gray; font-style: italic; }
        .join { color: green; }
        .leave { color: red; }
    </style>
</head>
<body>
    <div id="messages"></div>
    <input id="input" type="text" placeholder="Type message or /join <room>">
    <button onclick="send()">Send</button>
    
    <script>
        const username = prompt('Enter username:');
        const ws = new WebSocket(`ws://localhost:8080/ws?username=${username}`);
        
        ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            const div = document.createElement('div');
            div.className = 'message ' + msg.type;
            
            if (msg.type === 'message') {
                div.textContent = `${msg.user}: ${msg.message}`;
            } else if (msg.type === 'system') {
                div.textContent = msg.message;
            } else if (msg.type === 'join') {
                div.textContent = `${msg.user} joined ${msg.room}`;
            } else if (msg.type === 'leave') {
                div.textContent = `${msg.user} left ${msg.room}`;
            }
            
            document.getElementById('messages').appendChild(div);
        };
        
        function send() {
            const input = document.getElementById('input');
            ws.send(input.value);
            input.value = '';
        }
        
        document.getElementById('input').addEventListener('keypress', (e) => {
            if (e.key === 'Enter') send();
        });
    </script>
</body>
</html>
```

## Configuration

```yaml
# Runtime config
http:
  - 0.0.0.0:8080

config_dir: "./config"

# WebSocket adapter
websocket:
  adapter_type: memory  # or redis for multi-server

# If using Redis
# websocket:
#   adapter_type: redis
#   redis:
#     host: localhost
#     port: 6379
#     db: 0
#     key_prefix: "nylon:ws"
```

```yaml
# Proxy config
plugins:
  - name: chat
    type: ffi
    file: ./chat.so

services:
  - name: websocket
    service_type: plugin
    plugin:
      name: chat
      entry: "chat"

  - name: static
    service_type: static
    static:
      root: ./public
      index: index.html

routes:
  - route:
      type: host
      value: localhost
    name: chat
    paths:
      - path: /ws
        service:
          name: websocket
      
      - path: /*
        service:
          name: static
```

## Best Practices

### 1. Validate Input

```go
OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
    if len(msg) > 1000 {
        ws.SendText("Message too long")
        return
    }
    // Process message
}
```

### 2. Handle Errors

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

### 3. Clean Up Resources

```go
OnClose: func(ws *sdk.WebSocketConn) {
    ws.LeaveRoom("lobby")
    // Clean up other resources
}
```

### 4. Use Rooms for Broadcasting

```go
// Efficient - only sends to room members
ws.BroadcastText("lobby", msg)

// Inefficient - manual iteration
for _, conn := range allConnections {
    conn.SendText(msg)
}
```

### 5. Authenticate Connections

```go
// Check token before upgrade
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
```

## See Also

- [WebSocket Support](/plugins/websocket) - WebSocket plugin guide
- [Request Handling](/plugins/request) - Access request information
- [Configuration](/core/configuration) - WebSocket configuration

