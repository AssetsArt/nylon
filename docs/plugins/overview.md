# Plugin Development Overview

Extend Nylon with custom logic running in the same process. Plugins can participate in every phase of the request lifecycle (request, response headers, response body, logging) and can also handle WebSocket connections.

**At a glance**

- Build shared libraries (`.so`) via the Go SDK (more languages on the roadmap).
- Register phase handlers to run code before/after routing, modify responses, or stream bodies.
- Leverage middleware payloads to pass configuration from YAML to your plugin.
- Use helper APIs to read/modify requests, responses, and WebSocket streams.

## Plugin Architecture

Plugins are shared libraries (`.so` files) that implement the Nylon plugin interface. They run in the same process as Nylon for maximum performance using Foreign Function Interface (FFI).

```
┌─────────────────────────────────┐
│         Nylon Core              │
│                                 │
│  ┌───────────────────────────┐  │
│  │     Request Flow          │  │
│  │                           │  │
│  │  1. RequestFilter         │  │◄── Plugin intercepts here
│  │  2. Route Matching        │  │
│  │  3. Backend Selection     │  │
│  │  4. ResponseFilter        │  │◄── Plugin intercepts here
│  │  5. ResponseBodyFilter    │  │◄── Plugin intercepts here
│  │  6. Logging               │  │◄── Plugin intercepts here
│  │                           │  │
│  └───────────────────────────┘  │
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

## Quick start plugin

```go
package main

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
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
}
```

## Request & response access

The SDK exposes ergonomic helpers to inspect the inbound request and craft responses without juggling raw pointers.

### Reading request data

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	
	// URL and path
	url := req.URL()           // Full URL
	path := req.Path()         // Path only
	query := req.Query()       // Query string
	
	// Headers
	auth := req.Header("Authorization")
		headers := req.Headers().GetAll()
	
	// Method and metadata
	method := req.Method()     // GET, POST, etc.
	host := req.Host()         // Host header
	clientIP := req.ClientIP() // Client IP address
	
	// Route parameters
	params := req.Params()     // URL parameters
	userId := params["user_id"]
	
	// Body
	body := req.RawBody()      // Read request body (byte slice)
	
	// Timestamp
	timestamp := req.Timestamp() // Request timestamp (ms)
	
	ctx.Next()
})
```

### Modifying responses

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	res := ctx.Response()
	
	// Set status
	res.SetStatus(200)
	
	// Set headers
	res.SetHeader("X-Custom-Header", "value")
	res.SetHeader("Cache-Control", "no-cache")
	res.RemoveHeader("Server")
	
	// Set body
	res.BodyRaw([]byte("Hello, World!"))
	res.BodyText("Hello, World!")
	
	ctx.Next()
})
```

### Access logging example

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

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
}
```

### 2. Build as Shared Library

```bash
go build -buildmode=c-shared -o myplugin.so
```

### 3. Configure in Nylon

```yaml
plugins:
  - name: myplugin
    type: ffi
    file: ./myplugin.so
    config:
      some_flag: true

services:
  - name: backend
    service_type: http
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: example.com
    name: main
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend
        middleware:
          - plugin: myplugin
            entry: "my-handler"
```

> **Tip:** The `entry` value must match the name you pass to `AddPhaseHandler` in your plugin (e.g. `"my-handler"` in the example above).

You can optionally provide structured data to the handler via `payload`:

```yaml
middleware:
  - plugin: myplugin
    entry: "my-handler"
    payload:
      api_key: "secret"
      mode: "strict"
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
	body := req.RawBody()
	
	var data map[string]interface{}
	if err := json.Unmarshal(body, &data); err != nil {
		res := ctx.Response()
		res.SetStatus(400)
		res.BodyText("Invalid JSON")
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
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
	// Access middleware payload provided in YAML config
	payload := ctx.GetPayload()
	if apiKey, ok := payload["api_key"].(string); ok {
		req := ctx.Request()
		if req.Header("X-API-Key") != apiKey {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Unauthorized")
			ctx.End()
			return
		}
	}

	ctx.Next()
})
```

## Next Steps

- Learn about [Plugin Phases](/plugins/phases) in detail
- Explore the [Go SDK API](/plugins/go-sdk)
- See [Plugin Examples](/examples/authentication)
- Reuse configuration patterns from [core middleware](/core/middleware)
