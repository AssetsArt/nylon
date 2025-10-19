# Configuration

Learn how to configure Nylon proxy server.

## Configuration File

Nylon uses YAML configuration files. By default, it looks for `config.yaml` in the current directory.

```bash
nylon -c config.yaml
```

## Basic Structure

```yaml
runtime:
  threads: 4
  work_stealing: true

plugins:
  - name: my-plugin
    path: "./plugin.so"

proxy:
  - name: my-proxy
    listen:
      - "0.0.0.0:8080"
    routes:
      - path: "/*"
        service: backend

services:
  - name: backend
    backend:
      round_robin:
        - "127.0.0.1:3000"
```

## Runtime Configuration

```yaml
runtime:
  threads: 4              # Number of worker threads
  work_stealing: true     # Enable work stealing between threads
```

## Proxy Configuration

```yaml
proxy:
  - name: api-proxy
    listen:
      - "0.0.0.0:8080"
      - "[::]:8080"
    tls:
      - domains:
          - "example.com"
        cert: "/path/to/cert.pem"
        key: "/path/to/key.pem"
    routes:
      - path: "/api/*"
        service: api-service
```

More details coming soon...

