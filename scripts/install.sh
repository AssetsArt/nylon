#!/usr/bin/env bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
GITHUB_REPO="AssetsArt/nylon"
BINARY_NAME="nylon"
INSTALL_DIR="/usr/local/bin"

# Print colored message
print_message() {
    local color=$1
    shift
    echo -e "${color}$@${NC}"
}

print_info() {
    print_message "$BLUE" "ℹ️  $@"
}

print_success() {
    print_message "$GREEN" "✅ $@"
}

print_warning() {
    print_message "$YELLOW" "⚠️  $@"
}

print_error() {
    print_message "$RED" "❌ $@"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)
            echo "linux"
            ;;
        Darwin*)
            echo "darwin"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Detect libc variant (gnu or musl)
detect_libc() {
    if command_exists ldd; then
        if ldd --version 2>&1 | grep -q musl; then
            echo "musl"
        else
            echo "gnu"
        fi
    else
        # Default to musl if ldd not found (safer for static linking)
        echo "musl"
    fi
}

# Get latest version from GitHub
get_latest_version() {
    if command_exists curl; then
        curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | \
            grep '"tag_name":' | \
            sed -E 's/.*"v([^"]+)".*/\1/'
    else
        print_error "curl is required but not installed"
        exit 1
    fi
}

# Download file
download_file() {
    local url=$1
    local output=$2
    
    if command_exists curl; then
        curl -fsSL -o "$output" "$url"
    elif command_exists wget; then
        wget -q -O "$output" "$url"
    else
        print_error "curl or wget is required but neither is installed"
        exit 1
    fi
}

# Verify checksum
verify_checksum() {
    local file=$1
    local checksum_file=$2
    local binary_name=$3
    
    if command_exists shasum; then
        local expected_checksum=$(grep "$binary_name" "$checksum_file" | awk '{print $1}')
        local actual_checksum=$(shasum -a 256 "$file" | awk '{print $1}')
        
        if [ "$expected_checksum" = "$actual_checksum" ]; then
            return 0
        else
            return 1
        fi
    else
        print_warning "shasum not found, skipping checksum verification"
        return 0
    fi
}

# Main installation
main() {
    print_info "🚀 Nylon Proxy Installer"
    echo ""
    
    # Check required commands
    if ! command_exists curl && ! command_exists wget; then
        print_error "curl or wget is required for installation"
        exit 1
    fi
    
    # Detect system
    print_info "Detecting system information..."
    OS=$(detect_os)
    ARCH=$(detect_arch)
    
    if [ "$OS" = "unknown" ]; then
        print_error "Unsupported operating system: $(uname -s)"
        exit 1
    fi
    
    if [ "$ARCH" = "unknown" ]; then
        print_error "Unsupported architecture: $(uname -m)"
        exit 1
    fi
    
    print_success "OS: $OS, Architecture: $ARCH"
    
    # Check if macOS
    if [ "$OS" = "darwin" ]; then
        print_error "macOS binaries are not available yet."
        print_info "Please build from source:"
        echo ""
        echo "  git clone https://github.com/${GITHUB_REPO}.git"
        echo "  cd nylon"
        echo "  cargo build --release"
        echo "  sudo cp target/release/nylon /usr/local/bin/"
        echo ""
        exit 1
    fi
    
    # Detect libc for Linux
    LIBC=$(detect_libc)
    print_success "Libc: $LIBC"
    
    # Construct binary name
    BINARY_VARIANT="${BINARY_NAME}-${ARCH}-linux-${LIBC}"
    print_info "Binary variant: $BINARY_VARIANT"
    
    # Get latest version
    print_info "Fetching latest version..."
    VERSION=$(get_latest_version)
    
    if [ -z "$VERSION" ]; then
        print_error "Failed to fetch latest version"
        exit 1
    fi
    
    print_success "Latest version: v${VERSION}"
    
    # Construct download URLs
    BASE_URL="https://github.com/${GITHUB_REPO}/releases/download/v${VERSION}"
    BINARY_URL="${BASE_URL}/${BINARY_VARIANT}"
    CHECKSUM_URL="${BASE_URL}/linux-checksums.txt"
    
    # Create temp directory
    TMP_DIR=$(mktemp -d)
    trap "rm -rf $TMP_DIR" EXIT
    
    print_info "Downloading ${BINARY_VARIANT}..."
    if ! download_file "$BINARY_URL" "$TMP_DIR/$BINARY_VARIANT"; then
        print_error "Failed to download binary"
        exit 1
    fi
    print_success "Downloaded binary"
    
    print_info "Downloading checksums..."
    if ! download_file "$CHECKSUM_URL" "$TMP_DIR/checksums.txt"; then
        print_warning "Failed to download checksums, skipping verification"
    else
        print_info "Verifying checksum..."
        if verify_checksum "$TMP_DIR/$BINARY_VARIANT" "$TMP_DIR/checksums.txt" "$BINARY_VARIANT"; then
            print_success "Checksum verified"
        else
            print_error "Checksum verification failed"
            exit 1
        fi
    fi
    
    # Determine install directory
    if [ -w "$INSTALL_DIR" ]; then
        FINAL_INSTALL_DIR="$INSTALL_DIR"
    elif [ "$EUID" -eq 0 ] || [ "$(id -u)" -eq 0 ]; then
        FINAL_INSTALL_DIR="$INSTALL_DIR"
    else
        FINAL_INSTALL_DIR="$HOME/.local/bin"
        print_warning "No write permission to $INSTALL_DIR, installing to $FINAL_INSTALL_DIR"
        mkdir -p "$FINAL_INSTALL_DIR"
    fi
    
    # Install binary
    print_info "Installing to ${FINAL_INSTALL_DIR}/${BINARY_NAME}..."
    
    if [ -w "$FINAL_INSTALL_DIR" ]; then
        mv "$TMP_DIR/$BINARY_VARIANT" "$FINAL_INSTALL_DIR/$BINARY_NAME"
        chmod +x "$FINAL_INSTALL_DIR/$BINARY_NAME"
    else
        sudo mv "$TMP_DIR/$BINARY_VARIANT" "$FINAL_INSTALL_DIR/$BINARY_NAME"
        sudo chmod +x "$FINAL_INSTALL_DIR/$BINARY_NAME"
    fi
    
    print_success "Installed successfully!"
    
    # Check if in PATH
    if ! echo "$PATH" | grep -q "$FINAL_INSTALL_DIR"; then
        print_warning "$FINAL_INSTALL_DIR is not in your PATH"
        print_info "Add this to your shell profile:"
        echo ""
        echo "  export PATH=\"\$PATH:$FINAL_INSTALL_DIR\""
        echo ""
    fi
    
    # Verify installation
    if command_exists "$BINARY_NAME"; then
        echo ""
        print_success "Installation complete! 🎉"
        echo ""
        print_info "Installed version:"
        "$BINARY_NAME" --version 2>/dev/null || echo "  nylon v${VERSION}"
        echo ""
        print_info "Quick start:"
        echo "  nylon run -c config.yaml"
        echo ""
        print_info "Documentation: https://nylon.sh/"
    else
        print_warning "Installation complete, but 'nylon' command not found in PATH"
        print_info "Try running: $FINAL_INSTALL_DIR/$BINARY_NAME --version"
    fi
}

main "$@"

