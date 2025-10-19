# Middleware

Middleware allows you to process requests and responses at different stages of the proxy lifecycle.

## Overview

Middleware in Nylon can be:
1. **Built-in plugins** - Native Rust plugins for common tasks
2. **Go plugins** - Custom logic written in Go

## Middleware Groups

Reusable sets of middleware that can be applied to multiple routes:

```yaml
middleware_groups:
  # Security headers
  security:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${uuid(v7)}"
          - name: x-forwarded-for
            value: "${request(client_ip)}"
    
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-frame-options
            value: "DENY"
          - name: x-content-type-options
            value: "nosniff"

  # Authentication
  auth:
    - plugin: auth-plugin
      entry: "jwt"
    - plugin: auth-plugin
      entry: "rbac"

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    middleware:
      - group: security
      - group: auth
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: api-service
```

## Built-in Plugins

### RequestHeaderModifier

Modify request headers before forwarding to backend:

```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      # Add or set headers
      set:
        - name: x-custom-header
          value: "custom-value"
        - name: x-request-id
          value: "${uuid(v7)}"
        - name: x-client-ip
          value: "${request(client_ip)}"
        - name: x-forwarded-for
          value: "${request(client_ip)}"
      
      # Remove headers
      remove:
        - x-internal-header
        - x-debug-token
```

### ResponseHeaderModifier

Modify response headers from backend:

```yaml
middleware:
  - plugin: ResponseHeaderModifier
    payload:
      # Add or set headers
      set:
        - name: x-server
          value: "nylon"
        - name: cache-control
          value: "no-cache, no-store, must-revalidate"
        - name: x-frame-options
          value: "DENY"
        - name: strict-transport-security
          value: "max-age=31536000"
      
      # Remove headers
      remove:
        - server
        - x-powered-by
```

## Template Expressions

Use dynamic values in header modifications:

### Available Functions

| Function | Description | Example |
|----------|-------------|---------|
| `${header(name)}` | Request header value | `${header(user-agent)}` |
| `${query(name[, default])}` | Query parameter | `${query(version, 'v1')}` |
| `${cookie(name[, default])}` | Cookie value | `${cookie(session_id)}` |
| `${param(name[, default])}` | Route parameter | `${param(user_id)}` |
| `${request(field)}` | Request metadata (`client_ip`, `host`, `method`, `path`, `scheme`, `tls`) | `${request(method)}` |
| `${env(VAR_NAME)}` | Environment variable | `${env(SERVER_NAME)}` |
| `${uuid(v4|v7)}` | Generate UUID | `${uuid(v7)}` |
| `${timestamp()}` | Current timestamp (RFC3339) | `${timestamp()}` |
| `${or(a, b, …)}` | First non-empty value | `${or(env(NAME), 'default')}` |
| `${eq(a, b[, value])}` | Optional value when `a == b` | `${eq(request(method), 'GET', 'cacheable')}` |
| `${neq(a, b[, value])}` | Optional value when `a != b` | `${neq(request(scheme), 'https', 'insecure')}` |
| `${concat(values…)}` | Concatenate all arguments | `${concat(header(host), '-', uuid(v4))}` |
| `${upper(value)}` / `${lower(value)}` | Case conversion | `${upper(param(region))}` |
| `${len(value)}` | String length | `${len(header(user-agent))}` |
| `${if_cond(condition, then, else)}` | Conditional evaluation | `${if_cond(request(tls), 'https', 'http')}` |

### Examples

```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      set:
        # Generate unique request ID
        - name: x-request-id
          value: "${uuid(v7)}"
        
        # Forward client IP
        - name: x-forwarded-for
          value: "${request(client_ip)}"
        
        # Copy header
        - name: x-original-host
          value: "${header(host)}"
        
        # Environment variable with fallback
        - name: x-server-name
          value: "${or(env(SERVER_NAME), 'nylon-proxy')}"
        
        # Timestamp
        - name: x-request-time
          value: "${timestamp()}"
```

## Go Plugin Middleware

Use custom Go plugins for complex logic:

