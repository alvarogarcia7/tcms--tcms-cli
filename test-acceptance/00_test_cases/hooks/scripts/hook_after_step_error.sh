#!/usr/bin/env bash
# Hook script that fails after step 2 for testing after_step error handling

STEP="${TEST_STEP:-1}"

if [ "$STEP" = "2" ]; then
    echo "ERROR: after_step hook failed for step $STEP" >&2
    exit 1
else
    echo "after_step hook: Step $STEP - validation successful"
    exit 0
fi
