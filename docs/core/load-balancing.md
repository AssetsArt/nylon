# Load Balancing

Nylon supports multiple load balancing algorithms to distribute traffic across backend servers.

## Algorithms

### Round Robin (Default)

Distributes requests evenly across all healthy backends in sequence.

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000
```

**Use when:**
- All backends have equal capacity
- Simple, predictable load distribution needed
- No session affinity required

### Weighted Round Robin

Distributes requests based on assigned weights. Higher weight = more requests.

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: weighted
    endpoints:
      - ip: 10.0.0.1    # Gets 50% of traffic
        port: 3000
        weight: 5
      - ip: 10.0.0.2    # Gets 30% of traffic
        port: 3000
        weight: 3
      - ip: 10.0.0.3    # Gets 20% of traffic
        port: 3000
        weight: 2
```

**Use when:**
- Backends have different capacities
- Gradual rollout of new versions
- Cost optimization (cheaper servers get less traffic)

### Consistent Hashing

Routes requests to backends based on hash of request attributes. Same request always goes to same backend.

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: consistent
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000
```

**Use when:**
- Session affinity needed (e.g., WebSocket connections)
- Backend caching (same user hits same cache)
- Stateful applications

**Hashing key:** By default, uses client IP. Can be customized in plugins.

### Random

Randomly selects a backend for each request.

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: random
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000
```

**Use when:**
- Simple distribution with minimal overhead
- Stateless applications
- Testing and development

## Health Checks

Nylon automatically removes unhealthy backends from the load balancing pool.

### Configuration

```yaml
services:
  - name: api-service
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
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

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | bool | `false` | Enable health checks |
| `path` | string | `/` | Health check endpoint path |
| `interval` | duration | `10s` | Time between checks |
| `timeout` | duration | `5s` | Request timeout |
| `healthy_threshold` | int | `2` | Consecutive successes to mark healthy |
| `unhealthy_threshold` | int | `3` | Consecutive failures to mark unhealthy |

### Health Check Behavior

1. **Initial State:** All backends start as healthy
2. **Check:** HTTP GET request to `http://backend:port/path`
3. **Success:** HTTP 2xx or 3xx response within timeout
4. **Failure:** Timeout, connection error, or 5xx response
5. **Marking Unhealthy:** After `unhealthy_threshold` consecutive failures
6. **Marking Healthy:** After `healthy_threshold` consecutive successes

## Endpoint Configuration

### Basic Endpoint

```yaml
endpoints:
  - ip: 10.0.0.1
    port: 3000
```

### Weighted Endpoint

```yaml
endpoints:
  - ip: 10.0.0.1
    port: 3000
    weight: 5
```

Weight must be a positive integer. Only used with `weighted` algorithm.

## Examples

### High Availability Setup

```yaml
services:
  - name: production-api
    service_type: http
    algorithm: round_robin
    endpoints:
      # Primary datacenter
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      # Backup datacenter
      - ip: 10.1.0.1
        port: 3000
      - ip: 10.1.0.2
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 3s
      timeout: 1s
      healthy_threshold: 2
      unhealthy_threshold: 2
```

### Canary Deployment

```yaml
services:
  - name: api-canary
    service_type: http
    algorithm: weighted
    endpoints:
      # Stable version - 95% of traffic
      - ip: 10.0.0.1
        port: 3000
        weight: 95
      - ip: 10.0.0.2
        port: 3000
        weight: 95
      # Canary version - 5% of traffic
      - ip: 10.0.0.10
        port: 3000
        weight: 5
    health_check:
      enabled: true
      path: /health
      interval: 5s
```

### Session Affinity (WebSocket)

```yaml
services:
  - name: websocket-service
    service_type: http
    algorithm: consistent  # Same client always hits same backend
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 10s
```

### Multi-Region Load Balancing

```yaml
services:
  # US Region
  - name: api-us
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s

  # EU Region
  - name: api-eu
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.1.0.1
        port: 3000
      - ip: 10.1.0.2
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s

# Route based on geo-location (using plugin)
routes:
  - route:
      type: host
      value: api.example.com
    name: api
    paths:
      - path: /*
        service:
          name: api-us  # Default
        middleware:
          - plugin: geo-router
            entry: "route"
```

## Monitoring

### Health Check Logs

Nylon logs health check status changes:

```
[INFO] Health check passed: 10.0.0.1:3000 (2/2 healthy)
[WARN] Health check failed: 10.0.0.2:3000 (1/3 unhealthy)
[ERROR] Backend marked unhealthy: 10.0.0.2:3000
[INFO] Backend marked healthy: 10.0.0.2:3000 (2/2 healthy)
```

### Metrics

Health check metrics available on metrics endpoint:

```
nylon_backend_health{service="api-service",backend="10.0.0.1:3000"} 1
nylon_backend_health{service="api-service",backend="10.0.0.2:3000"} 0
nylon_health_check_total{service="api-service",status="success"} 1234
nylon_health_check_total{service="api-service",status="failure"} 56
```

## Best Practices

### 1. Always Enable Health Checks

```yaml
health_check:
  enabled: true
  path: /health
  interval: 5s
  timeout: 2s
```

### 2. Use Appropriate Algorithm

- **Round Robin:** Equal capacity backends, stateless apps
- **Weighted:** Different capacity backends, canary deployments
- **Consistent:** Session affinity, caching, WebSocket
- **Random:** Simple apps, testing

### 3. Tune Health Check Parameters

```yaml
# For critical services (fail fast)
health_check:
  interval: 3s
  timeout: 1s
  unhealthy_threshold: 2

# For stable services (avoid flapping)
health_check:
  interval: 10s
  timeout: 5s
  unhealthy_threshold: 5
```

### 4. Implement Proper Health Endpoints

```go
// Backend health endpoint
http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
    // Check dependencies
    if !database.Healthy() {
        w.WriteHeader(503)
        return
    }
    
    w.WriteHeader(200)
    w.Write([]byte("OK"))
})
```

### 5. Monitor Backend Status

Use metrics and logs to track:
- Health check success/failure rates
- Backend up/down events
- Request distribution across backends

## See Also

- [Configuration](/core/configuration) - Full configuration reference
- [Health Checks](#health-checks) - Detailed health check configuration
- [Examples](/examples/basic-proxy) - Load balancing examples

