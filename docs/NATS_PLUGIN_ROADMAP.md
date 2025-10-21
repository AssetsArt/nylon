# Nylon NATS Plugin Implementation Roadmap

## 📋 Overview

เพิ่มความสามารถให้ Nylon รองรับ **NATS-based plugins** ควบคู่กับ FFI plugins ที่มีอยู่ โดยใช้ [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue) เพื่อให้ได้:
- ✅ Horizontal scaling (unlimited workers)
- ✅ Multi-language support (Go, Python, Node.js, etc.)
- ✅ Fault tolerance & auto recovery
- ✅ Zero configuration load balancing

### Design Principles
1. **Transparent API** - Plugin developers ใช้ API เดิม ไม่ต้องเปลี่ยน code
2. **Explicit Constructor** - `NewNylonPlugin()` (FFI) vs `NewNatsPlugin()` (NATS)
3. **Config-driven** - เปลี่ยน deployment mode ผ่าน config
4. **Backward Compatible** - FFI plugins ทำงานเหมือนเดิม 100%

---

## 🎯 Goals

### Performance Targets
```
FFI (Current):    44,302 req/s, 4.51ms latency
NATS (Target):    30,000+ req/s, 6-8ms latency

Trade-off: -30% performance BUT unlimited scaling
```

### Feature Parity
- ✅ All phases: RequestFilter, ResponseFilter, ResponseBodyFilter, Logging
- ✅ WebSocket support
- ✅ Session management
- ✅ Error handling
- ✅ Metrics & monitoring

---

## 📦 Phase 1: Core Infrastructure (Week 1-2)

### 1.1 Rust: NATS Client Integration

**Files to create:**
```
crates/nylon-messaging/
├── Cargo.toml
└── src/
    ├── lib.rs           # Main exports
    ├── nats_client.rs   # NATS connection management
    ├── protocol.rs      # Message protocol (MessagePack)
    └── error.rs         # Error types
```

**Tasks:**
- [ ] Create `nylon-messaging` crate
- [ ] Add dependency: `async-nats = "0.33"`
- [ ] Add dependency: `rmp-serde = "1.1"` (MessagePack)
- [ ] Implement `NatsClient` struct
- [ ] Connection pooling
- [ ] Auto reconnect logic
- [ ] Health check mechanism

**Code Structure:**
```rust
// crates/nylon-messaging/src/lib.rs
pub struct NatsClient {
    client: async_nats::Client,
    timeout: Duration,
}

impl NatsClient {
    pub async fn connect(servers: Vec<String>) -> Result<Self>;
    pub async fn request(&self, subject: &str, data: &[u8]) -> Result<Vec<u8>>;
    pub async fn publish(&self, subject: &str, data: &[u8]) -> Result<()>;
}
```

### 1.2 Message Protocol

**Protocol Definition:**
```rust
// crates/nylon-messaging/src/protocol.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct PluginRequest {
    pub session_id: u32,
    pub phase: u8,           // 0=Zero, 1=RequestFilter, etc.
    pub method: u32,         // Method ID
    pub data: Vec<u8>,       // Payload
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PluginResponse {
    pub session_id: u32,
    pub action: ResponseAction,
    pub data: Vec<u8>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum ResponseAction {
    Next,
    End,
    Error,
}
```

### 1.3 Config Support

**Update config types:**
```rust
// crates/nylon-types/src/plugins.rs

#[derive(Serialize, Deserialize)]
pub enum PluginType {
    Ffi,
    Messaging,  // New!
}

#[derive(Serialize, Deserialize)]
pub struct MessagingConfig {
    pub name: String,
    pub servers: Vec<String>,
    pub auth: Option<AuthConfig>,
    pub timeout: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PluginItem {
    pub name: String,
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
    
    // FFI fields
    pub file: Option<String>,
    
    // Messaging fields
    pub messaging: Option<String>,  // Messaging config name
    pub group: Option<String>,      // Queue group name
}
```

**YAML Config:**
```yaml
messaging:
  - name: local_nats
    type: nats
    servers:
      - nats://localhost:4222
    timeout: 5s

plugins:
  - name: auth_service
    type: messaging
    messaging: local_nats
    group: auth_workers
```

### 1.4 Plugin Manager Updates

**Files to modify:**
```
crates/nylon-plugin/src/
├── plugin_manager.rs  # Add NATS plugin support
└── messaging.rs       # New: NATS plugin implementation
```

