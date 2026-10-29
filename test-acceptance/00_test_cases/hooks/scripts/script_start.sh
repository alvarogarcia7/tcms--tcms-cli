#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

TEST_START_TIME=$(date +%s)
echo "$TEST_START_TIME" > /tmp/tc_hooks_001_start_time.txt

log_info "HOOK: script_start - Test execution started at $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
log_info "HOOK: script_start - Start time stored: $TEST_START_TIME"
