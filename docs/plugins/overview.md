# Plugin Development Overview

Nylon's plugin system allows you to extend its functionality with custom logic written in Go. Plugins can intercept and modify requests, responses, and WebSocket messages.

## Plugin Architecture

Plugins are Go shared libraries (`.so` files) that implement the Nylon plugin interface. They run in the same process as Nylon for maximum performance.

```
┌─────────────────────────────────┐
│         Nylon Core              │
│                                 │
│  ┌───────────────────────────┐ │
│  │     Request Flow          │ │
│  │                           │ │
│  │  1. RequestFilter         │ │◄── Plugin intercepts here
│  │  2. Route Matching        │ │
│  │  3. Backend Selection     │ │
│  │  4. ResponseFilter        │ │◄── Plugin intercepts here
│  │  5. ResponseBodyFilter    │ │◄── Plugin intercepts here
│  │  6. Logging               │ │◄── Plugin intercepts here
│  │                           │ │
│  └───────────────────────────┘ │
└─────────────────────────────────┘
```

## Plugin Phases

Plugins can hook into different phases of request processing:

### 1. RequestFilter
Execute before the request is sent to the backend:
- Authentication and authorization
- Request validation
- Header manipulation
- Rate limiting
- Request transformation

### 2. ResponseFilter
Execute after receiving response headers from backend:
- Response header modification
- Status code changes
- Redirect logic
- Caching decisions

### 3. ResponseBodyFilter
Execute while streaming response body:
- Content transformation
- Compression
- Filtering
- Body modification

### 4. Logging
Execute after the request is complete:
- Access logging
- Metrics collection
- Analytics
- Error tracking

## Basic Plugin Structure

```go
package main

import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

//export NewNylonPlugin
func NewNylonPlugin() *sdk.NylonPlugin {
	plugin := sdk.NylonPlugin{}
	
	// Register phase handlers
	plugin.AddPhaseHandler("my-handler", func(phase *sdk.PhaseHandler) {
		// RequestFilter phase
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			// Your logic here
			ctx.Next() // Continue to next phase
		})
		
		// ResponseFilter phase
		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			// Your logic here
			ctx.Next()
		})
		
		// ResponseBodyFilter phase
		phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
			// Your logic here
			ctx.Next()
		})
		
		// Logging phase
		phase.Logging(func(ctx *sdk.PhaseLogging) {
			// Your logic here
			ctx.Next()
		})
	})
	
	return &plugin
}
```

## Request/Response Access

### Reading Request Data

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	
	// URL and path
	url := req.URL()           // Full URL
	path := req.Path()         // Path only
	query := req.Query()       // Query string
	
	// Headers
	auth := req.Header("Authorization")
	headers := req.Headers()   // All headers
	
	// Method and metadata
	method := req.Method()     // GET, POST, etc.
	host := req.Host()         // Host header
	clientIP := req.ClientIP() // Client IP address
	
	// Route parameters
	params := req.Params()     // URL parameters
	userId := params["user_id"]
	
	// Body
	body := req.ReadBody()     // Read request body
	
	// Timestamp
	timestamp := req.Timestamp() // Request timestamp (ms)
	
	ctx.Next()
})
```

### Modifying Response

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	res := ctx.Response()
	
	// Set status
	res.SetStatus(200)
	
	// Set headers
	res.SetHeader("X-Custom-Header", "value")
	res.SetResponseHeader("Cache-Control", "no-cache")
	res.RemoveResponseHeader("Server")
	
	// Set body
	res.BodyRaw([]byte("Hello, World!"))
	res.BodyText("Hello, World!")
	
	ctx.Next()
})
```

### Access Logging Example

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
	req := ctx.Request()
	res := ctx.Response()
	
	log.Printf(
		"%s %s | Status: %d | ReqBytes: %d | ResBytes: %d | Duration: %dms | Client: %s",
		req.Method(),
		req.Path(),
		res.Status(),
		req.Bytes(),
		res.Bytes(),
		res.Duration(),
		req.ClientIP(),
	)
	
	// Log errors if any
	if err := res.Error(); err != "" {
		log.Printf("Error: %s", err)
	}
	
	ctx.Next()
})
```

## Building Plugins

### 1. Create Plugin File

```go
// plugin.go
package main

import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

//export NewNylonPlugin
func NewNylonPlugin() *sdk.NylonPlugin {
	// Your plugin implementation
	plugin := sdk.NylonPlugin{}
	return &plugin
}
```

### 2. Build as Shared Library

```bash
go build -buildmode=plugin -o myplugin.so plugin.go
```

### 3. Configure in Nylon

```yaml
plugins:
  - name: myplugin
    path: "./myplugin.so"

proxy:
  - name: my-proxy
    routes:
      - path: "/*"
        service: backend
        middlewares:
          - type: Plugin
            name: myplugin
            config:
              phase: request_filter
```

## Best Practices

### 1. Always Call ctx.Next()
Unless you want to stop the request flow:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	// Do your work
	
	// Continue processing
	ctx.Next()
})
```

### 2. Handle Errors Gracefully

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	body := req.ReadBody()
	
	var data map[string]interface{}
	if err := json.Unmarshal(body, &data); err != nil {
		res := ctx.Response()
		res.SetStatus(400)
		res.BodyText("Invalid JSON")
		return // Stop processing
	}
	
	ctx.Next()
})
```

### 3. Minimize Allocations

```go
// Good: Reuse buffers
var bufPool = sync.Pool{
	New: func() interface{} {
		return new(bytes.Buffer)
	},
}

phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
	buf := bufPool.Get().(*bytes.Buffer)
	defer bufPool.Put(buf)
	buf.Reset()
	
	// Use buffer
	
	ctx.Next()
})
```

### 4. Use Context for State

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	// Store data in payload for later phases
	payload := map[string]interface{}{
		"user_id": "12345",
		"role": "admin",
	}
	ctx.SetPayload(payload)
	
	ctx.Next()
})

phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	// Retrieve data from earlier phase
	payload := ctx.GetPayload()
	if userID, ok := payload["user_id"].(string); ok {
		res := ctx.Response()
		res.SetHeader("X-User-ID", userID)
	}
	
	ctx.Next()
})
```

## Next Steps

- Learn about [Plugin Phases](/plugins/phases) in detail
- Explore the [Go SDK API](/plugins/go-sdk)
- See [Plugin Examples](/examples/authentication)

