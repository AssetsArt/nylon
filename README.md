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

routes:
  - route:
      type: host
      value: localhost # domain.com|domain2.com|domain3.com
    name: http-route-1
    middleware:
      - group: example
    paths:
      - path: 
          - /
          - /{*path}
        methods:
          - GET
          - POST
          - OPTIONS
        service:
          name: http-service
```

**Go & SDK**

```go
//go:build cgo

package main

import "C"
import (
	"encoding/json"
	"fmt"
	"unsafe"

	"github.com/AssetsArt/easy-proxy/sdk/go/sdk"
)

func main() {}

//export initialize
func initialize(config *C.char, length C.int) {
	configBytes := C.GoBytes(unsafe.Pointer(config), C.int(length))
	configData := struct {
		Debug bool `json:"debug"`
	}{
		Debug: false,
	}
	err := json.Unmarshal(configBytes, &configData)
	if err != nil {
		fmt.Println("[NylonPlugin] Error unmarshalling config", err)
		return
	}

	// Print the config data
	fmt.Println("[NylonPlugin] Plugin initialized", string(configBytes))

	// Create a new plugin
	plugin := sdk.NewNylonPlugin()

	// Register shutdown handler
	plugin.Shutdown(func() {
		fmt.Println("[NylonPlugin] Plugin shutdown")
	})

	// Register middleware
	plugin.AddRequestFilter("authz", func(ctx *sdk.PhaseRequestFilter) {
		// payload := ctx.GetPayload()
		// fmt.Println("Payload", payload)

		// // set headers
		ctx.Response().
			SetHeader("x-authz", "true")

		// next middleware
		ctx.Next()
	})

	// example of streaming response
	plugin.AddRequestFilter("stream_body", func(ctx *sdk.PhaseRequestFilter) {
		res := ctx.Response()
		// set status and headers
		res.SetStatus(200)
		res.SetHeader("Content-Type", "text/plain")

		// Start streaming response
		stream, err := res.Stream()
		if err != nil {
			fmt.Println("[NylonPlugin] Error streaming response", err)
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
