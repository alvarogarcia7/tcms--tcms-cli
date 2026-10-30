#!/usr/bin/env bash
# End-to-end test of the verifier behaviour changes of TCMS-41 (docs/TESTING_AFTER_PR28.md, section 4).
#
# Runs the real `verifier` (and, for the hook and control cases, the real `test-executor` and a
# generated script) on throw-away test cases and checks:
#
#   1. Empty logs folder                       -> non-zero exit, no report written
#   2. Log that cannot be verified             -> non-zero exit, even with --success-on-completion
#                                                 (unknown test case, source hash missing, unparsable file)
#   3. Test case with zero steps               -> not a pass
#   4. Failed hook                             -> non-zero exit (log produced by a real generated script)
#   5. Manual step without a log entry         -> counts as failed (not executed), exit 1
#   6. Output without --output                 -> report goes to stdout (and only there)
#
# Every case also has a control that must still pass, so a verifier that fails everything cannot
# pass this test. A verdict that is only "failed" (not unverifiable) is still forgiven by
# --success-on-completion; that difference is checked too.
#
# Usage: crates/testcase-manager/tests/integration/test_verifier_behaviour_changes_e2e.sh [--no-remove]
# Env:   VERIFIER_BIN, TEST_EXECUTOR_BIN override the binaries (default: target/{release,debug}, PATH)
# Needs: jq. bash 3.2 compatible.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# The verifier resolves its default --schema relative to the repository root
cd "$PROJECT_ROOT"

VERIFIER_BIN="${VERIFIER_BIN:-$(find_binary "verifier")}"
TEST_EXECUTOR_BIN="${TEST_EXECUTOR_BIN:-$(find_binary "test-executor")}"
for b in "$VERIFIER_BIN" "$TEST_EXECUTOR_BIN"; do
    if [[ -z "$b" || ! -x "$b" ]]; then
        echo "[ERROR] verifier and test-executor are required: cargo build -p verifier -p test-executor" >&2
        exit 1
    fi
done
if ! command -v jq > /dev/null 2>&1; then
    echo "[ERROR] jq is required" >&2
    exit 1
fi

REMOVE_TEMP=1
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-remove) REMOVE_TEMP=0; shift ;;
        *) shift ;;
    esac
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/verifier-behaviour-e2e.XXXXXX")"
setup_cleanup "$WORK"
if [[ "$REMOVE_TEMP" -eq 0 ]]; then
    disable_cleanup
    echo "Keeping $WORK"
fi

PASSED=0
FAILED=0
RC=0

check() { # check DESCRIPTION CONDITION-EXIT-STATUS
    if [[ "$2" -eq 0 ]]; then
        pass "$1"
        PASSED=$((PASSED + 1))
    else
        fail "$1"
        FAILED=$((FAILED + 1))
    fi
}

# ---------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------

# verify DIR [verifier args...]: folder mode on DIR (test cases and logs live in DIR);
# sets RC, and writes DIR.out / DIR.err
verify() {
    local dir="$1"
    shift
    RC=0
    "$VERIFIER_BIN" --folder "$dir" --test-case-dir "$dir" "$@" > "$dir.out" 2> "$dir.err" || RC=$?
}

