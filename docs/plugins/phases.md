# Plugin Phases

Understanding the different phases of request processing and when to use each one.

| Phase | When it runs | Common uses |
|-------|--------------|-------------|
| **RequestFilter** | Before routing/backends | AuthN/Z, validation, rewrites, early responses. |
| **ResponseFilter** | After upstream headers, before body | Header tweaks, status overrides, caching decisions. |
| **ResponseBodyFilter** | While streaming body chunks | Transformations, compression, redaction. |
| **Logging** | After request finishes | Metrics, structured logging, cleanup. |

## Request Lifecycle

```
Client Request
      │
      ▼
┌─────────────────┐
│ RequestFilter   │ ◄── Phase 1: Before routing
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Route Matching  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Backend Request │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ResponseFilter  │ ◄── Phase 2: After receiving headers
└────────┬────────┘
         │
         ▼
┌─────────────────────┐
│ ResponseBodyFilter  │ ◄── Phase 3: While streaming body
└────────┬────────────┘
         │
         ▼
┌─────────────────┐
│    Logging      │ ◄── Phase 4: After completion
└────────┬────────┘
         │
         ▼
   Client Response
```

## Phase 1: RequestFilter

Execute **before** the request is sent to the backend.

### When to Use
- Authentication and authorization
- Request validation
- Rate limiting
- Header manipulation
- Request transformation
- Early response (bypass backend)

### Available Methods

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Read request data
	method := req.Method()
	path := req.Path()
	url := req.URL()
	query := req.Query()
	headers := req.Headers().GetAll()
	body := req.RawBody()
	params := req.Params()
	host := req.Host()
	clientIP := req.ClientIP()
	timestamp := req.Timestamp()
	
	// Modify or send early response
	res.SetStatus(200)
	res.SetHeader("X-Custom", "value")
	res.BodyText("Early response")
	res.RemoveHeader("Content-Length")
	res.SetHeader("Transfer-Encoding", "chunked")
	ctx.End()
	
	// Continue or stop
	ctx.Next() // Continue to backend
	return     // Stop and send response
})
```

### Example: Authentication

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	token := req.Header("Authorization")
	
	if token == "" {
		res := ctx.Response()
		res.SetStatus(401)
		res.BodyText("Missing authorization token")
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
	
	// Validate token (pseudo-code)
	user, err := validateToken(token)
	if err != nil {
		res := ctx.Response()
		res.SetStatus(401)
		res.BodyText("Invalid token")
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
	
	ctx.Next() // Continue to backend
})
```

### Example: Rate Limiting

```go
var limiter = rate.NewLimiter(rate.Limit(100), 100) // 100 req/s

phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	if !limiter.Allow() {
		res := ctx.Response()
		res.SetStatus(429)
		res.SetHeader("Retry-After", "1")
		res.BodyText("Rate limit exceeded")
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
	
	ctx.Next()
})
```

## Phase 2: ResponseFilter

Execute **after** receiving response headers from backend, **before** body is sent.

### When to Use
- Response header modification
- Status code changes
- Caching logic
- Redirect handling
- Response validation

### Available Methods

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Access request data
	path := req.Path()
	method := req.Method()
	
	// Access response data
	status := res.Status()
	
	// Modify response headers
	res.SetHeader("X-Powered-By", "Nylon")
	res.RemoveHeader("Server")
	res.SetHeader("Cache-Control", "max-age=3600")
	
	// Access stored data
	payload := ctx.GetPayload()
	
	ctx.Next()
})
```

### Example: Add Security Headers

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	res := ctx.Response()
	
	// Add security headers
	res.SetHeader("X-Content-Type-Options", "nosniff")
	res.SetHeader("X-Frame-Options", "DENY")
	res.SetHeader("X-XSS-Protection", "1; mode=block")
	res.SetHeader("Strict-Transport-Security", "max-age=31536000")
	
	ctx.Next()
})
```

### Example: Custom Caching

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Cache GET requests with 200 status
	if req.Method() == "GET" && res.Status() == 200 {
		res.SetHeader("Cache-Control", "public, max-age=3600")
	} else {
		res.SetHeader("Cache-Control", "no-cache, no-store, must-revalidate")
	}
	
	ctx.Next()
})
```

## Phase 3: ResponseBodyFilter

Execute **while streaming** the response body.

### When to Use
- Body transformation
- Content filtering
- Compression
- Body modification

### Available Methods

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
	res := ctx.Response()
	
	// Read response body
	body := res.ReadBody()
	
	// Modify body
	modifiedBody := append(body, []byte("\n<!-- Modified -->")...)
	res.BodyRaw(modifiedBody)
	
	ctx.Next()
})
```

### Example: Add Footer to HTML

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
	res := ctx.Response()
	
	// Check if HTML response
	headers := res.Headers()
	if contentType, ok := headers["content-type"]; ok {
		if strings.Contains(contentType, "text/html") {
			body := res.ReadBody()
			
			// Add footer before </body>
			modified := bytes.Replace(
				body,
				[]byte("</body>"),
				[]byte(`<footer>Powered by Nylon</footer></body>`),
				1,
			)
			
			res.BodyRaw(modified)
		}
	}
	
	ctx.Next()
})
```

### Important Notes

::: warning
When modifying response body, you need to:
1. Remove `Content-Length` header
2. Set `Transfer-Encoding: chunked`
:::

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
	res := ctx.Response()
	res.RemoveHeader("Content-Length")
	res.SetHeader("Transfer-Encoding", "chunked")
	ctx.Next()
})
```

