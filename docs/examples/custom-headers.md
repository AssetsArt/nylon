# Custom Headers

Add, modify, and remove HTTP headers using built-in plugins and Go plugins.

## Built-in Header Modifiers

### Request Headers

Modify request headers before forwarding to backend:

```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      set:
        - name: x-request-id
          value: "${uuid(v7)}"
        - name: x-forwarded-for
          value: "${request(client_ip)}"
        - name: x-original-host
          value: "${header(host)}"
      remove:
        - x-internal-token
        - x-debug-mode
```

### Response Headers

Modify response headers from backend:

```yaml
middleware:
  - plugin: ResponseHeaderModifier
    payload:
      set:
        - name: x-server
          value: "nylon"
        - name: cache-control
          value: "no-cache"
        - name: x-frame-options
          value: "DENY"
      remove:
        - server
        - x-powered-by
```

## Go Plugin Examples

### Security Headers

```go
package main

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
	plugin := sdk.NewNylonPlugin()
	
	plugin.AddPhaseHandler("security-headers", func(phase *sdk.PhaseHandler) {
		phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
			res := ctx.Response()
			
			// Security headers
			res.SetHeader("X-Frame-Options", "DENY")
			res.SetHeader("X-Content-Type-Options", "nosniff")
			res.SetHeader("X-XSS-Protection", "1; mode=block")
			res.SetHeader("Referrer-Policy", "no-referrer")
			res.SetHeader("Content-Security-Policy", "default-src 'self'")
			res.SetHeader("Strict-Transport-Security", "max-age=31536000")
			
			// Remove sensitive headers
			res.RemoveHeader("Server")
			res.RemoveHeader("X-Powered-By")
			res.RemoveHeader("X-AspNet-Version")
			
			ctx.Next()
		})
	})
}
```

### CORS Headers

```go
plugin.AddPhaseHandler("cors", func(phase *sdk.PhaseHandler) {
	phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
		req := ctx.Request()
		res := ctx.Response()
		
		origin := req.Header("Origin")
		
		// Check if origin is allowed
		allowedOrigins := []string{
			"https://example.com",
			"https://app.example.com",
		}
		
		allowed := false
		for _, o := range allowedOrigins {
			if origin == o {
				allowed = true
				break
			}
		}
		
		if allowed {
			res.SetHeader("Access-Control-Allow-Origin", origin)
			res.SetHeader("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
			res.SetHeader("Access-Control-Allow-Headers", "Content-Type, Authorization")
			res.SetHeader("Access-Control-Allow-Credentials", "true")
			res.SetHeader("Access-Control-Max-Age", "3600")
		}
		
	// Handle preflight
	if req.Method() == "OPTIONS" {
		res.SetStatus(204)
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
		
		ctx.Next()
	})
})
```

### Request ID Tracking

```go
plugin.AddPhaseHandler("request-id", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		res := ctx.Response()
		
		// Generate request ID
		requestID := uuid.New().String()
		
		// Add to outbound response so clients can correlate
		res.SetHeader("X-Request-ID", requestID)
		res.SetHeader("X-Correlation-ID", requestID)

		log.Printf("[request-id] %s %s -> %s",
			ctx.Request().Method(),
			ctx.Request().Path(),
			requestID,
		)
		
		ctx.Next()
	})
})
```

### Client Information Headers

```go
plugin.AddPhaseHandler("client-info", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		res := ctx.Response()
		
		// Add client information for backend
		res.SetHeader("X-Client-IP", req.ClientIP())
		res.SetHeader("X-Forwarded-For", req.ClientIP())
		res.SetHeader("X-Forwarded-Proto", "https")
		res.SetHeader("X-Forwarded-Host", req.Host())
		res.SetHeader("X-Real-IP", req.ClientIP())
		
		ctx.Next()
	})
})
```

### Cache Control

```go
plugin.AddPhaseHandler("cache-control", func(phase *sdk.PhaseHandler) {
	phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
		req := ctx.Request()
		res := ctx.Response()
		
		path := req.Path()
		
		// Static assets - cache for 1 year
		if strings.HasPrefix(path, "/static/") ||
		   strings.HasPrefix(path, "/assets/") {
			res.SetHeader("Cache-Control", "public, max-age=31536000, immutable")
			return
		}
		
		// API - no cache
		if strings.HasPrefix(path, "/api/") {
			res.SetHeader("Cache-Control", "no-store, no-cache, must-revalidate")
			res.SetHeader("Pragma", "no-cache")
			res.SetHeader("Expires", "0")
			return
		}
		
		// HTML - short cache
		if strings.HasSuffix(path, ".html") || path == "/" {
			res.SetHeader("Cache-Control", "public, max-age=300")
			return
		}
		
		ctx.Next()
	})
})
```

