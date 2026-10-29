#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

log_info "HOOK: teardown_test - Cleaning up test resources"

TEST_DIR=$(cat /tmp/tc_hooks_001_workspace_dir.txt 2>/dev/null || echo "/tmp/tc_hooks_001_test_workspace")
SEQUENCE_DIR=$(cat /tmp/tc_hooks_001_sequence_dir.txt 2>/dev/null || echo "/tmp/tc_hooks_001_sequences")

if [ -d "$TEST_DIR" ]; then
    log_info "HOOK: teardown_test - Removing workspace directory: $TEST_DIR"
    rm -rf "$TEST_DIR"
fi

if [ -d "$SEQUENCE_DIR" ]; then
    log_info "HOOK: teardown_test - Removing sequence directory: $SEQUENCE_DIR"
    rm -rf "$SEQUENCE_DIR"
fi

log_info "HOOK: teardown_test - Cleanup completed"
