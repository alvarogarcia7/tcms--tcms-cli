#!/usr/bin/env bash
# Hook script that fails on step 3 for testing before_step error handling

STEP="${TEST_STEP:-1}"

if [ "$STEP" = "3" ]; then
    echo "ERROR: before_step hook failed for step $STEP" >&2
    exit 1
else
    echo "before_step hook: Step $STEP - preparation successful"
    exit 0
fi
