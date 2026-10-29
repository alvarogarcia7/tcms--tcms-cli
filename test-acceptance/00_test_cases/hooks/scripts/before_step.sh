#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-unknown}"
STEP_NUMBER="${TEST_STEP_NUMBER:-unknown}"
STEP_DESC="${TEST_STEP_DESCRIPTION:-no description}"

log_info "HOOK: before_step - Sequence $SEQUENCE_ID, Step $STEP_NUMBER"
log_info "HOOK: before_step - Description: $STEP_DESC"

if [ -n "$TEST_VAR_1" ]; then
    log_verbose "HOOK: before_step - TEST_VAR_1 = $TEST_VAR_1"
fi

if [ -n "$TEST_VAR_2" ]; then
    log_verbose "HOOK: before_step - TEST_VAR_2 = $TEST_VAR_2"
fi

log_info "HOOK: before_step - Step execution about to begin"
