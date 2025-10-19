---
layout: home

hero:
  name: Nylon
  text: High-Performance Extensible Proxy
  tagline: Built with Rust • Extended with Plugins
  image:
    src: /logo.svg
    alt: Nylon
  actions:
    - theme: brand
      text: Get Started
      link: /introduction/quick-start
    - theme: alt
      text: View on GitHub
      link: https://github.com/AssetsArt/nylon

features:
  - icon: ⚡️
    title: Blazing Fast
    details: Built on Cloudflare Pingora. Handle millions of requests per second with minimal latency.
  
  - icon: 🔌
    title: Plugin Ecosystem
    details: Extensible plugin system. Go SDK ready, more languages coming soon.
  
  - icon: 🎯
    title: Smart Routing
    details: Host and path-based routing. Parameters, rewrites, middleware chains.
  
  - icon: 🔒
    title: Automatic TLS
    details: Let's Encrypt integration. Zero-config HTTPS with automatic renewal.
  
  - icon: 🔄
    title: Load Balancing
    details: Round robin, weighted, consistent hashing, or random selection.
  
  - icon: 📊
    title: Observability
    details: Request logging, metrics, duration tracking. Production-grade monitoring.
---

<style scoped>
.install-box {
  background: var(--vp-c-bg-soft);
  border: 2px solid var(--vp-c-brand-1);
  border-radius: 16px;
  padding: 48px;
  text-align: center;
  margin: 64px auto;
  max-width: 800px;
}

.install-box h2 {
  font-size: 32px;
  margin-bottom: 24px;
  font-weight: 700;
}

.install-box div[class*="language-"] {
  margin: 24px 0;
  background: #1e293b;
}

.dark .install-box div[class*="language-"] {
  background: #0f172a;
}

.install-box .vp-code-group {
  margin: 24px 0;
}

.section-title {
  text-align: center;
  font-size: 36px;
  font-weight: 700;
  margin: 80px 0 24px 0;
}

.section-subtitle {
  text-align: center;
  font-size: 18px;
  color: var(--vp-c-text-2);
  margin-bottom: 48px;
}

.code-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  margin: 48px 0;
}

.code-card {
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 12px;
  padding: 24px;
  transition: all 0.3s ease;
}

.code-card:hover {
  border-color: var(--vp-c-brand-1);
  transform: translateY(-4px);
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.1);
}

.dark .code-card:hover {
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.4);
}

.code-card h3 {
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 16px;
}

.comparison-table {
  margin: 48px auto;
  max-width: 1000px;
  overflow-x: auto;
}

.comparison-table table {
  width: 100%;
  border-collapse: collapse;
}

.comparison-table th,
.comparison-table td {
  padding: 16px;
  text-align: left;
  border-bottom: 1px solid var(--vp-c-divider);
}

.comparison-table th {
  background: var(--vp-c-bg-soft);
  font-weight: 600;
}

.cta-section {
  text-align: center;
  padding: 80px 24px;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%);
  border-radius: 24px;
  margin: 80px auto;
  max-width: 1000px;
}

.cta-section h2 {
  font-size: 42px;
  font-weight: 700;
  margin-bottom: 24px;
}

.cta-buttons {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 32px;
}

.cta-button {
  display: inline-block;
  padding: 14px 32px;
  border-radius: 12px;
  font-size: 16px;
  font-weight: 600;
  text-decoration: none;
  transition: all 0.3s ease;
}

.cta-button.primary {
  background: var(--vp-c-brand-1);
  color: white;
}

.cta-button.primary:hover {
  background: var(--vp-c-brand-2);
  transform: translateY(-2px);
}

.cta-button.secondary {
  background: transparent;
  color: var(--vp-c-brand-1);
  border: 2px solid var(--vp-c-brand-1);
}

.cta-button.secondary:hover {
  background: var(--vp-c-brand-1);
  color: white;
}

.phase-flow {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 16px;
  flex-wrap: wrap;
  background: var(--vp-c-bg-soft);
  padding: 48px 24px;
  border-radius: 16px;
  margin: 32px 0;
}

.phase-item {
  text-align: center;
  min-width: 120px;
}

.phase-icon {
  font-size: 48px;
  margin-bottom: 8px;
}

.phase-arrow {
  font-size: 24px;
  color: var(--vp-c-brand-1);
}

@media (max-width: 768px) {
  .code-grid {
    grid-template-columns: 1fr;
  }
  
  .phase-arrow {
    display: none;
  }
}
</style>

## 🚀 Install in One Command

<div class="install-box">

```bash
curl -fsSL https://nylon.sh/install | bash
```

  <p style="color: var(--vp-c-text-2); margin-top: 16px;">
    Linux x86_64/aarch64 • Automatic detection • No dependencies
  </p>
</div>

## Start in 60 Seconds

<p class="section-subtitle">
  Two files. One command. Your reverse proxy is ready.
</p>

<div class="code-grid">
  <div class="code-card">
    <h3>📝 config.yaml</h3>

```yaml
http:
  - 0.0.0.0:8080

config_dir: "./config"

pingora:
  threads: 4
  work_stealing: true
```

  </div>

  <div class="code-card">
    <h3>🎯 config/proxy.yaml</h3>

```yaml
services:
  - name: backend
    service_type: http
    endpoints:
      - ip: 127.0.0.1
        port: 3000

routes:
  - route:
      type: host
      value: localhost
    name: api
    paths:
      - path: /*
        service:
          name: backend
```

  </div>
