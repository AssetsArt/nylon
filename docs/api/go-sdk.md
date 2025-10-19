# Go SDK API

Complete API reference for Nylon Go SDK.

## Installation

```bash
go get github.com/AssetsArt/nylon/sdk/go/sdk
```

## Plugin Initialization

### NewNylonPlugin()

Create a new plugin instance:

```go
plugin := sdk.NewNylonPlugin()
```

### Initialize()

Register initialization handler:

```go
type Config struct {
    APIKey string `json:"api_key"`
    Debug  bool   `json:"debug"`
}

plugin.Initialize(sdk.NewInitializer(func(config Config) {
    fmt.Println("Plugin initialized")
    fmt.Printf("Config: %+v\n", config)
}))
```

### Shutdown()

Register shutdown handler:

```go
plugin.Shutdown(func() {
    fmt.Println("Plugin shutting down")
    // Clean up resources
})
```

### AddPhaseHandler()

Register phase handlers:

```go
plugin.AddPhaseHandler("my-handler", func(phase *sdk.PhaseHandler) {
    // Register phase callbacks
    phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) { /* ... */ })
    phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) { /* ... */ })
    phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) { /* ... */ })
    phase.Logging(func(ctx *sdk.PhaseLogging) { /* ... */ })
})
```

## Phase Contexts

### PhaseRequestFilter

Request filtering phase:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    res := ctx.Response()
    
    // Process request
    
    ctx.Next()  // Continue to next phase
    // or
    return  // Stop processing
})
```

**Methods:**
- `Request() *Request` - Get request object
- `Response() *Response` - Get response object
- `GetPayload() map[string]interface{}` - Middleware payload from YAML
- `Next()` - Continue to next phase
- `End()` - Stop processing and send response
- `WebSocketUpgrade(callbacks WebSocketCallbacks) error` - Upgrade to WebSocket

### PhaseResponseFilter

Response filtering phase:

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    // Modify response headers
    
    ctx.Next()
})
```

**Methods:**
- `Request() *Request` - Inspect original request headers
- `Response() *Response` - Modify response status/headers
- `GetPayload() map[string]interface{}` - Middleware payload from YAML
- `Next()` - Continue

### PhaseResponseBodyFilter

Response body filtering phase:

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    body := res.ReadBody()
    
    // Modify body
    modifiedBody := transform(body)
    res.BodyRaw(modifiedBody)
    
    ctx.Next()
})
```

**Methods:**
- `Response() *Response` - Get response object
- `GetPayload() map[string]interface{}` - Get payload
- `Next()` - Continue

### PhaseLogging

Logging phase:

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
    req := ctx.Request()
    res := ctx.Response()
    
    log.Printf("%s %s -> %d (%dms)",
        req.Method(),
        req.Path(),
        res.Status(),
        res.Duration(),
    )
    
    ctx.Next()
})
```

**Methods:**
- `Request() *Request` - Get request object
- `Response() *Response` - Get response object
- `GetPayload() map[string]interface{}` - Get payload
- `Next()` - Continue

## Request Object

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `Method()` | `string` | HTTP method (GET, POST, etc.) |
| `URL()` | `string` | Full URL |
| `Path()` | `string` | Request path |
| `Query()` | `string` | Query string |
| `Params()` | `map[string]string` | Path parameters |
| `Host()` | `string` | Hostname |
| `ClientIP()` | `string` | Client IP address |
| `Headers()` | `*Headers` | All headers (`Get`, `GetAll`) |
| `Header(name string)` | `string` | Single header |
| `RawBody()` | `[]byte` | Request body |
| `Bytes()` | `int64` | Request body size |
| `Timestamp()` | `int64` | Request timestamp (ms) |

### Example

```go
req := ctx.Request()

method := req.Method()        // "GET"
url := req.URL()             // "http://example.com/api/users?id=123"
path := req.Path()           // "/api/users"
query := req.Query()         // "id=123"
params := req.Params()       // map["id": "123"]
host := req.Host()           // "example.com"
clientIP := req.ClientIP()   // "192.168.1.1"
headers := req.Headers().GetAll()
auth := req.Header("Authorization")
bytes := req.Bytes()         // 1024
timestamp := req.Timestamp() // 1704067200000
```

## Response Object

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `SetStatus(code int)` | `*Response` | Set status code |
| `Status()` | `int` | Get status code |
| `SetHeader(name, value string)` | `*Response` | Set header |
| `RemoveHeader(name string)` | `*Response` | Remove header |
| `Headers()` | `map[string]string` | Get all headers |
| `BodyRaw(data []byte)` | `*Response` | Set raw body |
| `BodyText(text string)` | `*Response` | Set text body |
| `BodyJSON(data interface{})` | `*Response` | Set JSON body |
| `ReadBody()` | `[]byte` | Read body (ResponseBodyFilter only) |
| `Redirect(url string, code ...uint16)` | `*Response` | Set redirect |
| `Bytes()` | `int64` | Response body size |
| `Duration()` | `int64` | Request duration (ms) |
| `Error()` | `string` | Error message (if any) |
| `Stream()` | `(*ResponseStream, error)` | Create streaming writer |

### Example