**Tasks:**
- [ ] Extend `PluginManager::get_plugin()` to handle NATS
- [ ] Create `MessagingPlugin` struct
- [ ] Implement `PluginSessionStream` for `MessagingPlugin`
- [ ] Subject naming convention

**Code:**
```rust
// crates/nylon-plugin/src/messaging.rs
pub struct MessagingPlugin {
    client: Arc<NatsClient>,
    plugin_name: String,
    queue_group: String,
    timeout: Duration,
}

#[async_trait]
impl PluginSessionStream for MessagingPlugin {
    async fn event_stream(
        &self,
        phase: PluginPhase,
        method: u32,
        data: &[u8],
    ) -> Result<(), NylonError> {
        let subject = format!("nylon.plugin.{}.req", self.plugin_name);
        let inbox = format!("nylon.plugin.{}.resp.{}", 
            self.plugin_name, self.session_id);
        
        let request = PluginRequest {
            session_id: self.session_id,
            phase: phase.to_u8(),
            method,
            data: data.to_vec(),
            timestamp: current_timestamp(),
        };
        
        let response = self.client.request(&subject, &request).await?;
        // Handle response...
        Ok(())
    }
}
```

---

## 📦 Phase 2: Go SDK (Week 2-3)

### 2.1 NATS Transport Implementation

**Files to create:**
```
sdk/go/sdk/
├── transport.go       # Interface definition
├── transport_ffi.go   # FFI implementation (refactor existing)
├── transport_nats.go  # NATS implementation (new)
└── nats_plugin.go     # Public API for NATS
```

**Tasks:**
- [ ] Define `PluginTransport` interface
- [ ] Refactor existing FFI code to implement interface
- [ ] Implement NATS transport
- [ ] Message serialization (MessagePack)
- [ ] Queue subscription with group

**Interface:**
```go
// sdk/go/sdk/transport.go
type PluginTransport interface {
    AddHandler(name string, handler func(*PhaseHandler)) error
    Initialize(fn func(map[string]interface{}))
    Shutdown(fn func())
    Start() error
    Close() error
}
```

### 2.2 NATS Plugin Implementation

```go
// sdk/go/sdk/nats_plugin.go
package sdk

import (
    "github.com/nats-io/nats.go"
    "github.com/vmihailenco/msgpack/v5"
)

type NatsPlugin struct {
    nc         *nats.Conn
    queueGroup string
    handlers   sync.Map
}

func NewNatsPlugin(servers string, group string) (*NatsPlugin, error) {
    nc, err := nats.Connect(servers)
    if err != nil {
        return nil, err
    }
    
    return &NatsPlugin{
        nc:         nc,
        queueGroup: group,
    }, nil
}

func (p *NatsPlugin) AddPhaseHandler(name string, handler func(*PhaseHandler)) {
    subject := fmt.Sprintf("nylon.plugin.%s.req", name)
    
    p.nc.QueueSubscribe(subject, p.queueGroup, func(msg *nats.Msg) {
        var req PluginRequest
        msgpack.Unmarshal(msg.Data, &req)
        
        // Create phase handler
        phase := &PhaseHandler{
            SessionId: req.SessionId,
            http_ctx: &NylonHttpPluginCtx{
                sessionID: req.SessionId,
                // ... create context from request
            },
        }
        
        // Call user handler
        handler(phase)
        
        // Serialize response
        resp := PluginResponse{
            SessionId: req.SessionId,
            Action: "next",
            Data: nil,
        }
        
        data, _ := msgpack.Marshal(resp)
        msg.Respond(data)
    })
    
    p.handlers.Store(name, handler)
}

func (p *NatsPlugin) Initialize(fn func(map[string]interface{})) {
    // Implementation
}

func (p *NatsPlugin) Shutdown(fn func()) {
    // Implementation
}
```

### 2.3 Shared Handler Code

**Files to update:**
```
sdk/go/sdk/
├── plugin.go          # FFI plugin (existing, minimal changes)
├── nats_plugin.go     # NATS plugin (new)
├── phase_handler.go   # Shared handler logic
└── http_context.go    # Shared context logic
```

**Goal:** Reuse existing `PhaseHandler`, `PhaseRequestFilter`, etc. for both FFI and NATS

---

## 📦 Phase 3: Integration & Testing (Week 3-4)

