# NATS Plugin Quick Start Guide

## 🎯 TL;DR

**NATS plugins = FFI plugins with unlimited scaling**

```
FFI:  Fast (0.8ms) but single-process
NATS: Slower (2-3ms) but unlimited workers + multi-language
```

---

## 🤔 Should I Use NATS or FFI?

### Choose FFI When:
- ✅ Performance critical (need < 2ms latency)
- ✅ Simple deployment (single node)
- ✅ Stable load (no need to scale)
- ✅ Go/Rust only

### Choose NATS When:
- ✅ Need horizontal scaling
- ✅ Variable load (auto-scale workers)
- ✅ Multi-language plugins
- ✅ Fault tolerance critical
- ✅ Microservices architecture

---

## 📊 Performance Comparison

```
┌────────────────────────────────────────────────────────────┐
│ Metric          │ FFI         │ NATS (Local) │ NATS (Net) │
├────────────────────────────────────────────────────────────┤
│ Latency         │ 0.8ms       │ 2-3ms        │ 3-5ms      │
│ Throughput      │ 44K req/s   │ 30K req/s    │ 20K req/s  │
│ Max Workers     │ 1           │ Unlimited    │ Unlimited  │
│ Languages       │ Go, Rust    │ Any          │ Any        │
│ Fault Tolerance │ None        │ Auto         │ Auto       │
│ Scaling         │ Manual      │ Dynamic      │ Dynamic    │
└────────────────────────────────────────────────────────────┘
```

---

## 🚀 Quick Example

### FFI Plugin (Existing)

```go
package main

import "C"
import "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
    plugin := sdk.NewNylonPlugin()
    
    plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
        phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
            // Your code here
            ctx.Next()
        })
    })
}
```

**Build & Run:**
```bash
go build -buildmode=c-shared -o plugin.so main.go
./nylon --config config.yaml
```

### NATS Plugin (New!)

```go
package main

import "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {
    // Only difference: NewNatsPlugin instead of NewNylonPlugin
    plugin, _ := sdk.NewNatsPlugin(
        "nats://localhost:4222",
        "auth_workers",
    )
    defer plugin.Close()
    
    // Same API!
    plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
        phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
            // Same code!
            ctx.Next()
        })
    })
    
    select {} // Keep running
}
```

**Build & Run:**
```bash
go build -o auth-plugin main.go

# Run 1 worker
./auth-plugin

# Scale to 10 workers!
for i in {1..10}; do ./auth-plugin & done
```

---

## ⚙️ Configuration

### Nylon Config (FFI)
```yaml
plugins:
  - name: auth_plugin
    type: ffi
    file: ./plugin.so
```

### Nylon Config (NATS)
```yaml
messaging:
  - name: my_nats
    type: nats
    servers:
      - nats://localhost:4222

plugins:
  - name: auth_plugin
    type: messaging
    messaging: my_nats
    group: auth_workers
```

---

## 🎨 Migration Guide

### Step 1: Change Constructor
```go
// FROM
plugin := sdk.NewNylonPlugin()

// TO
plugin, err := sdk.NewNatsPlugin("nats://localhost:4222", "workers")
if err != nil {
    panic(err)
}
defer plugin.Close()
```

### Step 2: Keep Running
```go
// Add at the end of main()
select {} // Keep running
```

### Step 3: Build Differently
```bash
# FFI
go build -buildmode=c-shared -o plugin.so

# NATS
go build -o plugin
```

**That's it!** Everything else stays the same.

---

## 🐳 Docker Example

```dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY . .
RUN go build -o plugin main.go

FROM alpine:latest
COPY --from=builder /app/plugin /plugin
ENV NATS_SERVERS=nats://nats:4222
ENV NATS_GROUP=workers
CMD ["/plugin"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  nats:
    image: nats:latest
    ports:
      - "4222:4222"
      
  nylon:
    image: nylon:latest
    ports:
      - "8088:8088"
    depends_on:
      - nats
      
  auth-plugin:
    build: ./plugins/auth
    depends_on:
      - nats
    deploy:
      replicas: 5  # 5 workers!
```

---

## 🔥 Common Patterns

### Pattern 1: Hybrid
```yaml
# Use both FFI and NATS!
plugins:
  - name: fast_plugin
    type: ffi              # Low latency
    file: ./fast.so
    
  - name: scalable_plugin
    type: messaging        # Auto-scaled
    messaging: my_nats
    group: workers
```

### Pattern 2: Dev → Prod
```bash
# Development (FFI)
go build -buildmode=c-shared -o plugin.so
./nylon --config dev.yaml

# Production (NATS)
go build -o plugin
docker-compose up --scale auth-plugin=10
```

### Pattern 3: Blue-Green Deployment
```bash
# Old version (blue)
NATS_GROUP=auth_v1 ./auth-plugin &

# New version (green)
NATS_GROUP=auth_v2 ./auth-plugin &

# Switch traffic in Nylon config
# messaging: auth_v1 → auth_v2

# Zero downtime! 🎉
```

---

## ❓ FAQ

**Q: Can I use the same code for FFI and NATS?**  
A: Almost! Only constructor changes. Everything else is identical.

**Q: What's the performance overhead?**  
A: +2-3ms latency, -30% throughput, BUT unlimited scaling.

**Q: Do I need to run NATS server?**  
A: Yes. But it's easy: `docker run -p 4222:4222 nats`

**Q: Can I mix FFI and NATS plugins?**  
A: Yes! Use FFI for fast path, NATS for scalable path.

**Q: How do I monitor NATS plugins?**  
A: NATS has built-in monitoring: http://localhost:8222/varz

**Q: What if NATS server goes down?**  
A: Auto-reconnect. Workers keep trying to reconnect.

**Q: How do I scale up/down?**  
A: Just start/stop plugin processes. NATS handles the rest.

---

## 🎓 Next Steps

1. **Read Full Roadmap**: `docs/NATS_PLUGIN_ROADMAP.md`
2. **Try Example**: `examples/nats-plugin-go/`
3. **Join Discussion**: GitHub Issues

---

## 📝 Code Comparison

### Identical API
```go
// These work in BOTH FFI and NATS!
plugin.AddPhaseHandler(...)
phase.RequestFilter(...)
phase.ResponseFilter(...)
phase.ResponseBodyFilter(...)
phase.Logging(...)

req := ctx.Request()
req.Path()
req.Headers()
req.Body()

res := ctx.Response()
res.SetStatus(200)
res.SetHeader("X-Custom", "value")
res.BodyJSON(data)

ctx.Next()
ctx.End()
```

### Only Difference
```go
// FFI
plugin := sdk.NewNylonPlugin()
// (runs in Nylon process)

// NATS
plugin := sdk.NewNatsPlugin("nats://...", "group")
// (runs as separate process)
select {} // keep running
```

---

## 🎉 Summary

**NATS Plugin = Same Code + Unlimited Scale**

```
Same API ✅
Same behavior ✅
Different deployment ✅
Unlimited workers ✅
```

**When in doubt, start with FFI, scale with NATS!** 🚀

---

**Created:** 2025-10-21  
**Version:** 1.0  
**For:** Nylon v1.0-beta.5+

