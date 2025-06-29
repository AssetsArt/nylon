# 🧬 Nylon: The Extensible Proxy Server

[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-online-blue)](https://nylon.sh/)

**Nylon** is a lightweight, high-performance, and extensible proxy server built on top of the robust [Cloudflare Pingora](https://blog.cloudflare.com/introducing-pingora/) framework. Designed for modern infrastructure.

---

## 🚀 Overview

- **Extensible**: Write plugins in Go, Rust, Zig, C, and more. Extend routing, filtering, and business logic without patching the core.
- **Modern Configuration**: Manage everything with a single, declarative YAML file. GitOps-friendly.
- **Advanced Routing & Load Balancing**: Route by host, header, path (wildcard support), and balance traffic with round robin, random, or consistent hashing.
- **Automatic TLS Management**: ACME (Let's Encrypt, Buypass, etc.) and custom certs supported.
- **Cloud-Native**: Designed for scale, reliability, and observability.

---

## 🛠️ Quick Start

```sh
# Download or build Nylon binary (see Releases or build instructions below)
nylon run -c config.yaml
````

See the [Getting Started Guide](https://nylon.sh/getting-started/installation) for detailed setup.

---

## 🧩 Extending Nylon

Nylon features a **powerful plugin system** — use any language with FFI.

**Example: Minimal Go Middleware Plugin**

```go
plugin := sdk.NewNylonPlugin()
// Register middleware
plugin.HttpPlugin("authz", func(ctx *sdk.NylonHttpPluginCtx) {
	// fmt.Println("authz")
	// fmt.Println("Ctx", ctx)
	// payload := ctx.GetPayload()
	// fmt.Println("Payload", payload)
	// set headers
	ctx.Response().SetHeader("x-test", "test")
	ctx.Response().SetHeader("Transfer-Encoding", "chunked")
	// set Basic Auth
	// ctx.Response().SetHeader("WWW-Authenticate", "Basic realm=\"Restricted\"")
	// remove  headers
	ctx.Response().RemoveHeader("Content-Type")
	ctx.Response().RemoveHeader("Content-Length")
	// set status
	// ctx.Response().SetStatus(401)
	// sleep 3 second
	// time.Sleep(3 * time.Second)
	// next middleware
	ctx.Next()
})
```

> See [plugin docs](https://nylon.sh/plugin-system/go) and [real-world examples](https://github.com/AssetsArt/nylon/tree/main/examples/go)

## 📚 Documentation

* **[nylon.sh](https://nylon.sh/)** — Full documentation & guides
* **[Getting Started](https://nylon.sh/getting-started/installation)**
* **[Plugin System](https://nylon.sh/plugin-system)**
* **[Config Reference](https://nylon.sh/config-reference)**

---

## 📦 Building from Source

```sh
git clone https://github.com/AssetsArt/nylon.git
cd nylon
make build
```

---

Nylon is an open-source project by [AssetsArt](https://github.com/AssetsArt).
