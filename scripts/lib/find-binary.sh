#!/usr/bin/env bash
#
# find-binary.sh - Shared functions for finding binaries in target directory
#
# DESCRIPTION:
#   This library provides functions to locate Rust binaries in the target directory,
#   checking both release and debug builds, and falling back to system PATH.
#
# USAGE:
#   source scripts/lib/find-binary.sh
#   
#   # Find validate-yaml binary
#   VALIDATE_YAML=$(find_binary "validate-yaml")
#   if [[ -z "$VALIDATE_YAML" ]]; then
#       echo "Binary not found"
#       exit 1
#   fi
#
# FUNCTIONS:
#   find_binary <binary-name> [env-var-name]
#       Finds a binary by checking:
#       1. Environment variable (if provided)
#       2. target/release/<binary-name>
#       3. target/debug/<binary-name>
#       4. System PATH (command -v)
#       Returns the path to the binary or empty string if not found
#
#   find_binary_or_exit <binary-name> [env-var-name]
#       Same as find_binary but exits with error message if not found
#
#   ensure_binary_built <binary-name>
#       Checks if binary exists, builds it if missing
#       Returns 0 if binary exists or was built successfully, 1 otherwise
#

# Find a binary in target directory or system PATH
# Arguments:
#   $1 - binary name (required)
#   $2 - environment variable name to check first (optional)
# Returns:
#   Path to binary on stdout, or empty string if not found
# 
# Workspace-aware: Searches for binaries in both workspace and crate-specific
# target directories to support both single-crate and workspace builds.
find_binary() {
    local binary_name="$1"
    local env_var_name="${2:-}"
    
    # Check environment variable first if provided
    if [[ -n "$env_var_name" ]]; then
        local env_value="${!env_var_name:-}"
        if [[ -n "$env_value" ]]; then
            echo "$env_value"
            return 0
        fi
    fi
    
    # Workspace-aware search: Check both workspace root and crate-specific locations
    # This supports:
    # 1. Workspace builds: binaries in workspace_root/target/{release,debug}/
    # 2. Single-crate builds: binaries in crate_dir/target/{release,debug}/
    
    # A shared target directory (CARGO_TARGET_DIR, for example one directory for several
    # checkouts) holds the binaries instead of ./target: release before debug, as below
    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        if [[ -x "$CARGO_TARGET_DIR/release/$binary_name" ]]; then
            echo "$CARGO_TARGET_DIR/release/$binary_name"
            return 0
        fi
        if [[ -x "$CARGO_TARGET_DIR/debug/$binary_name" ]]; then
            echo "$CARGO_TARGET_DIR/debug/$binary_name"
            return 0
        fi
    fi

    # Check workspace root target/release
    if [[ -x "target/release/$binary_name" ]]; then
        echo "target/release/$binary_name"
        return 0
    fi
    
    # Check workspace root target/debug
    if [[ -x "target/debug/$binary_name" ]]; then
        echo "target/debug/$binary_name"
        return 0
    fi
    
    # Check crate-specific target directories (for non-workspace builds)
    # Search in crates/*/target/{release,debug}/
    if [[ -d "crates" ]]; then
        # Try release builds in crates
        for crate_dir in crates/*/; do
            if [[ -x "${crate_dir}target/release/$binary_name" ]]; then
                echo "${crate_dir}target/release/$binary_name"
                return 0
            fi
        done
        
        # Try debug builds in crates
        for crate_dir in crates/*/; do
            if [[ -x "${crate_dir}target/debug/$binary_name" ]]; then
                echo "${crate_dir}target/debug/$binary_name"
                return 0
            fi
        done
    fi
    
    # Check system PATH
    if command -v "$binary_name" >/dev/null 2>&1; then
        echo "$binary_name"
        return 0
    fi
    
    # Not found
    echo ""
    return 0
}

# Find a binary or exit with error
# Arguments:
#   $1 - binary name (required)
#   $2 - environment variable name to check first (optional)
# Returns:
#   Path to binary on stdout
#   Exits with code 1 if not found
find_binary_or_exit() {
    local binary_name="$1"
    local env_var_name="${2:-}"
    
    local binary_path
    binary_path=$(find_binary "$binary_name" "$env_var_name")
    
    if [[ -z "$binary_path" ]]; then
        echo "[ERROR] $binary_name binary not found" >&2
        echo "[ERROR] Please build it with: cargo build --bin $binary_name" >&2
        exit 1
    fi
    
    echo "$binary_path"
    return 0
}

# Ensure a binary exists, building it if necessary
# Arguments:
#   $1 - binary name (required)
# Returns:
#   0 if binary exists or was built successfully
#   1 if build failed
ensure_binary_built() {
    local binary_name="$1"
    
    # Check if binary already exists
    local binary_path
    binary_path=$(find_binary "$binary_name")
    
    if [[ -n "$binary_path" ]]; then
        return 0
    fi
    
    # Binary not found, try to build it
    echo "[WARNING] $binary_name binary not found" >&2
    echo "[WARNING] Building the binary with: cargo build --bin $binary_name" >&2
    
    if cargo build --bin "$binary_name" 2>&1; then
        return 0
    else
        echo "[ERROR] Failed to build $binary_name" >&2
        return 1
    fi
}

# Find the best available Python interpreter (Python 3.14 preferred)
# Returns:
#   Path to python interpreter on stdout, or empty string if not found
# Priority:
#   1. python3.14 (explicit version)
#   2. uv run python3.14 (via uv)
#   3. python3 (system default)
find_python() {
    # Check for python3.14 directly
    if command -v python3.14 >/dev/null 2>&1; then
        echo "python3.14"
        return 0
    fi
    
    # Check if uv is available and can run python3.14
    if command -v uv >/dev/null 2>&1; then
        if uv run python3.14 --version >/dev/null 2>&1; then
            echo "uv run python3.14"
            return 0
        fi
    fi
    
    # Fall back to system python3
    if command -v python3 >/dev/null 2>&1; then
        echo "python3"
        return 0
    fi
    
    # No Python found
    echo ""
    return 1
}
