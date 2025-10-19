# Basic Proxy

A simple reverse proxy example.

## Configuration

```yaml
runtime:
  threads: 4
  work_stealing: true

proxy:
  - name: basic-proxy
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

## Usage

```bash
nylon -c config.yaml
```

Test the proxy:

```bash
curl http://localhost:8080
```

More examples coming soon...