## Phase 4: Logging

Execute **after** the request is complete.

### When to Use
- Access logging
- Metrics collection
- Analytics
- Error tracking
- Audit logging

### Available Methods

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Request info
	method := req.Method()
	path := req.Path()
	host := req.Host()
	clientIP := req.ClientIP()
	reqBytes := req.Bytes()
	timestamp := req.Timestamp()
	
	// Response info
	status := res.Status()
	resBytes := res.Bytes()
	duration := res.Duration()
	headers := res.Headers()
	errorMsg := res.Error()
	
	// Stored data
	payload := ctx.GetPayload()
	
	ctx.Next()
})
```

### Example: Access Logging

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Log in Apache Combined format
	log.Printf(
		"%s - - [%s] \"%s %s\" %d %d \"%s\" \"%s\" %dms",
		req.ClientIP(),
		time.Unix(req.Timestamp()/1000, 0).Format("02/Jan/2006:15:04:05 -0700"),
		req.Method(),
		req.Path(),
		res.Status(),
		res.Bytes(),
		req.Header("Referer"),
		req.Header("User-Agent"),
		res.Duration(),
	)
	
	ctx.Next()
})
```

### Example: Metrics Collection

```go
var (
	requestCounter = prometheus.NewCounterVec(
		prometheus.CounterOpts{
			Name: "http_requests_total",
		},
		[]string{"method", "path", "status"},
	)
	requestDuration = prometheus.NewHistogramVec(
		prometheus.HistogramOpts{
			Name: "http_request_duration_ms",
		},
		[]string{"method", "path"},
	)
)

phase.Logging(func(ctx *sdk.PhaseLogging) {
	req := ctx.Request()
	res := ctx.Response()
	
	// Increment counter
	requestCounter.WithLabelValues(
		req.Method(),
		req.Path(),
		fmt.Sprintf("%d", res.Status()),
	).Inc()
	
	// Record duration
	requestDuration.WithLabelValues(
		req.Method(),
		req.Path(),
	).Observe(float64(res.Duration()))
	
	ctx.Next()
})
```

## Phase Communication

Per-request payload mutation is not yet supported in the Go SDK.  
`ctx.GetPayload()` returns the static `payload` that you configure in YAML for the middleware entry.

```yaml
middleware:
  - plugin: auth-plugin
    entry: "auth-handler"
    payload:
      audience: "admin-api"
      api_key: "${env(API_KEY)}"
```

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	payload := ctx.GetPayload()
	requiredKey, _ := payload["api_key"].(string)

	if ctx.Request().Header("X-API-Key") != requiredKey {
		res := ctx.Response()
		res.SetStatus(401)
		res.BodyText("Unauthorized")
		ctx.End()
		return
	}

	ctx.Next()
})
```

If you need to share dynamic state across phases today, store it in your own package-level cache keyed by request metadata (e.g., `req.ClientIP()` or UUID headers).

## Best Practices

### 1. Choose the Right Phase

- **RequestFilter**: Before backend (auth, validation, rate limiting)
- **ResponseFilter**: Modify headers only
- **ResponseBodyFilter**: Modify body content
- **Logging**: Read-only metrics and logs

### 2. Always Call ctx.Next()

Unless you want to stop processing:

```go
// Good: Continue processing
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	// do work
	ctx.Next() // ✅
})

// Good: Stop with early response
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	res := ctx.Response()
	res.SetStatus(401)
	res.BodyText("Unauthorized")
	res.RemoveHeader("Content-Length")
	res.SetHeader("Transfer-Encoding", "chunked")
	ctx.End()
	return
})
```

### 3. Handle Errors Gracefully

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	req := ctx.Request()
	body := req.RawBody()
	
	var data map[string]interface{}
	if err := json.Unmarshal(body, &data); err != nil {
		res := ctx.Response()
		res.SetStatus(400)
		res.BodyText("Invalid JSON: " + err.Error())
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
	
	ctx.Next()
})
```

### 4. Don't Block

Keep phase handlers fast and non-blocking:

```go
// Bad: Blocking operation
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	time.Sleep(5 * time.Second) // ❌ Don't do this
	ctx.Next()
})

// Good: Quick check
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
	token := req.Header("Authorization")
	if token == "" {
		// Fast validation
		res := ctx.Response()
		res.SetStatus(401)
		res.BodyText("Unauthorized")
		res.RemoveHeader("Content-Length")
		res.SetHeader("Transfer-Encoding", "chunked")
		ctx.End()
		return
	}
	ctx.Next()
})
```

---

**Next steps**

- [Go SDK reference](/plugins/go-sdk) – browse the complete helper API.
- [Request handling guide](/plugins/request) – focus on inbound request helpers.
- [Authentication example](/examples/authentication) – see multiple phases working together.
