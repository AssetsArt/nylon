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

Extend Nylon's functionality with plugins:
- Request/Response filtering
- Authentication and authorization
- Custom logging and metrics
- WebSocket message handling
- Go SDK ready, more languages coming soon

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

## What Makes Nylon Different?

- **🦀 Built with Rust on Pingora** - Leverages Cloudflare's battle-tested framework for unmatched performance and reliability
- **🔌 Flexible Plugin System** - FFI-based architecture supporting multiple languages (Go SDK ready, more coming)
- **⚡️ True Zero-Downtime** - Hot reload configuration and code without dropping a single connection
- **🔒 Security First** - Automatic TLS with ACME, built-in security headers, and safe plugin isolation
- **📊 Observable by Default** - Comprehensive logging, metrics, and health checks out of the box
- **🎯 Developer Friendly** - Clean YAML config, intuitive APIs, and extensive documentation

## Next Steps

<div class="tip custom-block">
  <p class="custom-block-title">Ready to get started?</p>
  <p>Check out the <a href="/introduction/quick-start">Quick Start</a> guide to begin using Nylon.</p>
</div>

