# 🛡️ Nylon: The Extensible Proxy Server

**Nylon** is a lightweight, high-performance, and extensible proxy server built on top of the robust [Cloudflare Pingora](https://github.com/cloudflare/pingora) framework. Designed for modern infrastructure, Nylon empowers you with advanced routing capabilities, seamless TLS management, versatile load balancing, and a powerful plugin system to tailor its behavior to your exact needs.

-----

## ✨ Key Features

  * **🚀 High-Performance Core:** Leverages Pingora for a fast and reliable foundation.
  * **Flexible Routing:**
      * **Path Matching:** Supports `Exact`, `Prefix`, and `Wildcard` (e.g., `/{*path}`, `/hello/{name}`) matching.
      * **Routing Types:** Implement `Host-based` and `Header-based` routing logic.
  * **🔐 TLS Management:**
      * Automatic certificate provisioning via **ACME**.
      * Support for **Custom SSL/TLS certificates**.
  * **⚖️ Load Balancing:**
      * Algorithms: `Round Robin`, `Random`, `Consistent Hashing` (Ketama), and `Weighted`.
  * **🔌 Extensible Plugin System:**
      * Interface via **FlatBuffers FFI**, compatible with Go, Rust, Zig, and other languages supporting C interop.
      * Plugins can act as **Middleware** (transforming requests/responses) or **Service Handlers** (generating full responses).
  * **📝 YAML Configuration:** Clean, intuitive, and powerful YAML-based setup.
      * Supports dynamic value templating (e.g., `${env(VAR)}`, `${uuid(v7)}`, `${request(client_ip)}`).
  * **🧩 Middleware Groups:** Define reusable sets of middleware for cleaner route configurations.
  * **📊 Observability:** (Implicit via Pingora, can be explicitly stated if Nylon adds more) logging capabilities.