# case_yaml FILE ID [manual|hook=PATH|zero]
case_yaml() {
    local file="$1" id="$2" kind="${3:-}"
    {
        printf 'type: test_case\nschema: tcms/test-case.schema.v1.json\nrequirement: R\nitem: 1\ntc: 1\n'
        printf 'id: %s\ndescription: behaviour change e2e\ngeneral_initial_conditions: {}\ninitial_conditions: {}\n' "$id"
        case "$kind" in
            hook=*) printf 'hooks:\n  script_start:\n    command: %s\n  on_error: fail\n' "${kind#hook=}" ;;
        esac
        if [[ "$kind" == "zero" ]]; then
            printf 'test_sequences: []\n'
            return
        fi
        printf 'test_sequences:\n- id: 1\n  name: seq1\n  description: d\n  initial_conditions: {}\n  steps:\n'
        printf "  - step: 1\n    description: automated\n    command: echo a\n"
        printf "    expected: {success: true, result: '0', output: a}\n"
        printf "    verification: {result: '[[ \$? -eq 0 ]]', output: 'true'}\n"
        if [[ "$kind" == "manual" ]]; then
            printf "  - step: 2\n    description: by hand\n    manual: true\n    command: echo b\n"
            printf "    expected: {success: true, result: '0', output: b}\n"
            printf "    verification: {result: '[[ \$? -eq 0 ]]', output: 'true'}\n"
        fi
    } > "$file"
}

# log_entry ID STEP OUTPUT
log_entry() {
    printf '{"test_case_id":"%s","test_sequence":1,"step":%s,"command":"echo","exit_code":0,"output":"%s","timestamp":"2026-10-29T10:00:00","result_verification_pass":true,"output_verification_pass":true}' "$1" "$2" "$3"
}

new_dir() {
    mkdir -p "$WORK/$1"
    echo "$WORK/$1"
}

echo "======================================"
echo "Verifier behaviour changes E2E test"
echo "======================================"
echo "verifier:      $VERIFIER_BIN"
echo "test-executor: $TEST_EXECUTOR_BIN"
echo "work dir:      $WORK"

# ---------------------------------------------------------------------------------------------
section "Control: a passing run still passes"
# ---------------------------------------------------------------------------------------------
D=$(new_dir control)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[%s]' "$(log_entry TC_OK 1 a)" > "$D/TC_OK_execution_log.json"
verify "$D"
check "passing log exits 0 (got $RC)" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "1. Empty logs folder -> non-zero exit"
# ---------------------------------------------------------------------------------------------
D=$(new_dir empty)
rm -f "$WORK/empty-report.yaml"
RC=0
"$VERIFIER_BIN" --folder "$D" --test-case-dir "$D" --output "$WORK/empty-report.yaml" > "$WORK/empty.out" 2> "$WORK/empty.err" || RC=$?
check "empty folder exits non-zero (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
check "the error says no execution log files were found" "$(grep -q 'No execution log files' "$WORK/empty.err" && echo 0 || echo 1)"
check "no report is written for an empty folder" "$([[ ! -e "$WORK/empty-report.yaml" ]] && echo 0 || echo 1)"
RC=0
"$VERIFIER_BIN" --folder "$D" --test-case-dir "$D" --success-on-completion > /dev/null 2>&1 || RC=$?
check "empty folder still exits non-zero with --success-on-completion (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "2. Log that cannot be verified -> non-zero exit, even with --success-on-completion"
# ---------------------------------------------------------------------------------------------
# 2a. a log whose test case does not exist
D=$(new_dir unknown-case)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[%s]' "$(log_entry TC_GHOST 1 a)" > "$D/TC_GHOST_execution_log.json"
verify "$D"
check "log of an unknown test case exits non-zero (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
verify "$D" --success-on-completion
check "... and with --success-on-completion (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
check "... the error says the log could not be verified" "$(grep -q 'could not be verified' "$D.err" && echo 0 || echo 1)"

# 2b. a log without source hashes under --verify-source-hash
D=$(new_dir no-hash)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[%s]' "$(log_entry TC_OK 1 a)" > "$D/TC_OK_execution_log.json"
verify "$D" --verify-source-hash
check "log without source hashes exits non-zero with --verify-source-hash (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
verify "$D" --verify-source-hash --success-on-completion
check "... and with --success-on-completion (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"

# 2c. contrast: a plain failed verdict is forgiven by --success-on-completion, an unverifiable log is not
D=$(new_dir failed-verdict)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[%s]' "$(log_entry TC_OK 1 WRONG)" > "$D/TC_OK_execution_log.json"
verify "$D"
check "a failed verdict exits 1 (got $RC)" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)"
verify "$D" --success-on-completion
check "a failed verdict exits 0 with --success-on-completion (got $RC)" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"

