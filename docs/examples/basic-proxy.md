# Basic Proxy

A simple reverse proxy example forwarding HTTP requests to a backend.

## Configuration

### Runtime Config (`config.yaml`)

```yaml
http:
  - "0.0.0.0:8080"

config_dir: "./config"

pingora:
  daemon: false
  threads: 4
  work_stealing: true
```

### Proxy Config (`config/proxy.yaml`)

```yaml
services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: localhost
    name: default
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: backend
```

## Running

```bash
nylon run -c config.yaml
```

## Testing

```bash
# Make a request
curl http://localhost:8080

# With headers
curl -H "X-Custom-Header: value" http://localhost:8080/api

# POST request
curl -X POST -d '{"key":"value"}' \
  -H "Content-Type: application/json" \
  http://localhost:8080/api
```

## Multiple Backends

Add more endpoints for load balancing:

```yaml
services:
  - name: backend
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000
      - ip: 127.0.0.1
        port: 3001
      - ip: 127.0.0.1
        port: 3002
```

## Health Checks

Enable health checking:

```yaml
services:
  - name: backend
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
      interval: 5s
      timeout: 2s
      healthy_threshold: 2
      unhealthy_threshold: 3
```

## Path-Based Routing

Route different paths to different services:

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

  - name: admin-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 4000

routes:
  - route:
      type: host
      value: localhost
    name: default
    paths:
      - path: /api/{*path}
        service:
          name: api-service
      
      - path: /admin/{*path}
        service:
          name: admin-service
```

## Host-Based Routing

Route by hostname:

```yaml
services:
  - name: app1-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 3000

  - name: app2-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 127.0.0.1
        port: 4000

routes:
  - route:
      type: host
      value: app1.example.com
    name: app1
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: app1-service

  - route:
      type: host
      value: app2.example.com
    name: app2
    paths:
      - path:
          - /
          - /{*path}
        service:
          name: app2-service
```

## Next Steps

- [Authentication Example](/examples/authentication)
- [Static Files](/core/configuration#static-service)
- [Load Balancing](/core/configuration#services)
