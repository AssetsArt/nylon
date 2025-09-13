# 🧬 Nylon: The Extensible Proxy Server

[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://nylon.sh/)

**Nylon** is a lightweight, high-performance, and extensible proxy server built on top of the robust [Cloudflare Pingora](https://blog.cloudflare.com/introducing-pingora/) framework. Designed for modern infrastructure.

---

## 🚀 Overview

- **Extensible**: Write plugins in Go, Rust, Zig, C, and more. Extend routing, filtering, and business logic without patching the core.
- **Modern Configuration**: Manage everything with a single, declarative YAML file. GitOps-friendly.
- **Advanced Routing & Load Balancing**: Route by host, header, path (wildcard support), and balance traffic with round robin, random, or consistent hashing.
- **Automatic TLS Management**: ACME (Let's Encrypt, Buypass, etc.) and custom certs supported.
- **Cloud-Native**: Designed for scale, reliability, and observability.

---

## 🛠️ Quick Start

```sh
# Download or build Nylon binary (see Releases or build instructions below)
nylon run -c config.yaml
````

See the [Getting Started Guide](https://nylon.sh/getting-started/installation) for detailed setup.

---

## 🧩 Extending Nylon

Nylon features a **powerful plugin system** — use any language with FFI.

**Example: Minimal Go Middleware Plugin**

```yaml
# proxy/my-config.yaml
plugins:
  - name: plugin_sdk
    type: ffi
    file: ./target/examples/go/plugin_sdk.so
    config:
      debug: true
      # ... other config

middleware_groups:
  example:
    - plugin: plugin_sdk
      request_filter: "authz"
      payload:
        client_ip: "${request(client_ip)}"
				
    - plugin: plugin_sdk
      entry: "stream"

services:
  - name: http-service
    service_type: http
    algorithm: round_robin # Options: round_robin, random, consistent, weighted
    endpoints:
      - ip: 127.0.0.1
        port: 3001
        # weight: 10 # Optional
      - ip: 127.0.0.1
        port: 3002
        # weight: 1 # Optional

  - name: ws-service
    service_type: plugin
    plugin:
      name: plugin_sdk
      entry: ws

routes:
  - route:
      type: host
      value: localhost # domain.com|domain2.com|domain3.com
    name: http-route-1
    paths:
      - path:
          - /ws
        methods:
          - GET
          - POST
          - OPTIONS
        service:
          name: ws-service
      - path: 
          - /
          - /{*path}
        methods:
          - GET
          - POST
          - OPTIONS
        middleware:
          - group: example
        service:
          name: http-service
```

**Go & SDK**

```go
package main

import "C"
import (
	"fmt"
	"time"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

type PluginConfig struct {
	Debug bool `json:"debug"`
}

func main() {}
func init() {

	// Create a new plugin
	plugin := sdk.NewNylonPlugin()

	// Register initialize handler
	plugin.Initialize(sdk.NewInitializer(func(config PluginConfig) {
		fmt.Println("[NylonPlugin] Plugin initialized")
		fmt.Println("[NylonPlugin] Config: Debug", config.Debug)
	}))

	// Register shutdown handler
	plugin.Shutdown(func() {
		fmt.Println("[NylonPlugin] Plugin shutdown")
	})

	// phase
	// - RequestFilter // Can return a full response
	//   |
	//   V
	// - ResponseFilter // Can modify the response headers
	//   |
	//   V
	// - ResponseBodyFilter // Can modify the response body
	//   |
	//   V
	// - Logging // Can log the request and response

	// Register middleware
	plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start Authz[Go] sessionID", phase.SessionId)
		// Initialize phase state per request
		myPhaseState := map[string]bool{
			"authz": false,
		}

		// Phase request filter
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("Authz[Go] RequestFilter sessionID", phase.SessionId)
			myPhaseState["authz"] = true

			payload := ctx.GetPayload()
			fmt.Println("[Authz][NylonPlugin] Payload", payload)

			response := ctx.Response()
			response.SetHeader("X-RequestFilter", "authz-1")
			// sleep 2 seconds
			time.Sleep(2 * time.Second)
			// next phase
			ctx.Next()
		})

		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			fmt.Println("Authz[Go] ResponseFilter sessionID", phase.SessionId)
			ctx.SetResponseHeader("X-ResponseFilter", "authz-2")

			ctx.Next()
		})

	})

	plugin.AddPhaseHandler("stream", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start Stream[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("Stream[Go] RequestFilter sessionID", phase.SessionId)
			res := ctx.Response()
			// set status and headers
			res.SetStatus(200)
			res.SetHeader("Content-Type", "text/plain")

			// Start streaming response
			stream, err := res.Stream()
			if err != nil {
				fmt.Println("[Stream][NylonPlugin] Error streaming response", err)
				ctx.Next()
				return
			}
			stream.Write([]byte("Hello"))
			w := ", World"
			for i := 0; i < len(w); i++ {
				stream.Write([]byte(w[i : i+1]))
			}

			// End streaming response
			stream.End()
		})
	})

	// WebSocket example
	plugin.AddPhaseHandler("ws", func(phase *sdk.PhaseHandler) {
		fmt.Println("Start WS[Go] sessionID", phase.SessionId)
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			fmt.Println("WS[Go] RequestFilter sessionID", phase.SessionId)
			err := ctx.WebSocketUpgrade(sdk.WebSocketCallbacks{
				OnOpen: func(ws *sdk.WebSocketConn) {
					fmt.Println("[WS][Go] onOpen")
					ws.SendText("hello from plugin")
				},
				OnMessageText: func(ws *sdk.WebSocketConn, msg string) {
					fmt.Println("[WS][Go] onMessageText:", msg)
					ws.SendText("echo: " + msg)
				},
				OnMessageBinary: func(ws *sdk.WebSocketConn, data []byte) {
					fmt.Println("[WS][Go] onMessageBinary", len(data))
					ws.SendBinary(data)
				},
				OnClose: func(ws *sdk.WebSocketConn) {
					fmt.Println("[WS][Go] onClose")
				},
				OnError: func(ws *sdk.WebSocketConn, err string) {
					fmt.Println("[WS][Go] onError:", err)
				},
			})
			if err != nil {
				fmt.Println("[WS][Go] upgrade error:", err)
				// On error fallback to HTTP
				ctx.Next()
			}
		})
	})
}
```

> See [plugin docs](https://nylon.sh/plugin-system/go) and [real-world examples](https://github.com/AssetsArt/nylon/tree/main/examples/go)

## 📚 Documentation

* **[nylon.sh](https://nylon.sh/)** — Full documentation & guides
* **[Getting Started](https://nylon.sh/getting-started/installation)**
* **[Plugin System](https://nylon.sh/plugin-system)**
* **[Config Reference](https://nylon.sh/config-reference)**

---

## 📦 Building from Source

```sh
git clone https://github.com/AssetsArt/nylon.git
cd nylon
make build
```

---

Nylon is an open-source project by [AssetsArt](https://github.com/AssetsArt).
