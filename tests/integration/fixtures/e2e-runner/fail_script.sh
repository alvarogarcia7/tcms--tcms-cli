#!/usr/bin/env bash
# Fixture for test_run_e2e_scripts.sh: a script that reports a failure and exits non-zero.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../../../../scripts/lib/logger.sh" || exit 1
echo "e2e-runner-fixture: fail-marker"
fail "planted failure"
exit 1
