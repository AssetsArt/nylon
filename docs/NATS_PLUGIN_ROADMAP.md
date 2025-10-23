# Nylon NATS Plugin Implementation Roadmap

## Objective

- Deliver a production-ready NATS-backed plugin transport that mirrors the behaviour of today's FFI plugins.
- Keep the Go SDK API stable while letting plugin authors opt-in to horizontal scaling through configuration.
- Provide the operational guardrails (metrics, logging, failure handling) needed for staging and production rollouts.

## Current Status (December 2024)

### ✅ Completed (Phase 1-3)
- ✅ **Transport Abstraction**: `PluginTransport` trait with `TransportEvent`, `TransportInvoke`, and `TraceMeta` in `nylon-types/src/transport.rs`
- ✅ **NATS Messaging Crate**: `crates/nylon-messaging` with `NatsClient`, `MessagingTransport`, protocol types, and MessagePack serialization
- ✅ **Runtime Integration**: Nylon routes plugin sessions through either FFI or messaging via unified transport layer
- ✅ **Method Processing**: Dispatcher for NATS invokes with support for control and response write methods
- ✅ **Retry Logic**: Full retry support with `PhasePolicy` (max_attempts, backoff, on_error policies)
- ✅ **Error Handling**: Comprehensive error handling with Continue/End/Retry strategies
- ✅ **Tracing**: Request ID, trace ID propagation, and span tracking per plugin session
- ✅ **Configuration**: Parse and validate `messaging:` blocks, store in `KEY_MESSAGING_PLUGINS`
- ✅ **FFI Transport Path**: Optional transport-based FFI path via `NYLON_USE_FFI_TRANSPORT` env var

### 🚧 In Progress
- 🚧 **Read Methods**: Response data flow for messaging transport (GET_PAYLOAD, READ_REQUEST_*, READ_RESPONSE_*)
- 🚧 **Go SDK Transport**: NATS backend implementation for Go SDK
- 🚧 **Integration Tests**: End-to-end tests with NATS broker

### ⏳ Not Started
- ⏳ **WebSocket Support**: WebSocket methods over NATS (see `docs/WEBSOCKET_NATS_DESIGN.md`)
- ⏳ **Load Testing**: Performance benchmarks and parity validation
- ⏳ **Production Hardening**: Circuit breakers, DLQ, advanced metrics

### 🎯 WebSocket Support Strategy
See detailed design in `docs/WEBSOCKET_NATS_DESIGN.md`. Summary:

