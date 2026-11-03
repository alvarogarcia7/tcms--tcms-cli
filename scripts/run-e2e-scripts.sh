#!/usr/bin/env bash
# Runs each E2E script given on argv, in order, and reports the result of each (TCMS-46).
#
# Usage: scripts/run-e2e-scripts.sh SCRIPT [SCRIPT...]
#
# - Each script is run with bash from the repository root.
# - A missing script counts as a failure (it is never skipped).
# - A failing script does not stop the run; the remaining scripts still run.
# - Exit 0 only when every script exited 0. Otherwise exit 1.
#
# Compatible with bash 3.2 (no mapfile, no associative arrays).

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/lib/logger.sh" || exit 1

if [ "$#" -eq 0 ]; then
    log_error "run-e2e-scripts.sh: no scripts given"
    exit 1
fi

# Resolve relative paths against the caller's directory before changing to the repo root.
ABS_ARGS=()
for arg in "$@"; do
    case "$arg" in
        /*) ABS_ARGS+=("$arg") ;;
        *)  ABS_ARGS+=("$PWD/$arg") ;;
    esac
done

cd "$ROOT" || exit 1

TOTAL=0
FAILED=0
FAILED_LIST=""

for script in "${ABS_ARGS[@]}"; do
    TOTAL=$((TOTAL + 1))
    if [ ! -f "$script" ]; then
        fail "MISSING E2E script: $script"
        FAILED=$((FAILED + 1))
        FAILED_LIST="$FAILED_LIST
  $script (missing)"
        continue
    fi

    echo "Running: $script"
    bash "$script"
    rc=$?
    if [ "$rc" -eq 0 ]; then
        pass "PASSED: $script"
    else
        fail "FAILED (exit $rc): $script"
        FAILED=$((FAILED + 1))
        FAILED_LIST="$FAILED_LIST
  $script (exit $rc)"
    fi
done

echo ""
echo "E2E scripts: $TOTAL run, $((TOTAL - FAILED)) passed, $FAILED failed"
if [ "$FAILED" -gt 0 ]; then
    log_error "Failed E2E scripts:$FAILED_LIST"
    exit 1
fi
exit 0
