# Configuration Schema

Exhaustive field reference for both runtime and proxy configuration files. See also the [conceptual guide](/core/configuration) for workflows and examples.

---

## Runtime configuration (`config.yaml`)

```yaml
http:
  - 0.0.0.0:80
https:
  - 0.0.0.0:443

metrics:
  - 127.0.0.1:6192

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

websocket:
  adapter_type: redis  # memory | redis | cluster
  redis:
    host: localhost
    port: 6379
    password: null
    db: 0
    key_prefix: "nylon:ws"
```

### Field reference

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `http` | `[]string` | No | `[]` | HTTP listener addresses (`host:port`). |
| `https` | `[]string` | No | `[]` | HTTPS listeners (requires TLS in proxy config). |
| `metrics` | `[]string` | No | `[]` | Reserved for future Prometheus exporter. |
| `config_dir` | `string` | No | `/etc/nylon/config` | Root directory for proxy YAML files. |
| `acme` | `string` | No | `/etc/nylon/acme` | ACME storage (certificates + account). |
| `pingora` | `object` | No | `{}` | Pingora runtime configuration (see below). |
| `websocket` | `object` | No | `null` | WebSocket adapter. Required for `redis`/`cluster`. |

#### `pingora` object

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `daemon` | `bool` | `false` | Start as daemon (Linux). |
| `threads` | `int` | CPU cores - 2 | Worker threads (clamped to ≥1). |
| `work_stealing` | `bool` | `false` | Enable work stealing across threads. |
| `grace_period_seconds` | `int` | `60` | Grace period before shutdown. |
| `graceful_shutdown_timeout_seconds` | `int` | `10` | Hard shutdown deadline. |
| `upstream_keepalive_pool_size` | `int` | `null` | Cap for upstream keepalive pool. |
| `error_log` | `string` | `null` | Pingora error log path. |
| `pid_file` | `string` | `null` | PID file path. |
| `upgrade_sock` | `string` | `null` | Domain socket for zero-downtime upgrades. |
| `user` / `group` | `string` | `null` | Drop privileges after binding ports. |
| `ca_file` | `string` | `null` | Custom CA bundle for upstream TLS. |

#### `websocket` object (optional)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `adapter_type` | `string` | No | `memory`, `redis`, or `cluster` (default `redis`). |
| `redis` | `object` | For redis/cluster | Connection details: `host`, `port`, `password`, `db`, `key_prefix`. |
| `cluster` | `object` | For cluster | Seed `nodes` and optional `key_prefix`. |

---

## Proxy configuration (`config_dir`)

Every YAML file within `config_dir` is merged. Example scaffold:

```yaml
header_selector: x-nylon-proxy

plugins:
  - name: auth
    type: ffi
    file: /opt/nylon/plugins/auth.so
    config:
      issuer: https://auth.example.com

services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3

middleware_groups:
  security:
    - plugin: RequestHeaderModifier
      payload:
        set:
          - name: x-request-id
            value: "${uuid(v7)}"

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

tls:
  - type: acme
    provider: letsencrypt
    domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

### Plugins

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | `string` | Yes | Plugin identifier (unique). |
| `type` | `string` | Yes | Currently only `ffi`. |
| `file` | `string` | Yes | Shared library path. |
| `config` | `object` | No | Arbitrary configuration passed to plugin. |

### Services

| Field | Type | Required | Applies to |
|-------|------|----------|-----------|
| `name` | `string` | Yes | All services. |
| `service_type` | `string` | Yes | `http`, `plugin`, or `static`. |
| `algorithm` | `string` | No | HTTP services (`round_robin`, `weighted`, `consistent`, `random`). |
| `endpoints` | `[]object` | For http | Each endpoint requires `ip`, `port`, optional `weight`. |
| `health_check` | `object` | For http | See table below. |
| `plugin` | `object` | For plugin | Plugin invocation (`name`, `entry`, optional `payload`). |
| `static` | `object` | For static | `root`, `index`, optional `spa`. |

#### Health check object

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Enable active health checks. |
| `path` | `string` | `/` | Probe path. |
| `interval` | `string` | `10s` | Frequency (must end with `s`). |
| `timeout` | `string` | `5s` | Probe timeout (must end with `s`). |
| `healthy_threshold` | `int` | `2` | Successes before healthy. |
| `unhealthy_threshold` | `int` | `3` | Failures before unhealthy. |

### Middleware groups

Dictionary of reusable middleware chains:

```yaml
middleware_groups:
  security:
    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-frame-options
            value: "DENY"