**Core NATS Queue Groups Only** (No JetStream required):
- ✅ Nylon handles WebSocket protocol (frames, handshake)
- ✅ Workers receive high-level events via request-reply
- ✅ Queue groups auto-balance (per [NATS Queue Groups](https://docs.nats.io/nats-concepts/core-nats/queue/queues_walkthrough))
- ✅ Room broadcasting via pub/sub (no queue group = fan-out)
- ✅ Stateless workers (no session state)
- ✅ Simple deployment (NATS Core server only)

**Subjects:**
- `nylon.ws.{plugin}.events` → Queue group workers (one receives)
- `nylon.ws.room.{room}` → Pub/sub (all receive)

**Benefits:** Simple, scalable, no JetStream complexity, workers can scale independently.

## Critical Path

1. Build a reusable messaging layer that encapsulates NATS connectivity, retries and serialization.
2. Teach the Rust runtime to route plugin sessions through either FFI or messaging with identical semantics and at-least-once delivery guarantees.
3. Introduce a Go SDK transport that implements the existing handler API on top of NATS request/reply with backpressure controls.
4. Wire configuration, feature flags and dependency injection so Nylon can boot with NATS plugins enabled, including timeout/retry and overflow policies.
5. Validate the end-to-end flow with tests, instrumentation and operational tooling that capture retries, slow consumers and schema drift.
6. Update examples and docs to reflect the real implementation, security posture and migration steps.

## Non-Negotiable Requirements

- **At-least-once semantics**: include a deterministic `request_id` (UUID/u128) per message; Nylon stores last-seen values per `session_id + phase` to deduplicate retries. Echo the id in responses and expose `ctx.IdempotencyKey()` so handlers stay idempotent.
- **Backpressure & concurrency**: configurable `max_inflight` and `overflow_policy` (`reject | queue | shed`) on the Nylon side; SDK exposes worker concurrency (`SetMaxWorkers`) and queue prefetch limits.
- **Timeouts & retries**: per-plugin (and per-phase overrides) for `request_timeout_ms` plus exponential backoff settings (`retry.max`, `backoff_ms_initial`, `backoff_ms_max`). Allow soft-fail phases (e.g. logging) via `on_error: continue`.
- **Observability**: propagate `traceparent` in NATS headers, log `subject`, `queue`, `session_id`, `request_id`; export metrics `plugins_messaging_inflight`, `..._retries_total`, `..._timeouts_total`, `..._dlq_total`, and latency histograms p50/p90/p99 per plugin/phase.
- **Schema versioning**: embed `version: u16` in both request and response; Nylon accepts `N-1..N` and warns on unknown versions.
- **Subject naming & isolation**: adopt `nylon.{env}.{plugin}.{phase}` to prevent cross-environment collisions; queue group names map to worker pools.
- **Error taxonomy**: distinguish `TransportError` vs `HandlerError` and map to `NylonError::Messaging { kind: Timeout | NoResponder | Decode | Plugin }`. Respect `on_error` policy (`continue | end | retry`).
- **Payload safety**: enforce maximum payload size per phase (default 1-2 MB). For larger bodies, pass handles/keys to shared storage instead of raw bytes.
- **Security**: support unauthenticated dev setups but require TLS + nkeys/JWT for staging/prod. Configuration must capture `auth` and TLS material.
- **Graceful shutdown & health**: drain inflight requests on both Nylon and worker shutdown. Provide `/healthz` and `/metrics` endpoints with NATS connection status and instrumentation.

## Milestone 0 - Repository Preparation ✅ COMPLETED

- ✅ Add `crates/nylon-messaging` to the workspace and share crate-level lint configuration.
- ✅ Decide on MessagePack (`rmp-serde`) as the canonical encoding and document versioning policy.
- [ ] Add a `scripts/dev-nats.sh` helper that starts a local NATS server (docker-compose or `nats-server` binary).
- ✅ Gate the new code behind a `messaging` cargo feature flag so we can merge incrementally.

## Milestone 1 - Messaging Foundation (Rust) ✅ COMPLETED

`crates/nylon-messaging/`
- ✅ Add dependencies: `async-nats = "0.33"`, `tokio` (full), `rmp-serde`, `serde`, `tracing`, `futures`.
- ✅ Implement `NatsClient::connect(servers: &[String], options: NatsClientOptions)` returning `Arc<NatsClient>`.
- ✅ Provide `request(subject, payload)` and `publish(subject, payload)` helpers with timeout handling and trace header propagation.
- ✅ Generate a `request_id` (or accept caller-provided) via `new_request_id()` using UUID v7; dedupe in `TransportSessionHandler`.
- ✅ Add queue subscription helper for worker-side consumption (`subscribe_queue`).
- ✅ Implement reconnect logic (async-nats handles automatically); `max_inflight` accounting via `Semaphore`.
- ✅ Define `protocol.rs` with `PluginRequest`, `PluginResponse`, `ResponseAction` enums matching plugin phases, including schema `version` and `request_id`.
- ✅ Add serialization utilities (`encode_request`, `decode_response`, `encode_response`, `decode_request`) using MessagePack.
- [ ] Integration tests with in-process NATS server and version interoperability assertions.
- [ ] Expose metrics hooks (`messaging_requests_total`, `messaging_request_duration_seconds`, `messaging_retries_total`, `messaging_timeouts_total`).

## Milestone 2 - Runtime Integration (Rust) ✅ MOSTLY COMPLETED

`crates/nylon-types/src/plugins.rs`
- ✅ Extend `PluginType` with `Messaging`.
- ✅ Introduce `MessagingConfig` and related structs with serde support for YAML/TOML.
- ✅ Add `transport.rs` with `PluginTransport` trait, `TransportEvent`, `TransportInvoke`, and `TraceMeta`.

`crates/nylon-config/src/plugins.rs`
- ✅ Parse the `messaging` block, validate URLs, authentication and default queue groups.
- ✅ Store messaging configs in `nylon-store` (`KEY_MESSAGING_CONFIG`, `KEY_MESSAGING_PLUGINS`) for fast lookup.

`crates/nylon-plugin/src`
- ✅ Create `messaging.rs` implementing `MessagingPlugin` that wraps `Arc<NatsClient>` plus metadata and per-phase policies.
- ✅ Update `plugin_manager.rs` so `get_plugin` returns `PluginHandle::Ffi` or `PluginHandle::Messaging` enum.
- ✅ Create `transport_handler.rs` with generic `TransportSessionHandler<T: PluginTransport>` for unified session handling.
- ✅ Create `ffi_transport.rs` implementing `PluginTransport` for FFI path (optional via `NYLON_USE_FFI_TRANSPORT`).
- ✅ Create `messaging_methods.rs` for method dispatch in messaging transport.
- ✅ Implement request/reply flow: construct subject `nylon.plugin.{plugin}.{phase}`, setup reply subscription, await responses with timeout.
- ✅ Handle `ResponseAction::{Next,End,Error}` with retry logic based on `PhasePolicy` (max_attempts, on_error).
- ✅ Implement dedupe via `request_id` cache in `TransportSessionHandler` (HashSet).
- [ ] Add graceful shutdown: draining subscriptions and reporting inflight counts.
- ✅ Surface tracing spans (request_id, trace_id, span_id) and translate messaging errors into `NylonError` variants.

## Milestone 3 - Go SDK Transport 🚧 IN PROGRESS

`sdk/go/sdk`
- 🚧 Introduce `PluginTransport` interface and refactor the current FFI implementation into `transport_ffi.go`.
- 🚧 Implement `transport_nats.go` using `github.com/nats-io/nats.go` with connection pooling, queue subscriptions, and concurrency/timeout controls.
- 🚧 Add `nats_plugin.go` that exposes `NewNatsPlugin(config)` returning a struct satisfying the existing plugin API and surfacing idempotency info (`ctx.IdempotencyKey()`).
- [ ] Reuse existing `PhaseHandler`, `PhaseRequestFilter`, and related types by translating `PluginRequest` into current structs.
- [ ] Support synchronous replies: after a user handler calls `ctx.Next`, `ctx.End`, or `ctx.Error`, marshal `PluginResponse` with `version`, `request_id`, and optional headers, then `msg.Respond`.
- [ ] Provide lifecycle hooks (`Initialize`, `Shutdown`, `Close`) that mirror FFI behaviour, drain subscriptions, and honour `MaxHandlers`.
- [ ] Add unit tests with a local NATS server using `go test ./sdk/...`.

## Milestone 4 - Configuration and CLI Wiring ✅ COMPLETED

- ✅ Config loader parses `messaging:` blocks and registers plugins in `KEY_MESSAGING_PLUGINS`.
- ✅ `PluginManager::get_plugin` returns FFI or messaging handle; messaging pulled from the store.
- [ ] Support overriding NATS servers via environment variables such as `NYLON_NATS_URLS` for container deployments.
- ✅ Validate at startup that every `type: messaging` plugin references an existing messaging config and queue group.
- ✅ Emit clear diagnostics when NATS connection fails via `map_messaging_error`.

## Milestone 5 - Validation, Testing, and Observability

- [ ] Add integration tests under `tests/integration/nats_plugin_test.rs` covering request filter, response filter, and error paths.
- [ ] Create a hybrid test (`tests/integration/hybrid_test.rs`) ensuring FFI and NATS plugins coexist.
- [ ] Write a soak test or benchmark (`tests/benchmarks/nats_benchmark.rs`) to compare throughput versus FFI.
- [ ] Implement failure simulations (drop worker, restart NATS) to verify auto-reconnect, timeout handling, and dedupe (`request_id`).
- [ ] Expose metrics (`plugins_messaging_inflight`, `plugins_messaging_retries_total`, `plugins_messaging_timeouts_total`, `plugins_messaging_dlq_total`) and document scraping via `/metrics`.
- [ ] Ensure logging includes session id, phase, method, request subject, queue group, request id, retry count, and traceparent.
- [ ] Capture latency histograms (p50/p90/p99) per plugin and phase; alert when p99 > target thresholds.

## Milestone 6 - Examples and Documentation

- [ ] Add `examples/nats-plugin-go` with Makefile, README, and a runnable worker.
- [ ] Provide `examples/distributed/docker-compose.yml` spinning up Nylon, NATS, and sample workers.
- [ ] Update `docs/NATS_PLUGIN_QUICK_START.md` and the Go SDK README to reflect the real API surface.
- [ ] Document migration steps from FFI to NATS including config diff and operational trade-offs.
- [ ] Publish a troubleshooting guide (connection refused, slow consumers, incompatible schema, duplicate request handling).
- [ ] Add an operations runbook covering retries, DLQ usage, TLS rotation, and subject namespacing.

## Message Protocol (Authoritative)

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginRequest {
    pub version: u16,         // Schema version, allows N-1 compatibility.
    pub request_id: u128,     // Stable idempotency key per message.
    pub session_id: u32,
    pub phase: u8,            // 0=None, 1=RequestFilter, 2=ResponseFilter, 3=ResponseBodyFilter, 4=Logging
    pub method: u32,          // Matches nylon_types::MethodId
    pub data: Vec<u8>,        // FlatBuffers payload produced by Nylon core
    pub timestamp: u64,       // Unix millis
    // Optional: structured headers for tracing / user metadata
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginResponse {
    pub version: u16,
    pub request_id: u128,     // Echoes request for dedupe.
    pub session_id: u32,
    pub action: ResponseAction,
    pub data: Vec<u8>,        // Optional payload (same FlatBuffers schema as FFI)
    pub error: Option<String>,// Populated when action == Error
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ResponseAction {
    Next,
    End,
    Error,
}
```

- Use MessagePack for over-the-wire encoding and include headers for `traceparent` and environment metadata.
- Reserve subject layout: `nylon.{env}.{plugin}.{phase}` for requests, reply inbox generated per session.
- Carry tracing headers via `NatsClient::publish_with_headers` and ensure Go SDK mirrors them in responses.

## Runtime Flow

1. Nylon accepts an API request and builds the FlatBuffers payload for the plugin phase.
2. `MessagingPlugin::event_stream` sends a NATS request, increments inflight counters, and waits for a response with the configured timeout/backoff.
3. Nylon stores the `(session_id, phase, request_id)` tuple to dedupe retries.
4. A worker receives the message via queue subscription, translates it into Go SDK types, exposes `ctx.IdempotencyKey()`, and invokes the registered handler under the concurrency limit.
5. The handler calls `Next`, `End`, or `Error`; the transport serializes `PluginResponse` (including `version`, `request_id`, and trace headers) and responds via `msg.Respond`.
6. Nylon resumes the pipeline, applying the same branching logic as the FFI transport and updating metrics.
7. On errors or timeouts, Nylon emits `NylonError::Messaging { kind, plugin, phase }`, evaluates the configured `on_error` policy, and may retry with exponential backoff.

## Extended Configuration Example

```yaml
messaging:
  - name: my_nats
    type: nats
    servers: ["nats://localhost:4222"]
    subject_prefix: "nylon.dev"
    request_timeout_ms: 800
    max_inflight: 2048
    overflow_policy: queue        # queue | reject | shed
    retry:
      max: 2
      backoff_ms_initial: 50
      backoff_ms_max: 250
    tls:
      enabled: false
      # ca_file: /etc/ssl/certs/ca.pem
    auth:
      enabled: false
      # nkey: "SU..."           # or reference to credentials file

plugins:
  - name: auth_plugin
    type: messaging
    messaging: my_nats
    group: auth_workers
    per_phase:
      request_filter:
        timeout_ms: 500
        on_error: retry
      logging:
        timeout_ms: 200
        on_error: continue
```

## Implementation Notes

### Transport Architecture
The implementation uses a trait-based approach:
- `PluginTransport` trait defines `send_event`, `try_recv_invoke`, and `trace_meta`
- `FfiTransport` wraps existing FFI `SessionStream` for compatibility
- `MessagingTransport` implements NATS pub/sub with buffering and reply subscriptions
- `TransportSessionHandler<T>` provides unified session handling with deduplication

### Subject Naming Convention
- Request subject: `nylon.plugin.{plugin_name}.{phase_fragment}`
- Reply subject: `nylon.plugin.{plugin_name}.reply.{session_id}`
- Phase fragments: `zero`, `request_filter`, `response_filter`, `response_body_filter`, `logging`

### Method Support Matrix
**Messaging Transport (Currently Supported):**
- ✅ Control: `NEXT`, `END`
- ✅ Response Write: `SET_RESPONSE_HEADER`, `REMOVE_RESPONSE_HEADER`, `SET_RESPONSE_STATUS`, `SET_RESPONSE_FULL_BODY`, `SET_RESPONSE_STREAM_*`
- ⏳ Read Methods: Require response data flow back through NATS (TODO)
- ❌ WebSocket: Not supported (requires persistent connection)

### Retry Behavior
Retry logic follows `PhasePolicy.retry` and `on_error`:
- `on_error: continue` → Log error, continue processing
- `on_error: end` → Fail immediately
- `on_error: retry` → Retry up to `max_attempts` with exponential backoff

### Open Questions

- Should multiple Nylon instances share the same NATS subjects, or should we namespace per environment?
  - **Current**: Subject prefix configurable per messaging config
- Do we require authentication (NKEY or JWT) in the initial release, or is anonymous `nats://` enough for dev and staging?
  - **Current**: Auth config exists but not yet enforced; TLS optional
- What back-pressure strategy do we enforce when workers are slower than Nylon (timeout, retry, drop)?
  - **Current**: Configurable via `overflow_policy: queue | reject | shed` with `max_inflight` semaphore

## Success Criteria

- 🚧 **Functional parity**: Write methods supported; read methods and WebSocket pending
- ⏳ **Performance**: p99 latency <= 8 ms and >= 30k req/s (benchmark pending)
- 🚧 **Reliability**: async-nats handles reconnect automatically; dedupe via request_id implemented
- 🚧 **Developer experience**: Config-based switching works; Go SDK transport in progress
- 🚧 **Observability**: Tracing metadata propagated; metrics hooks pending

## Rollout Checklist

- ✅ Land messaging crate and runtime integration behind the feature flag.
- ✅ Implement transport abstraction and unified session handler.
- 🚧 Complete Go SDK transport implementation.
- [ ] Enable the flag in staging, run existing regression suites plus new NATS integration tests.
- [ ] Execute load test comparing FFI versus NATS; record baseline numbers in docs.
- [ ] Implement read methods for messaging transport.
- [ ] Add metrics, health checks, and graceful shutdown.
- [ ] Update customer-facing docs and sample repositories.
- [ ] Promote the feature flag to production defaults once confidence targets are met.

## Next Immediate Steps

1. **Complete Go SDK NATS Transport** - Implement `transport_nats.go` with queue subscriptions
2. **Implement Read Methods** - Add response data flow for `GET_PAYLOAD`, `READ_REQUEST_*`, `READ_RESPONSE_*`
3. **Integration Tests** - E2E tests with actual NATS broker
4. **Metrics & Observability** - Export prometheus metrics for retries, timeouts, latency
5. **Load Testing** - Benchmark throughput and latency vs FFI baseline
6. **Documentation** - Update examples and quick start guides
