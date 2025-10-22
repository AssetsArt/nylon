# Nylon NATS Plugin Implementation Roadmap

## Objective

- Deliver a production-ready NATS-backed plugin transport that mirrors the behaviour of today's FFI plugins.
- Keep the Go SDK API stable while letting plugin authors opt-in to horizontal scaling through configuration.
- Provide the operational guardrails (metrics, logging, failure handling) needed for staging and production rollouts.

## Current Status (May 2024)

- Nylon core only understands `PluginType::Ffi`; the loader path in `crates/nylon-plugin` has no awareness of messaging transports.
- There is no shared messaging crate; NATS connectivity, pooling and protocol handling do not exist.
- `nylon-types` and `nylon-config` cannot parse or validate messaging config blocks.
- The Go SDK exposes only FFI constructors; there is no transport abstraction to plug in a NATS backend.
- Documentation (for example `docs/NATS_PLUGIN_QUICK_START.md`) references `NewNatsPlugin`, but the symbol is not implemented yet.
- No automated tests cover request/response over a message broker, and we have no load or failure scenarios to measure parity.

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

## Milestone 0 - Repository Preparation (0.5 day)

- [ ] Add `crates/nylon-messaging` to the workspace and share crate-level lint configuration.
- [ ] Decide on MessagePack (`rmp-serde`) as the canonical encoding and document versioning policy.
- [ ] Add a `scripts/dev-nats.sh` helper that starts a local NATS server (docker-compose or `nats-server` binary).
- [ ] Gate the new code behind a `messaging` cargo feature flag so we can merge incrementally.

## Milestone 1 - Messaging Foundation (Rust)

`crates/nylon-messaging/`
- [ ] Add dependencies: `async-nats = "0.33"`, `tokio` (full), `rmp-serde`, `serde`, `tracing`.
- [ ] Implement `NatsClient::connect(servers: &[String], options: NatsClientOptions)` returning `Arc<NatsClient>`.
- [ ] Provide `request(subject, payload)` and `publish(subject, payload)` helpers with timeout handling and trace header propagation.
- [ ] Generate a `request_id` (or accept caller-provided) and attach to each outbound message; export dedupe helpers for Nylon core.
- [ ] Add queue subscription helper for worker-side consumption (`subscribe_with_group`).
- [ ] Implement automatic reconnect and health probes (`check_connection`, `on_connection_lost`) with `max_inflight` accounting.
- [ ] Define `protocol.rs` with `PluginRequest`, `PluginResponse`, `ResponseAction` enums matching plugin phases, including schema `version` and `request_id`.
- [ ] Add serialization utilities (`to_bytes`, `from_bytes`) plus integration tests that spin up an in-process NATS server and assert version interoperability.
- [ ] Expose metrics hooks (`messaging_requests_total`, `messaging_request_duration_seconds`, `messaging_retries_total`, `messaging_timeouts_total`) via callbacks.

## Milestone 2 - Runtime Integration (Rust)

`crates/nylon-types/src/plugins.rs`
- [ ] Extend `PluginType` with `Messaging`.
- [ ] Introduce `MessagingConfig` and `MessagingPluginRef` structs with serde support for YAML/TOML.

`crates/nylon-config/src/plugins.rs`
- [ ] Parse the `messaging` block, validate URLs, authentication and default queue groups.
- [ ] Store messaging configs in `nylon-store` for fast lookup during plugin load.

`crates/nylon-plugin/src`
- [ ] Create `messaging.rs` implementing `MessagingPlugin` that wraps `Arc<NatsClient>` plus metadata.
- [ ] Update `plugin_manager.rs` so `get_plugin` returns either `FfiPlugin` or `MessagingPlugin` via an enum or trait.
- [ ] Extend `stream.rs` and `session_handler.rs` with a `SessionTransport` trait so phases call into NATS transparently.
- [ ] Implement request/reply flow: construct subject `nylon.{env}.{plugin}.{phase}`, attach inbox for replies, await `PluginResponse`, enforce `max_inflight` and `overflow_policy`.
- [ ] Handle `ResponseAction::{Next,End,Error}` identically to the FFI pipeline, including session cleanup and dedupe via `request_id` cache.
- [ ] Add graceful shutdown: draining subscriptions and closing NATS connections with a timeout while reporting inflight counts and outstanding retries.
- [ ] Surface tracing spans (phase, method, session) and translate messaging errors into `NylonError` variants.

## Milestone 3 - Go SDK Transport

`sdk/go/sdk`
- [ ] Introduce `PluginTransport` interface and refactor the current FFI implementation into `transport_ffi.go`.
- [ ] Implement `transport_nats.go` using `github.com/nats-io/nats.go` with connection pooling, queue subscriptions, and concurrency/timeout controls.
- [ ] Add `nats_plugin.go` that exposes `NewNatsPlugin(config)` returning a struct satisfying the existing plugin API and surfacing idempotency info (`ctx.IdempotencyKey()`).
- [ ] Reuse existing `PhaseHandler`, `PhaseRequestFilter`, and related types by translating `PluginRequest` into current structs.
- [ ] Support synchronous replies: after a user handler calls `ctx.Next`, `ctx.End`, or `ctx.Error`, marshal `PluginResponse` with `version`, `request_id`, and optional headers, then `msg.Respond`.
- [ ] Provide lifecycle hooks (`Initialize`, `Shutdown`, `Close`) that mirror FFI behaviour, drain subscriptions, and honour `MaxHandlers`.
- [ ] Add unit tests with a local NATS server using `go test ./sdk/...`.

## Milestone 4 - Configuration and CLI Wiring

- [ ] Update CLI entrypoints to load messaging configs before plugin registration (for example `crates/nylon/src/main.rs`).
- [ ] Support overriding NATS servers via environment variables such as `NYLON_NATS_URLS` for container deployments.
- [ ] Validate at startup that every `type: messaging` plugin references an existing messaging config and queue group.
- [ ] Emit clear diagnostics when NATS connection fails, including actionable hints in error messages.

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

## Open Questions

- Should multiple Nylon instances share the same NATS subjects, or should we namespace per environment?
- Do we require authentication (NKEY or JWT) in the initial release, or is anonymous `nats://` enough for dev and staging?
- What back-pressure strategy do we enforce when workers are slower than Nylon (timeout, retry, drop)?

## Success Criteria

- ✅ Functional parity: all four plugin phases supported with identical semantics to FFI.
- ✅ Performance: p99 latency <= 8 ms and >= 30k req/s with 10 Go workers on a single Nylon node.
- ✅ Reliability: automatic reconnect within 5 seconds, no message loss under worker crash or NATS restart.
- ✅ Developer experience: existing plugin code compiles with only the constructor or config change (<10 LOC diff) while surfacing idempotency helpers.
- ✅ Observability: metrics and logs allow operators to pinpoint slow or failing workers quickly, with alerting on retries/timeouts.

## Rollout Checklist

- [ ] Land messaging crate and runtime integration behind the feature flag.
- [ ] Enable the flag in staging, run existing regression suites plus new NATS integration tests.
- [ ] Execute load test comparing FFI versus NATS; record baseline numbers in docs.
- [ ] Update customer-facing docs and sample repositories.
- [ ] Promote the feature flag to production defaults once confidence targets are met.
