#!/usr/bin/env bash
#
# enable-sccache.sh - Enable sccache for local development
#
# This script helps set up RUSTC_WRAPPER=sccache for the current shell session
# and optionally adds it to your shell profile for persistence.
#
# Usage:
#   source ./scripts/enable-sccache.sh
#   OR
#   . ./scripts/enable-sccache.sh
#
# Options:
#   --permanent    Add to shell profile (~/.bashrc, ~/.zshrc, etc.)
#   --check        Check if sccache is properly configured
#   --help         Show this help message

# NOTE: Do NOT use 'set -e' in a sourced script as it will exit the shell on errors

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Global sccache cache directory, shared across all worktrees and git checkouts
# of this repository. Set here rather than in .cargo/config.toml because Cargo's
# [env] table performs no variable expansion ("$HOME/..." would be taken literally).
# Respects a pre-existing SCCACHE_DIR so users can override the location.
SCCACHE_CACHE_DIR="${SCCACHE_DIR:-$HOME/.cache/sccache/testcase-manager}"

# Source logger library
if [ -f "$SCRIPT_DIR/lib/logger.sh" ]; then
    source "$SCRIPT_DIR/lib/logger.sh"
else
    # Fallback if logger is not available
    log_error() { echo "[ERROR] $*" >&2; }
    log_warning() { echo "[WARNING] $*" >&2; }
    log_info() { echo "[INFO] $*"; }
    pass() { echo "[✓] $*"; }
    section() { echo ""; echo "=== $* ==="; echo ""; }
fi

# Show help message
show_help() {
    cat << EOF
Usage: source $0 [OPTIONS]

Enable sccache for Rust compilation in the current shell session.

OPTIONS:
    --permanent    Add RUSTC_WRAPPER=sccache to shell profile for persistence
    --check        Check if sccache is properly configured
    --help         Show this help message

EXAMPLES:
    # Enable for current session only
    source $0
    
    # Enable and add to shell profile
    source $0 --permanent
    
    # Check configuration
    source $0 --check

NOTES:
    - This script must be sourced (not executed) to affect the current shell
    - sccache must be installed first (run: make install-sccache)
    - The global cache directory is: ~/.cache/sccache/testcase-manager
    - This script exports SCCACHE_DIR; override it by setting SCCACHE_DIR first

EOF
}

# Check if sccache is installed
check_sccache_installed() {
    if ! command -v sccache >/dev/null 2>&1; then
        log_error "sccache is not installed"
        log_info "Install sccache first:"
        log_info "  make install-sccache"
        return 1
    fi
    return 0
}

# Check current configuration
check_configuration() {
    section "Checking sccache configuration"
    
    # Check if sccache is installed
    if check_sccache_installed; then
        local version
        version=$(sccache --version 2>&1 | head -n1)
        pass "sccache is installed: $version"
    else
        return 1
    fi
    
    # Check if RUSTC_WRAPPER is set
    if [ -n "$RUSTC_WRAPPER" ]; then
        if [ "$RUSTC_WRAPPER" = "sccache" ]; then
            pass "RUSTC_WRAPPER is set to 'sccache'"
        else
            log_warning "RUSTC_WRAPPER is set to '$RUSTC_WRAPPER' (expected: 'sccache')"
        fi
    else
        log_warning "RUSTC_WRAPPER is not set"
        log_info "Run: source $0"
    fi
    
    # Check cache directory configuration
    if [ -n "$SCCACHE_DIR" ]; then
        log_info "Cache directory: $SCCACHE_DIR (from SCCACHE_DIR)"
    else
        log_warning "SCCACHE_DIR is not set; sccache would use its own default"
        log_info "Run: source ./scripts/enable-sccache.sh  (sets SCCACHE_DIR to $SCCACHE_CACHE_DIR)"
    fi
    
    # Check if cache directory exists
    if [ -d "$SCCACHE_CACHE_DIR" ]; then
        local cache_size
        cache_size=$(du -sh "$SCCACHE_CACHE_DIR" 2>/dev/null | cut -f1)
        log_info "Cache directory exists (size: $cache_size)"
    else
        log_info "Cache directory does not exist yet (will be created on first build)"
    fi
    
    # Show cache statistics if sccache is running
    if [ -n "$RUSTC_WRAPPER" ] && [ "$RUSTC_WRAPPER" = "sccache" ]; then
        echo
        log_info "Cache statistics:"
        sccache --show-stats 2>/dev/null || log_warning "sccache server not running (starts automatically on first build)"
    fi
    
    echo
    log_info "Configuration check complete"
}

