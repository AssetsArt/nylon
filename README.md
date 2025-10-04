# 🧬 Nylon — The Extensible Proxy Server

[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://nylon.sh/)

Nylon is a lightweight, high‑performance, and extensible proxy built on top of the battle‑tested [Cloudflare Pingora](https://github.com/cloudflare/pingora) framework.

---

## What you get

- Extensible: write plugins in Go, Rust, Zig, C via FFI
- Simple YAML config: one place to manage routes, services, middleware
- Smart routing & load balancing: host/header/path matching, round‑robin/random/consistent hashing
- TLS built‑in: custom certs or ACME (Let’s Encrypt, Buypass)
- Cloud‑native: observability and scalability friendly

---

## Quick start

```sh
# Build (choose one)
make build
# or
cargo build --release

# Run with bundled examples
./target/release/nylon run -c ./examples/config.yaml

# Ports
# HTTP   : 0.0.0.0:8088
# HTTPS  : 0.0.0.0:8443 (enable with certs)
# Metrics: 127.0.0.1:6192
```

Test quickly:

```sh
curl -H "Host: localhost" http://127.0.0.1:8088/
curl -H "Host: localhost" http://127.0.0.1:8088/static/
# if TLS enabled
curl -k -H "Host: localhost" https://127.0.0.1:8443/
```

---

## Minimal config

Top‑level `examples/config.yaml`:

```yaml
http:
  - 0.0.0.0:8088
https:
  - 0.0.0.0:8443
metrics:
  - 127.0.0.1:6192
config_dir: "./examples/proxy"
acme: "./examples/acme"
pingora:
  daemon: false
  grace_period_seconds: 1
  graceful_shutdown_timeout_seconds: 1
```

- http/https: listening addresses
- metrics: Prometheus‑compatible metrics endpoint
- config_dir: folder containing proxy configs
- acme: ACME storage path (optional)

---

## Examples

Proxy `base.yaml` (services, middleware, static):

```yaml
header_selector: x-nylon-proxy

plugins:
  - name: plugin_sdk
    type: ffi
    file: ./target/examples/go/plugin_sdk.so
    config:
      debug: true

services:
  - name: http-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000
      - ip: 127.0.0.1
        port: 3001
    health_check:
      enabled: true
      path: /health
      interval: 3s
      timeout: 1s
      healthy_threshold: 2
      unhealthy_threshold: 2

  - name: ws-service
    service_type: plugin
    plugin:
      name: plugin_sdk
      entry: ws

  - name: stream-service
    service_type: plugin
    plugin:
      name: plugin_sdk
      entry: stream

  - name: static
    service_type: static
    static:
      root: ./examples/static
      index: index.html
      spa: true

middleware_groups:
  example:
    - plugin: plugin_sdk
      entry: "authz"
      payload:
        client_ip: "${request(client_ip)}"

    - plugin: RequestHeaderModifier
      payload:
        remove:
          - x-version
        set:
          - name: x-hb-conf
            value: "env-${or(env(MY_APP_NAME), 'default')}"
          - name: x-request-id
            value: "${uuid(v7)}"
          - name: x-timestamp
            value: "${timestamp()}"
          - name: x-forwarded-for
            value: "${request(client_ip)}-${eq(request(client_ip), '127.0.0.1', 'local')}"
          - name: x-host
            value: "${header(host)}"

    - plugin: ResponseHeaderModifier
      payload:
        set:
          - name: x-request-id-false
            value: "${or(header(x-request-id-false), concat('foo', '-', uuid(v4)))}"
          - name: x-request-id
            value: "${header(x-request-id)}"
          - name: x-server
            value: ${or(env(SERVER_NAME), 'my-server')}
```

Proxy `host_route.yaml` (host/path routing):

```yaml
# https://github.com/ibraheemdev/matchit
routes:
  - route:
      type: host
      value: localhost
    name: http-route-1
    paths:
      - path:
          - /static
          - /static/{*path}
        service:
          name: static
          rewrite: /static
      - path:
          - /ws
        methods: [GET, POST, OPTIONS]
        service:
          name: ws-service
      - path:
          - /stream
        methods: [GET, POST, OPTIONS]
        service:
          name: stream-service
      - path:
          - /
          - /{*path}
        methods: [GET, POST, OPTIONS]
        middleware:
          - group: example
        service:
          name: http-service
```

Proxy `tls.yaml` (custom certs or ACME):

```yaml
tls:
  - type: custom
    cert: ./examples/cert/localhost.crt
    key: ./examples/cert/localhost.key
    # chain:
    #   - ./examples/cert/chain.pem
    domains:
      - localhost

  # - type: acme
  #   email: test@example.com
  #   provider: letsencrypt # letsencrypt, buypass
  #   domains:
  #     - localhost
```

Static page `examples/static/index.html` is served at `/static`.

---

## TLS quick start

Generate local certs (choose one):

```sh
# mkcert (recommended for local)
mkcert -install
mkcert -key-file ./examples/cert/localhost.key -cert-file ./examples/cert/localhost.crt localhost

# openssl (alternative)
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout ./examples/cert/localhost.key \
  -out ./examples/cert/localhost.crt \
  -subj "/CN=localhost"
```

---

## Plugins

- Nylon supports FFI plugins. A Go example lives in `examples/go/main.go`.
- Build a shared object and reference it in `base.yaml`:

```sh
mkdir -p ./target/examples/go
go build -buildmode=c-shared -o ./target/examples/go/plugin_sdk.so ./examples/go
```

See the docs for more: `Plugin System` and language‑specific guides.

---

## Links

- Docs: https://nylon.sh/
- Getting started: https://nylon.sh/getting-started/installation
- Config reference: https://nylon.sh/config-reference
- Plugin system: https://nylon.sh/plugin-system

---

## Build from source

```sh
git clone https://github.com/AssetsArt/nylon.git
cd nylon
make build
```

MIT Licensed. © AssetsArt.