### 3.1 Config Loading

**File:** `crates/nylon-config/src/plugins.rs`

**Tasks:**
- [ ] Parse `messaging` section
- [ ] Parse `plugins.type = messaging`
- [ ] Validate config
- [ ] Load NATS connections at startup

### 3.2 Plugin Loading Flow

```rust
// crates/nylon-plugin/src/loaders.rs

pub fn load_messaging_plugin(
    plugin: &PluginItem,
    messaging_config: &MessagingConfig,
) -> Result<Arc<MessagingPlugin>> {
    // Connect to NATS
    let client = NatsClient::connect(messaging_config.servers.clone()).await?;
    
    // Create plugin
    let plugin = MessagingPlugin {
        client: Arc::new(client),
        plugin_name: plugin.name.clone(),
        queue_group: plugin.group.clone().unwrap_or("default".to_string()),
        timeout: parse_timeout(&messaging_config.timeout),
    };
    
    Ok(Arc::new(plugin))
}
```

### 3.3 Testing

**Create test suite:**
```
tests/
├── integration/
│   ├── nats_plugin_test.rs
│   ├── ffi_plugin_test.rs
│   └── hybrid_test.rs
└── benchmarks/
    ├── ffi_benchmark.rs
    └── nats_benchmark.rs
```

**Test cases:**
- [ ] NATS connection
- [ ] Request-reply flow
- [ ] Multiple workers (load balancing)
- [ ] Worker failure & recovery
- [ ] Timeout handling
- [ ] Error propagation
- [ ] Performance benchmarks

---

## 📦 Phase 4: Examples & Documentation (Week 4)

### 4.1 Example Plugins

**Create examples:**
```
examples/
├── nats-plugin-go/
│   ├── main.go           # NATS plugin example
│   ├── Makefile
│   └── README.md
├── hybrid-config/
│   └── config.yaml       # FFI + NATS mixed
└── distributed/
    ├── docker-compose.yml
    ├── nylon-config.yaml
    └── README.md
```

**Example: Auth Plugin (NATS)**
```go
// examples/nats-plugin-go/main.go
package main

import (
    "fmt"
    "github.com/AssetsArt/nylon/sdk/go/sdk"
)

func main() {
    plugin, err := sdk.NewNatsPlugin(
        "nats://localhost:4222",
        "auth_workers",
    )
    if err != nil {
        panic(err)
    }
    defer plugin.Close()
    
    plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
        phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
            req := ctx.Request()
            res := ctx.Response()
            
            token := req.Header("Authorization")
            if token == "" {
                res.SetStatus(401)
                res.BodyJSON(map[string]string{"error": "unauthorized"})
                ctx.End()
                return
            }
            
            res.SetHeader("X-Auth-User", "john")
            ctx.Next()
        })
    })
    
    fmt.Println("Auth plugin started on NATS")
    select {} // Keep running
}
```

### 4.2 Documentation

**Create docs:**
```
docs/
├── nats-plugins/
│   ├── getting-started.md
│   ├── configuration.md
│   ├── api-reference.md
│   ├── deployment.md
│   └── troubleshooting.md
└── migration/
    └── ffi-to-nats.md
```

**Content:**
- Getting started guide
- Configuration reference
- API documentation
- Deployment patterns (Docker, K8s)
- Performance tuning
- Migration guide from FFI to NATS
- Best practices

---

## 📦 Phase 5: Advanced Features (Week 5+)

### 5.1 Monitoring & Metrics

**Tasks:**
- [ ] NATS connection metrics
- [ ] Request/response latency
- [ ] Error rate tracking
- [ ] Worker health checks
- [ ] Prometheus integration

### 5.2 Enhanced Features

**Optional enhancements:**
- [ ] Request batching
- [ ] Response caching
- [ ] Circuit breaker pattern
- [ ] Rate limiting per worker
- [ ] Distributed tracing (OpenTelemetry)
- [ ] JetStream integration (persistent queues)

### 5.3 Additional SDKs

**Language support:**
- [ ] Python SDK
- [ ] Node.js SDK
- [ ] Rust SDK (native NATS plugins)
- [ ] .NET SDK

---

## 🔧 Implementation Checklist

### Rust (Nylon Core)

