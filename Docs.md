# Project Documentation

## Overview
Nylon is a lightweight and extensible proxy server built on Cloudflare's Pingora framework. The repository is organized as a Rust workspace with multiple crates implementing command parsing, configuration, runtime server logic, plugin management, and an in-memory store.

## Workspace Layout
- `crates/nylon` – executable proxy server. Handles CLI commands, loads configuration, and starts the runtime server.
- `crates/nylon-command` – defines CLI structure for running the server or managing it as a service.
- `crates/nylon-config` – parses and validates YAML configuration for runtime settings, services, routes, plugins, and middleware.
- `crates/nylon-plugin` – plugin system supporting FFI and built‑in middleware with stream handling and WebSocket support.
- `crates/nylon-store` – global in‑memory store backed by `DashMap` to share runtime data such as backends, routes, TLS certificates, and plugin instances.
- `sdk/rust` – Rust SDK and FlatBuffers definitions for building plugins.

## Runtime Flow
1. `main` parses CLI arguments and loads runtime and proxy configuration files.
2. Configuration is validated and persisted into the global store.
3. A Pingora server is created with HTTP/HTTPS listeners, optional TLS, and a background service.
4. Each incoming request is matched against routes, balanced across backends, and processed through middleware or plugins.

## Key Components
### Configuration
`RuntimeConfig` defines listening addresses and Pingora runtime options, while `ProxyConfigExt` merges multiple YAML files, validates uniqueness of routes/services, and stores TLS, load balancer backends, routes, and plugins in the store.

### Plugin System
Plugins are loaded dynamically and executed through session streams. The system manages opening sessions, handling plugin events, and relaying WebSocket frames between the client and plugin.

### Global Store
Provides typed insertion and retrieval APIs to share state across crates. It stores constants such as header selector keys, runtime config, load balancer backends, routes, TLS data, and plugin registries.

## Additional Notes
- Background service handles plugin shutdown and periodic maintenance tasks like certificate checks.
- Example plugins and SDKs exist in `examples` and `sdk` directories for languages like Go and Rust.

