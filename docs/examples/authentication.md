# Authentication

Implement authentication and authorization with plugins.

## Simple API Key Authentication

### Plugin Code

Create `auth.go`:

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

	// Initialize handler (receives config from YAML)
	plugin.Initialize(sdk.NewInitializer(func(config map[string]interface{}) {
		fmt.Println("[Auth] Plugin initialized")
		if apiKey, ok := config["api_key"].(string); ok {
			fmt.Println("[Auth] API Key:", apiKey)
		}
	}))

	plugin.AddPhaseHandler("check", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
		// Check API key
		apiKey := req.Header("X-API-Key")
		if apiKey == "" {
			res := ctx.Response()
			res.SetStatus(401)
			res.SetHeader("WWW-Authenticate", "API-Key")
			res.BodyText("Missing API key")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		if apiKey != "secret-key-123" {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Invalid API key")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			// Store auth info for logging
			ctx.SetPayload(map[string]interface{}{
				"authenticated": true,
				"api_key": apiKey,
			})
			
			fmt.Printf("[Auth] Authenticated: %s\n", req.ClientIP())
			ctx.Next()
		})
		
		phase.Logging(func(ctx *sdk.PhaseLogging) {
			payload := ctx.GetPayload()
			if auth, ok := payload["authenticated"].(bool); ok && auth {
				req := ctx.Request()
				fmt.Printf("[Auth] Access: %s %s from %s\n",
					req.Method(),
					req.Path(),
					req.ClientIP(),
				)
			}
			ctx.Next()
		})
	})
}
```

### Build Plugin

```bash
go build -buildmode=plugin -o auth.so auth.go
```

### Configuration

**Runtime config (`config.yaml`):**

```yaml
http:
  - 0.0.0.0:8080

config_dir: "./config"

pingora:
  daemon: false
  threads: 4
```

**Proxy config (`config/proxy.yaml`):**

```yaml
plugins:
  - name: auth
    type: ffi
    file: ./auth.so
    config:
      api_key: "secret-key-123"

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
      - path: /api/*
        service:
          name: protected-api
        middleware:
          - plugin: auth
            entry: "check"
```

### Testing

**Without API key (fails):**
```bash
curl http://localhost:8080/api/users
# Response: 401 Missing API key
```

**With invalid key (fails):**
```bash
curl -H "X-API-Key: wrong-key" http://localhost:8080/api/users
# Response: 401 Invalid API key
```

**With valid key (success):**
```bash
curl -H "X-API-Key: secret-key-123" http://localhost:8080/api/users
# Response: forwarded to backend
```

## JWT Authentication

### Plugin Code

```go
package main

import "C"
import (
	"encoding/json"
	"fmt"
	"strings"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()

	plugin.AddPhaseHandler("jwt", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
		// Get Authorization header
		auth := req.Header("Authorization")
		if auth == "" {
			res := ctx.Response()
			res.SetStatus(401)
			res.SetHeader("WWW-Authenticate", "Bearer")
			res.BodyText("Missing authorization token")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		// Check Bearer scheme
		if !strings.HasPrefix(auth, "Bearer ") {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Invalid authorization scheme")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		token := strings.TrimPrefix(auth, "Bearer ")
		
		// Validate JWT (simplified - use a real JWT library)
		if token == "" {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Invalid token")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			// TODO: Actually validate JWT signature
			// For demo, just extract claims
			
			// Store user info
			ctx.SetPayload(map[string]interface{}{
				"user_id": "123",
				"role": "admin",
			})
			
			ctx.Next()
		})
	})
}
```

## Role-Based Access Control

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

	plugin.AddPhaseHandler("rbac", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
			req := ctx.Request()
			
		// Get required role from payload (set by previous middleware)
		payload := ctx.GetPayload()
		if payload == nil {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Authentication required")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		userRole, ok := payload["role"].(string)
		if !ok {
			res := ctx.Response()
			res.SetStatus(401)
			res.BodyText("Invalid authentication")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
		
		// Check if path requires admin
		path := req.Path()
		if strings.HasPrefix(path, "/admin") && userRole != "admin" {
			res := ctx.Response()
			res.SetStatus(403)
			res.BodyText("Access denied: admin role required")
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			fmt.Printf("[RBAC] Access granted: %s with role %s\n", 
				req.ClientIP(), userRole)
			ctx.Next()
		})
	})
}
```

### Configuration with Multiple Middleware

```yaml
plugins:
  - name: auth
    type: ffi
    file: ./auth.so

middleware_groups:
  authenticated:
    - plugin: auth
      entry: "jwt"
    - plugin: auth
      entry: "rbac"

routes:
  - route:
      type: host
      value: localhost
    name: protected
    paths:
      # Public endpoints
      - path: /public/*
        service:
          name: api

      # Authenticated endpoints
      - path: /api/*
        service:
          name: api
        middleware:
          - group: authenticated

      # Admin endpoints
      - path: /admin/*
        service:
          name: admin-api
        middleware:
          - group: authenticated
```

## IP Whitelist

```go
package main

import "C"
import (
	"fmt"
	sdk "github.com/AssetsArt/nylon/sdk/go/sdk"
)

var allowedIPs = map[string]bool{
	"127.0.0.1": true,
	"10.0.0.1": true,
	"10.0.0.2": true,
}

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()

	plugin.AddPhaseHandler("ipfilter", func(phase *sdk.PhaseHandler) {
		phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		clientIP := req.ClientIP()
		
		if !allowedIPs[clientIP] {
			res := ctx.Response()
			res.SetStatus(403)
			res.BodyText(fmt.Sprintf("Access denied for IP: %s", clientIP))
			ctx.RemoveResponseHeader("Content-Length")
			ctx.SetResponseHeader("Transfer-Encoding", "chunked")
			ctx.End()
			return
		}
			
			ctx.Next()
		})
	})
}
```

## Next Steps

- [Plugin Phases](/plugins/phases)
- [Go SDK API](/plugins/go-sdk)
- [More Examples](/examples/basic-proxy)
