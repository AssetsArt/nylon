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
    details: Flexible path-based and host-based routing with parameter extraction
  
  - icon: 📊
    title: Observability
    details: Comprehensive logging phase with request/response metrics, duration tracking, and error handling
---

## Quick Example

```yaml
# config.yaml
runtime:
  threads: 4
  work_stealing: true

proxy:
  - name: api-proxy
    listen: 
      - "0.0.0.0:8080"
    routes:
      - path: "/*"
        service: backend-api
        middlewares:
          - type: Plugin
            name: authz
            config:
              phase: request_filter

services:
  - name: backend-api
    backend:
      round_robin:
        - "127.0.0.1:3000"
        - "127.0.0.1:3001"
```

## Plugin Example

```go
package main

import (
	"fmt"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

func main() {}

//export NewNylonPlugin
func NewNylonPlugin() *sdk.NylonPlugin {
	plugin := sdk.NylonPlugin{}
	
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
			// Check authentication
			token := req.Header("Authorization")
			if token == "" {
				res := ctx.Response()
				res.SetStatus(401)
				res.BodyRaw([]byte("Unauthorized"))
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
	
	return &plugin
}
```

