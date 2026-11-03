#!/bin/bash
#
# Ultra-quick smoke test to verify basic functionality
# Runs in <5 seconds for immediate feedback
#
# Usage: ./tests/integration/smoke_test.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

# Source shared libraries
source "$PROJECT_ROOT/scripts/lib/find-binary.sh"
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# logger.sh's fail() only prints. Count failures here so the script's exit status reflects them (TCMS-46 D-1).
FAILURE_COUNT=0
fail() {
    FAILURE_COUNT=$((FAILURE_COUNT + 1))
    echo -e "${RED}✗${NC} $1" >&2
}
finish_run() {
    if [[ $FAILURE_COUNT -gt 0 ]]; then
        echo -e "${RED}✗${NC} $FAILURE_COUNT check(s) failed" >&2
        exit 1
    fi
}

# Change to project root for binary search
cd "$PROJECT_ROOT"

section "Quick Smoke Test"

# Find the trm (testcase-runner-manager) binary
BINARY=$(find_binary "trm")
if [[ -z "$BINARY" ]]; then
    fail "Binary not found in target/release or target/debug"
    log_info "Run: cargo build"
    exit 1
fi
pass "Binary found: $BINARY"

# Check help command works
if "$BINARY" --help > /dev/null 2>&1; then
    pass "Help command works"
else
    fail "Help command failed"
    exit 1
fi

# Check version command works
if "$BINARY" --version > /dev/null 2>&1; then
    pass "Version command works"
else
    fail "Version command failed"
    exit 1
fi

# Check expect is available
if command -v expect &> /dev/null; then
    pass "Expect available"
else
    log_warning "Expect not available (integration tests will fail)"
fi

# Check git is available
if command -v git &> /dev/null; then
    pass "Git available"
else
    fail "Git not available"
    exit 1
fi

section "Smoke Test Passed"
info "Ready to run full integration tests:"
info "  make test-e2e        # Run complete workflow test"
info "  make test-e2e-all    # Run all integration tests"

finish_run
exit 0
