# Routing

Nylon routes requests by combining host, header, and path rules with optional rewrites and middleware. This guide walks through the most common scenarios—from the basics to more advanced matching—so you can design clear, maintainable route layouts.

## Quick Start

```yaml
routes:
  - route:
      type: host
      value: api.example.com
    name: api
    paths:
      - path: /v1/{*path}
        service:
          name: api-v1
      - path: /{*path}
        service:
          name: fallback
```

1. Choose a matcher (`host` or `header`) for the route.
2. Name the route so you can refer to it in logs and dashboards.
3. Describe one or more `paths` and map each to a backend service.
4. Optionally attach middleware, TLS, or rewrites per path.

## Building Blocks

- **Route matcher** (`route.type`, `route.value`): Determines when the route is eligible. You can list multiple hosts separated by `|`.
- **Path entry** (`paths[].path`): Checked in order to match the request path and HTTP method.
- **Service block** (`service`): Points to the upstream service and optional rewrite target.
- **Middleware** (`middleware` or `middleware_groups`): Attach reusable filters or plugin handlers.

## Matching Strategies

### Host-based routing

```yaml
routes:
  - route:
      type: host
      value: api.example.com|api.internal
    paths:
      - path: /{*path}
        service:
          name: api-service
```

Use host matching to separate traffic by domain or subdomain. Wildcard `*` catches any host that did not match previously defined routes.

### Header-based routing

Enable multi-tenant or environment-specific configurations by inspecting a request header. Set a `header_selector` at the top of your config, then bind each header value to a route.

```yaml
header_selector: x-nylon-environment

routes:
  - route:
      type: header
      value: staging
    name: staging-app
    paths:
      - path: /{*path}
        service:
          name: staging-backend
```

### Method filtering

Restrict a path to specific HTTP methods. Nylon accepts `GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`, `CONNECT`, and `TRACE`.

```yaml
paths:
  - path: /api/users
    methods:
      - GET
      - POST
    service:
      name: user-service
```

## Path Patterns

| Pattern             | Matches                              | Notes                                 |
| ------------------- | ------------------------------------ | ------------------------------------- |
| `/status`           | Exact `/status`                      | Fastest match                         |
| `/users/{id}`       | Any single segment (`/users/42`)     | Captured as `params["id"]`            |
| `/assets/{*path}`   | All trailing segments                | Catch-all; lowest priority            |
| `/{*path}`          | Everything                           | Use as a final fallback               |

Extracted parameters are available inside plugins:

```go
phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    if userID, ok := ctx.Request().Params()["id"]; ok {
        ctx.Logger().Info("routing user", "id", userID)
    }
    ctx.Next()
})
```

### Multiple paths per route

Organize related paths under the same route and share middleware when needed.

```yaml
routes:
  - route:
      type: host
      value: app.example.com
    paths:
      - path: /api/{*path}
        service:
          name: api
        middleware:
          - plugin: auth
            entry: check
      - path: /static/{*path}
        service:
          name: cdn
      - path:
          - /
          - /{*path}
        service:
          name: web
```

## Path Rewrites

Rewrites adjust the upstream request path without changing the path matched by the client.

```yaml
paths:
  - path: /old-api/{*path}
    service:
      name: new-api
      rewrite: /v2

  - path: /api/v1/{*path}
    service:
      name: api-v1
      rewrite: /
```

- When the route matches `/old-api/users`, Nylon proxies to `/v2/users`.
- Use `/` to strip a prefix entirely.

## How Matching Order Works

Nylon uses [`matchit` v0.8](https://docs.rs/crate/matchit/latest) to score routes:

1. Exact segments (`/health`) take priority.
2. Named parameters (`/{user}`) run next.
3. Catch-all parameters (`/{*path}`) match last.

```yaml
paths:
  - path: /api/health      # 1 — exact
  - path: /api/{resource}  # 2 — named parameter
  - path: /{*path}         # 3 — catch-all fallback
    service:
      name: fallback
```

Order still matters when two paths have the same precedence—define the most specific entries first.

## Dynamic Routing & Segmentation

Combine host, header, and method rules to isolate workloads or tenants.

```yaml
header_selector: x-nylon-proxy

routes:
  - route:
      type: header
      value: tenant-a
    name: tenant-a
    paths:
      - path: /admin/{*path}
        service:
          name: admin
        methods:
          - GET
          - POST
        middleware:
          - plugin: auth
            entry: admin
      - path: /{*path}
        service:
          name: app
```

Requests with `x-nylon-proxy: tenant-a` use the above layout, while other values can map to different services or environments.

## End-to-end Example

```yaml
header_selector: x-nylon-proxy

routes:
  - route:
      type: host
      value: api.example.com
    name: api
    tls:
      enabled: true
    middleware:
      - group: observability
    paths:
      - path: /public/{*path}
        service:
          name: public-api
      - path: /v1/{*path}
        service:
          name: api-v1
        middleware:
          - plugin: auth
            entry: jwt
      - path: /admin/{*path}
        methods:
          - GET
          - POST
        service:
          name: admin-api

  - route:
      type: host
      value: static.example.com
    name: static
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: cdn

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

## Best Practices

1. **Lead with specificity**: Put the narrowest path first and reserve catch-all entries for the bottom.
2. **Group shared behavior**: Use middleware groups to apply authentication, rate limiting, or logging policies consistently.
3. **Segment by domain**: Split public, admin, API, and static traffic into separate routes for clarity.
4. **Prefer parameters over wildcards**: Named segments make logs and plugins easier to reason about.
5. **Document rewrites**: Include comments or naming conventions so teams understand why a rewrite exists.

## Next Steps

- [Configuration](/core/configuration) for every available field.
- [Middleware](/core/middleware) to attach logic to routes.
- [Examples](/examples/basic-proxy) for complete proxy configurations.
