---
layout: home

hero:
  name: "Nylon"
  text: "High-Performance Reverse Proxy"
  tagline: Built with Rust and Pingora, powered by plugin ecosystem
  actions:
    - theme: brand
      text: Get Started
      link: /introduction/quick-start
    - theme: alt
      text: View on GitHub
      link: https://github.com/yourusername/nylon

features:
  - icon: ⚡️
    title: Blazing Fast
    details: Built on Cloudflare's Pingora framework, delivering exceptional performance and low latency
  
  - icon: 🔌
    title: Plugin System
    details: Extend functionality with Go plugins - request/response filters, WebSocket support, and more
  
  - icon: 🔄
    title: Load Balancing
    details: Multiple strategies including Round Robin, Weighted, Consistent Hashing, and Random
  
  - icon: 🔒
    title: TLS/HTTPS
    details: Built-in TLS support with automatic certificate management via ACME (Let's Encrypt)
  
  - icon: 🎯
    title: Advanced Routing
    details: Flexible host-based and path-based routing with parameter extraction
  
  - icon: 📊
    title: Observability
    details: Comprehensive logging phase with request/response metrics, duration tracking, and error handling
---

## Quick Example

### Runtime Config (`config.yaml`)

```yaml
http:
  - 0.0.0.0:8080

config_dir: "./config"

pingora:
  daemon: false
  threads: 4
  work_stealing: true
```

### Proxy Config (`config/proxy.yaml`)

```yaml
services:
  - name: backend-api
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000
      - ip: 127.0.0.1
        port: 3001

routes:
  - route:
      type: host
      value: localhost
    name: api-proxy
    paths:
      - path: /*
        service:
          name: backend-api
        middleware:
          - plugin: auth
            entry: "authz"
```

## Plugin Example

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
	
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
			// Check authentication
			token := req.Header("Authorization")
			if token == "" {
				res := ctx.Response()
				res.SetStatus(401)
				res.BodyText("Unauthorized")
				return
			}
			
			ctx.Next()
		})
		
		phase.Logging(func(ctx *sdk.PhaseLogging) {
			req := ctx.Request()
			res := ctx.Response()
			
			fmt.Printf("%s %s | Status: %d | Duration: %dms\n",
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

### Build and Use

```bash
# Build plugin
go build -buildmode=plugin -o auth.so plugin.go

# Configure in proxy.yaml
plugins:
  - name: auth
    type: ffi
    file: ./auth.so

# Start Nylon
nylon -c config.yaml
```

## Service Types

### HTTP Service
Forward to HTTP backends with load balancing

### Plugin Service  
Handle requests with custom Go plugins

### Static Service
Serve static files with SPA support

## Learn More

<div class="tip custom-block">
  <p class="custom-block-title">Ready to get started?</p>
  <p>Check out the <a href="/introduction/quick-start">Quick Start</a> guide to begin using Nylon.</p>
</div>
