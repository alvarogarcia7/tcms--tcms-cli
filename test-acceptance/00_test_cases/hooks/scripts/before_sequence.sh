#!/usr/bin/env bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1

SEQUENCE_ID="${TEST_SEQUENCE_ID:-unknown}"
SEQUENCE_NAME="${TEST_SEQUENCE_NAME:-unknown}"

log_info "HOOK: before_sequence - Starting sequence: $SEQUENCE_ID - $SEQUENCE_NAME"
log_info "HOOK: before_sequence - Timestamp: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"

SEQUENCE_DIR=$(cat /tmp/tc_hooks_001_sequence_dir.txt 2>/dev/null || echo "/tmp/tc_hooks_001_sequences")
SEQUENCE_LOG="$SEQUENCE_DIR/sequence_${SEQUENCE_ID}.log"

echo "Sequence $SEQUENCE_ID started at $(date -u '+%Y-%m-%d %H:%M:%S UTC')" > "$SEQUENCE_LOG"
log_info "HOOK: before_sequence - Created sequence log: $SEQUENCE_LOG"
