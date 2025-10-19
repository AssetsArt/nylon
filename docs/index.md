---
layout: home

hero:
  name: Nylon
  text: Modern Edge Proxy for Programmable Infrastructure
  tagline: Built in Rust • Powered by Pingora • Extensible with Plugins
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
    title: High-Performance Core
    details: Pingora’s async engine delivers Cloudflare-grade latency and throughput.
  - icon: 🧩
    title: Programmable at Every Phase
    details: Drop in Go plugins today—Rust/Zig/other FFI bindings on the roadmap.
  - icon: 🎯
    title: Declarative Control Plane
    details: Human-friendly YAML with hot reloads, templating, and routing logic that scales.
  - icon: 🔐
    title: Automatic TLS
    details: Built-in ACME automation, health checks, structured logs, and zero downtime reloads.
---

<style scoped>
:root {
  --ny-card-shadow: 0 18px 32px rgba(15, 23, 42, 0.08);
}

.page-intro {
  text-align: center;
  max-width: 760px;
  margin: 0 auto 48px;
  font-size: 20px;
  color: var(--vp-c-text-2);
}

.quickstart-grid,
.capabilities-grid,
.usecases-grid,
.ops-grid {
  display: grid;
  gap: 24px;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  margin: 32px 0 48px;
}

.card {
  background: var(--vp-c-bg-soft);
  border: 1px solid var(--vp-c-divider);
  border-radius: 16px;
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  transition: border-color 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
}

.card:hover {
  border-color: var(--vp-c-brand-1);
  transform: translateY(-4px);
  box-shadow: var(--ny-card-shadow);
}

.card h3 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.section-title {
  text-align: center;
  font-size: 32px;
  font-weight: 700;
  margin: 72px 0 12px;
}

.section-subtitle {
  text-align: center;
  font-size: 18px;
  color: var(--vp-c-text-2);
  margin-bottom: 32px;
}

.shell-callout {
  background: var(--vp-c-bg-soft);
  border-radius: 18px;
  border: 2px solid var(--vp-c-brand-1);
  padding: 40px;
  text-align: center;
  margin: 48px auto 60px;
  max-width: 780px;
}

.shell-callout div[class*="language-"] {
  background: #111827;
  margin: 24px 0;
}

.highlight-points {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: 16px;
}

.highlight-points li {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 16px;
}

.phase-flow {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 16px;
  padding: 32px 24px;
  border-radius: 18px;
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg-soft);
  margin: 24px 0 48px;
}

.phase-item {
  text-align: center;
  min-width: 120px;
}

.phase-icon {
  font-size: 44px;
  margin-bottom: 8px;
}

.phase-arrow {
  font-size: 24px;
  color: var(--vp-c-brand-1);
  align-self: center;
}

.cta-section {
  text-align: center;
  padding: 64px 24px;
  border-radius: 24px;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.12), rgba(139, 92, 246, 0.12));
  margin: 72px auto;
  max-width: 980px;
}

.cta-buttons {
  display: flex;
  gap: 16px;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 24px;
}

.cta-button {
  display: inline-block;
  padding: 12px 28px;
  border-radius: 12px;
  font-size: 16px;
  font-weight: 600;
  text-decoration: none;
  transition: transform 0.2s ease, background 0.2s ease, color 0.2s ease;
}

.cta-button.primary {
  background: var(--vp-c-brand-1);
  color: #fff;
}

.cta-button.primary:hover {
  transform: translateY(-2px);
  background: var(--vp-c-brand-2);
}

.cta-button.secondary {
  border: 2px solid var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}

.cta-button.secondary:hover {
  transform: translateY(-2px);
  background: var(--vp-c-brand-1);
  color: #fff;
}

@media (max-width: 768px) {
  .shell-callout {
    padding: 32px 20px;
  }

  .phase-arrow {
    display: none;
  }
}
</style>

## Quick start

<div class="shell-callout">

```bash
curl -fsSL https://nylon.sh/install | bash
```

  <p style="color: var(--vp-c-text-2); margin-top: 8px;">
    One binary • Linux aarch64 / x86_64 • No external dependencies
  </p>
