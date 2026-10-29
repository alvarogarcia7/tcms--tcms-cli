#!/usr/bin/env bash
# Hook script that fails during script_end phase

echo "ERROR: script_end hook failed at final script termination" >&2
exit 1
