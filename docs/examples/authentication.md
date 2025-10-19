# Authentication

Implement authentication with plugins.

## Simple API Key Authentication

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
			
			// Check API key
			apiKey := req.Header("X-API-Key")
			if apiKey != "secret-key" {
				res := ctx.Response()
				res.SetStatus(401)
				res.BodyText("Unauthorized")
				return
			}
			
			// Log successful auth
			fmt.Printf("Authenticated: %s\n", req.ClientIP())
			
			ctx.Next()
		})
	})
	
	return &plugin
}
```

## Configuration

```yaml
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
```

More examples coming soon...

