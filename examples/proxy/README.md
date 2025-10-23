# Nylon Proxy Examples

This directory contains example configurations for Nylon proxy.

## Configuration Files

### `base.yaml` - Traditional FFI Plugin
Uses in-process FFI plugins for low-latency processing.

```bash
# Start with FFI plugin
nylon start -c examples/proxy/base.yaml
```

**Features:**
- ✅ Low latency (~100μs)
- ✅ Simple deployment
- ❌ No horizontal scaling

### `nats.yaml` - NATS-Based Plugin
Uses NATS for distributed, horizontally scalable plugins.

```bash
# 1. Start NATS server
docker run -d --name nats -p 4222:4222 nats:latest

# 2. Start NATS workers (multiple instances for load balancing)
cd examples/go-nats
go run main.go  # Terminal 1
go run main.go  # Terminal 2
go run main.go  # Terminal 3

# 3. Start Nylon with NATS config
nylon start -c examples/proxy/nats.yaml
```

**Features:**
- ✅ Horizontal scaling
- ✅ Zero-downtime deployments
- ✅ Language-agnostic workers
- ⚡ ~0.5-1ms latency

## Switching Between FFI and NATS

### FFI Plugin (`base.yaml`)
```yaml
plugins:
  - name: plugin_sdk
    type: ffi
    file: ./target/examples/go/plugin_sdk.so

middleware_groups:
  auth:
    - plugin: plugin_sdk
      entry: "authz"
```

### NATS Plugin (`nats.yaml`)
```yaml
messaging:
  - name: default
    servers:
      - nats://localhost:4222

plugins:
  - name: plugin_nats
    messaging: default
    group: demo-workers

middleware_groups:
  auth:
    - plugin: plugin_nats
      entry: "authz"
```

**Same handler code in both cases!** Just change the plugin type.

## Configuration Options

### NATS Messaging Block

```yaml
messaging:
  - name: default
    servers:
      - nats://nats-1:4222
      - nats://nats-2:4222
    request_timeout_ms: 5000  # Default: 5000
    max_inflight: 1024        # Default: 1024
    subject_prefix: nylon.plugin  # Default
    retry:
      max: 3
      backoff_ms_initial: 100
      backoff_ms_max: 1000
```

### Plugin Configuration

```yaml
plugins:
  - name: plugin_nats
    messaging: default      # Reference to messaging block
    group: my-workers       # Queue group for load balancing
    config:                 # Passed to plugin.Initialize()
      debug: true
    per_phase:              # Optional: override smart defaults
      request_filter:
        timeout_ms: 8000
        on_error: retry     # Options: retry, continue, end
      logging:
        timeout_ms: 200
        on_error: continue  # Never block response
```

### Smart Defaults (No Config Needed!)

If you don't specify `per_phase`, these defaults apply:

| Phase | Timeout | On Error | Retries | Rationale |
|-------|---------|----------|---------|-----------|
| `request_filter` | 5000ms | retry | 3x | Critical path |
| `response_filter` | 3000ms | continue | 2x | Non-blocking |
| `response_body_filter` | 3000ms | continue | 2x | Non-blocking |
| `logging` | 200ms | continue | 1x | Fast, never blocks |

## NATS Subject Layout

Workers subscribe to these subjects with queue groups:

```
nylon.plugin.{plugin_name}.request_filter   (queue: "demo-workers")
nylon.plugin.{plugin_name}.response_filter  (queue: "demo-workers")
nylon.plugin.{plugin_name}.response_body_filter
nylon.plugin.{plugin_name}.logging
```

NATS automatically picks **ONE worker** per request from the queue group.

## Monitoring

### View NATS Subjects
```bash
# Subscribe to all plugin messages
nats sub "nylon.plugin.>"

# Subscribe with queue group (simulate worker)
nats sub --queue demo-workers "nylon.plugin.plugin_nats.>"
```

### Worker Logs
```
[NatsPlugin] Connected to NATS: nats://localhost:4222
[NatsPlugin] Subscribed to nylon.plugin.plugin_nats.request_filter with queue group demo-workers
[NatsPlugin] Plugin plugin_nats started successfully
[NatsPlugin] Received request: session=1 phase=1 method=
Authz[Go-NATS] RequestFilter sessionID 1
```

## Testing

### Load Test FFI Plugin
```bash
# Terminal 1: Start Nylon with FFI
nylon start -c examples/proxy/base.yaml

# Terminal 2: Benchmark
ab -n 10000 -c 100 http://localhost:8000/api/users
```

### Load Test NATS Plugin
```bash
# Terminal 1: Start NATS
docker run -p 4222:4222 nats

# Terminal 2-4: Start 3 workers
go run examples/go-nats/main.go

# Terminal 5: Start Nylon
nylon start -c examples/proxy/nats.yaml

# Terminal 6: Benchmark
ab -n 10000 -c 100 http://localhost:8000/api/users
```

Compare latency and throughput!

## Migration Path

1. **Start with FFI** (`base.yaml`) for development
2. **Add NATS config** when you need scaling
3. **Run workers** in separate processes
4. **Switch config** to `nats.yaml`
5. **No code changes** to plugin logic!

## Troubleshooting

### "No NATS connection"
- Ensure NATS server is running: `docker ps | grep nats`
- Check connectivity: `nats server check`

### "No workers responding"
- Verify workers are running: `ps aux | grep main`
- Check queue group matches config
- View NATS logs: `nats sub "nylon.plugin.>"`

### "Timeout errors"
- Increase `request_timeout_ms` in messaging config
- Check worker processing time
- Verify network latency to NATS

## Resources

- [NATS Quick Start](../../docs/NATS_PLUGIN_QUICKSTART.md)
- [NATS Configuration](../../docs/NATS_PLUGIN_CONFIG.md)
- [NATS Roadmap](../../docs/NATS_PLUGIN_ROADMAP.md)
- [NATS Queue Groups Docs](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough)