- [ ] Create `nylon-messaging` crate
- [ ] NATS client wrapper (`async-nats`)
- [ ] Message protocol (MessagePack serialization)
- [ ] Config types for messaging
- [ ] Plugin manager updates
- [ ] `MessagingPlugin` implementation
- [ ] Session management for NATS
- [ ] Error handling & timeouts
- [ ] Metrics integration

### Go SDK

- [ ] `PluginTransport` interface
- [ ] Refactor FFI code to use interface
- [ ] `NatsPlugin` implementation
- [ ] NATS connection management
- [ ] Queue subscription
- [ ] Message handling
- [ ] Context creation from NATS messages
- [ ] Response serialization
- [ ] Error handling

### Configuration

- [ ] Parse `messaging` section
- [ ] Parse `plugins.type = messaging`
- [ ] Validation logic
- [ ] Documentation

### Testing

- [ ] Unit tests (Rust)
- [ ] Unit tests (Go)
- [ ] Integration tests
- [ ] Load tests
- [ ] Fault tolerance tests
- [ ] Benchmarks

### Documentation

- [ ] Getting started guide
- [ ] API reference
- [ ] Configuration guide
- [ ] Deployment guide
- [ ] Migration guide
- [ ] Troubleshooting guide

### Examples

- [ ] Basic NATS plugin (Go)
- [ ] Auth service example
- [ ] Multi-language example
- [ ] Docker compose setup
- [ ] Kubernetes deployment
- [ ] Hybrid (FFI + NATS) setup

---

## 📊 Dependencies

### Rust Crates
```toml
[dependencies]
async-nats = "0.33"       # NATS client
rmp-serde = "1.1"         # MessagePack
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

### Go Packages
```go
require (
    github.com/nats-io/nats.go v1.31.0
    github.com/vmihailenco/msgpack/v5 v5.4.1
)
```

---

## 🎯 Success Metrics

### Performance
- [ ] NATS latency < 8ms (p99)
- [ ] Throughput > 30K req/s (single Nylon + 10 workers)
- [ ] No message loss under normal conditions
- [ ] Graceful degradation on worker failures

### Reliability
- [ ] Auto-reconnect on NATS server failure
- [ ] Worker crash doesn't affect other workers
- [ ] No responder detection < 100ms
- [ ] 99.9% uptime

### Developer Experience
- [ ] API identical to FFI plugins
- [ ] < 10 lines of code to migrate FFI → NATS
- [ ] Clear error messages
- [ ] Good documentation
- [ ] Working examples

---

## 🚀 Deployment Patterns

### Pattern 1: Hybrid (Dev/Staging)
```yaml
plugins:
  - name: simple_plugin
    type: ffi              # Low latency
    file: ./plugin.so
    
  - name: complex_plugin
    type: messaging        # Scalable
    messaging: local_nats
    group: workers
```

### Pattern 2: Full NATS (Production)
```yaml
messaging:
  - name: prod_nats
    type: nats
    servers:
      - nats://nats1:4222
      - nats://nats2:4222
      - nats://nats3:4222

plugins:
  - name: auth
    type: messaging
    messaging: prod_nats
    group: auth_workers
    
  - name: rate_limit
    type: messaging
    messaging: prod_nats
    group: ratelimit_workers
```

### Pattern 3: Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: auth-plugin
spec:
  replicas: 10  # Scale easily!
  template:
    spec:
      containers:
      - name: auth-plugin
        image: my-auth-plugin:latest
        env:
        - name: NATS_SERVERS
          value: "nats://nats:4222"
        - name: NATS_GROUP
          value: "auth_workers"
```

---

## 📝 Notes

### Trade-offs
- **Latency**: +2-3ms overhead vs FFI
- **Complexity**: Additional NATS infrastructure
- **Benefits**: Unlimited scaling, multi-language, fault tolerance

### When to Use
- **FFI**: Simple plugins, low latency critical, single node
- **NATS**: Complex plugins, need scaling, multi-language, distributed

### Future Considerations
- JetStream for persistent queues
- NATS key-value for shared state
- NATS object store for large payloads
- Multi-region NATS superclusters

---

## 🎓 References

- [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue)
- [NATS Request-Reply](https://docs.nats.io/nats-concepts/core-nats/reqreply)
- [async-nats Rust Client](https://docs.rs/async-nats/)
- [nats.go Documentation](https://docs.nats.io/using-nats/developer/connecting)

---

**Created:** 2025-10-21  
**Version:** 1.0  
**Status:** 📋 Planning Phase

