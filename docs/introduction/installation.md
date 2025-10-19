# Installation

Install Nylon in multiple ways depending on your needs.

## Quick Install (Linux)

The fastest way to install Nylon on Linux:

```bash
curl -fsSL https://nylon.sh/install | bash
```

This script will:
- Detect your OS and architecture (Linux x86_64/aarch64)
- Detect libc variant (glibc/musl)
- Download the latest release binary
- Verify checksums
- Install to `/usr/local/bin/nylon` (or `~/.local/bin/nylon` if no sudo)

### Supported Platforms

- **Linux**: x86_64 and aarch64
- **Libc**: GNU libc and musl
- **macOS**: Not yet available (build from source)

### Installation Locations

The installer will install to:
- `/usr/local/bin/nylon` - if you have write permission or use sudo
- `~/.local/bin/nylon` - if no write permission to system directories

### Verify Installation

```bash
nylon --version
```

## From Source

Build Nylon from source for maximum compatibility or development:

### Prerequisites

- **Rust**: 1.70 or later ([install](https://rustup.rs/))
- **Git**: To clone the repository

### Build Steps

```bash
# Clone the repository
git clone https://github.com/AssetsArt/nylon.git
cd nylon

# Build release binary
cargo build --release

# Binary will be at target/release/nylon
./target/release/nylon --version

# Install to system (optional)
sudo cp target/release/nylon /usr/local/bin/
```

### Build Options

**Optimized build (faster binary):**
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

**Smaller binary (strip symbols):**
```bash
cargo build --release
strip target/release/nylon
```

## Docker

Run Nylon in a container:

```bash
# Pull image
docker pull ghcr.io/assetsart/nylon:latest

# Run with config
docker run -d \
  --name nylon \
  -p 80:8080 \
  -p 443:8443 \
  -v $(pwd)/config.yaml:/etc/nylon/config.yaml \
  -v $(pwd)/config:/etc/nylon/config \
  ghcr.io/assetsart/nylon:latest
```

### Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  nylon:
    image: ghcr.io/assetsart/nylon:latest
    ports:
      - "80:8080"
      - "443:8443"
      - "6192:6192"  # metrics
    volumes:
      - ./config.yaml:/etc/nylon/config.yaml
      - ./config:/etc/nylon/config
      - ./acme:/etc/nylon/acme
    restart: unless-stopped
```

Run it:
```bash
docker-compose up -d
```

## System Service (Linux)

Install Nylon as a systemd service with automatic configuration:

### Install Service

```bash
# Install service and create default configs
sudo nylon service install
```

This creates:
- `/etc/nylon/config.yaml` - Runtime configuration
- `/etc/nylon/config/base.yaml` - Proxy configuration
- `/etc/nylon/static/index.html` - Welcome page
- `/etc/nylon/acme/` - Certificate directory
- `/etc/systemd/system/nylon.service` - Systemd unit

### Service Management

```bash
# Start service
sudo nylon service start

# Check status
sudo nylon service status

# Stop service
sudo nylon service stop

# Restart service
sudo nylon service restart

# Reload config (zero downtime)
sudo nylon service reload

# Uninstall service
sudo nylon service uninstall
```

### Verify Service

```bash
# Check status
sudo systemctl status nylon

# View logs
sudo journalctl -u nylon -f

# Test endpoint
curl http://localhost:8088
```

## Verify Installation

After installation, verify Nylon is working:

```bash
# Check version
nylon --version

# Display help
nylon --help

# Test configuration
nylon run -c config.yaml
```

## Go SDK for Plugin Development

If you want to develop Go plugins, install the SDK:

```bash
# Add to your Go project
go get github.com/AssetsArt/nylon/sdk/go/sdk
```

Create `plugin.go`:
```go
package main

import "C"
import sdk "github.com/AssetsArt/nylon/sdk/go/sdk"

func main() {}

func init() {
    plugin := sdk.NewNylonPlugin()
    // Your plugin code here
}
```

Build plugin:
```bash
go build -buildmode=plugin -o myplugin.so plugin.go
```

## Troubleshooting

### Linux: Binary not in PATH

If installed to `~/.local/bin`, add to your shell profile:

**Bash** (`~/.bashrc`):
```bash
export PATH="$PATH:$HOME/.local/bin"
```

**Zsh** (`~/.zshrc`):
```bash
export PATH="$PATH:$HOME/.local/bin"
```

Then reload:
```bash
source ~/.bashrc  # or ~/.zshrc
```

### Permission Denied

If you get permission errors:

```bash
# Install to user directory
curl -fsSL https://nylon.sh/install | bash

# Or use sudo for system-wide install
curl -fsSL https://nylon.sh/install | sudo bash
```

### macOS: Build from Source

macOS binaries are not available yet. Build from source:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/AssetsArt/nylon.git
cd nylon
cargo build --release

# Install
sudo cp target/release/nylon /usr/local/bin/
```

### Checksum Verification Failed

If checksum verification fails, try:

```bash
# Download manually
VERSION=$(curl -s https://api.github.com/repos/AssetsArt/nylon/releases/latest | grep tag_name | cut -d '"' -f 4)
curl -LO "https://github.com/AssetsArt/nylon/releases/download/${VERSION}/nylon-x86_64-linux-gnu"

# Verify checksum
curl -LO "https://github.com/AssetsArt/nylon/releases/download/${VERSION}/linux-checksums.txt"
shasum -a 256 -c linux-checksums.txt

# Install
chmod +x nylon-x86_64-linux-gnu
sudo mv nylon-x86_64-linux-gnu /usr/local/bin/nylon
```

## Upgrade

### Using Install Script

```bash
# Re-run install script to get latest version
curl -fsSL https://nylon.sh/install | bash
```

### Manual Upgrade

```bash
# Build latest from source
cd nylon
git pull origin main
cargo build --release
sudo cp target/release/nylon /usr/local/bin/

# Restart service if installed
sudo nylon service restart
```

## Uninstall

### Remove Binary

```bash
# System-wide
sudo rm /usr/local/bin/nylon

# User install
rm ~/.local/bin/nylon
```

### Remove Service

```bash
# Uninstall systemd service
sudo nylon service uninstall

# Remove configs (optional)
sudo rm -rf /etc/nylon
```

### Docker

```bash
# Stop and remove container
docker stop nylon
docker rm nylon

# Remove image
docker rmi ghcr.io/assetsart/nylon:latest
```

## Next Steps

- [Quick Start](/introduction/quick-start) - Get started with Nylon
- [Configuration](/core/configuration) - Configure Nylon
- [Plugin Development](/plugins/overview) - Extend with plugins
