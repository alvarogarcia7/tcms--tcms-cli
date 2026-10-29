#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

TEST_DIR="/tmp/tc_hooks_001_test_workspace"
SEQUENCE_DIR="/tmp/tc_hooks_001_sequences"

log_info "HOOK: setup_test - Creating test workspace directories"

mkdir -p "$TEST_DIR"
mkdir -p "$SEQUENCE_DIR"

echo "$TEST_DIR" > /tmp/tc_hooks_001_workspace_dir.txt
echo "$SEQUENCE_DIR" > /tmp/tc_hooks_001_sequence_dir.txt

log_info "HOOK: setup_test - Created workspace: $TEST_DIR"
log_info "HOOK: setup_test - Created sequence directory: $SEQUENCE_DIR"