# 2d. an unparsable log is never a pass
D=$(new_dir garbage)
case_yaml "$D/TC_OK.yml" TC_OK
printf '{{{{ not json' > "$D/TC_OK_execution_log.json"
verify "$D"
check "an unparsable log exits non-zero (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
verify "$D" --success-on-completion
check "... and with --success-on-completion (got $RC): it is unverifiable, not a failed verdict" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
check "... the error says the log could not be verified" "$(grep -q 'could not be verified' "$D.err" && echo 0 || echo 1)"

# 2e. contrast: a valid JSON array with no entries is a real log of a run that did nothing: a failed verdict
D=$(new_dir empty-array)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[]' > "$D/TC_OK_execution_log.json"
verify "$D"
check "a valid empty JSON array exits 1 (got $RC)" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)"
verify "$D" --success-on-completion
check "... and 0 with --success-on-completion (got $RC): a failed verdict, not unverifiable" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "3. Test case with zero steps -> not a pass"
# ---------------------------------------------------------------------------------------------
D=$(new_dir zero)
case_yaml "$D/TC_ZERO.yml" TC_ZERO zero
printf '[%s]' "$(log_entry TC_ZERO 1 a)" > "$D/TC_ZERO_execution_log.json"
verify "$D" --format json
check "a case with zero steps exits non-zero (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
check "the report has overall_pass=false and total_steps=0" \
    "$(jq -e '.test_results[0] | (.overall_pass == false) and (.total_steps == 0)' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"
check "the batch counts it as failed, not passed" \
    "$(jq -e '.metadata | (.passed_test_cases == 0) and (.failed_test_cases == 1)' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "4. Failed hook -> non-zero exit (real generated script)"
# ---------------------------------------------------------------------------------------------
D=$(new_dir hook)
printf '#!/usr/bin/env bash\necho "hook boom" >&2\nexit 7\n' > "$D/fail_hook.sh"
chmod +x "$D/fail_hook.sh"
case_yaml "$D/TC_HOOK.yml" TC_HOOK "hook=$D/fail_hook.sh"
case_yaml "$D/TC_FINE.yml" TC_FINE
mkdir -p "$D/logs"
for id in TC_HOOK TC_FINE; do
    "$TEST_EXECUTOR_BIN" generate --json-log "$D/$id.yml" -o "$D/$id.sh" > "$D/gen-$id.out" 2>&1 || true
    check "test-executor generates a script for $id" "$([[ -s "$D/$id.sh" ]] && echo 0 || echo 1)"
    SRC_RC=0
    (cd "$D/logs" && bash "$D/$id.sh" > "$D/run-$id.out" 2>&1) || SRC_RC=$?
    if [[ "$id" == "TC_HOOK" ]]; then
        check "the generated script stops with the hook's exit code (got $SRC_RC)" "$([[ $SRC_RC -eq 7 ]] && echo 0 || echo 1)"
    else
        check "the script of the case without hook succeeds (got $SRC_RC)" "$([[ $SRC_RC -eq 0 ]] && echo 0 || echo 1)"
    fi
done
check "the failed hook is in the execution log" \
    "$(jq -e '.[] | select(.hook_type == "script_start" and .exit_code == 7)' "$D/logs/TC_HOOK_execution_log.json" > /dev/null 2>&1 && echo 0 || echo 1)"

