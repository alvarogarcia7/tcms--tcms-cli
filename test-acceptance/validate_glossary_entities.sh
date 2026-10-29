#!/usr/bin/env bash
#
# validate_glossary_entities.sh - Acceptance scenarios for the verification glossary entities (TCMS-40.11)
#
# DESCRIPTION:
#   Runs the real binaries (test-executor, verifier, validate-yaml) on the fixtures in
#   test-acceptance/glossary_entities/fixtures/ and checks the behaviour documented in
#   docs/GLOSSARY_ENTITIES.md:
#     1. Runs with metadata (TCMS-40.3, TCMS-40.4): run IDs on execution logs, run metadata on
#        the verifier output, a test run record that validates against tcms-schemas, and refusal
#        of a missing tester or of a reviewer who is the author of an independent test case
#     2. Traceability matrix (TCMS-40.5, TCMS-40.6): a complete matrix passes the coverage check,
#        an incomplete one fails and names the requirement, --no-coverage-check writes it anyway
#     3. Regression selection (TCMS-40.7): the selected test cases and their reasons, and refusal
#        of a plan that names another previous baseline
#     4. Independence (TCMS-40.8): independence-assignment create and check, and rejection of a
#        record that says the author is an independent verifier
#
# USAGE:
#   ./test-acceptance/validate_glossary_entities.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose   Print the output of every command, not only of failed ones
#   -h, --help      Show this help message
#
# EXIT CODES:
#   0 - All scenarios passed
#   1 - One or more checks failed, or a required binary is missing
#
# NOTES:
#   Scripts stay bash 3.2 compatible and use only portable options of grep, cmp and mktemp.

# Note: no 'set -e': every check runs, so that all failures are reported.

# Get script directory and source shared libraries
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1
source "$REPO_ROOT/scripts/lib/find-binary.sh" || exit 1

VERBOSE=0