```yaml
plugins:
  - name: auth
    type: ffi
    file: ./auth.so
    config:
      jwt_secret: "your-secret"

middleware_groups:
  authenticated:
    - plugin: auth
      entry: "jwt-check"
      payload:
        required_scope: "api:read"

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    paths:
      # Public endpoint
      - path: /public/*
        service:
          name: api-service
      
      # Protected endpoint
      - path: /private/*
        service:
          name: api-service
        middleware:
          - group: authenticated
```

### Plugin with Payload

Pass configuration to plugins:

```yaml
paths:
  - path: /admin/*
    service:
      name: admin-service
    middleware:
      - plugin: auth
        entry: "check"
        payload:
          role: "admin"
          permissions:
            - "admin:read"
            - "admin:write"
```

Access payload in plugin:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    payload := ctx.GetPayload()
    role := payload["role"].(string)
    
    if role != "admin" {
        res := ctx.Response()

        res.RemoveHeader("Content-Length")
        res.SetHeader("Transfer-Encoding", "chunked")

        res.SetStatus(403)
        res.BodyText("Forbidden")

        ctx.End()
        return
    }
    
    ctx.Next()
})
```

## Middleware Execution Order

Middleware executes in the order defined:

```yaml
middleware_groups:
  api:
    - plugin: logging        # 1. First
      entry: "start"
    - plugin: auth          # 2. Second
      entry: "check"
    - plugin: rate-limit    # 3. Third
      entry: "limit"
    - plugin: transform     # 4. Last
      entry: "modify"
```

### Route-Level vs Path-Level

```yaml
routes:
  - route:
      type: host
      value: api.example.com
    name: api
    middleware:
      - group: security  # Applied to ALL paths
    paths:
      - path: /private/*
        service:
          name: api-service
        middleware:
          - group: auth    # Applied only to /private/*
```

**Execution order:**
1. Route-level middleware
2. Path-level middleware
3. Backend request
4. Path-level middleware (response phase)
5. Route-level middleware (response phase)

## Common Middleware Patterns

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

### Request ID Tracking

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
    
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${header(x-request-id)}"
```

### CORS Headers

```yaml
middleware_groups:
  cors:
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: access-control-allow-origin
            value: "*"
          - name: access-control-allow-methods
            value: "GET, POST, PUT, DELETE, OPTIONS"
          - name: access-control-allow-headers
            value: "Content-Type, Authorization"
          - name: access-control-max-age
            value: "3600"
```

### Client Information

```yaml
middleware_groups:
  client_info:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-client-ip
            value: "${request(client_ip)}"
          - name: x-forwarded-for
            value: "${request(client_ip)}"
          - name: x-forwarded-proto
            value: "https"
          - name: x-original-host
            value: "${header(host)}"
```

## Best Practices

### 1. Use Middleware Groups

```yaml
# ✅ Good - Reusable
middleware_groups:
  api:
    - plugin: auth
    - plugin: rate-limit

routes:
  - middleware:
      - group: api

# ❌ Bad - Repetitive
routes:
  - middleware:
      - plugin: auth
      - plugin: rate-limit
```

### 2. Order Matters

```yaml
# ✅ Good - Auth before rate limit
middleware:
  - group: security
  - group: auth
  - group: rate-limit
  - group: transform

# ❌ Bad - Rate limit before auth
middleware:
  - group: rate-limit  # Wastes resources on unauthorized requests
  - group: auth
```

### 3. Minimize Middleware

```yaml
# ✅ Good - Only what's needed
paths:
  - path: /public/*
    middleware: []  # No auth for public

  - path: /private/*
    middleware:
      - group: auth

# ❌ Bad - Unnecessary middleware
paths:
  - path: /public/*
    middleware:
      - group: auth  # Not needed for public
```

### 4. Use Template Expressions

```yaml
# ✅ Good - Dynamic values
set:
  - name: x-request-id
    value: "${uuid(v7)}"

# ❌ Bad - Static values
set:
  - name: x-request-id
    value: "fixed-id"  # Same for all requests
```

### 5. Security First

```yaml
# Always include security headers
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

## See Also

- [Configuration](/core/configuration) - Middleware configuration reference
- [Plugin Phases](/plugins/phases) - Understanding plugin execution
- [Examples](/examples/custom-headers) - Middleware examples
