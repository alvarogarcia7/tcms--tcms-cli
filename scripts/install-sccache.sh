#!/usr/bin/env bash
#
# install-sccache.sh - Install sccache for accelerating Rust compilation
#
# This script installs sccache for:
# - Local development (pre-built binary or cargo install)
# - CI environments (pre-built binary for faster installation)
#
# Usage:
#   ./scripts/install-sccache.sh [OPTIONS]
#
# Options:
#   --local         Install for local development (default)
#   --ci            Install for CI environments (uses pre-built binary)
#   --help          Show this help message
#
# Environment:
#   CI              Set to "true" in CI environments (auto-detected)
#   SCCACHE_VERSION Version to install (default: latest release)

set -e

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source logger library
source "$SCRIPT_DIR/lib/logger.sh" || {
    echo "[ERROR] Failed to source logger library" >&2
    exit 1
}

# Default sccache version
SCCACHE_VERSION="${SCCACHE_VERSION:-v0.7.7}"

# Show help message
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Install sccache for accelerating Rust compilation across different environments.

OPTIONS:
    --local         Install for local development (auto-detect platform)
    --ci            Install for CI environments (uses pre-built binary)
    --help          Show this help message

ENVIRONMENT DETECTION:
    The script automatically detects CI environments and platform:
    - CI environment (CI=true)
    - Operating system (Linux, macOS)
    - Architecture (x86_64, aarch64/arm64)

ENVIRONMENT VARIABLES:
    SCCACHE_VERSION    Version to install (default: $SCCACHE_VERSION)
    CI                 Set to "true" in CI environments

EXAMPLES:
    # Install for local development (auto-detect platform)
    $0 --local

    # Install for CI environment
    $0 --ci

    # Auto-detect environment and install
    $0

    # Install specific version
    SCCACHE_VERSION=v0.7.7 $0

WHAT IS SCCACHE:
    sccache is a ccache-like compiler caching tool that:
    - Caches compilation results to speed up rebuilds
    - Supports various backends (local disk, Redis, S3, GCS)
    - Works with Rust, C, C++, and other languages
    - Dramatically reduces compilation times

INSTALLATION METHODS:
    Local:   Downloads pre-built binary for your platform
    CI:      Downloads pre-built binary (fastest for CI builds)

SUPPORTED PLATFORMS:
    - Linux (x86_64, aarch64)
    - macOS (x86_64, aarch64/arm64)

POST-INSTALLATION:
    After installation, configure your environment:
    
    export RUSTC_WRAPPER=sccache
    
    Or add to your shell profile (~/.bashrc, ~/.zshrc, etc.)

EOF
}

# Detect operating system
detect_os() {
    case "$(uname -s)" in
        Linux*)     echo "linux" ;;
        Darwin*)    echo "macos" ;;
        *)          
            log_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
}

# Detect architecture
detect_arch() {
    local arch
    arch="$(uname -m)"
    
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
}

# Get platform-specific binary name
get_binary_name() {
    local os=$1
    local arch=$2
    
    case "$os" in
        linux)
            case "$arch" in
                x86_64)
                    echo "sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl"
                    ;;
                aarch64)
                    echo "sccache-${SCCACHE_VERSION}-aarch64-unknown-linux-musl"
                    ;;
            esac
            ;;
        macos)
            case "$arch" in
                x86_64)
                    echo "sccache-${SCCACHE_VERSION}-x86_64-apple-darwin"
                    ;;
                aarch64)
                    echo "sccache-${SCCACHE_VERSION}-aarch64-apple-darwin"
                    ;;
            esac
            ;;
    esac
}

# Check if sccache is already installed
check_existing_installation() {
    if command -v sccache >/dev/null 2>&1; then
        local current_version
        current_version=$(sccache --version | head -n1 || echo "unknown")
        log_warning "sccache is already installed: $current_version"
        
        # In CI, skip the interactive prompt
        if [ "${CI:-false}" = "true" ]; then
            log_info "Running in CI, skipping reinstallation"
            return 1
        fi
        
        read -p "Do you want to reinstall/update? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping sccache installation"
            return 1
        fi
        
        return 0
    fi
    
    return 0
}

