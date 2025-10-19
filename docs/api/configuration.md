# Configuration Schema

Complete reference for Nylon configuration files.

## Runtime Configuration

Main configuration file (`config.yaml`):

```yaml
# HTTP listening addresses
http:
  - "0.0.0.0:80"

# HTTPS listening addresses
https:
  - "0.0.0.0:443"

# Prometheus metrics addresses (reserved; currently unused)
metrics:
  - "127.0.0.1:6192"

# Directory containing proxy configurations
config_dir: "/etc/nylon/config"

# Directory for ACME certificates
acme: "/etc/nylon/acme"

# Pingora runtime configuration
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

# WebSocket adapter configuration (optional)
websocket:
  adapter_type: redis  # memory | redis | cluster
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"
```

### Runtime Schema

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `http` | `[]string` | No | `[]` | HTTP listening addresses |
| `https` | `[]string` | No | `[]` | HTTPS listening addresses |
| `metrics` | `[]string` | No | `[]` | Reserved for future Prometheus metrics endpoint |
| `config_dir` | `string` | No | `/etc/nylon/config` | Proxy config directory |
| `acme` | `string` | No | `/etc/nylon/acme` | ACME certificates directory |
| `pingora` | `object` | No | See below | Pingora configuration |
| `websocket` | `object` | No | `null` | WebSocket adapter config |

### Pingora Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `daemon` | `bool` | `false` | Run as daemon |
| `threads` | `int` | CPU cores - 2 | Worker threads |
| `work_stealing` | `bool` | `false` | Enable work stealing |
| `grace_period_seconds` | `int` | `60` | Grace period for shutdown |
| `graceful_shutdown_timeout_seconds` | `int` | `10` | Max shutdown wait time |
| `upstream_keepalive_pool_size` | `int` | `null` | Upstream connection pool |
| `error_log` | `string` | `null` | Error log file path |
| `pid_file` | `string` | `null` | PID file path |
| `upgrade_sock` | `string` | `null` | Upgrade socket path |
| `user` | `string` | `null` | User to drop privileges |
| `group` | `string` | `null` | Group to drop privileges |
| `ca_file` | `string` | `null` | CA certificates file |

## Proxy Configuration

Proxy configuration files in `config_dir`:

```yaml
# Header selector for multi-config support
header_selector: x-nylon-proxy

# Plugin definitions
plugins:
  - name: auth
    type: ffi
    file: /path/to/auth.so
    config:
      key: value

# Service definitions
services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
        weight: 1
    health_check:
      enabled: true
      path: /health
      interval: 5s
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3

# Middleware groups
middleware_groups:
  security:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${uuid(v7)}"

# Route definitions
routes:
  - route:
      type: host
      value: example.com
    name: main
    tls:
      enabled: true
      redirect: https://example.com
    middleware:
      - group: security
    paths:
      - path: /*
        service:
          name: backend
        methods:
          - GET
          - POST

# TLS configuration
tls:
  - type: acme
    provider: letsencrypt
    domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

### Plugin Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | Yes | Plugin name |
| `type` | `string` | Yes | Plugin type (always `ffi`) |
| `file` | `string` | Yes | Path to plugin .so file |
| `config` | `object` | No | Plugin configuration |

### Service Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | Yes | Service name |
| `service_type` | `string` | Yes | `http`, `plugin`, or `static` |
| `algorithm` | `string` | No | Load balancing algorithm |
| `endpoints` | `[]object` | For http | Backend endpoints |
| `health_check` | `object` | No | Health check configuration |
| `plugin` | `object` | For plugin | Plugin configuration |
| `static` | `object` | For static | Static file configuration |

#### HTTP Service

```yaml
services:
  - name: api
    service_type: http
    algorithm: round_robin  # round_robin | weighted | consistent | random
    endpoints:
      - ip: 10.0.0.1
        port: 3000
        weight: 5  # for weighted algorithm
```

#### Plugin Service

```yaml
services:
  - name: custom
    service_type: plugin
    plugin:
      name: my-plugin
      entry: "handler"
```

#### Static Service

```yaml
services:
  - name: frontend
    service_type: static
    static:
      root: /var/www/html
      index: index.html
      spa: true
