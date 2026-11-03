#!/usr/bin/env bash
# Shared test case discovery for acceptance suite Stage 1 (validation) and Stage 2 (generation).
# Sourced only. The caller must source scripts/lib/logger.sh first (uses log_error).
# Bash 3.2 compatible: no mapfile, no associative arrays.

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    echo "discover_test_cases.sh is a library; source it instead of running it" >&2
    exit 1
fi

# Usage: discover_test_case_files DIR
# Prints NUL-separated paths of *.yml and *.yaml files under DIR, sorted, unique.
# Returns 1 if DIR is missing.
discover_test_case_files() {
    local dir="$1"
    if [ -z "$dir" ] || [ ! -d "$dir" ]; then
        return 1
    fi
    find "$dir" -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 | sort -z -u
}

# Usage: require_test_case_files COUNT
# Returns 1 (and logs an error) when COUNT is zero or not a non-negative integer.
# A stage with zero inputs must fail rather than report success.
require_test_case_files() {
    local count="$1"
    case "$count" in
        ''|*[!0-9]*)
            log_error "Invalid discovered test case count: '$count'"
            return 1
            ;;
    esac
    if [ "$count" -eq 0 ]; then
        log_error "No test case files discovered (*.yml, *.yaml); refusing to report success"
        return 1
    fi
    return 0
}