usage() {
    sed -n '/^# USAGE:/,/^# NOTES:/p' "$0" | sed 's/^# \{0,1\}//' | sed '$d'
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Fixtures (committed) and the scratch area (removed on exit)
FIXTURES="$SCRIPT_DIR/glossary_entities/fixtures"
RUN_CASES="$FIXTURES/runs/cases"
TRACE_DIR="$FIXTURES/traceability"
REG_DIR="$FIXTURES/regression"
IND_DIR="$FIXTURES/independence"
SCHEMAS_TCMS="$REPO_ROOT/schemas/tcms-schemas/tcms"

WORK_DIR=$(mktemp -d)
setup_cleanup "$WORK_DIR"

# Binaries: $CARGO_TARGET_DIR when cargo builds elsewhere, else the repository's target directories
cd "$REPO_ROOT" || exit 1
locate_binary() {
    local name="$1" profile
    if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
        for profile in debug release; do
            if [[ -x "$CARGO_TARGET_DIR/$profile/$name" ]]; then
                echo "$CARGO_TARGET_DIR/$profile/$name"
                return 0
            fi
        done
    fi
    find_binary "$name"
}
TEST_EXECUTOR=$(locate_binary "test-executor")
VERIFIER=$(locate_binary "verifier")
VALIDATE_YAML=$(locate_binary "validate-yaml")
for binary in TEST_EXECUTOR VERIFIER VALIDATE_YAML; do
    if [[ -z "${!binary}" ]]; then
        log_error "Required binary not found for $binary (build it first: make build)"
        exit 1
    fi
done

PASSED=0
FAILED=0

record_pass() {
    pass "$1"
    PASSED=$((PASSED + 1))
}

record_fail() {
    fail "$1"
    FAILED=$((FAILED + 1))
}

# show_output FILE: print the last lines of a command's output (on failure)
show_output() {
    sed 's/^/      | /' "$1" | tail -20 >&2
}

# run_expect EXPECTED_STATUS DESCRIPTION OUTPUT_FILE COMMAND...
# Runs COMMAND from the repository root and compares its exit status.
run_expect() {
    local expected="$1" description="$2" output="$3"
    shift 3
    local status=0
    (cd "$REPO_ROOT" && "$@") >"$output" 2>&1 || status=$?
    if [[ $status -eq $expected ]]; then
        record_pass "$description"
        [[ $VERBOSE -eq 1 ]] && show_output "$output"
    else
        record_fail "$description (exit $status, expected $expected)"
        show_output "$output"
    fi
}


# expect_line FILE LINE DESCRIPTION: a file has a line exactly equal to LINE
expect_line() {
    if grep -qxF -- "$2" "$1" 2>/dev/null; then
        record_pass "$3"
    else
        record_fail "$3 (no line '$2' in $(basename "$1"))"
    fi
}

# expect_text FILE TEXT DESCRIPTION: a file contains TEXT
expect_text() {
    if grep -qF -- "$2" "$1" 2>/dev/null; then
        record_pass "$3"
    else
        record_fail "$3 (no '$2' in $(basename "$1"))"
    fi
}

# expect_count FILE LINE COUNT DESCRIPTION: exactly COUNT lines equal LINE
expect_count() {
    local found
    found=$(grep -cxF -- "$2" "$1" 2>/dev/null || true)
    if [[ "$found" == "$3" ]]; then
        record_pass "$4"
    else
        record_fail "$4 (found $found lines '$2', expected $3)"
    fi
}

# expect_file DESCRIPTION PATH: the file exists
expect_file() {
    if [[ -s "$2" ]]; then
        record_pass "$1"
    else
        record_fail "$1 (missing or empty: $(basename "$2"))"
    fi
}

# expect_absent DESCRIPTION PATH: the file was not written
expect_absent() {
    if [[ ! -e "$2" ]]; then
        record_pass "$1"
    else
        record_fail "$1 (written anyway: $(basename "$2"))"
    fi
}

# ============================================================================
# Scenario 1: runs with metadata (TCMS-40.3, TCMS-40.4)
# ============================================================================
scenario_runs_with_metadata() {
    section "Runs with metadata (TCMS-40.3, TCMS-40.4)"
    local dir="$WORK_DIR/runs"
    mkdir -p "$dir/scripts" "$dir/exec" "$dir/logs"

    local tc
    for tc in TC_GLOSS_RUN_001 TC_GLOSS_RUN_002; do
        run_expect 0 "generate $tc with a run ID" "$dir/gen_$tc.out" \
            "$TEST_EXECUTOR" --no-audit generate --json-log --run-id RUN-GLOSS-1 \
            "$RUN_CASES/$tc.yaml" --output "$dir/scripts/$tc.sh"
        (cd "$dir/exec" && bash "$dir/scripts/$tc.sh") >"$dir/exec_$tc.out" 2>&1
        local status=$?
        if [[ $status -eq 0 ]]; then
            record_pass "execute $tc"
        else
            record_fail "execute $tc (exit $status)"
            show_output "$dir/exec_$tc.out"
        fi
        expect_file "execution log for $tc" "$dir/exec/${tc}_execution_log.json"
        expect_text "$dir/exec/${tc}_execution_log.json" "\"run_id\": \"RUN-GLOSS-1\"" \
            "execution log of $tc carries the run ID"
    done
    cp "$dir"/exec/*_execution_log.json "$dir/logs/"

    run_expect 0 "verifier writes the container and the test run record" "$dir/verify.out" \
        "$VERIFIER" -f "$dir/logs" --test-case-dir "$RUN_CASES" --run-id RUN-GLOSS-1 \
        --build-id build-gloss-1 --tester ci-bot --environment Staging --reviewer carol \
        --output "$dir/container.yaml" --test-run "$dir/run.yaml"
    expect_file "container YAML" "$dir/container.yaml"
    expect_line "$dir/run.yaml" "run_id: RUN-GLOSS-1" "test run has the run ID"
    expect_line "$dir/run.yaml" "build_id: build-gloss-1" "test run has the build ID"
    expect_line "$dir/run.yaml" "environment: Staging" "test run has the environment"
    expect_line "$dir/run.yaml" "tester: ci-bot" "test run has the tester"
    expect_line "$dir/run.yaml" "reviewer: carol" "test run has the reviewer"
    expect_count "$dir/run.yaml" "  verdict: pass" 2 "test run has one passing result per test case"
    expect_text "$dir/container.yaml" "run_id: RUN-GLOSS-1" "container results carry the run ID"

    run_expect 0 "test run record validates against tcms-schemas test-run" "$dir/validate_run.out" \
        "$VALIDATE_YAML" -s "$SCHEMAS_TCMS/test-run.schema.json" "$dir/run.yaml"

    # Negative: a reviewer who is the author of an independent test case
    run_expect 1 "reviewer who authored an independent test case is refused" "$dir/refused.out" \
        "$VERIFIER" -f "$dir/logs" --test-case-dir "$RUN_CASES" --run-id RUN-GLOSS-1 \
        --build-id build-gloss-1 --tester ci-bot --environment Staging --reviewer alice \
        --output "$dir/container_refused.yaml" --test-run "$dir/run_refused.yaml"
    expect_absent "no test run record is written for a refused reviewer" "$dir/run_refused.yaml"
    expect_text "$dir/refused.out" "reviewer alice is its author" \
        "refusal names the author as the reviewer"

    # Negative: the tester is required on a test run record
    run_expect 1 "test run without a tester is refused" "$dir/no_tester.out" \
        env -u TCMS_TESTER "$VERIFIER" -f "$dir/logs" --test-case-dir "$RUN_CASES" \
        --run-id RUN-GLOSS-1 --build-id build-gloss-1 --environment Staging --reviewer carol \
        --output "$dir/container_no_tester.yaml" --test-run "$dir/run_no_tester.yaml"
    expect_absent "no test run record is written without a tester" "$dir/run_no_tester.yaml"
}

# ============================================================================
# Scenario 2: traceability matrix and coverage check (TCMS-40.5, TCMS-40.6)
# ============================================================================
scenario_traceability_matrix() {
    section "Traceability matrix and coverage check (TCMS-40.5, TCMS-40.6)"
    local dir="$WORK_DIR/trace"
    mkdir -p "$dir"

    run_expect 0 "complete matrix passes the coverage check" "$dir/complete.out" \
        "$TEST_EXECUTOR" --no-audit trace-matrix "$TRACE_DIR/complete" \
        --requirements "$TRACE_DIR/requirements.txt" --baseline-id BL-GLOSS-1 \
        --id TM-GLOSS-C -o "$dir/matrix_complete.yaml"
    expect_count "$dir/matrix_complete.yaml" "  normal_range_covered: true" 2 \
        "both requirements have a normal-range test"
    expect_count "$dir/matrix_complete.yaml" "  robustness_covered: true" 2 \
        "both requirements have a robustness test"
    run_expect 0 "complete matrix validates against tcms-schemas trace-matrix" \
        "$dir/validate_complete.out" \
        "$VALIDATE_YAML" -s "$SCHEMAS_TCMS/trace-matrix.schema.json" "$dir/matrix_complete.yaml"

    run_expect 1 "incomplete matrix fails the coverage check" "$dir/incomplete.out" \
        "$TEST_EXECUTOR" --no-audit trace-matrix "$TRACE_DIR/incomplete" \
        --requirements "$TRACE_DIR/requirements.txt" --baseline-id BL-GLOSS-1 \
        --id TM-GLOSS-I -o "$dir/matrix_incomplete.yaml"
    expect_text "$dir/incomplete.out" "HLR-2 (HLR): no robustness test" \
        "coverage check names the requirement and what is missing"
    expect_file "incomplete matrix is still written" "$dir/matrix_incomplete.yaml"

    run_expect 0 "--no-coverage-check accepts the incomplete matrix" "$dir/no_check.out" \
        "$TEST_EXECUTOR" --no-audit trace-matrix "$TRACE_DIR/incomplete" \
        --requirements "$TRACE_DIR/requirements.txt" --baseline-id BL-GLOSS-1 \
        --id TM-GLOSS-I --no-coverage-check -o "$dir/matrix_unchecked.yaml"
}

# ============================================================================
# Scenario 3: regression selection (TCMS-40.7)
# ============================================================================
scenario_regression_selection() {
    section "Regression selection (TCMS-40.7)"
    local dir="$WORK_DIR/regression"
    mkdir -p "$dir"

    run_expect 0 "regression selection with the plan" "$dir/select.out" \
        "$TEST_EXECUTOR" --no-audit regression-select "$REG_DIR/cases" \
        --baseline "$REG_DIR/BL-2.yaml" --plan "$REG_DIR/plan.yaml" -o "$dir/selected.txt"
    printf 'TC_REG_001\nTC_REG_003\n' >"$dir/expected.txt"
    if cmp -s "$dir/expected.txt" "$dir/selected.txt"; then
        record_pass "selects the test case tracing to the impacted requirement and the listed one"
    else
        record_fail "selection differs from TC_REG_001 and TC_REG_003"
        show_output "$dir/selected.txt"
    fi

    run_expect 0 "regression selection as JSON" "$dir/select_json.out" \
        "$TEST_EXECUTOR" --no-audit regression-select "$REG_DIR/cases" \
        --baseline "$REG_DIR/BL-2.yaml" --plan "$REG_DIR/plan.yaml" --format json \
        -o "$dir/selected.json"
    expect_text "$dir/selected.json" "\"reasons\"" "JSON output lists the reasons"
    expect_text "$dir/selected.json" "\"listed\"" "the listed test case is selected as listed"
    expect_text "$dir/selected.json" "\"HLR-1\"" "the requirement-traced test case names HLR-1"

    # Negative: the plan measures the baseline from another previous baseline
    sed 's/previous_baseline_id: BL-1/previous_baseline_id: BL-0/' "$REG_DIR/plan.yaml" \
        >"$dir/plan_other.yaml"
    run_expect 1 "plan with another previous baseline is refused" "$dir/mismatch.out" \
        "$TEST_EXECUTOR" --no-audit regression-select "$REG_DIR/cases" \
        --baseline "$REG_DIR/BL-2.yaml" --plan "$dir/plan_other.yaml"
    expect_text "$dir/mismatch.out" "measures baseline" "refusal explains the mismatch"
}

# ============================================================================
# Scenario 4: independence (TCMS-40.8)
# ============================================================================
scenario_independence_check() {
    section "Independence check (TCMS-40.8)"
    local dir="$WORK_DIR/independence"
    mkdir -p "$dir"

    run_expect 0 "valid independence assignment passes the check" "$dir/valid.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment check "$IND_DIR/ia_valid.yml"

    run_expect 1 "record that makes the author an independent verifier is rejected" \
        "$dir/author.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment check "$IND_DIR/ia_author_is_verifier.yml"
    expect_text "$dir/author.out" "the verifier alice is the author" \
        "rejection names the verifier who is the author"

    run_expect 0 "create an assignment with an independent verifier" "$dir/create.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment create --id IA-GLOSS-1 \
        --objective "Test results reviewed by someone other than the author" \
        --artifact RUN-GLOSS-1 --author alice --verifier bob -o "$dir/ia_bob.yaml"
    expect_line "$dir/ia_bob.yaml" "verifier_is_independent: true" \
        "created record says the verifier is independent"
    run_expect 0 "created assignment passes the check" "$dir/check_bob.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment check "$dir/ia_bob.yaml"

    run_expect 0 "create an assignment where the author verifies" "$dir/create_self.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment create --id IA-GLOSS-2 \
        --objective "Test results reviewed by someone other than the author" \
        --artifact RUN-GLOSS-1 --author alice --verifier alice -o "$dir/ia_self.yaml"
    expect_line "$dir/ia_self.yaml" "verifier_is_independent: false" \
        "record of a self-check says the verifier is not independent"
    run_expect 0 "self-check record passes the check" "$dir/check_self.out" \
        "$TEST_EXECUTOR" --no-audit independence-assignment check "$dir/ia_self.yaml"
}

scenario_runs_with_metadata
scenario_traceability_matrix
scenario_regression_selection
scenario_independence_check

echo "" >&2
section "Glossary entity acceptance summary"
info "Checks passed: $PASSED"
if [[ $FAILED -eq 0 ]]; then
    pass "All glossary entity checks passed"
    exit 0
fi
fail "$FAILED glossary entity check(s) failed"
exit 1
