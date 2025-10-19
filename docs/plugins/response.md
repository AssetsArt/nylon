# Response Handling

Learn how to handle and modify HTTP responses in your Go plugins.

## Response Object

Access and modify responses through the `Response` object:

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    // Set status
    res.SetStatus(200)
    
    // Set headers
    res.SetHeader("X-Server", "Nylon")
    
    // Remove headers
    res.RemoveHeader("Server")
    
    ctx.Next()
})
```

## Response Phases

### ResponseFilter

Modify response headers and status before body is processed:

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    // Inject security headers
    res.SetHeader("X-Frame-Options", "DENY")
    res.SetHeader("X-Content-Type-Options", "nosniff")
    
    ctx.Next()
})
```

### ResponseBodyFilter

Modify response body content:

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    
    // Read body
    body := res.ReadBody()
    
    // Modify body
    modifiedBody := append(body, []byte("\n<!-- Injected -->")...)
    
    // Write modified body
    res.BodyRaw(modifiedBody)
    
    ctx.Next()
})
```

**Important:** When modifying body, you must handle Content-Length:

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    // Remove Content-Length
    ctx.RemoveResponseHeader("Content-Length")
    
    // Use chunked encoding
    ctx.SetResponseHeader("Transfer-Encoding", "chunked")
    
    ctx.Next()
})
```

### Logging

Access final response information:

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
    req := ctx.Request()
    res := ctx.Response()
    
    log.Printf("%s %s -> %d (%d bytes, %dms)",
        req.Method(),
        req.Path(),
        res.Status(),
        res.Bytes(),
        res.Duration(),
    )
    
    ctx.Next()
})
```

## Response Methods

### SetStatus(code int)

Set HTTP status code:

```go
res.SetStatus(200)  // OK
res.SetStatus(404)  // Not Found
res.SetStatus(500)  // Internal Server Error
```

### Status() int

Get response status:

```go
status := res.Status()
// 200, 404, 500, etc.

if status >= 500 {
    // Server error
}
```

### SetHeader(name, value string)

Set response header:

```go
res.SetHeader("Content-Type", "application/json")
res.SetHeader("Cache-Control", "no-cache")
res.SetHeader("X-Server", "Nylon")
```

### RemoveHeader(name string)

Remove response header:

```go
res.RemoveHeader("Server")
res.RemoveHeader("X-Powered-By")
```

### Headers() map[string]string

Get all response headers:

```go
headers := res.Headers()

contentType := headers["content-type"]
cacheControl := headers["cache-control"]
```

### BodyRaw(data []byte)

Set response body (raw bytes):

```go
body := []byte("Hello, World!")
res.BodyRaw(body)
```

### BodyText(text string)

Set response body (text):

```go
res.BodyText("Hello, World!")
```

### BodyJSON(data interface{})

Set response body (JSON):

```go
data := map[string]interface{}{
    "message": "Success",
    "code": 200,
}
res.BodyJSON(data)
```

### ReadBody() []byte

Read response body (in ResponseBodyFilter phase):

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    body := res.ReadBody()
    
    // Modify body
    // ...
    
    ctx.Next()
})
```

### Bytes() int64

Get response body size:

```go
bytes := res.Bytes()
fmt.Printf("Response size: %d bytes\n", bytes)
```

### Duration() int64

Get request duration in milliseconds:

```go
duration := res.Duration()
fmt.Printf("Request took: %dms\n", duration)
```

### Error() string

Get error message (if any):

```go
err := res.Error()
if err != "" {
    log.Printf("Error: %s\n", err)
}
```

## Examples

### Security Headers

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    // Security headers
    res.SetHeader("X-Frame-Options", "DENY")
    res.SetHeader("X-Content-Type-Options", "nosniff")
    res.SetHeader("X-XSS-Protection", "1; mode=block")
    res.SetHeader("Referrer-Policy", "no-referrer")
    res.SetHeader("Content-Security-Policy", "default-src 'self'")
    
    // Remove server info
    res.RemoveHeader("Server")
    res.RemoveHeader("X-Powered-By")
    
    ctx.Next()
})
```

### CORS Headers

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    res.SetHeader("Access-Control-Allow-Origin", "*")
    res.SetHeader("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE")
    res.SetHeader("Access-Control-Allow-Headers", "Content-Type, Authorization")
    res.SetHeader("Access-Control-Max-Age", "3600")
    
    ctx.Next()
})
```

### Custom Error Responses

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res := ctx.Response()
    
    status := res.Status()
    
    if status == 404 {
        res.SetHeader("Content-Type", "application/json")
        res.BodyJSON(map[string]interface{}{
            "error": "Not Found",
            "code": 404,
            "message": "The requested resource was not found",
        })
    }
    
    if status >= 500 {
        res.SetHeader("Content-Type", "application/json")
        res.BodyJSON(map[string]interface{}{
            "error": "Internal Server Error",
            "code": status,
        })
    }
    
    ctx.Next()
})
```

### Response Body Injection

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    // Prepare for body modification
    ctx.RemoveResponseHeader("Content-Length")
    ctx.SetResponseHeader("Transfer-Encoding", "chunked")
    ctx.Next()
})

phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    body := res.ReadBody()
    
    // Inject analytics script before </body>
    script := []byte(`<script src="/analytics.js"></script></body>`)
    modifiedBody := bytes.Replace(body, []byte("</body>"), script, 1)
    
    res.BodyRaw(modifiedBody)
    ctx.Next()
})
```