### Content Type Override

```go
plugin.AddPhaseHandler("content-type", func(phase *sdk.PhaseHandler) {
	phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
		req := ctx.Request()
		res := ctx.Response()
		
		path := req.Path()
		
		// Set content type based on path
		if strings.HasSuffix(path, ".json") {
			res.SetHeader("Content-Type", "application/json; charset=utf-8")
		} else if strings.HasSuffix(path, ".xml") {
			res.SetHeader("Content-Type", "application/xml; charset=utf-8")
		} else if strings.HasSuffix(path, ".js") {
			res.SetHeader("Content-Type", "application/javascript; charset=utf-8")
		} else if strings.HasSuffix(path, ".css") {
			res.SetHeader("Content-Type", "text/css; charset=utf-8")
		}
		
		ctx.Next()
	})
})
```

### Custom Analytics Headers

```go
plugin.AddPhaseHandler("analytics", func(phase *sdk.PhaseHandler) {
	phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
		req := ctx.Request()
		res := ctx.Response()
		
		// Add analytics headers
		res.SetHeader("X-Request-Time", time.Now().Format(time.RFC3339))
		res.SetHeader("X-User-Agent", req.Header("User-Agent"))
		res.SetHeader("X-Referer", req.Header("Referer"))
		
		ctx.Next()
	})
	
	phase.Logging(func(ctx *sdk.PhaseLogging) {
		req := ctx.Request()
		res := ctx.Response()
		
		// Log with all analytics data
		log.Printf("[Analytics] %s %s | Status: %d | Duration: %dms | UA: %s",
			req.Method(),
			req.Path(),
			res.Status(),
			res.Duration(),
			req.Header("User-Agent"),
		)
		
		ctx.Next()
	})
})
```

## Configuration Examples

### Security Headers

```yaml
middleware_groups:
  security:
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-frame-options
            value: "DENY"
          - name: x-content-type-options
            value: "nosniff"
          - name: x-xss-protection
            value: "1; mode=block"
          - name: referrer-policy
            value: "no-referrer"
          - name: content-security-policy
            value: "default-src 'self'"
          - name: strict-transport-security
            value: "max-age=31536000; includeSubDomains"
        remove:
          - server
          - x-powered-by
```

### Request Tracking

```yaml
middleware_groups:
  tracking:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${uuid(v7)}"
          - name: x-request-time
            value: "${timestamp()}"
          - name: x-client-ip
            value: "${request(client_ip)}"
    
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${header(x-request-id)}"
          - name: x-response-time
            value: "${timestamp()}"
```

### Combined Example

```yaml
plugins:
  - name: headers
    type: ffi
    file: ./headers.so

middleware_groups:
  web:
    # Built-in security headers
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-frame-options
            value: "DENY"
        remove:
          - server
    
    # Custom CORS handler
    - plugin: headers
      entry: "cors"
    
    # Request tracking
    - plugin: headers
      entry: "request-id"

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    middleware:
      - group: web
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: api-service
```

## Best Practices

### 1. Always Include Security Headers

```yaml
middleware_groups:
  security:
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-frame-options
            value: "DENY"
          - name: strict-transport-security
            value: "max-age=31536000"
```

### 2. Remove Sensitive Information

```yaml
remove:
  - server
  - x-powered-by
  - x-aspnet-version
  - x-version
```

### 3. Use Template Expressions

```yaml
set:
  - name: x-request-id
    value: "${uuid(v7)}"  # Dynamic
  - name: x-client-ip
    value: "${request(client_ip)}"
```

### 4. Set Headers Early

```go
// ✅ In RequestFilter for backend
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    ctx.Response().SetHeader("X-Custom", "value")
    ctx.Next()
})

// ✅ In ResponseFilter for client
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    ctx.Response().SetHeader("X-Custom", "value")
    ctx.Next()
})
```

### 5. Handle CORS Properly

```go
// Always handle preflight
if req.Method() == "OPTIONS" {
    res.SetStatus(204)
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    ctx.End()
    return
}
```

## See Also

- [Middleware](/core/middleware) - Middleware configuration
- [Response Handling](/plugins/response) - Response modification
- [Configuration](/core/configuration) - Header configuration