# Install sccache from pre-built binary
install_prebuilt() {
    local os
    local arch
    local binary_name
    local download_url
    
    os=$(detect_os)
    arch=$(detect_arch)
    binary_name=$(get_binary_name "$os" "$arch")
    
    if [ -z "$binary_name" ]; then
        log_error "No pre-built binary available for $os-$arch"
        return 1
    fi
    
    download_url="https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${binary_name}.tar.gz"
    
    log_info "Detected platform: $os-$arch"
    log_info "Downloading sccache ${SCCACHE_VERSION}..."
    log_info "URL: $download_url"
    
    # Create temporary directory for download
    local temp_dir
    temp_dir=$(mktemp -d)
    
    # Download and extract
    if curl -L -f "${download_url}" -o "${temp_dir}/sccache.tar.gz"; then
        pass "Downloaded sccache archive"
    else
        log_error "Failed to download sccache from $download_url"
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Extract archive
    if tar -xzf "${temp_dir}/sccache.tar.gz" -C "${temp_dir}"; then
        pass "Extracted sccache archive"
    else
        log_error "Failed to extract sccache archive"
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Find sccache binary in extracted directory
    local sccache_binary
    sccache_binary=$(find "$temp_dir" -type f -name "sccache" | head -n1)
    
    if [ -z "$sccache_binary" ]; then
        log_error "Could not find sccache binary in extracted archive"
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Make binary executable
    chmod +x "$sccache_binary"
    
    # Determine installation directory
    local install_dir
    if [ "${CI:-false}" = "true" ]; then
        # In CI, prefer /usr/local/bin (always in PATH) when running as root
        if [ "$(id -u)" = "0" ] && [ -d "/usr/local/bin" ]; then
            install_dir="/usr/local/bin"
        else
            # Use CARGO_HOME/bin if set, otherwise fall back to ~/.cargo/bin
            install_dir="${CARGO_HOME:-$HOME/.cargo}/bin"
        fi
    else
        # For local development, prefer ~/.cargo/bin if it exists
        if [ -d "$HOME/.cargo/bin" ]; then
            install_dir="$HOME/.cargo/bin"
        else
            # Otherwise, use /usr/local/bin (may require sudo)
            install_dir="/usr/local/bin"
        fi
    fi
    
    # Create installation directory if it doesn't exist
    if [ ! -d "$install_dir" ]; then
        mkdir -p "$install_dir" 2>/dev/null || {
            log_error "Cannot create installation directory: $install_dir"
            log_error "You may need to run with sudo or choose a different location"
            rm -rf "$temp_dir"
            return 1
        }
    fi
    
    # Copy binary to installation directory
    if cp "$sccache_binary" "$install_dir/sccache" 2>/dev/null; then
        pass "Installed sccache to $install_dir/sccache"
    else
        log_error "Failed to copy sccache to $install_dir"
        log_error "You may need to run with sudo"
        rm -rf "$temp_dir"
        return 1
    fi
    
    # Clean up temporary directory
    rm -rf "$temp_dir"

    # Ensure install_dir is in PATH for the current process
    case ":$PATH:" in
        *":$install_dir:"*) ;;
        *) export PATH="$install_dir:$PATH" ;;
    esac

    # Verify installation
    if command -v sccache >/dev/null 2>&1; then
        pass "sccache is now available in PATH"
    else
        log_warning "sccache installed but not in PATH"
        log_info "Add $install_dir to your PATH:"
        log_info "  export PATH=\"$install_dir:\$PATH\""
    fi

    return 0
}

# Install sccache using cargo install (fallback)
install_from_cargo() {
    log_info "Installing sccache using cargo install..."
    
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Rust/Cargo is not installed. Please install Rust first:"
        log_error "  https://rustup.rs/"
        return 1
    fi
    
    log_warning "This may take several minutes to compile..."
    
    if cargo install sccache; then
        pass "Installed sccache via cargo"
        return 0
    else
        log_error "Failed to install sccache via cargo"
        return 1
    fi
}

