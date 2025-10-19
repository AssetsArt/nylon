# 🧬 Nylon — The Extensible Proxy Server

[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://nylon.sh/)

Nylon is a lightweight, high‑performance, and extensible proxy built on top of the battle‑tested [Cloudflare Pingora](https://github.com/cloudflare/pingora) framework.

## ✨ Features

- **🔌 Extensible**: Write plugins in Go, Rust, Zig, C via FFI
- **📝 Simple YAML Config**: One place to manage routes, services, middleware
- **🎯 Smart Routing**: Host/header/path matching with multiple load balancing algorithms
- **🔒 TLS Built-in**: Custom certificates or ACME (Let's Encrypt, Buypass)
- **☁️ Cloud-native**: Observability and scalability friendly
- **⚡ High Performance**: Built on Cloudflare Pingora framework

## 🚀 Quick Start

```sh
# Install (macOS/Linux)
curl -fsSL https://nylon.sh/install | sh

# Run with example config
nylon run -c ./examples/config.yaml
```

Test it:
```sh
curl http://127.0.0.1:8088/
```

## 📖 Documentation

For complete documentation, visit **[nylon.sh](https://nylon.sh/)**

- [Installation Guide](https://nylon.sh/introduction/installation)
- [Quick Start](https://nylon.sh/introduction/quick-start)
- [Configuration](https://nylon.sh/core/configuration)
- [Routing & Load Balancing](https://nylon.sh/core/routing)
- [TLS Setup](https://nylon.sh/core/tls)
- [Plugin System](https://nylon.sh/plugins/overview)
- [Examples](https://nylon.sh/examples/basic-proxy)

## 🛠️ Build from Source

```sh
git clone https://github.com/AssetsArt/nylon.git
cd nylon
make build
./target/release/nylon run -c ./examples/config.yaml
```

## 🔗 Links

- 📚 Documentation: [nylon.sh](https://nylon.sh/)
- 🐛 Issues: [GitHub Issues](https://github.com/AssetsArt/nylon/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/AssetsArt/nylon/discussions)

## 📄 License

MIT Licensed. © AssetsArt.
