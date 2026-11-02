#!/usr/bin/env bash
# Hook script for teardown_test that creates a marker file
# This verifies the hook was executed even after a test step failure (TCMS-9)

MARKER_FILE="/tmp/teardown_hook_executed_marker"

# Create marker file with content
echo "teardown_test hook was executed" > "$MARKER_FILE"

# Exit successfully
exit 0
