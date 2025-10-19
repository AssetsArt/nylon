# Request Handling

Learn how to handle HTTP requests in your Go plugins.

## Request Object

Access request information through the `Request` object:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    // Request details
    method := req.Method()      // GET, POST, etc.
    url := req.URL()           // Full URL
    path := req.Path()         // /api/users
    query := req.Query()       // ?key=value
    host := req.Host()         // example.com
    clientIP := req.ClientIP() // 192.168.1.1
    
    ctx.Next()
})
```

## Request Methods

### Method()

Get HTTP method:

```go
method := req.Method()
// "GET", "POST", "PUT", "DELETE", etc.

if method == "POST" {
    // Handle POST request
}
```

### URL()

Get full URL (excluding standard ports):

```go
url := req.URL()
// http://example.com/api/users?id=123
// https://example.com/api/users (no :443)

fmt.Printf("Full URL: %s\n", url)
```

### Path()

Get request path:

```go
path := req.Path()
// /api/users/123

if strings.HasPrefix(path, "/api/") {
    // API request
}
```

### Query()

Get query string:

```go
query := req.Query()
// key1=value1&key2=value2

// Parse query parameters
params, _ := url.ParseQuery(query)
id := params.Get("id")
```

### Params()

Get path parameters (from route matching):

```go
// Route: /users/:id/posts/:post_id
params := req.Params()

userID := params["id"]      // "123"
postID := params["post_id"] // "456"
```

### Host()

Get hostname:

```go
host := req.Host()
// example.com
// api.example.com:8080

if host == "admin.example.com" {
    // Admin request
}
```

### ClientIP()

Get client IP address:

```go
clientIP := req.ClientIP()
// 192.168.1.1
// 10.0.0.1

fmt.Printf("Request from: %s\n", clientIP)
```

### Headers()

Get all request headers:

```go
headers := req.Headers()
// map[string]string

userAgent := headers["user-agent"]
contentType := headers["content-type"]
auth := headers["authorization"]
```

### Header()

Get single header:

```go
userAgent := req.Header("User-Agent")
auth := req.Header("Authorization")
apiKey := req.Header("X-API-Key")

if apiKey == "" {
    res := ctx.Response()
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    res.SetStatus(401)
    res.BodyText("Missing API key")
    ctx.End()
    return
}
```

### Bytes()

Get request body size:

```go
bytes := req.Bytes()
// Request content-length in bytes

fmt.Printf("Request size: %d bytes\n", bytes)
```

### Timestamp()

Get request timestamp (milliseconds since epoch):

```go
timestamp := req.Timestamp()
// 1704067200000

fmt.Printf("Request time: %d\n", timestamp)
```

## Examples

### Authentication

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    // Check API key
    apiKey := req.Header("X-API-Key")
    if apiKey == "" {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(401)
        res.BodyText("Missing API key")
        ctx.End()
        return
    }
    
    if !validateAPIKey(apiKey) {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(401)
        res.BodyText("Invalid API key")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

### Rate Limiting by IP

```go
var rateLimiter = make(map[string]int)
var mu sync.Mutex

phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    clientIP := req.ClientIP()
    
    mu.Lock()
    count := rateLimiter[clientIP]
    count++
    rateLimiter[clientIP] = count
    mu.Unlock()
    
    if count > 100 {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(429)
        res.BodyText("Too many requests")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

### Method Filtering

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    // Only allow GET and POST
    if req.Method() != "GET" && req.Method() != "POST" {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(405)
        res.BodyText("Method not allowed")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

### Path-Based Routing

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    path := req.Path()
    
    if strings.HasPrefix(path, "/admin/") {
        // Check admin permission
        if !isAdmin(req.Header("Authorization")) {
            res := ctx.Response()
            res.RemoveHeader("Content-Length")
            res.SetHeader("Transfer-Encoding", "chunked")
            res.SetStatus(403)
            res.BodyText("Admin access required")
            ctx.End()
            return
        }
    }
    
    ctx.Next()
})
```

### Request Logging

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    log.Printf("[%s] %s %s from %s",
        req.Method(),
        req.Path(),
        req.Host(),
        req.ClientIP(),
    )
    
    // Add request ID
    requestID := uuid.New().String()
    ctx.Response().SetHeader("X-Request-ID", requestID)
    
    ctx.Next()
})
```