# Add to shell profile
add_to_profile() {
    local profile_file=""
    
    # Detect shell and profile file
    if [ -n "$BASH_VERSION" ]; then
        if [ -f "$HOME/.bashrc" ]; then
            profile_file="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            profile_file="$HOME/.bash_profile"
        fi
    elif [ -n "$ZSH_VERSION" ]; then
        profile_file="$HOME/.zshrc"
    else
        log_warning "Unknown shell. Please manually add to your profile:"
        log_info "  export RUSTC_WRAPPER=sccache"
        log_info "  export SCCACHE_DIR=\"$SCCACHE_CACHE_DIR\""
        return 1
    fi
    
    if [ -z "$profile_file" ]; then
        log_error "Could not determine shell profile file"
        log_info "Please manually add to your profile:"
        log_info "  export RUSTC_WRAPPER=sccache"
        log_info "  export SCCACHE_DIR=\"$SCCACHE_CACHE_DIR\""
        return 1
    fi
    
    # Check if already in profile
    if grep -q "^export RUSTC_WRAPPER=sccache" "$profile_file" 2>/dev/null; then
        log_info "RUSTC_WRAPPER=sccache already in $profile_file"
        if ! grep -q "^export SCCACHE_DIR=" "$profile_file" 2>/dev/null; then
            echo "export SCCACHE_DIR=\"$SCCACHE_CACHE_DIR\"" >> "$profile_file"
            pass "Added SCCACHE_DIR to $profile_file"
        fi
        return 0
    fi
    
    # Add to profile
    echo "" >> "$profile_file"
    echo "# Enable sccache for Rust compilation (testcase-manager)" >> "$profile_file"
    echo "export RUSTC_WRAPPER=sccache" >> "$profile_file"
    echo "export SCCACHE_DIR=\"$SCCACHE_CACHE_DIR\"" >> "$profile_file"
    
    pass "Added RUSTC_WRAPPER=sccache and SCCACHE_DIR to $profile_file"
    log_info "Reload your shell or run: source $profile_file"
}

# Main logic
main() {
    local permanent=false
    local check_only=false
    
    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --permanent)
                permanent=true
                ;;
            --check)
                check_only=true
                ;;
            --help|-h)
                show_help
                return 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                return 1
                ;;
        esac
        shift
    done
    
    # Check if script is being sourced
    if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
        log_error "This script must be sourced, not executed"
        log_info "Run: source $0"
        log_info "Or: . $0"
        return 1
    fi
    
    # Check-only mode
    if [ "$check_only" = true ]; then
        check_configuration
        return 0
    fi
    
    section "Enabling sccache for Rust compilation"
    
    # Check if sccache is installed
    if ! check_sccache_installed; then
        return 1
    fi
    
    # Set RUSTC_WRAPPER and SCCACHE_DIR for current session
    export RUSTC_WRAPPER=sccache
    pass "RUSTC_WRAPPER set to 'sccache' for current session"
    export SCCACHE_DIR="$SCCACHE_CACHE_DIR"
    pass "SCCACHE_DIR set to '$SCCACHE_DIR' for current session"
    
    # Add to profile if requested
    if [ "$permanent" = true ]; then
        echo
        if add_to_profile; then
            log_info "sccache will be enabled in future shell sessions"
        fi
    else
        log_info "To make this permanent, run: source $0 --permanent"
        log_info "Or manually add to your shell profile:"
        log_info "  export RUSTC_WRAPPER=sccache"
        log_info "  export SCCACHE_DIR=\"$SCCACHE_CACHE_DIR\""
    fi
    
    echo
    log_info "sccache is now enabled!"
    log_info "Cache directory: $SCCACHE_DIR"
    log_info ""
    log_info "Next steps:"
    log_info "  1. Build your project: cargo build"
    log_info "  2. Check cache stats: sccache --show-stats"
    log_info "  3. Or use Make: make build && make sccache-stats"
    echo
    log_warning "If builds fail with exit status 254:"
    log_info "  - Run: unset RUSTC_WRAPPER"
    log_info "  - Then: cargo clean && cargo build"
    log_info "  - Or use: source ./scripts/disable-sccache.sh"
}

# Run main function
main "$@"
