# Configuration

Nylon’s configuration is split into two cooperating layers:

| Layer | Location | Purpose |
|-------|----------|---------|
| **Runtime** | `config.yaml` | Process-wide options: listeners, ACME folders, Pingora runtime, WebSocket adapter. |
| **Proxy** | Every file inside `config_dir` | Declarative routing surface: services, routes, middleware, plugins, TLS entries. |

Run Nylon with both layers in place:

```bash
nylon run -c config.yaml
```

> **Tip:** Keep `config.yaml` minimal and organise proxy files under `config/` (for example `services.yaml`, `routes.yaml`, `tls.yaml`) to keep reviews focused.

---

## Runtime Configuration

`config.yaml` sets up listeners, directories, and Pingora behaviour.

```yaml
# Listeners
http:
  - 0.0.0.0:80
https:
  - 0.0.0.0:443

# Reserved for a future Prometheus exporter
metrics:
  - 127.0.0.1:6192

# Directory layout
config_dir: "/etc/nylon/config"
acme: "/etc/nylon/acme"

pingora:
  daemon: false
  threads: 4
  work_stealing: true
  grace_period_seconds: 60
  graceful_shutdown_timeout_seconds: 10
  upstream_keepalive_pool_size: 128
  error_log: "/var/log/nylon/error.log"
  pid_file: "/var/run/nylon.pid"
  upgrade_sock: "/tmp/nylon_upgrade.sock"
  user: "nobody"
  group: "nobody"
  ca_file: "/etc/ssl/certs/ca-certificates.crt"

# WebSocket adapter (optional)
websocket:
  adapter_type: redis    # memory | redis | cluster
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"
```

### Runtime fields at a glance

| Field | Default | Notes |
|-------|---------|-------|
| `http` | `[]` | Bind addresses for HTTP listeners (`host:port`). |
| `https` | `[]` | HTTPS listeners; requires TLS configuration in proxy layer. |
| `metrics` | `[]` | Reserved for future use. |
| `config_dir` | `/etc/nylon/config` | Folder holding proxy configuration files. |
| `acme` | `/etc/nylon/acme` | ACME account + certificate storage. |
| `websocket.adapter_type` | `redis` | Choose `memory`, `redis`, or `cluster`. |

#### Pingora settings

| Field | Default | Purpose |
|-------|---------|---------|
| `daemon` | `false` | Detach process (Linux). |
| `threads` | auto | Worker threads (CPU cores minus 1–2). |
| `work_stealing` | `false` | Share load between workers. |
| `grace_period_seconds` | `60` | Wait before shutting down active connections. |
| `graceful_shutdown_timeout_seconds` | `10` | Hard stop after this timeout. |
| `upstream_keepalive_pool_size` | `null` | Cap upstream keepalive pool. |
| `error_log`, `pid_file`, `upgrade_sock` | `null` | Optional observability and upgrade plumbing. |
| `user`, `group` | `null` | Drop privileges after binding privileged ports. |
| `ca_file` | `null` | Custom CA bundle for upstream TLS. |

---

## Proxy Configuration

Every YAML file within `config_dir` is parsed and merged. A typical layout:

```yaml
header_selector: x-nylon-proxy  # Optional: switch configs via header

plugins:
  - name: my-plugin
    type: ffi
    file: /opt/nylon/plugins/my-plugin.so
    config:
      debug: true

services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s
      timeout: 1s
      healthy_threshold: 2
      unhealthy_threshold: 2

middleware_groups:
  security:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${uuid(v7)}"
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-server
            value: "${or(env(SERVICE_NAME), 'nylon')}"

routes:
  - route:
      type: host
      value: example.com
    name: main
    middleware:
      - group: security
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend

tls:
  - type: acme
    provider: letsencrypt
    domains:
      - example.com
    acme:
      email: admin@example.com
```

---

## Services

Services describe what to do once a route has matched.

### HTTP service – proxy to upstream servers

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: round_robin      # round_robin | weighted | consistent | random
    endpoints:
      - ip: 10.0.0.1
        port: 3000
        weight: 5               # used by weighted algorithm
      - ip: 10.0.0.2
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3
```

### Plugin service – delegate to an FFI plugin

```yaml
services:
  - name: websocket-handler
    service_type: plugin
    plugin:
      name: my-plugin
      entry: "ws"
      payload:
        api_key: "${env(WS_API_KEY)}"