# Verify installation
verify_installation() {
    log_info "Verifying sccache installation..."
    
    if command -v sccache >/dev/null 2>&1; then
        local version
        version=$(sccache --version 2>&1 | head -n1)
        pass "sccache is installed: $version"
        
        # Show sccache location
        local sccache_path
        sccache_path=$(command -v sccache)
        log_info "Location: $sccache_path"
        
        return 0
    else
        log_error "sccache verification failed"
        return 1
    fi
}

# Show post-installation instructions
show_instructions() {
    cat << EOF

Installation complete!

USAGE EXAMPLES:

1. Enable sccache for Rust compilation:
   
   export RUSTC_WRAPPER=sccache

2. Add to your shell profile for persistent use:
   
   # For bash (~/.bashrc):
   echo 'export RUSTC_WRAPPER=sccache' >> ~/.bashrc
   
   # For zsh (~/.zshrc):
   echo 'export RUSTC_WRAPPER=sccache' >> ~/.zshrc

3. Verify sccache is working:
   
   sccache --show-stats

4. Clear the cache:
   
   sccache --stop-server

5. Use with cargo:
   
   # sccache will automatically cache compilation results
   cargo build
   cargo build --release

6. Monitor cache statistics:
   
   # Show current cache statistics
   sccache --show-stats
   
   # Zero statistics
   sccache --zero-stats

CONFIGURATION:

This project uses a global cache directory for sccache to enable
cache sharing across multiple worktrees:

  ~/.cache/sccache/testcase-manager

This is exported by scripts/enable-sccache.sh. It is not set in
.cargo/config.toml because Cargo's [env] table does not expand \$HOME.

Additional sccache configuration options (environment variables):

- SCCACHE_DIR: Cache directory (default: set by scripts/enable-sccache.sh)
- SCCACHE_CACHE_SIZE: Maximum cache size (default: 10G)
- SCCACHE_REDIS: Redis server for distributed caching
- SCCACHE_S3_*: S3 bucket configuration for distributed caching

Example custom configuration (set before sourcing enable-sccache.sh):

export SCCACHE_DIR=~/my-custom-cache
export SCCACHE_CACHE_SIZE=20G

TROUBLESHOOTING:

If sccache is not working:

1. Check that RUSTC_WRAPPER is set:
   echo \$RUSTC_WRAPPER

2. Check sccache server status:
   sccache --show-stats

3. View server logs:
   sccache --show-log

4. Restart sccache server:
   sccache --stop-server
   # Next cargo build will start it automatically

DOCUMENTATION:
  https://github.com/mozilla/sccache

EOF
}

# Main installation logic
main() {
    local install_mode="auto"
    
    # Parse command line arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --local)
                install_mode="local"
                ;;
            --ci)
                install_mode="ci"
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
        shift
    done
    
    # Auto-detect mode if not specified
    if [ "$install_mode" = "auto" ]; then
        if [ "${CI:-false}" = "true" ]; then
            install_mode="ci"
            log_info "Auto-detected CI environment"
        else
            install_mode="local"
            log_info "Auto-detected local development environment"
        fi
    fi
    
    section "Starting sccache installation"
    
    # Check for existing installation
    if ! check_existing_installation; then
        # Already installed and user chose not to reinstall
        verify_installation
        show_instructions
        exit 0
    fi
    
    echo
    
    # Install sccache
    section "Installing sccache"
    
    local success=false
    
    # Try pre-built binary first
    if install_prebuilt; then
        success=true
    else
        log_warning "Pre-built binary installation failed"
        
        # In CI, don't fall back to cargo install (too slow)
        if [ "$install_mode" = "ci" ]; then
            log_error "Cannot use cargo install in CI mode (too slow)"
        else
            log_info "Falling back to cargo install..."
            if install_from_cargo; then
                success=true
            fi
        fi
    fi
    
    echo
    
    # Verify installation
    if [ "$success" = true ]; then
        section "Verifying installation"
        if verify_installation; then
            echo
            show_instructions
            exit 0
        else
            log_error "Installation verification failed"
            exit 1
        fi
    else
        log_error "Failed to install sccache"
        exit 1
    fi
}

# Run main function
main "$@"