### Query Parameter Validation

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    query := req.Query()
    params, _ := url.ParseQuery(query)
    
    // Require API version
    version := params.Get("v")
    if version == "" {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(400)
        res.BodyText("API version required")
        ctx.End()
        return
    }
    
    if version != "1" && version != "2" {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(400)
        res.BodyText("Invalid API version")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

### Host-Based Access Control

```go
var allowedHosts = map[string]bool{
    "api.example.com": true,
    "api-staging.example.com": true,
}

phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    host := req.Host()
    
    // Remove port if present
    if idx := strings.Index(host, ":"); idx != -1 {
        host = host[:idx]
    }
    
    if !allowedHosts[host] {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(403)
        res.BodyText("Host not allowed")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

### User Agent Blocking

```go
var blockedAgents = []string{"bot", "crawler", "spider"}

phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    userAgent := strings.ToLower(req.Header("User-Agent"))
    
    for _, blocked := range blockedAgents {
        if strings.Contains(userAgent, blocked) {
            res := ctx.Response()
            res.RemoveHeader("Content-Length")
            res.SetHeader("Transfer-Encoding", "chunked")
            res.SetStatus(403)
            res.BodyText("Blocked")
            ctx.End()
            return
        }
    }
    
    ctx.Next()
})
```

### Path Parameter Extraction

```go
// Route: /users/:id/posts/:post_id

phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    params := req.Params()
    
    userID := params["id"]
    postID := params["post_id"]
    
    // Validate IDs
    if userID == "" || postID == "" {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(400)
        res.BodyText("Invalid parameters")
        ctx.End()
        return
    }
    
    // Add to headers for backend
    ctx.Response().SetHeader("X-User-ID", userID)
    ctx.Response().SetHeader("X-Post-ID", postID)
    
    ctx.Next()
})
```

### Request Size Limit

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    // Limit to 10MB
    maxSize := int64(10 * 1024 * 1024)
    if req.Bytes() > maxSize {
        res := ctx.Response()
        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")
        res.SetStatus(413)
        res.BodyText("Request too large")
        ctx.End()
        return
    }
    
    ctx.Next()
})
```

## Working with Payload

Pass data between middleware phases:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    req := ctx.Request()
    
    // Extract user from token
    token := req.Header("Authorization")
    user := validateToken(token)
    
    // Store in payload
    ctx.SetPayload(map[string]interface{}{
        "user_id": user.ID,
        "role": user.Role,
        "timestamp": time.Now().Unix(),
    })
    
    ctx.Next()
})

phase.ResponseFilter(func(ctx *sdk.PhaseResponseFilter) {
    // Access payload from request phase
    payload := ctx.GetPayload()
    userID := payload["user_id"].(string)
    
    // Add to response
    ctx.SetResponseHeader("X-User-ID", userID)
    ctx.Next()
})
```

## Best Practices

### 1. Fail Fast

```go
// ✅ Good
if apiKey == "" {
    res := ctx.Response()
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    res.SetStatus(401)
    res.BodyText("Unauthorized")
    ctx.End()
    return
}
ctx.Next()

// ❌ Bad
if apiKey != "" {
    ctx.Next()
}
// Continues even if unauthorized
```

### 2. Use Early Returns

```go
// ✅ Good
if !authorized {
    res := ctx.Response()
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    res.SetStatus(403)
    res.BodyText("Forbidden")
    ctx.End()
    return
}

if !validMethod {
    res := ctx.Response()
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    res.SetStatus(405)
    res.BodyText("Method not allowed")
    ctx.End()
    return
}

ctx.Next()
```

### 3. Log Important Events

```go
req := ctx.Request()
log.Printf("[%s] %s %s from %s", 
    req.Method(), req.Path(), req.Host(), req.ClientIP())
```

### 4. Validate Input

```go
// Always validate before use
params := req.Params()
id := params["id"]

if id == "" {
    res := ctx.Response()
    res.RemoveHeader("Content-Length")
    res.SetHeader("Transfer-Encoding", "chunked")
    res.SetStatus(400)
    res.BodyText("Missing ID")
    ctx.End()
    return
}
```

### 5. Set Response Headers Early

```go
// Set before calling Next()
ctx.Response().SetHeader("X-Request-ID", requestID)
ctx.Next()
```

## See Also

- [Response Handling](/plugins/response) - Handle responses
- [Plugin Phases](/plugins/phases) - Understanding phases
- [Go SDK](/plugins/go-sdk) - Complete SDK reference

