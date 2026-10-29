#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-unknown}"
SEQUENCE_NAME="${TEST_SEQUENCE_NAME:-unknown}"

log_info "HOOK: after_sequence - Completed sequence: $SEQUENCE_ID - $SEQUENCE_NAME"
log_info "HOOK: after_sequence - Timestamp: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"

SEQUENCE_DIR=$(cat /tmp/tc_hooks_001_sequence_dir.txt 2>/dev/null || echo "/tmp/tc_hooks_001_sequences")
SEQUENCE_LOG="$SEQUENCE_DIR/sequence_${SEQUENCE_ID}.log"

if [ -f "$SEQUENCE_LOG" ]; then
    echo "Sequence $SEQUENCE_ID completed at $(date -u '+%Y-%m-%d %H:%M:%S UTC')" >> "$SEQUENCE_LOG"
    log_info "HOOK: after_sequence - Updated sequence log: $SEQUENCE_LOG"
fi

log_info "HOOK: after_sequence - Cleaning up sequence-specific resources"