```

Each entry mirrors route-level middleware (`group` or explicit `plugin`/`entry`/`payload`).

### Routes

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `route` | `object` | Yes | Matcher definition. `type` = `host` or `header` (requires `header_selector`). `value` supports `a|b`. |
| `name` | `string` | Yes | Unique route name. |
| `tls` | `object` | No | `enabled`, optional `redirect`. |
| `middleware` | `[]object` | No | Route-level middleware entries. |
| `paths` | `[]object` | Yes | Path matchers (see below). |

#### Path object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | `string` or `[]string` | Yes | Pattern(s) for MatchIt router. Supports `*` and `{param}`. |
| `service` | `object` | Yes | `name` (service), optional `rewrite`. |
| `methods` | `[]string` | No | Limit to specific HTTP methods. |
| `middleware` | `[]object` | No | Path-specific middleware. |

### Middleware entry

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `group` | `string` | Either | Reference a middleware group. |
| `plugin` | `string` | Either | Plugin name to execute. |
| `entry` | `string` | If plugin | Handler exported by plugin. |
| `payload` | `object` | No | Arbitrary JSON passed to handler. |

### TLS entries

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | `string` | Yes | `custom` or `acme`. |
| `domains` | `[]string` | Yes | SAN list / hostnames. Must be unique across entries. |
| `cert` / `key` | `string` | For custom | PEM files for certificate and private key. |
| `chain` | `[]string` | No | Additional chain PEMs. |
| `provider` | `string` | For acme | ACME provider (e.g. `letsencrypt`). |
| `acme` | `object` | For acme | `email`, optional `directory_url`, `staging`, `eab_kid`, `eab_hmac_key`. |

---

## Template expressions

Expressions can appear inside payloads to reference request context.

| Function | Description | Example |
|----------|-------------|---------|
| `${header(name)}` | Request header (case-sensitive). | `${header(user-agent)}` |
| `${query(name[, default])}` | Query string value. | `${query(version, 'v1')}` |
| `${cookie(name[, default])}` | Cookie lookup. | `${cookie(session_id)}` |
| `${param(name[, default])}` | Route/path parameter. | `${param(account_id)}` |
| `${request(field)}` | Request metadata (`client_ip`, `host`, `method`, `path`, `scheme`, `tls`). | `${request(method)}` |
| `${env(VAR)}` | Environment variable. | `${env(SERVICE_NAME)}` |
| `${uuid(v4\|v7)}` | Generate UUID string. | `${uuid(v7)}` |
| `${timestamp()}` | RFC3339 timestamp with millisecond precision. | `${timestamp()}` |
| `${or(a, b, …)}` | First non-empty argument. | `${or(env(NAME), 'default')}` |
| `${eq(a, b[, value])}` | Returns `value` (or `a`) if `a == b`. | `${eq(request(method), 'GET', 'cacheable')}` |
| `${neq(a, b[, value])}` | Returns `value` (or `a`) if `a != b`. | `${neq(request(scheme), 'https', 'insecure')}` |
| `${concat(values…)}` | Concatenate arguments. | `${concat(header(host), '-', uuid(v4))}` |
| `${upper(value)}` / `${lower(value)}` | Case conversion. | `${upper(param(region))}` |
| `${len(value)}` | String length. | `${len(header(user-agent))}` |
| `${if_cond(condition, then, else)}` | Conditional evaluation (truthy when non-empty). | `${if_cond(request(tls), 'https', 'http')}` |

---

## Related documentation

- [Core configuration](/core/configuration) – narrative guide and best practices. |
- [Routing reference](/core/routing) – path syntax, ordering, TLS redirects. |
- [Middleware guide](/core/middleware) – built-in modifiers and reusable groups. |
- [TLS guide](/core/tls) – ACME lifecycle and manual certificate handling. |
