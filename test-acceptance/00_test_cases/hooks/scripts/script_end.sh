#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

TEST_END_TIME=$(date +%s)
TEST_START_TIME=$(cat /tmp/tc_hooks_001_start_time.txt 2>/dev/null || echo "$TEST_END_TIME")

DURATION=$((TEST_END_TIME - TEST_START_TIME))

log_info "HOOK: script_end - Test execution completed at $(date -u '+%Y-%m-%d %H:%M:%S UTC')"
log_info "HOOK: script_end - Total duration: ${DURATION} seconds"

rm -f /tmp/tc_hooks_001_start_time.txt
rm -f /tmp/tc_hooks_001_workspace_dir.txt
rm -f /tmp/tc_hooks_001_sequence_dir.txt

log_info "HOOK: script_end - Cleanup of tracking files completed"
