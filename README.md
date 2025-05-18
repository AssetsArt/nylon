## 🛡️ Nylon

**Nylon** is a lightweight, extensible proxy server built on top of [Cloudflare's Pingora](https://github.com/cloudflare/pingora). Designed for modern infrastructure, it enables advanced routing, TLS management, and load balancing with minimal configuration.

### 📦 Configuration Overview

Nylon uses a YAML-based config to define global settings, services, TLS, and routes. It supports:

- **Path matching**: Exact & Prefix
- **Routing types**: Header-based, Host-based
- **TLS**: ACME & Custom certs
- **Load Balancing**: Round robin, Random, Consistent Hashing (Weighted Ketama)
- **FFI**: written in **Go**, **Rust**, **Zig**, or any language that can compile to native libraries.

Full examples available in the [documentation](https://nylon.assetsart.com).

---