</div>

## Configure in minutes

<div class="quickstart-grid">
  <div class="card">
    <h3>1. Define runtime</h3>
    <p>Create <code>config.yaml</code> for listeners, Pingora tuning, and ACME storage.</p>

```yaml
http:
  - 0.0.0.0:8080
config_dir: ./config
pingora:
  daemon: false
  threads: 4
```

  </div>
  <div class="card">
    <h3>2. Add services & routes</h3>
    <p>Describe backends, plugins, and routing rules inside <code>config/</code>.</p>

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
    paths:
      - path: /*
        service:
          name: backend
```

  </div>
  <div class="card">
    <h3>3. Run it</h3>
    <p>Bring everything online and hot reload as you iterate.</p>

```bash
nylon run -c config.yaml
```

  </div>
</div>

## Built for real workloads

<div class="usecases-grid">
  <div class="card">
    <h3>API gateways</h3>
    <p>Inject auth, rate limiting, caching, and observability logic without touching upstream services.</p>
  </div>
  <div class="card">
    <h3>Edge platforms & SaaS</h3>
    <p>Serve thousands of domains with automated TLS, granular routing, and templated configs.</p>
  </div>
  <div class="card">
    <h3>Real-time applications</h3>
    <p>Harness WebSocket callbacks, broadcast rooms, and streaming response filters.</p>
  </div>
  <div class="card">
    <h3>Migration buffers</h3>
    <p>Rewrite payloads, split traffic, and de-risk backend migrations with per-route middleware.</p>
  </div>
</div>

## Programmable pipeline

<div class="phase-flow">
  <div class="phase-item">
    <div>Request Filter</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div>Routing</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div>Response Filter</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div>Body Filter</div>
  </div>
  <div class="phase-arrow">→</div>
  <div class="phase-item">
    <div>Logging</div>
  </div>
</div>

```go
plugin.AddPhaseHandler("authz", func(phase *sdk.PhaseHandler) {
  phase.RequestFilter(func(ctx *sdk.PhaseRequestFilter) {
    if ctx.Request().Header("X-API-Key") != ctx.GetPayload()["api_key"] {
      res := ctx.Response()
      res.RemoveHeader("Content-Length")
      res.SetHeader("Transfer-Encoding", "chunked")
      res.SetStatus(401)
      res.BodyText("Unauthorized")
      ctx.End()
      return
    }
    ctx.Next()
  })

  phase.Logging(func(ctx *sdk.PhaseLogging) {
    req, res := ctx.Request(), ctx.Response()
    log.Printf("%s %s -> %d (%dms)", req.Method(), req.Path(), res.Status(), res.Duration())
    ctx.Next()
  })
})
```

> Go SDK is available today. Additional language SDKs (Rust, Zig, …) are in active development.

## Operational muscle

<div class="ops-grid">
  <div class="card">
    <h3>Hot reloads & zero downtime</h3>

```bash
# Update configs without dropping connections
nylon service reload
```

  </div>
  <div class="card">
    <h3>Automated certificates</h3>

```yaml
tls:
  - type: acme
    provider: letsencrypt
    domains:
      - api.example.com
      - app.example.com
    acme:
      email: ops@example.com
```

  </div>
  <div class="card">
    <h3>Template everything</h3>

```yaml
middleware:
  - plugin: RequestHeaderModifier
    payload:
      set:
        - name: x-request-id
          value: "${uuid(v7)}"
        - name: x-forwarded-for
          value: "${request(client_ip)}"
```

  </div>
</div>

## Ready to build?

<div class="cta-section">
  <h2>Ship faster with Nylon</h2>
  <p style="font-size: 18px; color: var(--vp-c-text-2);">
    The programmable proxy your edge, SaaS, and real-time workloads deserve.
  </p>
  <div class="cta-buttons">
    <a class="cta-button primary" href="/introduction/quick-start">Launch Quick Start</a>
    <a class="cta-button secondary" href="https://github.com/AssetsArt/nylon">Star on GitHub</a>
  </div>
</div>
