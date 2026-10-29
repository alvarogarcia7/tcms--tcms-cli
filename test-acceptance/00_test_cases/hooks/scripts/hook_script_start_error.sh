#!/usr/bin/env bash
# Hook script that intentionally fails for testing script_start error handling

echo "ERROR: script_start hook failed intentionally" >&2
exit 1
