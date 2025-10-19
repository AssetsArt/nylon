# Configuration

Complete configuration reference for Nylon.

## Configuration Files

Nylon uses two types of configuration:

1. **Runtime Configuration** - Server and runtime settings
2. **Proxy Configuration** - Services, routes, and plugins

```bash
nylon -c config.yaml
```

## Runtime Configuration

The main `config.yaml` file controls server behavior:

```yaml
# HTTP listening addresses
http:
  - "0.0.0.0:80"
  - "[::]:80"

# HTTPS listening addresses  
https:
  - "0.0.0.0:443"
  - "[::]:443"

# Prometheus metrics endpoint
metrics:
  - "127.0.0.1:6192"

# Directory containing proxy configurations
config_dir: "/etc/nylon/config"

# Directory for ACME certificates
acme: "/etc/nylon/acme"

# Pingora runtime settings
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
  adapter_type: redis  # memory | redis
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"
```

### Runtime Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `http` | list | `[]` | HTTP listening addresses |
| `https` | list | `[]` | HTTPS listening addresses |
| `metrics` | list | `[]` | Metrics endpoint addresses |
| `config_dir` | path | `/etc/nylon/config` | Proxy config directory |
| `acme` | path | `/etc/nylon/acme` | ACME certificates directory |

### Pingora Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `daemon` | bool | `false` | Run as daemon |
| `threads` | int | CPU cores - 2 | Number of worker threads |
| `work_stealing` | bool | `false` | Enable work stealing |
| `grace_period_seconds` | int | `60` | Grace period for shutdown |
| `graceful_shutdown_timeout_seconds` | int | `10` | Max wait before forced shutdown |
| `upstream_keepalive_pool_size` | int | - | Upstream connection pool size |
| `error_log` | path | - | Error log file path |
| `pid_file` | path | - | PID file path |
| `upgrade_sock` | path | - | Unix socket for upgrades |
| `user` | string | - | User to drop privileges to |
| `group` | string | - | Group to drop privileges to |
| `ca_file` | path | - | CA certificates file |

## Proxy Configuration

Files in `config_dir` define services and routes:

### Basic Structure

```yaml
# Optional: Header to select proxy config
header_selector: x-nylon-proxy

# Plugin definitions
plugins:
  - name: my-plugin
    type: ffi
    file: /path/to/plugin.so
    config:
      key: value

# Service definitions
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
      interval: 3s
      timeout: 1s
      healthy_threshold: 2
      unhealthy_threshold: 2

# Middleware groups
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
            value: "nylon"

# Route definitions  
routes:
  - route:
      type: host
      value: example.com
    name: main
    tls:
      enabled: true
    middleware:
      - group: security
    paths:
      - path: /*
        service:
          name: backend
        methods:
          - GET
          - POST

# TLS/Certificate configuration
tls:
  - domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

## Services

### HTTP Service

Forward requests to HTTP backends:

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: round_robin  # round_robin | weighted | consistent | random
    endpoints:
      - ip: 10.0.0.1
        port: 3000
        weight: 5  # for weighted algorithm
      - ip: 10.0.0.2
        port: 3000
        weight: 3
    health_check:
      enabled: true
      path: /health
      interval: 5s
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3
```

### Plugin Service

Handle requests with plugins:

```yaml
services:
  - name: websocket-handler
    service_type: plugin
    plugin:
      name: my-plugin
      entry: "ws"
```

### Static Service

Serve static files:

```yaml
services:
  - name: static-files
    service_type: static
    static:
      root: /var/www/html
      index: index.html
      spa: true  # SPA mode: serve index.html on 404
```

## Routes

### Route Matchers

Routes can match by host or path:

```yaml
routes:
  # Match by hostname
  - route:
      type: host
      value: api.example.com
    name: api
    paths:
      - path: /*
        service:
          name: api-service

  # Match by path prefix
  - route:
      type: path
      value: /admin
    name: admin
    paths:
      - path: /*
        service:
          name: admin-service
```

### Path Configuration

```yaml
paths:
  # Simple path
  - path: /api/*
    service:
      name: api-service
    methods:
      - GET
      - POST
    middleware:
      - plugin: auth
        entry: "check"

  # Path with rewrite
  - path: /old-api/*
    service:
      name: api-service
      rewrite: /new-api/*

  # Path with parameters
  - path: /users/:id/*
    service:
      name: user-service
```

### TLS Configuration

```yaml
routes:
  - route:
      type: host
      value: secure.example.com
    name: secure
    tls:
      enabled: true
      redirect: https://secure.example.com  # Optional redirect
    paths:
      - path: /*
        service:
          name: backend
```

## Plugins

### Plugin Definition

```yaml
plugins:
  - name: auth-plugin
    type: ffi
    file: /path/to/auth.so
    config:
      secret: "my-secret"
      timeout: 30
```

### Using Plugins in Routes

```yaml
# As middleware
paths:
  - path: /*
    service:
      name: backend
    middleware:
      - plugin: auth-plugin
        entry: "auth"
        payload:
          role: "admin"
```

### Built-in Plugins

Nylon includes built-in middleware plugins:

**RequestHeaderModifier:**
```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      set:
        - name: x-custom
          value: "${header(user-agent)}"
      remove:
        - x-internal
```

**ResponseHeaderModifier:**
```yaml
middleware:
  - plugin: ResponseHeaderModifier
    payload:
      set:
        - name: cache-control
          value: "no-cache"
      remove:
        - server
```

## Middleware Groups

Reusable middleware sets:

```yaml
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
          - name: x-frame-options
            value: "DENY"

routes:
  - route:
      type: host
      value: example.com
    name: main
    middleware:
      - group: security  # Apply group
    paths:
      - path: /*
        service:
          name: backend
```

## Template Expressions

Use template expressions in configuration:

### Available Functions

- `${header(name)}` - Request header value
- `${request(client_ip)}` - Client IP address
- `${uuid(v7)}` - Generate UUID v7
- `${timestamp()}` - Current timestamp
- `${env(VAR_NAME)}` - Environment variable
- `${or(value1, value2)}` - First non-empty value

### Example

```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      set:
        - name: x-request-id
          value: "${uuid(v7)}"
        - name: x-forwarded-for
          value: "${request(client_ip)}"
        - name: x-server
          value: "${or(env(SERVER_NAME), 'nylon')}"
```

## TLS/HTTPS

### Manual Certificates

```yaml
tls:
  - domains:
      - example.com
      - www.example.com
    cert: /path/to/cert.pem
    key: /path/to/key.pem
```

### ACME/Let's Encrypt

```yaml
tls:
  - domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

## Health Checks

```yaml
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
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3
```

## WebSocket

### Redis Adapter

```yaml
websocket:
  adapter_type: redis
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"
```

### Memory Adapter

```yaml
websocket:
  adapter_type: memory
```

## Hot Reload

Nylon supports hot-reloading of proxy configurations:

```bash
# Send SIGHUP to reload
kill -HUP $(cat /var/run/nylon.pid)

# Or use systemd
systemctl reload nylon
```

Runtime config changes require a restart.

## Examples

See [Examples](/examples/basic-proxy) for complete configuration examples.
