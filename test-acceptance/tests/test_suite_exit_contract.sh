#!/usr/bin/env bash
# Exit-contract tests for the acceptance suite (TCMS-45 AC 2 and 3, plan 10-03, decision D-45-3).
# Each case runs run_acceptance_suite.sh with overrides and checks the exit status and the output.
# Overrides: ACCEPTANCE_TEST_CASES_DIR is not used here; the suite reads TEST_CASES_DIR from
# ACCEPTANCE_TEST_CASES_DIR, GLOSSARY_VALIDATOR, JIRA_VALIDATOR and KNOWN_FAILURES_FILE.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SUITE="$PROJECT_ROOT/test-acceptance/run_acceptance_suite.sh"
REAL_CASES="$PROJECT_ROOT/test-acceptance/00_test_cases"

FAILED=0
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/suite_exit_contract.XXXXXX")" || exit 1
trap 'rm -rf "$SCRATCH"' EXIT

# Stages 2 to 7 are skipped in every case except Test E, so the cases stay fast.
SKIP_2_7="--skip-generation --skip-execution --skip-verification --skip-documentation --skip-consolidated-docs"
SKIP_8_10="--skip-glossary --skip-jira --skip-report-exit-codes"

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; FAILED=1; }

# run_case LABEL EXPECTED_RC NEEDLE(optional) -- env assignments and flags via the globals RUN_ENV and RUN_ARGS
OUT_FILE="$SCRATCH/out.txt"
run_suite() {
    # shellcheck disable=SC2086
    env $RUN_ENV bash "$SUITE" $RUN_ARGS > "$OUT_FILE" 2>&1
    RC=$?
}

check_rc() {
    local label="$1" expected="$2"
    if [ "$RC" -eq "$expected" ]; then
        pass "$label (exit $RC)"
    else
        fail "$label (expected exit $expected, got $RC)"
        tail -20 "$OUT_FILE" | sed 's/^/    | /'
    fi
}

check_grep() {
    local label="$1" pattern="$2"
    if grep -q -- "$pattern" "$OUT_FILE"; then
        pass "$label"
    else
        fail "$label (no line matching: $pattern)"
    fi
}

# Test A: zero test case files is a failure (D-45-2)
EMPTY_DIR="$SCRATCH/empty"
mkdir -p "$EMPTY_DIR"
RUN_ENV="ACCEPTANCE_TEST_CASES_DIR=$EMPTY_DIR KNOWN_FAILURES_FILE=$SCRATCH/none.txt"
RUN_ARGS="$SKIP_2_7 $SKIP_8_10"
run_suite
check_rc "A: empty test case folder exits 1" 1

# Test B: a known failure whose backlog id does not exist makes the suite exit 1
printf '8  TCMS-999999  made-up id\n' > "$SCRATCH/bad_id.txt"
RUN_ENV="KNOWN_FAILURES_FILE=$SCRATCH/bad_id.txt"
RUN_ARGS="$SKIP_2_7 $SKIP_8_10"
run_suite
check_rc "B: known failure with missing backlog id exits 1" 1

# Test C: an unlisted glossary failure makes the suite exit 1
RUN_ENV="GLOSSARY_VALIDATOR=/bin/false KNOWN_FAILURES_FILE=$SCRATCH/none.txt"
RUN_ARGS="$SKIP_2_7 --skip-jira --skip-report-exit-codes"
run_suite
check_rc "C: unlisted stage 8 failure exits 1" 1

# Test D: the same failure, listed with an existing backlog id, is KNOWN and exits 0
printf '# stage id, backlog id, reason\n8  TCMS-45  stub failure for the contract test\n' > "$SCRATCH/known.txt"
RUN_ENV="GLOSSARY_VALIDATOR=/bin/false KNOWN_FAILURES_FILE=$SCRATCH/known.txt"
RUN_ARGS="$SKIP_2_7 --skip-jira --skip-report-exit-codes"
run_suite
check_rc "D: listed stage 8 failure with existing backlog id exits 0" 0
check_grep "D: summary prints KNOWN FAILURE for stage 8" "KNOWN FAILURE"

# Test E: all stages pass or are skipped on the real tree, and every stage 1 to 10 has a summary line
RUN_ENV="GLOSSARY_VALIDATOR=/bin/true JIRA_VALIDATOR=/bin/true KNOWN_FAILURES_FILE=$SCRATCH/none.txt"
RUN_ARGS="$SKIP_2_7 --skip-report-exit-codes"
run_suite
check_rc "E: all-pass run on the real tree exits 0" 0
for n in 1 2 3 4 5 6 7 8 9 10; do
    check_grep "E: summary has a line for stage $n" "^Stage $n:"
done

if [ "$FAILED" -eq 0 ]; then
    echo "ALL CONTRACT TESTS PASSED"
    exit 0
fi
echo "CONTRACT TESTS FAILED"
exit 1