```go
res := ctx.Response()

// Set status and headers (must call separately)
res.SetStatus(200)
res.SetHeader("Content-Type", "application/json")
res.SetHeader("X-Server", "Nylon")

// Remove header
res.RemoveHeader("Server")

// Set body
res.BodyJSON(map[string]interface{}{
    "message": "Success",
})

// Or
res.BodyText("Hello, World!")

// Or
res.BodyRaw([]byte{0x01, 0x02})

// In ResponseBodyFilter
body := res.ReadBody()
// Modify body...
res.BodyRaw(modifiedBody)

// In Logging
status := res.Status()       // 200
bytes := res.Bytes()         // 1024
duration := res.Duration()   // 150
err := res.Error()           // ""
```

## WebSocket

### WebSocketUpgrade()

Upgrade connection to WebSocket:

```go
err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
    OnOpen:          func(ws *sdk.WebSocketConn) { /* ... */ },
    OnMessageText:   func(ws *sdk.WebSocketConn, msg string) { /* ... */ },
    OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) { /* ... */ },
    OnClose:         func(ws *sdk.WebSocketConn) { /* ... */ },
    OnError:         func(ws *sdk.WebSocketConn, err string) { /* ... */ },
})
```

### WebSocket Connection

| Method | Description |
|--------|-------------|
| `SendText(message string)` | Send text message |
| `SendBinary(data []byte)` | Send binary message |
| `Close()` | Close connection |
| `JoinRoom(room string)` | Join broadcast room |
| `LeaveRoom(room string)` | Leave room |
| `BroadcastText(room, message string)` | Broadcast text to room |
| `BroadcastBinary(room string, data []byte)` | Broadcast binary to room |

### Example

```go
ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
    OnOpen: func(ws *sdk.WebSocketConn) {
        ws.JoinRoom("lobby")
        ws.SendText("Welcome!")
    },
    
    OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
        ws.BroadcastText("lobby", msg)
    },
    
    OnClose: func(ws *sdk.WebSocketConn) {
        ws.LeaveRoom("lobby")
    },
})
```

## Stream Writer

For streaming responses:

```go
stream, err := res.Stream()
if err != nil {
    // Handle error
    return
}

// Write chunks
stream.Write([]byte("chunk 1"))
stream.Write([]byte("chunk 2"))

// End stream
stream.End()
```

## Error Handling

### Check Errors

```go
err := ctx.WebSocketUpgrade(callbacks)
if err != nil {
    res := ctx.Response()

    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")

    res.SetStatus(400)
    res.BodyText("Upgrade failed")

    ctx.End()
    return
}
```

### Return Early

```go
if !authorized {
    res := ctx.Response()

    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")

    res.SetStatus(403)
    res.BodyText("Forbidden")

    ctx.End()
    return
}

ctx.Next()
```

## Best Practices

### 1. Always Call Next()

```go
// ✅ Good
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    // Process...
    ctx.Next()
})

// ❌ Bad - response will hang
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    // Process...
    // Missing ctx.Next()
})
```

### 2. Handle Errors

```go
// ✅ Good
err := ctx.WebSocketUpgrade(callbacks)
if err != nil {
    res := ctx.Response()

    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")

    res.SetStatus(400)
    res.BodyText("Error")

    ctx.End()
    return
}

// ❌ Bad
ctx.WebSocketUpgrade(callbacks)  // Ignores error
```

### 3. Call Methods Separately

```go
// ✅ Correct - call methods separately
res.SetStatus(200)
res.SetHeader("Content-Type", "application/json")
res.BodyJSON(data)

// ❌ Wrong - method chaining not supported
// res.SetStatus(200).SetHeader("Content-Type", "application/json").BodyJSON(data)
```

### 4. Clean Up Resources

```go
plugin.Shutdown(func() {
    // Close database connections
    // Stop goroutines
    // Free resources
})
```

## Complete Example

```go
package main

import "C"
import (
    "fmt"
    sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

type Config struct {
    APIKey string `json:"api_key"`
}

func main() {}

func init() {
    plugin := sdk.NewNylonPlugin()
    
    // Initialize
    plugin.Initialize(sdk.NewInitializer(func(config Config) {
        fmt.Println("Initialized with key:", config.APIKey)
    }))
    
    // Shutdown
    plugin.Shutdown(func() {
        fmt.Println("Shutting down")
    })
    
    // Phase handler
    plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
        phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
            req := ctx.Request()
            
            if req.Header("X-API-Key") == "" {
                res := ctx.Response()

                res.RemoveHeader("Content-Length")
                res.SetHeader("Transfer-Encoding", "chunked")

                res.SetStatus(401)
                res.BodyText("Unauthorized")

                ctx.End()
                return
            }
            
            ctx.Next()
        })
        
        phase.Logging(func(ctx *sdk.PhaseLogging) {
            req := ctx.Request()
            res := ctx.Response()
            
            fmt.Printf("%s %s -> %d (%dms)\n",
                req.Method(),
                req.Path(),
                res.Status(),
                res.Duration(),
            )
            
            ctx.Next()
        })
    })
}
```

## See Also

- [Plugin Phases](/plugins/phases) - Understanding plugin phases
- [Request Handling](/plugins/request) - Request handling guide
- [Response Handling](/plugins/response) - Response handling guide
- [WebSocket Support](/plugins/websocket) - WebSocket guide
