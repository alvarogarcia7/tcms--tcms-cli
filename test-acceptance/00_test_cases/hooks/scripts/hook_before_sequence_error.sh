#!/usr/bin/env bash
# Hook script that fails on sequence 2 for testing before_sequence error handling

SEQUENCE_ID="${TEST_SEQUENCE_ID:-1}"

if [ "$SEQUENCE_ID" = "2" ]; then
    echo "ERROR: before_sequence hook failed for sequence $SEQUENCE_ID" >&2
    exit 1
else
    echo "before_sequence hook: Sequence $SEQUENCE_ID - allowing execution"
    exit 0
fi