</div>

<div style="text-align: center; margin: 32px 0;">

```bash
nylon -c config.yaml
```

<p style="color: var(--vp-c-text-2); margin-top: 16px; font-size: 18px;">
  ✅ Your proxy is running on <code>http://localhost:8080</code>
</p>

</div>

## Add Superpowers with Plugins

<p class="section-subtitle">
  Write custom logic with our plugin SDK. Go is ready, more languages coming soon.
</p>

```go
package main

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
    plugin := sdk.NewNylonPlugin()

    plugin.AddPhaseHandler("auth", func(phase *sdk.PhaseHandler) {
        // 🔒 Request Filter - Check authentication
        phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
            if ctx.Request().Header("X-API-Key") != "secret" {
                res := ctx.Response()

                res.RemoveHeader("Content-Length")
                res.SetHeader("Transfer-Encoding", "chunked")

                res.SetStatus(401)
                res.BodyText("Unauthorized")

                // end request
                ctx.End()
                return
            }
            ctx.Next()
        })

        // 📊 Logging - Track requests
        phase.Logging(func(ctx *sdk.PhaseLogging) {
            req, res := ctx.Request(), ctx.Response()
            println(req.Method(), req.Path(), "->", res.Status(), res.Duration(), "ms")
            ctx.Next()
        })
    })
}
```

<div style="display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-top: 32px; max-width: 800px; margin-left: auto; margin-right: auto;">

```bash
# Build plugin
go build -buildmode=plugin -o auth.so
```

```yaml
# Use in config
plugins:
  - name: auth
    type: ffi
    file: ./auth.so
```

</div>

## Why Choose Nylon?

Nylon combines the best of modern proxy technology:

- **🚀 Built on Pingora** - Same technology powering Cloudflare's edge network
- **⚡️ High Performance** - Rust's memory safety without garbage collection overhead
- **🔌 Extensible** - Plugin system with Go SDK (more languages coming)
- **🔄 Zero Downtime** - Hot reload configuration without dropping connections
- **🔒 Auto HTTPS** - Built-in ACME support for Let's Encrypt
- **📊 Observability** - Comprehensive logging, metrics, and health checks

## Features

<p class="section-subtitle">Everything you need, out of the box.</p>

<div class="code-grid">
  <div class="code-card">
    <h3>🔄 Load Balancing</h3>

```yaml
services:
  - name: api
    service_type: http
    algorithm: round_robin
    endpoints:
      - ip: 10.0.0.1
        port: 3000
      - ip: 10.0.0.2
        port: 3000
      - ip: 10.0.0.3
        port: 3000
```

  </div>

  <div class="code-card">
    <h3>💚 Health Checks</h3>

```yaml
services:
  - name: api
    service_type: http
    endpoints:
      - ip: 10.0.0.1
        port: 3000
    health_check:
      enabled: true
      path: /health
      interval: 5s
```

  </div>

  <div class="code-card">
    <h3>🔒 Automatic HTTPS</h3>

```yaml
tls:
  - domains:
      - example.com
    acme:
      email: admin@example.com
      directory_url: https://acme-v02.api.letsencrypt.org/directory
```

  </div>

  <div class="code-card">
    <h3>📁 Static Files</h3>

```yaml
services:
  - name: frontend
    service_type: static
    static:
      root: ./public
      index: index.html
      spa: true
```

  </div>
</div>

## Plugin Phases

<p class="section-subtitle">Hook into any part of the request lifecycle</p>

<div class="phase-flow">
  <div class="phase-item">
    <div class="phase-icon">🌐</div>
    <strong>Client</strong>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div class="phase-icon">🔍</div>
    <strong>RequestFilter</strong>
    <div style="font-size: 12px; color: var(--vp-c-text-2);">Auth, Rate Limit</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div class="phase-icon">🖥️</div>
    <strong>Backend</strong>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div class="phase-icon">📝</div>
    <strong>ResponseFilter</strong>
    <div style="font-size: 12px; color: var(--vp-c-text-2);">Headers</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div class="phase-icon">✏️</div>
    <strong>BodyFilter</strong>
    <div style="font-size: 12px; color: var(--vp-c-text-2);">Transform</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div class="phase-icon">📊</div>
    <strong>Logging</strong>
    <div style="font-size: 12px; color: var(--vp-c-text-2);">Analytics</div>
  </div>
</div>

<p style="text-align: center; margin-top: 32px;">
  <a href="/plugins/phases" style="color: var(--vp-c-brand-1); font-weight: 600; font-size: 18px;">
    Learn More About Plugin Phases →
  </a>
</p>

## Ready to Get Started?

<div class="cta-section">
  <h2>Build Your Proxy in Minutes</h2>
  <p style="font-size: 20px; color: var(--vp-c-text-2); margin-bottom: 32px;">
    Install Nylon and start proxying requests right away
  </p>
  
  <div style="max-width: 600px; margin: 0 auto;">

```bash
curl -fsSL https://nylon.sh/install | bash
```

  </div>

  <div class="cta-buttons">
    <a href="/introduction/installation" class="cta-button primary">
      📥 Installation Guide
    </a>
    <a href="/introduction/quick-start" class="cta-button secondary">
      🚀 Quick Start
    </a>
    <a href="/examples/basic-proxy" class="cta-button secondary">
      📖 Examples
    </a>
  </div>
</div>
