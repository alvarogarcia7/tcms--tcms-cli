#!/usr/bin/env bash
# Hook script that fails on sequence 1 for testing after_sequence error handling

SEQUENCE_ID="${TEST_SEQUENCE_ID:-1}"

if [ "$SEQUENCE_ID" = "1" ]; then
    echo "ERROR: after_sequence hook failed for sequence $SEQUENCE_ID" >&2
    exit 1
else
    echo "after_sequence hook: Sequence $SEQUENCE_ID - cleanup successful"
    exit 0
fi
