#!/usr/bin/env bash
# RED-first tests for scripts/run-e2e-scripts.sh and the test-e2e Makefile wiring (TCMS-46).
#
#   A  runner exits 0 when the only script passes
#   B  runner exits 1 when a script fails, and still runs the scripts after it
#   C  runner exits 1 when a listed script does not exist
#   D  Makefile: the verifier steps in test-e2e fail the run (|| exit 1), and the runner is wired in
#   E  static audit: every E2E script listed in test-e2e that calls fail "..." also contains exit 1,
#      and every listed script exists
#
# Uses set -u only (not set -e) so that every failure is counted.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
RUNNER="$ROOT/scripts/run-e2e-scripts.sh"
FIXTURES="$ROOT/tests/integration/fixtures/e2e-runner"
MAKEFILE="$ROOT/Makefile"

PASS_COUNT=0
FAIL_COUNT=0

check() {
    # $1 = test name, $2 = 0 for pass / non-zero for fail, $3 = optional detail
    if [ "$2" -eq 0 ]; then
        echo "PASS: $1"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "FAIL: $1${3:+ ($3)}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
}

cd "$ROOT" || exit 1

# Test A: runner passes a passing script
out=$(bash "$RUNNER" "$FIXTURES/pass_script.sh" 2>&1)
rc=$?
if [ "$rc" -eq 0 ]; then
    check "A: runner exits 0 when every script passes" 0
else
    check "A: runner exits 0 when every script passes" 1 "exit=$rc"
fi

# Test B: runner fails on a failing script and still runs the one after it
out=$(bash "$RUNNER" "$FIXTURES/fail_script.sh" "$FIXTURES/pass_script.sh" 2>&1)
rc=$?
ran_after=1
case "$out" in
    *pass-marker*) ran_after=0 ;;
esac
if [ "$rc" -eq 1 ] && [ "$ran_after" -eq 0 ]; then
    check "B: runner exits 1 on failure and still runs later scripts" 0
else
    check "B: runner exits 1 on failure and still runs later scripts" 1 "exit=$rc ran_after=$ran_after"
fi

# Test C: a listed script that does not exist fails the run
out=$(bash "$RUNNER" "$FIXTURES/does_not_exist.sh" 2>&1)
rc=$?
if [ "$rc" -eq 1 ]; then
    check "C: runner exits 1 when a listed script is missing" 0
else
    check "C: runner exits 1 when a listed script is missing" 1 "exit=$rc"
fi

# Test D: Makefile wiring
# D1: the verifier steps inside test-e2e must each end in '|| exit 1', so a failure is not swallowed
TEST_E2E_BLOCK=$(sed -n '/^test-e2e: build/,/^\.PHONY: test-e2e$/p' "$MAKEFILE")
step_lines=$(printf '%s\n' "$TEST_E2E_BLOCK" | grep -E '\$\{MAKE\} test-verifier-edge-cases|BUILD_VARIANT="" \./scripts/run_verifier_and_generate_reports\.sh')
step_count=$(printf '%s\n' "$step_lines" | grep -c .)
unguarded=$(printf '%s\n' "$step_lines" | grep -v '|| exit 1' | grep -c .)
if [ "$step_count" -eq 2 ] && [ "$unguarded" -eq 0 ]; then
    check "D1: verifier steps in test-e2e fail the run (|| exit 1)" 0
else
    check "D1: verifier steps in test-e2e fail the run (|| exit 1)" 1 "steps=$step_count unguarded=$unguarded"
fi

# D2: test-e2e runs its scripts through the runner
if sed -n '/^test-e2e: build/,/^\.PHONY: test-e2e$/p' "$MAKEFILE" | grep -q 'run-e2e-scripts\.sh'; then
    check "D2: test-e2e invokes scripts/run-e2e-scripts.sh" 0
else
    check "D2: test-e2e invokes scripts/run-e2e-scripts.sh" 1
fi

# Test E: static audit of the E2E scripts listed in test-e2e.
# A script is FLAGGED when it calls fail "..." but its last top-level exit is 'exit 0' and no
# failure counter gates that exit (so the fail() calls cannot change the status the runner sees).
# Flagged scripts in this plan's files_modified fail the test. Flagged scripts outside files_modified
# are reported as DEFERRED (recorded in deferred-items.md) and do not fail this plan's test.
IN_SCOPE="
crates/testcase-manager/tests/integration/test_json_escape_e2e.sh
crates/testcase-manager/tests/integration/test_documentation_generation.sh
scripts/run_verifier_and_generate_reports.sh
scripts/generate_documentation_reports.sh
scripts/run_all_samples_and_generate_reports.sh
"
is_flagged() {
    # $1 = file. Returns 0 when flagged.
    grep -q 'fail "' "$1" || return 1
    last_exit=$(grep -E '^exit ' "$1" | tail -1)
    [ "$last_exit" = "exit 0" ] || return 1
    if grep -qE '^[[:space:]]*[A-Z_]*(FAIL|FAILED|FAILURES|ERRORS)[A-Z_]*=(0|[0-9]|\$\(\()' "$1"; then
        return 1
    fi
    return 0
}

LISTED=$(sed -n '/^test-e2e: build/,/^\.PHONY: test-e2e$/p' "$MAKEFILE" | grep -o '\./[^" ]*\.sh' | sort -u)
listed_total=0
missing_listed=0
audit_bad=0
deferred=0
for rel in $LISTED; do
    case "$rel" in
        */run-e2e-scripts.sh) continue ;;
    esac
    listed_total=$((listed_total + 1))
    rel_clean="${rel#./}"
    f="$ROOT/$rel_clean"
    if [ ! -f "$f" ]; then
        echo "  listed script missing: $rel"
        missing_listed=$((missing_listed + 1))
        continue
    fi
    if is_flagged "$f"; then
        if printf '%s\n' "$IN_SCOPE" | grep -qx "$rel_clean"; then
            echo "  in-scope, fail() without a failing exit: $rel_clean"
            audit_bad=$((audit_bad + 1))
        else
            echo "  DEFERRED (outside files_modified): $rel_clean"
            deferred=$((deferred + 1))
        fi
    fi
done
echo "  audited $listed_total listed scripts; deferred $deferred out-of-scope flagged script(s)"
if [ "$listed_total" -gt 0 ] && [ "$missing_listed" -eq 0 ] && [ "$audit_bad" -eq 0 ]; then
    check "E: listed E2E scripts exist and in-scope fail() callers exit non-zero" 0
else
    check "E: listed E2E scripts exist and in-scope fail() callers exit non-zero" 1 "listed=$listed_total missing=$missing_listed in_scope_bad=$audit_bad"
fi

echo ""
echo "Results: $PASS_COUNT passed, $FAIL_COUNT failed"
if [ "$FAIL_COUNT" -gt 0 ]; then
    exit 1
fi
exit 0