```

### Static service – serve files directly

```yaml
services:
  - name: static-files
    service_type: static
    static:
      root: /var/www/html
      index: index.html
      spa: true        # Serve index.html on 404 (SPA mode)
```

---

## Routes

Routes are evaluated in two stages: route matcher (host/header) and path patterns (MatchIt). Once a path matches, Nylon links to the configured service and middleware.

### Route matchers

```yaml
routes:
  - route:
      type: host
      value: api.example.com|api.internal
    name: api
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: api-service

  - route:
      type: header
      value: tenant-admin          # requires header_selector
    name: admin
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: admin-service
```

### Path patterns, rewrites, and methods

```yaml
routes:
  - route:
      type: host
      value: example.com
    name: app
    paths:
      - path: /api/*
        service:
          name: api-service
          rewrite: /v2/*
        methods:
          - GET
          - POST
        middleware:
          - plugin: auth
            entry: "check"

      - path: /static/*
        service:
          name: static-files

      - path: /admin/*
        service:
          name: admin-service
```

> **Order matters:** Nylon registers all HTTP methods for each pattern unless you specify `methods`. More specific paths take precedence.

---

## TLS / HTTPS

### Manual certificates

```yaml
tls:
  - type: custom
    domains:
      - example.com
      - www.example.com
    cert: /etc/ssl/example.com.crt
    key: /etc/ssl/example.com.key
    chain:
      - /etc/ssl/example.com.chain.pem
```

### ACME (Let's Encrypt and friends)

```yaml
tls:
  - type: acme
    provider: letsencrypt
    domains:
      - example.com
      - api.example.com
    acme:
      email: admin@example.com
```

Routes can enforce TLS and optionally redirect HTTP to HTTPS:

```yaml
routes:
  - route:
      type: host
      value: example.com
    name: secure
    tls:
      enabled: true
      redirect: ${host}
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend
```

---

## Template Expressions

Use expressions anywhere you need dynamic values (middleware payloads, plugin configs, static service metadata).

### Function catalogue

| Function | Description | Example |
|----------|-------------|---------|
| `${header(name)}` | Request header value (case-sensitive). | `${header(user-agent)}` |
| `${query(name[, default])}` | Query parameter with optional default. | `${query(version, 'v1')}` |
| `${cookie(name[, default])}` | Cookie lookup. | `${cookie(session_id)}` |
| `${param(name[, default])}` | Route/path parameter. | `${param(user_id)}` |
| `${request(field)}` | Request metadata (`client_ip`, `host`, `method`, `path`, `scheme`, `tls`). | `${request(method)}` |
| `${env(VAR)}` | Environment variable. | `${env(SERVICE_NAME)}` |
| `${uuid(v4|v7)}` | Generate UUID. | `${uuid(v7)}` |
| `${timestamp()}` | RFC3339 timestamp with millisecond precision. | `${timestamp()}` |
| `${or(a, b, …)}` | First non-empty argument. | `${or(env(NAME), 'default')}` |
| `${eq(a, b[, value])}` | Return `value` (or `a`) if equal; empty otherwise. | `${eq(request(method), 'GET', 'cacheable')}` |
| `${neq(a, b[, value])}` | Return `value` (or `a`) if not equal. | `${neq(request(scheme), 'https', 'insecure')}` |
| `${concat(values…)}` | Concatenate arguments. | `${concat(header(host), '-', uuid(v4))}` |
| `${upper(value)}` / `${lower(value)}` | Case conversion. | `${upper(param(region))}` |
| `${len(value)}` | Length of evaluated string. | `${len(header(user-agent))}` |
| `${if_cond(condition, then, else)}` | Branch by non-empty string. | `${if_cond(request(tls), 'https', 'http')}` |

### Example usage

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
        - name: x-env
          value: "${or(env(ENVIRONMENT), 'local')}"
```

---

## See also

- [Routing](/core/routing) – Path patterns, matching order, TLS redirects.
- [Middleware](/core/middleware) – Header modifiers and middleware groups.
- [TLS](/core/tls) – Certificate management and renewal pipeline.
