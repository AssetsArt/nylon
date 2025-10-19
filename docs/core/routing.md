# Routing

Nylon provides flexible routing based on hostnames and paths with parameter extraction and rewriting capabilities.

## Route Matchers

Routes can match requests based on two criteria:

### Host-Based Routing

Match requests by hostname:

```yaml
routes:
  - route:
      type: host
      value: api.example.com|api.internal|api.staging
    name: api-route
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: api-service

  - route:
      type: host
      value: admin.example.com
    name: admin-route
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: admin-service
```

### Header-Based Routing

Match requests by header value (requires `header_selector` to be set):

```yaml
header_selector: x-nylon-proxy

routes:
  - route:
      type: header
      value: tenant-admin
    name: admin-route
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: admin-service
```

## Path Patterns

### Wildcard Matching

Use `*` to match any path segment:

```yaml
paths:
  # Match /api/users, /api/posts, etc.
  - path: /api/{*path}
    service:
      name: api-service

  # Match all paths
  - path: /{*path}
    service:
      name: default-service
```

### Parameter Extraction

Extract path parameters using `:name` syntax:

```yaml
paths:
  # Extract user ID: /users/123 -> params["id"] = "123"
  - path: /users/{id}
    service:
      name: user-service

  # Multiple parameters: /users/123/posts/456
  - path: /users/{user_id}/posts/{post_id}
    service:
      name: post-service

  # With wildcard: /users/123/anything/else
  - path: /users/{id}/{*path}
    service:
      name: user-service
```

Access parameters in plugins:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    params := ctx.Request().Params()
    userID := params["id"]
    fmt.Printf("User ID: %s\n", userID)
    ctx.Next()
})
```

## Path Rewriting

Rewrite paths before forwarding to backend:

```yaml
paths:
  # Rewrite /old-api/* to /new-api/*
  - path: /old-api/{*path}
    service:
      name: api-service
      rewrite: /new-api

  # Remove prefix: /api/v1/* -> /*
  - path: /api/v1/{*path}
    service:
      name: api-service
      rewrite: /

  # Add prefix: /* -> /backend/*
  - path: /{*path}
    service:
      name: backend
      rewrite: /backend
```

## Method Filtering

Restrict paths to specific HTTP methods:

```yaml
paths:
  # Only GET and POST
  - path: /api/users
    service:
      name: api-service
    methods:
      - GET
      - POST

  # Only DELETE
  - path: /api/users/{id}
    service:
      name: api-service
    methods:
      - DELETE
```

Available methods: `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, `TRACE`

## Multiple Paths

Define multiple paths for a single route:

```yaml
routes:
  - route:
      type: host
      value: example.com
    name: main
    paths:
      # API endpoints
      - path: /api/{*path}
        service:
          name: api-service
        middleware:
          - plugin: auth
            entry: "check"

      # Static files
      - path: /static/{*path}
        service:
          name: static-files

      # Admin panel
      - path: /admin/{*path}
        service:
          name: admin-service
        middleware:
          - plugin: auth
            entry: "admin-check"

      # Default fallback
      - path:
          - /
          - /{*path}
        service:
          name: default-service
```

## Route Priority

Route matching is handled by [`matchit` v0.8](https://docs.rs/crate/matchit/latest), which evaluates routes segment by segment using a radix tree. When multiple paths can match the same request, `matchit` applies these priority rules:

- Static segments (for example, `/api/health`) are evaluated before dynamic segments at the same position.
- Named parameters such as `/{resource}` win over catch-all parameters.
- Catch-all segments (`/{*path}`) are only considered after no static or named parameter routes match.

```yaml
paths:
  # Static segment – matched first
  - path: /api/health
    service:
      name: health-service

  # Named parameter – matched after static routes
  - path: /api/{resource}
    service:
      name: api-service

  # Catch-all – matched last
  - path: /{*path}
    service:
      name: default-service
```

## Complex Routing Example

```yaml
routes:
  # API domain
  - route:
      type: host
      value: api.example.com
    name: api
    tls:
      enabled: true
    middleware:
      - group: security
    paths:
      # Public endpoints
      - path: /public/{*path}
        service:
          name: public-api

      # Authenticated endpoints
      - path: /v1/{*path}
        service:
          name: api-v1
        middleware:
          - plugin: auth
            entry: "jwt"

      # Admin endpoints
      - path: /admin/{*path}
        service:
          name: admin-api
        methods:
          - GET
          - POST
        middleware:
          - plugin: auth
            entry: "admin"

  # Static files domain
  - route:
      type: host
      value: static.example.com
    name: static
    tls:
      enabled: true
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: cdn

  # Catch-all
  - route:
      type: host
      value: "*"
    name: default
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: default-backend
```

## Dynamic Routing

Use header selectors to choose different routing configurations:

```yaml
# In proxy config
header_selector: x-nylon-proxy

# Multiple proxy configs can exist
# Request with header "x-nylon-proxy: staging" uses staging config
```

## Best Practices

### 1. Order Routes by Specificity

```yaml
paths:
  - path: /api/health      # Exact
  - path: /api/{id}         # Parameter
  - path: /{*path}               # Wildcard
```

### 2. Use Middleware for Common Logic

```yaml
middleware_groups:
  api:
    - plugin: auth
      entry: "check"
    - plugin: rate-limit
      entry: "limit"

paths:
  - path: /api/{id}
    service:
      name: api-service
    middleware:
      - group: api  # Apply all at once
```

### 3. Separate by Domain

```yaml
# api.example.com
routes:
  - route:
      type: host
      value: api.example.com
    name: api
    paths: [...]

# www.example.com
  - route:
      type: host
      value: www.example.com
    name: www
    paths: [...]
```

### 4. Use Path Parameters

```yaml
# Instead of this:
paths:
  - path: /users/profile/{*path}
    service: user-service

# Do this:
paths:
  - path: /users/{id}/{*path}
    service: user-service
```

## See Also

- [Configuration](/core/configuration) - Full configuration reference
- [Middleware](/core/middleware) - Apply logic to routes
- [Examples](/examples/basic-proxy) - Routing examples
