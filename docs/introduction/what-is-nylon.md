# What is Nylon?

Nylon is a high-performance HTTP/HTTPS reverse proxy built with Rust, powered by Cloudflare's Pingora framework. It's designed to be fast, reliable, and highly extensible through its plugin system.

## Why Nylon?

### ⚡️ Performance First

Built on top of Pingora, the same technology that powers Cloudflare's edge network, Nylon delivers:
- Low latency request handling
- Efficient memory usage
- High throughput
- Connection pooling and reuse

### 🔌 Plugin Ecosystem

Extend Nylon's functionality with plugins written in Go:
- Request/Response filtering
- Authentication and authorization
- Custom logging and metrics
- WebSocket message handling
- And more...

### 🎯 Enterprise Features

- **Multiple Load Balancing Strategies**: Round Robin, Weighted, Consistent Hashing, Random
- **TLS/HTTPS Support**: Automatic certificate management with ACME (Let's Encrypt)
- **Advanced Routing**: Path-based and host-based routing with parameter extraction
- **Dynamic Configuration**: Hot-reload configuration without downtime
- **Observability**: Comprehensive logging with request/response metrics

## Use Cases

### API Gateway
Use Nylon as a centralized entry point for your microservices:
- Route requests to appropriate services
- Handle authentication and authorization
- Rate limiting and throttling
- Request/response transformation

### Load Balancer
Distribute traffic across multiple backend servers:
- Health checks
- Connection pooling
- Automatic failover
- Session persistence

### WebSocket Proxy
Proxy WebSocket connections with:
- Message filtering and transformation
- Room-based broadcasting
- Connection management

## Architecture

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │
       │ HTTP/HTTPS/WebSocket
       │
┌──────▼──────────────────────┐
│        Nylon Proxy          │
│  ┌────────────────────────┐ │
│  │   Plugin System        │ │
│  │  ┌──────────────────┐  │ │
│  │  │ Request Filter   │  │ │
│  │  │ Response Filter  │  │ │
│  │  │ Body Filter      │  │ │
│  │  │ Logging          │  │ │
│  │  └──────────────────┘  │ │
│  └────────────────────────┘ │
│  ┌────────────────────────┐ │
│  │   Routing Engine       │ │
│  └────────────────────────┘ │
│  ┌────────────────────────┐ │
│  │   Load Balancer        │ │
│  └────────────────────────┘ │
└──────┬──────────────────────┘
       │
       │ Multiple strategies
       │
┌──────▼──────┐  ┌─────────────┐  ┌─────────────┐
│  Backend 1  │  │  Backend 2  │  │  Backend 3  │
└─────────────┘  └─────────────┘  └─────────────┘
```

## Comparison

| Feature | Nylon | Nginx | Traefik | Caddy |
|---------|-------|-------|---------|-------|
| Performance | ⚡️ Excellent | ⚡️ Excellent | ⚡️ Good | ⚡️ Good |
| Plugin System | ✅ Go Plugins | ⚠️ Limited | ✅ Middleware | ✅ Modules |
| Hot Reload | ✅ Yes | ⚠️ Partial | ✅ Yes | ✅ Yes |
| Built-in ACME | ✅ Yes | ❌ No | ✅ Yes | ✅ Yes |
| WebSocket | ✅ Full Support | ✅ Full Support | ✅ Full Support | ✅ Full Support |
| Configuration | 📝 YAML | 📝 Config File | 📝 YAML/TOML | 📝 Caddyfile |

## Next Steps

<div class="tip custom-block">
  <p class="custom-block-title">Ready to get started?</p>
  <p>Check out the <a href="/introduction/quick-start">Quick Start</a> guide to begin using Nylon.</p>
</div>

