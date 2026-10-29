#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-unknown}"
STEP_NUMBER="${TEST_STEP_NUMBER:-unknown}"
EXIT_CODE="${STEP_EXIT_CODE:-unknown}"

log_info "HOOK: after_step - Sequence $SEQUENCE_ID, Step $STEP_NUMBER completed"
log_info "HOOK: after_step - Exit code: $EXIT_CODE"

if [ "$EXIT_CODE" = "0" ]; then
    log_info "HOOK: after_step - Step validation: SUCCESS"
    pass "Step $STEP_NUMBER passed"
else
    log_warning "HOOK: after_step - Step validation: FAILED"
    fail "Step $STEP_NUMBER failed with exit code $EXIT_CODE"
fi

TEST_DIR=$(cat /tmp/tc_hooks_001_workspace_dir.txt 2>/dev/null || echo "/tmp/tc_hooks_001_test_workspace")
STEP_OUTPUT_FILE="$TEST_DIR/step_${SEQUENCE_ID}_${STEP_NUMBER}_output.txt"

if [ -n "$COMMAND_OUTPUT" ]; then
    echo "$COMMAND_OUTPUT" > "$STEP_OUTPUT_FILE"
    log_verbose "HOOK: after_step - Saved step output to $STEP_OUTPUT_FILE"
fi