```

### Health Check Schema

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Enable health checks |
| `path` | `string` | `/` | Health check path |
| `interval` | `duration` | `10s` | Check interval |
| `timeout` | `duration` | `5s` | Request timeout |
| `healthy_threshold` | `int` | `2` | Successes to mark healthy |
| `unhealthy_threshold` | `int` | `3` | Failures to mark unhealthy |

### Route Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `route` | `object` | Yes | Route matcher |
| `name` | `string` | Yes | Route name |
| `tls` | `object` | No | TLS configuration |
| `middleware` | `[]object` | No | Middleware list |
| `paths` | `[]object` | Yes | Path configurations |

#### Route Matcher

```yaml
route:
  type: host  # host | header
  value: example.com  # hostname or header value (when header_selector is configured)
```

#### Path Configuration

```yaml
paths:
  - path: /api/*  # Path pattern
    service:
      name: api-service
      rewrite: /v2/*  # optional rewrite
    methods:
      - GET
      - POST
    middleware:
      - plugin: auth
        entry: "check"
        payload:
          key: value
```

### Middleware Schema

```yaml
middleware:
  # Reference middleware group
  - group: security

  # Use plugin directly
  - plugin: auth
    entry: "check"
    payload:
      key: value
```

### TLS Schema

```yaml
tls:
  # Manual certificates
  - type: custom
    domains:
      - example.com
    cert: /path/to/cert.pem
    key: /path/to/key.pem

  # ACME (Let's Encrypt)
  - type: acme
    provider: letsencrypt
    domains:
      - api.example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

## Template Expressions

Available template functions:

| Function | Returns | Example |
|----------|---------|---------|
| `${header(name)}` | Request header | `${header(user-agent)}` |
| `${query(name[, default])}` | Query parameter | `${query(version, 'v1')}` |
| `${cookie(name[, default])}` | Cookie value | `${cookie(session_id)}` |
| `${param(name[, default])}` | Route parameter | `${param(user_id)}` |
| `${request(field)}` | Request metadata (`client_ip`, `host`, `method`, `path`, `scheme`, `tls`) | `${request(method)}` |
| `${env(VAR)}` | Environment variable | `${env(SERVICE_NAME)}` |
| `${uuid(v4|v7)}` | UUID string | `${uuid(v7)}` |
| `${timestamp()}` | RFC3339 timestamp | `${timestamp()}` |
| `${or(a, b, …)}` | First non-empty | `${or(env(NAME), 'default')}` |
| `${eq(a, b[, value])}` | `value` (or `a`) if equal | `${eq(request(method), 'GET', 'cacheable')}` |
| `${neq(a, b[, value])}` | `value` (or `a`) if not equal | `${neq(request(scheme), 'https', 'insecure')}` |
| `${concat(values…)}` | Concatenated string | `${concat(header(host), '-', uuid(v4))}` |
| `${upper(value)}` / `${lower(value)}` | Upper/lowercase | `${upper(param(region))}` |
| `${len(value)}` | Length of evaluated string | `${len(header(user-agent))}` |
| `${if_cond(condition, then, else)}` | Conditional evaluation | `${if_cond(request(tls), 'https', 'http')}` |

## Validation

Nylon validates configuration on startup and reload. Common errors:

### Missing Required Fields

```yaml
# ❌ Error: service name required
services:
  - service_type: http
```

```yaml
# ✅ Correct
services:
  - name: backend
    service_type: http
```

### Invalid Service Type

```yaml
# ❌ Error: invalid service type
services:
  - name: backend
    service_type: invalid
```

```yaml
# ✅ Correct: http, plugin, or static
services:
  - name: backend
    service_type: http
```

### Missing Endpoints

```yaml
# ❌ Error: http service requires endpoints
services:
  - name: backend
    service_type: http
```

```yaml
# ✅ Correct
services:
  - name: backend
    service_type: http
    endpoints:
      - ip: 127.0.0.1
        port: 3000
```

## Examples

See [Configuration Guide](/core/configuration) for detailed examples and best practices.

## See Also

- [Configuration Guide](/core/configuration) - Detailed configuration guide
- [Routing](/core/routing) - Route configuration
- [Load Balancing](/core/load-balancing) - Service configuration
- [TLS/HTTPS](/core/tls) - TLS configuration
