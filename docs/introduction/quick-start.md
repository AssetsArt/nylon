# Quick Start

Get up and running with Nylon in minutes.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/nylon.git
cd nylon

# Build the project
cargo build --release

# The binary will be at target/release/nylon
```

### Using the Install Script

```bash
curl -sSL https://raw.githubusercontent.com/yourusername/nylon/main/scripts/install.sh | bash
```

## Your First Proxy

Create a simple configuration file `config.yaml`:

```yaml
runtime:
  threads: 4
  work_stealing: true

proxy:
  - name: my-first-proxy
    listen:
      - "0.0.0.0:8080"
    routes:
      - path: "/*"
        service: backend

services:
  - name: backend
    backend:
      round_robin:
        - "127.0.0.1:3000"
```

Start the proxy:

```bash
nylon -c config.yaml
```

Test it:

```bash
curl http://localhost:8080
```

## With TLS/HTTPS

Enable HTTPS with automatic certificate management:

```yaml
runtime:
  threads: 4
  work_stealing: true

proxy:
  - name: https-proxy
    listen:
      - "0.0.0.0:443"
    tls:
      - domains:
          - "example.com"
        acme:
          email: "admin@example.com"
          directory_url: "https://acme-v02.api.letsencrypt.org/directory"
    routes:
      - path: "/*"
        service: backend

services:
  - name: backend
    backend:
      round_robin:
        - "127.0.0.1:3000"
```

## With Plugins

### 1. Build a Go Plugin

Create `plugin.go`:

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
	
	plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
			// Simple authentication
			apiKey := req.Header("X-API-Key")
			if apiKey != "secret-key" {
				res := ctx.Response()
				res.SetStatus(401)
				res.BodyRaw([]byte("Unauthorized"))
				return
			}
			
			ctx.Next()
		})
	})
	
	return &plugin
}
```

Build the plugin:

```bash
go build -buildmode=plugin -o auth.so plugin.go
```

### 2. Configure Nylon to Use the Plugin

```yaml
runtime:
  threads: 4
  work_stealing: true

plugins:
  - name: auth
    path: "./auth.so"

proxy:
  - name: protected-api
    listen:
      - "0.0.0.0:8080"
    routes:
      - path: "/*"
        service: backend
        middlewares:
          - type: Plugin
            name: auth

services:
  - name: backend
    backend:
      round_robin:
        - "127.0.0.1:3000"
```

### 3. Test the Protected Endpoint

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

## Load Balancing Example

Configure multiple backends with different strategies:

```yaml
services:
  - name: api-service
    backend:
      # Round Robin (default)
      round_robin:
        - "10.0.0.1:3000"
        - "10.0.0.2:3000"
        - "10.0.0.3:3000"

  - name: weighted-service
    backend:
      # Weighted Round Robin
      weighted:
        - addr: "10.0.0.1:3000"
          weight: 5
        - addr: "10.0.0.2:3000"
          weight: 3
        - addr: "10.0.0.3:3000"
          weight: 2

  - name: consistent-service
    backend:
      # Consistent Hashing
      consistent:
        - "10.0.0.1:3000"
        - "10.0.0.2:3000"
        - "10.0.0.3:3000"
```

## Next Steps

- Learn about [Configuration](/core/configuration) in detail
- Explore [Plugin Development](/plugins/overview)
- Check out [Examples](/examples/basic-proxy)