### Response Compression

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    body := res.ReadBody()
    
    // Compress if large
    if len(body) > 1024 {
        var buf bytes.Buffer
        gz := gzip.NewWriter(&buf)
        gz.Write(body)
        gz.Close()
        
        res.SetHeader("Content-Encoding", "gzip")
        res.RemoveHeader("Content-Length")
        res.BodyRaw(buf.Bytes())
    }
    
    ctx.Next()
})
```

### Cache Control

```go
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    req := ctx.Request()
    res := ctx.Response()
    
    path := req.Path()
    
    // Static assets - cache for 1 year
    if strings.HasPrefix(path, "/static/") {
        res.SetHeader("Cache-Control", "public, max-age=31536000, immutable")
    }
    
    // API - no cache
    if strings.HasPrefix(path, "/api/") {
        res.SetHeader("Cache-Control", "no-store, no-cache, must-revalidate")
    }
    
    ctx.Next()
})
```

### Response Transformation

```go
phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    res := ctx.Response()
    
    // Only transform JSON
    contentType := res.Headers()["content-type"]
    if !strings.Contains(contentType, "application/json") {
        ctx.Next()
        return
    }
    
    body := res.ReadBody()
    
    // Parse JSON
    var data map[string]interface{}
    json.Unmarshal(body, &data)
    
    // Add metadata
    data["_meta"] = map[string]interface{}{
        "timestamp": time.Now().Unix(),
        "version": "1.0",
    }
    
    // Encode back to JSON
    modifiedBody, _ := json.Marshal(data)
    res.BodyRaw(modifiedBody)
    
    ctx.Next()
})
```

### Performance Monitoring

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
    req := ctx.Request()
    res := ctx.Response()
    
    duration := res.Duration()
    status := res.Status()
    
    // Log slow requests
    if duration > 1000 {
        log.Printf("[SLOW] %s %s took %dms (status: %d)",
            req.Method(),
            req.Path(),
            duration,
            status,
        )
    }
    
    // Log errors
    if status >= 500 {
        log.Printf("[ERROR] %s %s failed with %d (error: %s)",
            req.Method(),
            req.Path(),
            status,
            res.Error(),
        )
    }
    
    ctx.Next()
})
```

### Access Logging

```go
phase.Logging(func(ctx *sdk.PhaseLogging) {
    req := ctx.Request()
    res := ctx.Response()
    
    log.Printf("%s - [%s] \"%s %s\" %d %d %dms \"%s\"",
        req.ClientIP(),
        time.Now().Format("02/Jan/2006:15:04:05 -0700"),
        req.Method(),
        req.Path(),
        res.Status(),
        res.Bytes(),
        res.Duration(),
        req.Header("User-Agent"),
    )
    
    ctx.Next()
})
```

## Early Response

Send response without contacting backend:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    res := ctx.Response()
    
    // Serve from cache
    if cached := getFromCache(req.Path()); cached != nil {
        res.SetStatus(200)
        res.SetHeader("X-Cache", "HIT")
        res.BodyRaw(cached)
        return  // Don't call Next() - skip backend
    }
    
    res.SetHeader("X-Cache", "MISS")
    ctx.Next()
})
```

## Streaming Responses

For streaming responses (SSE, chunked):

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    res := ctx.Response()
    
    res.SetStatus(200)
    res.SetHeader("Content-Type", "text/event-stream")
    res.SetHeader("Cache-Control", "no-cache")
    
    stream, err := res.Stream()
    if err != nil {
        res.SetStatus(500).BodyText("Stream error")
        return
    }
    
    // Write chunks
    stream.Write([]byte("data: hello\n\n"))
    stream.Write([]byte("data: world\n\n"))
    
    // End stream
    stream.End()
})
```

## Best Practices

### 1. Always Call Next()

```go
// ✅ Good
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res.SetHeader("X-Server", "Nylon")
    ctx.Next()  // Don't forget!
})

// ❌ Bad
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    res.SetHeader("X-Server", "Nylon")
    // Missing ctx.Next() - response will hang
})
```

### 2. Handle Body Modification Correctly

```go
// ✅ Good
phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    ctx.RemoveResponseHeader("Content-Length")
    ctx.SetResponseHeader("Transfer-Encoding", "chunked")
    ctx.Next()
})

phase.ResponseBodyFilter(func(ctx *sdk.PhaseResponseBodyFilter) {
    body := ctx.Response().ReadBody()
    // Modify body...
    ctx.Response().BodyRaw(modifiedBody)
    ctx.Next()
})
```

### 3. Check Content-Type

```go
contentType := res.Headers()["content-type"]
if strings.Contains(contentType, "application/json") {
    // Process JSON
}
```

### 4. Log in Logging Phase

```go
// ✅ Good - Use Logging phase
phase.Logging(func(ctx *sdk.PhaseLogging) {
    res := ctx.Response()
    log.Printf("Status: %d, Duration: %dms", res.Status(), res.Duration())
    ctx.Next()
})
```

### 5. Set Headers Before Body

```go
// ✅ Good
res.SetStatus(200)
res.SetHeader("Content-Type", "application/json")
res.BodyJSON(data)
```

## See Also

- [Request Handling](/plugins/request) - Handle requests
- [Plugin Phases](/plugins/phases) - Understanding phases
- [Go SDK](/plugins/go-sdk) - Complete SDK reference

