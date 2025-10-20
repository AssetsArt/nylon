# Quick Start

Get up and running with Nylon in minutes.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/AssetsArt/nylon.git
cd nylon

# Build the project
cargo build --release

# The binary will be at target/release/nylon
```

## Your First Proxy

Nylon uses two configuration files:

1. **Runtime config** (`config.yaml`) - Server settings
2. **Proxy config** (in `config_dir`) - Services, routes, plugins

### 1. Create Runtime Config

Create `config.yaml`:

```yaml
http:
  - 0.0.0.0:8080

config_dir: "./config"

pingora:
  daemon: false
  threads: 4
  work_stealing: true
  grace_period_seconds: 60
  graceful_shutdown_timeout_seconds: 10
```

### 2. Create Proxy Config

Create `config/proxy.yaml`:

```yaml
services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: localhost
    name: default
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend
```

### 3. Start the Proxy

```bash
nylon run -c config.yaml
```

### 4. Test It

```bash
curl http://localhost:8080
```

## With HTTPS/TLS

Enable HTTPS with automatic certificate management:

### Runtime Config

```yaml
http:
  - 0.0.0.0:80
https:
  - 0.0.0.0:443

config_dir: "./config"
acme: "./acme"

pingora:
  daemon: false
  threads: 4
```

### Proxy Config with TLS

```yaml
tls:
  - type: acme
    provider: letsencrypt
    domains:
      - example.com
    acme:
      email: admin@example.com

services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: example.com
    name: https-route
    tls:
      enabled: true
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend
```

## With Plugins

### 1. Create Plugin

Create `plugin.go`:

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

	// Initialize handler (optional)
	plugin.Initialize(sdk.NewInitializer(func(config map[string]interface{}) {
		fmt.Println("[Plugin] Initialized")
		fmt.Println("[Plugin] Config:", config)
	}))

	// Shutdown handler (optional)
	plugin.Shutdown(func() {
		fmt.Println("[Plugin] Shutdown")
	})

	// Register phase handler
	plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
		// Simple API key authentication
		apiKey := req.Header("X-API-Key")
		if apiKey != "secret-key" {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Unauthorized")
			res.RemoveHeader("Content-Length")
			res.SetHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			ctx.Next() // Continue
		})
	})
}
```

### 2. Build Plugin

```bash
go build -buildmode=c-shared -o auth.so
```

### 3. Configure Nylon

**Proxy config (`config/proxy.yaml`):**

```yaml
plugins:
  - name: auth
    type: ffi
    file: ./auth.so
    config:
      debug: true

services:
  - name: protected-api
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: localhost
    name: protected
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: protected-api
        middleware:
          - plugin: auth
            entry: "auth"
```

### 4. Test Protected Endpoint

Without API key (should fail):
```bash
curl http://localhost:8080/api
# Response: 401 Unauthorized
```

With API key (should succeed):
```bash
curl -H "X-API-Key: secret-key" http://localhost:8080/api
# Response: forwarded to backend
```

## Load Balancing

Configure multiple backends with different algorithms:

```yaml
services:
  # Round Robin (default)
  - name: api-roundrobin
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000

  # Weighted Round Robin
  - name: api-weighted
    service_type: http
    algorithm: weighted
    endpoints:
      - ip: 10.0.0.1
        port: 3000
        weight: 5
      - ip: 10.0.0.2
        port: 3000
        weight: 3
      - ip: 10.0.0.3
        port: 3000
        weight: 2

  # Consistent Hashing
  - name: api-consistent
    service_type: http
    algorithm: consistent
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000

  # Random
  - name: api-random
    service_type: http
    algorithm: random
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
```

## Service Types

Nylon supports three types of services:

### HTTP Service
Forward requests to HTTP backends:

```yaml
services:
  - name: http-backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000
```

### Plugin Service
Handle requests with Go plugins:

```yaml
services:
  - name: custom-handler
    service_type: plugin
    plugin:
      name: my-plugin
      entry: "handler"
```

### Static Service
Serve static files:

```yaml
services:
  - name: static-files
    service_type: static
    static:
      root: ./public
      index: index.html
      spa: true  # SPA fallback mode
```

## Next Steps

- Learn about [Configuration](/core/configuration) in detail
- Explore [Plugin Development](/plugins/overview)
- Check out [Examples](/examples/basic-proxy)