# the case with the failed hook alone
mkdir -p "$D/only-hook"
cp "$D/TC_HOOK.yml" "$D/logs/TC_HOOK_execution_log.json" "$D/only-hook/"
verify "$D/only-hook" --format json
check "a failed hook exits non-zero (got $RC)" "$([[ $RC -ne 0 ]] && echo 0 || echo 1)"
check "the error names the test case whose hook failed" "$(grep -q 'a hook failed: TC_HOOK' "$D/only-hook.err" && echo 0 || echo 1)"
check "the report still lists the case" "$(jq -e '.test_results[] | select(.test_case_id == "TC_HOOK")' "$D/only-hook.out" > /dev/null 2>&1 && echo 0 || echo 1)"

# the same run without a hook is fine
mkdir -p "$D/only-fine"
cp "$D/TC_FINE.yml" "$D/logs/TC_FINE_execution_log.json" "$D/only-fine/"
verify "$D/only-fine"
check "the case without a hook verifies and exits 0 (got $RC)" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "5. Manual step without a log entry -> counts as failed"
# ---------------------------------------------------------------------------------------------
D=$(new_dir manual-missing)
case_yaml "$D/TC_MAN.yml" TC_MAN manual
printf '[%s]' "$(log_entry TC_MAN 1 a)" > "$D/TC_MAN_execution_log.json"
verify "$D" --format json
check "a manual step without a log entry exits 1 (got $RC)" "$([[ $RC -eq 1 ]] && echo 0 || echo 1)"
check "the case is not passed (overall_pass=false)" \
    "$(jq -e '.test_results[0].overall_pass == false' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"
check "step 1 passed and the manual step was not executed (2 steps, 1 passed, 1 not executed)" \
    "$(jq -e '.test_results[0] | (.total_steps == 2) and (.passed_steps == 1) and (.not_executed_steps == 1)' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"
check "the batch counts the case as failed" \
    "$(jq -e '.metadata | (.passed_test_cases == 0) and (.failed_test_cases == 1)' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"

D=$(new_dir manual-present)
case_yaml "$D/TC_MAN.yml" TC_MAN manual
printf '[%s,%s]' "$(log_entry TC_MAN 1 a)" "$(log_entry TC_MAN 2 b)" > "$D/TC_MAN_execution_log.json"
verify "$D" --format json
check "the same case with a log entry for the manual step passes (got $RC)" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"
check "... with both steps passed" \
    "$(jq -e '.test_results[0] | (.overall_pass == true) and (.passed_steps == 2)' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "6. Output without --output -> report goes to stdout"
# ---------------------------------------------------------------------------------------------
D=$(new_dir stdout)
case_yaml "$D/TC_OK.yml" TC_OK
printf '[%s]' "$(log_entry TC_OK 1 a)" > "$D/TC_OK_execution_log.json"
verify "$D" --format json --log-level error
check "the run succeeds (got $RC)" "$([[ $RC -eq 0 ]] && echo 0 || echo 1)"
check "stdout is a JSON report for the test case, whatever the log level" \
    "$(jq -e '.test_results[0].test_case_id == "TC_OK"' "$D.out" > /dev/null 2>&1 && echo 0 || echo 1)"
verify "$D"
check "the default format (YAML) is on stdout too" "$(grep -q '^test_results:' "$D.out" && echo 0 || echo 1)"
check "log lines do not leak into the report on stdout" "$(! grep -q ' INFO ' "$D.out" && echo 0 || echo 1)"

rm -f "$D/report.json"
verify "$D" --format json --output "$D/report.json"
check "with --output the report is written to the file" "$(jq -e '.test_results[0].test_case_id == "TC_OK"' "$D/report.json" > /dev/null 2>&1 && echo 0 || echo 1)"
check "... and stdout stays empty" "$([[ ! -s "$D.out" ]] && echo 0 || echo 1)"

# ---------------------------------------------------------------------------------------------
section "Summary"
# ---------------------------------------------------------------------------------------------
echo "Passed: $PASSED"
echo "Failed: $FAILED"
if [[ "$FAILED" -gt 0 ]]; then
    fail "$FAILED check(s) failed"
    exit 1
fi
pass "All $PASSED checks passed"
