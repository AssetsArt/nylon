# Nylon Service Installation Guide

## Quick Install

```bash
# Build
cargo build --release

# Install service (requires sudo)
sudo ./target/release/nylon service install

# Start service
sudo ./target/release/nylon service start
```

## What Gets Created

When you run `nylon service install`, the following files and directories are automatically created:

```
/etc/nylon/
├── config.yaml                 # Main configuration
├── proxy/
│   └── base.yaml              # Proxy routing configuration
├── static/
│   └── index.html             # Welcome page (served at /static/)
└── acme/                      # SSL certificates directory
```

### Generated Configuration

**Main Config** (`/etc/nylon/config.yaml`):
- HTTP server on port 8088
- HTTPS server on port 8443
- Metrics on port 6192
- Points to `/etc/nylon/proxy` for proxy configs
- ACME directory for SSL certificates

**Proxy Config** (`/etc/nylon/proxy/base.yaml`):
- Example HTTP service (app-service) with health checks
- Static file service configured
- Security middleware (headers, request ID, etc.)
- Routes for static files and main app

**Static Files** (`/etc/nylon/static/index.html`):
- Beautiful welcome page
- Confirms service is running
- Modern, responsive design

## Service Commands

```bash
# Install and create default configs
sudo nylon service install

# Start the service
sudo nylon service start

# Check status
sudo nylon service status

# Stop the service
sudo nylon service stop

# Restart the service
sudo nylon service restart

# Reload configuration (no downtime)
sudo nylon service reload

# Uninstall service
sudo nylon service uninstall
```

## Testing Your Installation

### 1. Check Service Status

```bash
sudo nylon service status
```

### 2. Test Static Page

```bash
curl http://localhost:8088
```
