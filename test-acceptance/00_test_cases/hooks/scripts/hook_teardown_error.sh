#!/usr/bin/env bash
# Hook script that fails during teardown_test phase

echo "ERROR: teardown_test hook failed during test cleanup" >&2
exit 1
