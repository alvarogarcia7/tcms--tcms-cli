#!/usr/bin/env bash
#
# validate_jira_integration.sh - Acceptance scenarios for the Jira integration (TCMS-42)
#
# DESCRIPTION:
#   Runs the real `tcms` binary against standalone mock Jira servers (`mock-jira`, started and
#   stopped by this script) and against the StrictDoc export fixture, for three products. Each
#   product is one full cycle: import the features, run (the run record of the fixture; the
#   verifier's output form), sync, verify the mock state.
#     Product 1 (Jira requirement source, plain Jira test entities; mock 1):
#       1. test-connection with the service account succeeds
#       2. requirements import writes one record per feature
#       3. a sync without --apply is a dry run: it writes nothing and says so
#       4. sync --apply creates the test entity of each case, writes its lifecycle status and
#          the verdict of its run, records the keys in the sync state and in the test case
#       5. every audit entry of the run names the acting user (EXPECT_WHO)
#       6. the mock holds the lifecycle and the verdict TCMS wrote
#       7. the problem report got its defect key, and a second sync creates no new entity
#       8. the defect is never pushed twice; the token appears in no record, state, audit or output
#     Product 2 (StrictDoc requirement source, no Jira system):
#       9. requirements import from the StrictDoc export writes the records
#      10. a sync skips the Jira steps, plans the import, writes no state
#     Product 3 (Jira requirement source, Xray test entities; mock 2; the Xray API is an
#     ASSUMPTION from public documentation, modelled by the jira-client mock):
#      11. test-connection succeeds; 12. requirements import writes the records
#      13. a dry run writes nothing; 14. sync --apply records the Xray key in the test case
#      15. the mock holds one Test Execution for the run, with PASS for the passed case
#      16. the mock holds the lifecycle and the verdict on the Test issue
#      17. a second sync reuses the same Test Execution (no second one is created)
#      18. every audit entry names the acting user; the token appears nowhere
#     Product 4 (Jira requirement source, Zephyr Scale test entities; mock 3, Zephyr Scale on
#     the mock's /v2 routes; the Zephyr Scale API is an ASSUMPTION):
#      19. test-connection succeeds; 20. requirements import writes the records
#      21. a dry run writes nothing; 22. sync --apply records the Zephyr key in the record
#      23. the mock holds the Zephyr test case with the lifecycle written by TCMS
#      24. the mock holds one cycle for the run and the execution of the passed case (Pass)
#      25. a second sync creates no test case, cycle or execution
#      26. audit entries name the acting user; the token appears nowhere
#     Product 5 (Jira requirement source, Zephyr Squad test entities; mock 4, zapi routes of
#     the mock; the Zephyr Squad API is an ASSUMPTION):
#      27. test-connection succeeds; 28. requirements import writes the records
#      29. a dry run writes nothing; 30. sync --apply records the Squad key in the record
#      31. the mock holds one cycle for the run, with one execution per case
#      32. a second sync creates no test issue, cycle or execution
#      33. audit entries name the acting user; the token appears nowhere
#   Verified against the mock only (see docs/JIRA_INTEGRATION.md).
#
# USAGE:
#   ./test-acceptance/validate_jira_integration.sh [OPTIONS]
#
# OPTIONS:
#   -v, --verbose   Print the output of every command, not only of failed ones
#   -h, --help      Show this help message
#
# ENVIRONMENT:
#   JIRA_ACCEPTANCE_EXPECT_WHO   The acting user the audit entries must name (default: alice).
#                                Any other value makes checks 5 and 18 fail, which proves the
#                                script fails on a broken expectation.
#
# EXIT CODES:
#   0 - All scenarios passed
#   1 - One or more checks failed, or a required binary is missing
#
# NOTES:
#   Scripts stay bash 3.2 compatible and use only portable options of grep, sed and mktemp.

# Note: no 'set -e': every check runs, so that all failures are reported.

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

FIXTURES="$SCRIPT_DIR/jira_integration/fixtures"
SCHEMAS_ROOT="$REPO_ROOT/schemas/"
EXPECT_WHO="${JIRA_ACCEPTANCE_EXPECT_WHO:-alice}"
TOKEN="acceptance-token-$$"
ACTOR="alice"

WORK_DIR=$(mktemp -d)
setup_cleanup "$WORK_DIR"

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
TCMS=$(locate_binary "tcms")
MOCK_JIRA=$(locate_binary "mock-jira")
for binary in TCMS MOCK_JIRA; do
    if [[ -z "${!binary}" ]]; then
        log_error "Required binary not found for $binary (build it first: cargo build -p tcms -p jira-client --features mock)"
        exit 1
    fi
done

# The binaries live in a shared target directory that another build may rewrite while this run
# is going (TCMS-42.9 item 4). The run uses private copies made now, and logs their checksums so
# that a failure names the exact binaries that ran.
BIN_COPY_DIR="$WORK_DIR/bin"
mkdir -p "$BIN_COPY_DIR" || exit 1
cp "$TCMS" "$BIN_COPY_DIR/tcms" && cp "$MOCK_JIRA" "$BIN_COPY_DIR/mock-jira" || {
    log_error "could not copy the binaries to $BIN_COPY_DIR"
    exit 1
}
TCMS="$BIN_COPY_DIR/tcms"
MOCK_JIRA="$BIN_COPY_DIR/mock-jira"
info "binaries under test (copies): tcms cksum $(cksum < "$TCMS" | cut -d' ' -f1), mock-jira cksum $(cksum < "$MOCK_JIRA" | cut -d' ' -f1)"

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

show_output() {
    sed 's/^/      | /' "$1" | tail -20 >&2
}

# run_in DIR EXPECTED_STATUS DESCRIPTION OUTPUT_FILE COMMAND...
run_in() {
    local dir="$1" expected="$2" description="$3" output="$4"
    shift 4
    local status=0
    (cd "$dir" && "$@") >"$output" 2>&1 || status=$?
    if [[ $status -eq $expected ]]; then
        record_pass "$description"
        [[ $VERBOSE -eq 1 ]] && show_output "$output"
    else
        record_fail "$description (exit $status, expected $expected)"
        show_output "$output"
    fi
}

# expect_text FILE TEXT DESCRIPTION: FILE contains TEXT (fixed string)
expect_text() {
    if grep -qF -- "$2" "$1" 2>/dev/null; then
        record_pass "$3"
    else
        record_fail "$3 (no '$2' in $(basename "$1"))"
        show_output "$1"
    fi
}

# expect_no_text FILE TEXT DESCRIPTION: FILE does not contain TEXT
expect_no_text() {
    if grep -qF -- "$2" "$1" 2>/dev/null; then
        record_fail "$3 (found '$2' in $(basename "$1"))"
    else
        record_pass "$3"
    fi
}

# --- the mock Jira servers: started here, stopped on exit (setup_cleanup kills registered PIDs).
# start_mock LOG: sets LAST_MOCK_URL. One mock per product, so that the products do not share
# test entities.
start_mock() {
    local log="$1"
    MOCK_JIRA_TOKEN="$TOKEN" "$MOCK_JIRA" >"$log" 2>&1 &
    register_background_pid "$!"
    LAST_MOCK_URL=""
    for _ in $(seq 1 100); do
        LAST_MOCK_URL=$(sed -n 's/^MOCK_JIRA_URL=//p' "$log" 2>/dev/null | head -1)
        [[ -n "$LAST_MOCK_URL" ]] && break
        sleep 0.1
    done
    if [[ -z "$LAST_MOCK_URL" ]]; then
        log_error "mock-jira did not start"
        show_output "$log"
        exit 1
    fi
}

MOCK_OUT="$WORK_DIR/mock-jira.out"
start_mock "$MOCK_OUT"
MOCK_URL="$LAST_MOCK_URL"
info "mock Jira at $MOCK_URL"
export TCMS_JIRA_TOKEN="$TOKEN"
export TCMS_USER="$ACTOR"
unset TCMS_JIRA_USER

# ================================================================ product 1: Jira
section "Product 1: Jira requirement source, plain Jira test entities"
J="$WORK_DIR/jira"
mkdir -p "$J/testcases" "$J/test-runs" "$J/problem-reports"
sed "s|__MOCK_JIRA_URL__|$MOCK_URL|" "$FIXTURES/jira_product.yaml" > "$J/integration-config.yaml"
cp "$FIXTURES/jira/testcases/"*.yaml "$J/testcases/"
cp "$FIXTURES/jira/test-runs/"*.yaml "$J/test-runs/"
cp "$FIXTURES/jira/problem-reports/"*.yaml "$J/problem-reports/"
JSYNC=(--config integration-config.yaml --schemas-root "$SCHEMAS_ROOT")

run_in "$J" 0 "1. test-connection succeeds with the service account" "$WORK_DIR/j1.out" \
    "$TCMS" "${JSYNC[@]}" jira test-connection --as "$ACTOR"
expect_text "$WORK_DIR/j1.out" "accessible" "1. the project PAY is accessible"

run_in "$J" 0 "2. requirements import from Jira" "$WORK_DIR/j2.out" \
    "$TCMS" "${JSYNC[@]}" requirements import --out requirements --as "$ACTOR"
if [[ -f "$J/requirements/PAY-1.yaml" && -f "$J/requirements/PAY-2.yaml" ]]; then
    record_pass "2. one record per feature (PAY-1, PAY-2)"
else
    record_fail "2. one record per feature (PAY-1, PAY-2)"
fi

run_in "$J" 0 "3. a sync without --apply is a dry run" "$WORK_DIR/j3.out" \
    "$TCMS" "${JSYNC[@]}" jira sync --product PROD-JIRA-ACC --as "$ACTOR" \
        --state-dir state --only tests
expect_text "$WORK_DIR/j3.out" "dry run: nothing written" "3. the dry run says it wrote nothing"
if [[ ! -e "$J/state" ]]; then record_pass "3. the dry run wrote no state"; else record_fail "3. the dry run wrote no state"; fi

run_in "$J" 0 "4. sync --apply of the product" "$WORK_DIR/j4.out" \
    "$TCMS" "${JSYNC[@]}" jira sync --product PROD-JIRA-ACC --apply --as "$ACTOR" --state-dir state
expect_text "$WORK_DIR/j4.out" "write(s) applied" "4. the summary reports the applied writes"
expect_text "$J/state/state.yaml" "TC_TRACE_001" "4. the state names the test case"
TEST_KEY=$(sed -n '/TC_TRACE_001:/,/key:/s/.*key: //p' "$J/state/state.yaml" | head -1)
if [[ -n "$TEST_KEY" ]]; then record_pass "4. the test entity key is recorded ($TEST_KEY)"; else record_fail "4. the test entity key is recorded"; fi

# 4a. the test case record holds the entity key (targeted edit, system jira)
if grep -q "^external_refs:" "$J/testcases/TC_TRACE_001.yml" && grep -q "system: jira" "$J/testcases/TC_TRACE_001.yml"; then
    record_pass "4a. the test case record holds its Jira entity in external_refs"
else
    record_fail "4a. the test case record holds its Jira entity in external_refs"
fi

# 5. the audit entries name the acting user (EXPECT_WHO: a broken expectation fails here)
AUDIT_DIRS=("$J"/state/audit/*)
AUDIT_COUNT=0
AUDIT_OK=0
for f in "${AUDIT_DIRS[@]}"/*.yaml; do
    [[ -f "$f" ]] || continue
    AUDIT_COUNT=$((AUDIT_COUNT + 1))
    grep -qF "who: $EXPECT_WHO" "$f" && AUDIT_OK=$((AUDIT_OK + 1))
done
if [[ $AUDIT_COUNT -gt 0 && $AUDIT_OK -eq $AUDIT_COUNT ]]; then
    record_pass "5. all $AUDIT_COUNT audit entries name the acting user ($EXPECT_WHO)"
else
    record_fail "5. audit entries name the acting user $EXPECT_WHO ($AUDIT_OK of $AUDIT_COUNT do)"
fi

# 6. the mock holds the lifecycle value TCMS wrote
if [[ -n "$TEST_KEY" ]]; then
    curl -s -H "Authorization: Bearer $TOKEN" "$MOCK_URL/rest/api/3/issue/$TEST_KEY" > "$WORK_DIR/issue.json"
    expect_text "$WORK_DIR/issue.json" '"customfield_10002":"draft"' "6. the mock holds the lifecycle written by TCMS"
    expect_text "$WORK_DIR/issue.json" '"customfield_10003":"passed"' "6. the mock holds the verdict of the run (passed)"
else
    record_fail "6. the mock holds the lifecycle written by TCMS (no key)"
fi

# 7. a second sync creates no new entity; the defect was pushed and its key recorded
run_in "$J" 0 "7. a second sync --apply succeeds" "$WORK_DIR/j7.out" \
    "$TCMS" "${JSYNC[@]}" jira sync --product PROD-JIRA-ACC --apply --as "$ACTOR" --state-dir state
expect_text "$WORK_DIR/j7.out" "TC_TRACE_001 -> $TEST_KEY" "7. the same test entity is found again"
if grep -q "key: PAY-" "$J/problem-reports/PR-007.yaml"; then
    record_pass "7. the problem report holds its defect key"
else
    record_fail "7. the problem report holds its defect key"
fi
expect_no_text "$WORK_DIR/j7.out" "write: problem report" "7. the defect is not pushed a second time"

# 8. the token appears nowhere: records, state, audit, output
SECRET_HITS=$(grep -rl -- "$TOKEN" "$J" "$WORK_DIR"/j*.out 2>/dev/null | wc -l | tr -d ' ')
if [[ "$SECRET_HITS" -eq 0 ]]; then
    record_pass "8. the token is in no record, state, audit file or output"
else
    record_fail "8. the token is in $SECRET_HITS file(s) of the run"
fi

# ================================================================ product 2: StrictDoc
section "Product 2: StrictDoc requirement source, no Jira system"
S="$WORK_DIR/strictdoc"
mkdir -p "$S"
cp "$FIXTURES/strictdoc_product.yaml" "$S/integration-config.yaml"
cp "$FIXTURES/strictdoc_export.json" "$S/export.json"
SSYNC=(--config integration-config.yaml --schemas-root "$SCHEMAS_ROOT")

run_in "$S" 0 "9. requirements import from the StrictDoc export" "$WORK_DIR/s9.out" \
    "$TCMS" "${SSYNC[@]}" requirements import --input export.json --commit abc1234 --out requirements
COUNT=$(ls "$S/requirements" 2>/dev/null | grep -c '\.yaml$')
if [[ "$COUNT" -gt 0 ]]; then record_pass "9. $COUNT StrictDoc record(s) written"; else record_fail "9. StrictDoc records written"; fi

run_in "$S" 0 "10. a sync skips the Jira steps and writes no state" "$WORK_DIR/s10.out" \
    "$TCMS" "${SSYNC[@]}" jira sync --product PROD-SDOC-ACC --as "$ACTOR" --state-dir state \
        --strictdoc-input export.json --commit abc1234
expect_text "$WORK_DIR/s10.out" "no Jira system" "10. the Jira steps are reported as skipped"
expect_text "$WORK_DIR/s10.out" "dry run: nothing written" "10. the sync is a dry run without --apply"
if [[ ! -e "$S/state" ]]; then record_pass "10. no state for a StrictDoc product"; else record_fail "10. no state for a StrictDoc product"; fi

# ================================================================ product 3: Xray
section "Product 3: Jira requirement source, Xray test entities (assumed API, mock 2)"
start_mock "$WORK_DIR/xray-mock.out"
XRAY_URL="$LAST_MOCK_URL"
info "mock Jira for Xray at $XRAY_URL"
X="$WORK_DIR/xray"
mkdir -p "$X/testcases" "$X/test-runs" "$X/problem-reports"
sed "s|__MOCK_JIRA_URL__|$XRAY_URL|" "$FIXTURES/xray_product.yaml" > "$X/integration-config.yaml"
cp "$FIXTURES/jira/testcases/"*.yaml "$X/testcases/"
cp "$FIXTURES/jira/test-runs/"*.yaml "$X/test-runs/"
cp "$FIXTURES/jira/problem-reports/"*.yaml "$X/problem-reports/"
XSYNC=(--config integration-config.yaml --schemas-root "$SCHEMAS_ROOT")

run_in "$X" 0 "11. test-connection succeeds with the service account" "$WORK_DIR/x11.out" \
    "$TCMS" "${XSYNC[@]}" jira test-connection --as "$ACTOR"
expect_text "$WORK_DIR/x11.out" "accessible" "11. the project PAY is accessible"

run_in "$X" 0 "12. requirements import from Jira" "$WORK_DIR/x12.out" \
    "$TCMS" "${XSYNC[@]}" requirements import --out requirements --as "$ACTOR"
if [[ -f "$X/requirements/PAY-1.yaml" && -f "$X/requirements/PAY-2.yaml" ]]; then
    record_pass "12. one record per feature (PAY-1, PAY-2)"
else
    record_fail "12. one record per feature (PAY-1, PAY-2)"
fi

run_in "$X" 0 "13. a dry run writes nothing" "$WORK_DIR/x13.out" \
    "$TCMS" "${XSYNC[@]}" jira sync --product PROD-XRAY-ACC --as "$ACTOR" --state-dir state --only tests
expect_text "$WORK_DIR/x13.out" "dry run: nothing written" "13. the dry run says it wrote nothing"
if [[ ! -e "$X/state" ]]; then record_pass "13. the dry run wrote no state"; else record_fail "13. the dry run wrote no state"; fi

run_in "$X" 0 "14. sync --apply of the Xray product" "$WORK_DIR/x14.out" \
    "$TCMS" "${XSYNC[@]}" jira sync --product PROD-XRAY-ACC --apply --as "$ACTOR" --state-dir state --only tests,features,defects
expect_text "$WORK_DIR/x14.out" "write(s) applied" "14. the summary reports the applied writes"
X_TEST_KEY=$(sed -n '/TC_TRACE_001:/,/key:/s/.*key: //p' "$X/state/state.yaml" | head -1)
if [[ -n "$X_TEST_KEY" ]]; then record_pass "14. the Xray test entity key is recorded in the sync state ($X_TEST_KEY)"; else record_fail "14. the Xray test entity key is recorded in the sync state"; fi
expect_text "$X/testcases/TC_TRACE_001.yml" "system: xray" "14. the test case record holds the Xray entity (system xray)"
if [[ -n "$X_TEST_KEY" ]]; then
    expect_text "$X/testcases/TC_TRACE_001.yml" "key: $X_TEST_KEY" "14. the record holds the same key as the sync state"
fi

# the Test Execution of the run: found by a JQL search of the mock (the run id is its summary)
x_executions() {
    curl -s -G -H "Authorization: Bearer $TOKEN" \
        --data-urlencode "jql=project = \"PAY\" AND issuetype = \"Test Execution\"" \
        "$XRAY_URL/rest/api/3/search/jql" > "$WORK_DIR/xray-search.json"
    grep -o '"key":"PAY-[0-9]*"' "$WORK_DIR/xray-search.json" | sed 's/"key":"//; s/"//'
}
X_EXECS=$(x_executions)
X_EXEC=$(echo "$X_EXECS" | head -1)
X_EXEC_COUNT=$(echo "$X_EXECS" | grep -c . || true)
if [[ "$X_EXEC_COUNT" -eq 1 && -n "$X_EXEC" ]]; then
    record_pass "15. the mock holds one Test Execution for the run ($X_EXEC)"
else
    record_fail "15. the mock holds one Test Execution for the run (found $X_EXEC_COUNT)"
fi
curl -s -H "Authorization: Bearer $TOKEN" "$XRAY_URL/rest/raven/1.0/api/testexec/$X_EXEC" > "$WORK_DIR/xray-exec.json"
if [[ -n "$X_TEST_KEY" ]]; then
    expect_text "$WORK_DIR/xray-exec.json" "\"key\":\"$X_TEST_KEY\",\"status\":\"PASS\"" "15. the execution holds the passed case with status PASS"
fi

# 16. the Test issue holds the lifecycle and the latest verdict
if [[ -n "$X_TEST_KEY" ]]; then
    curl -s -H "Authorization: Bearer $TOKEN" "$XRAY_URL/rest/api/3/issue/$X_TEST_KEY" > "$WORK_DIR/xray-issue.json"
    expect_text "$WORK_DIR/xray-issue.json" '"customfield_10002":"draft"' "16. the Test issue holds the lifecycle written by TCMS"
    expect_text "$WORK_DIR/xray-issue.json" '"customfield_10003":"passed"' "16. the Test issue holds the verdict of the run (passed)"
fi

# 17. a second sync records the run in the same execution
run_in "$X" 0 "17. a second sync --apply succeeds" "$WORK_DIR/x17.out" \
    "$TCMS" "${XSYNC[@]}" jira sync --product PROD-XRAY-ACC --apply --as "$ACTOR" --state-dir state --only tests,features,defects
X_EXEC_COUNT_2=$(x_executions | grep -c . || true)
if [[ "$X_EXEC_COUNT_2" -eq 1 ]]; then
    record_pass "17. still one Test Execution after the second sync (no new one per sync)"
else
    record_fail "17. still one Test Execution after the second sync (found $X_EXEC_COUNT_2)"
fi

# 18. audit entries name the acting user; the token appears nowhere in the Xray run
X_AUDIT_COUNT=0
X_AUDIT_OK=0
for f in "$X"/state/audit/*/*.yaml; do
    [[ -f "$f" ]] || continue
    X_AUDIT_COUNT=$((X_AUDIT_COUNT + 1))
    grep -qF "who: $EXPECT_WHO" "$f" && X_AUDIT_OK=$((X_AUDIT_OK + 1))
done
if [[ $X_AUDIT_COUNT -gt 0 && $X_AUDIT_OK -eq $X_AUDIT_COUNT ]]; then
    record_pass "18. all $X_AUDIT_COUNT Xray audit entries name the acting user ($EXPECT_WHO)"
else
    record_fail "18. Xray audit entries name the acting user $EXPECT_WHO ($X_AUDIT_OK of $X_AUDIT_COUNT do)"
fi
X_SECRET_HITS=$(grep -rl -- "$TOKEN" "$X" "$WORK_DIR"/x*.out "$WORK_DIR/xray-search.json" "$WORK_DIR/xray-exec.json" 2>/dev/null | wc -l | tr -d ' ')
if [[ "$X_SECRET_HITS" -eq 0 ]]; then
    record_pass "18. the token is in no Xray record, state, audit file or output"
else
    record_fail "18. the token is in $X_SECRET_HITS Xray file(s)"
fi

# ================================================================ product 4: Zephyr Scale
section "Product 4: Jira requirement source, Zephyr Scale test entities (assumed API, mock 3)"
start_mock "$WORK_DIR/zscale-mock.out"
ZS_URL="$LAST_MOCK_URL"
info "mock Jira and Zephyr Scale at $ZS_URL (Zephyr Scale API at $ZS_URL/v2)"
Z="$WORK_DIR/zscale"
mkdir -p "$Z/testcases" "$Z/test-runs" "$Z/problem-reports"
sed "s|__MOCK_JIRA_URL__|$ZS_URL|" "$FIXTURES/zephyr_scale_product.yaml" > "$Z/integration-config.yaml"
cp "$FIXTURES/jira/testcases/"*.yaml "$Z/testcases/"
cp "$FIXTURES/jira/test-runs/"*.yaml "$Z/test-runs/"
cp "$FIXTURES/jira/problem-reports/"*.yaml "$Z/problem-reports/"
ZSYNC=(--config integration-config.yaml --schemas-root "$SCHEMAS_ROOT")

run_in "$Z" 0 "19. test-connection succeeds with the service account" "$WORK_DIR/z19.out" \
    "$TCMS" "${ZSYNC[@]}" jira test-connection --as "$ACTOR"
expect_text "$WORK_DIR/z19.out" "accessible" "19. the project PAY is accessible"

run_in "$Z" 0 "20. requirements import from Jira" "$WORK_DIR/z20.out" \
    "$TCMS" "${ZSYNC[@]}" requirements import --out requirements --as "$ACTOR"
if [[ -f "$Z/requirements/PAY-1.yaml" && -f "$Z/requirements/PAY-2.yaml" ]]; then
    record_pass "20. one record per feature (PAY-1, PAY-2)"
else
    record_fail "20. one record per feature (PAY-1, PAY-2)"
fi

run_in "$Z" 0 "21. a dry run writes nothing" "$WORK_DIR/z21.out" \
    "$TCMS" "${ZSYNC[@]}" jira sync --product PROD-ZSCALE-ACC --as "$ACTOR" --state-dir state --only tests
expect_text "$WORK_DIR/z21.out" "dry run: nothing written" "21. the dry run says it wrote nothing"
if [[ ! -e "$Z/state" ]]; then record_pass "21. the dry run wrote no state"; else record_fail "21. the dry run wrote no state"; fi

run_in "$Z" 0 "22. sync --apply of the Zephyr Scale product" "$WORK_DIR/z22.out" \
    env TCMS_ZEPHYR_API_URL="$ZS_URL/v2" "$TCMS" "${ZSYNC[@]}" jira sync --product PROD-ZSCALE-ACC --apply --as "$ACTOR" --state-dir state --only tests
expect_text "$WORK_DIR/z22.out" "write(s) applied" "22. the summary reports the applied writes"
ZS_KEY=$(sed -n '/TC_TRACE_001:/,/key:/s/.*key: //p' "$Z/state/state.yaml" | head -1)
if [[ -n "$ZS_KEY" ]]; then record_pass "22. the Zephyr Scale test key is recorded in the sync state ($ZS_KEY)"; else record_fail "22. the Zephyr Scale test key is recorded in the sync state"; fi
expect_text "$Z/testcases/TC_TRACE_001.yml" "system: zephyr_scale" "22. the test case record holds the Zephyr Scale entity (system zephyr_scale)"
if [[ -n "$ZS_KEY" ]]; then
    expect_text "$Z/testcases/TC_TRACE_001.yml" "key: $ZS_KEY" "22. the record holds the same key as the sync state"
fi

# 23. the Zephyr Scale test case holds the lifecycle written by TCMS
if [[ -n "$ZS_KEY" ]]; then
    curl -s -H "Authorization: Bearer $TOKEN" "$ZS_URL/v2/testcases/$ZS_KEY" > "$WORK_DIR/zscale-case.json"
    expect_text "$WORK_DIR/zscale-case.json" '"statusName":"Draft"' "23. the Zephyr Scale test case holds the lifecycle written by TCMS (Draft)"
else
    record_fail "23. the Zephyr Scale test case holds the lifecycle written by TCMS (no key)"
fi

# the cases, cycles and executions of the mock (listings; the run id is the cycle name)
zs_cases() {
    curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectKey=PAY" "$ZS_URL/v2/testcases" \
        | grep -o '"key":"PAY-T[0-9]*"' | wc -l | tr -d ' '
}
zs_cycle_key() {
    curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectKey=PAY" "$ZS_URL/v2/testcycles" \
        | grep -o '"key":"PAY-C[0-9]*","name":"RUN-ACC-1"' | head -1 | sed 's/"key":"\(PAY-C[0-9]*\)".*/\1/'
}
zs_execs() {
    curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectKey=PAY" --data-urlencode "testCycleKey=$1" "$ZS_URL/v2/testexecutions"
}
ZS_CYCLE=$(zs_cycle_key)
ZS_CYCLES=$(curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectKey=PAY" "$ZS_URL/v2/testcycles" | grep -o '"name":"RUN-ACC-1"' | wc -l | tr -d ' ')
ZS_EXECS=$(zs_execs "$ZS_CYCLE" | grep -o '"statusName":"Pass"' | wc -l | tr -d ' ')
if [[ "$ZS_CYCLES" -eq 1 ]]; then
    record_pass "24. the mock holds one cycle for the run (RUN-1)"
else
    record_fail "24. the mock holds one cycle for the run (found $ZS_CYCLES)"
fi
if [[ "$ZS_EXECS" -eq 1 ]]; then
    record_pass "24. the cycle holds the execution of the passed case with status Pass"
else
    record_fail "24. the cycle holds the execution of the passed case with status Pass (found $ZS_EXECS)"
fi
ZS_CASES_BEFORE=$(zs_cases)
ZS_EXECS_BEFORE=$(zs_execs "$ZS_CYCLE" | grep -o '"id":' | wc -l | tr -d ' ')

# 25. a second sync creates no test case, cycle or execution
run_in "$Z" 0 "25. a second sync --apply succeeds" "$WORK_DIR/z25.out" \
    env TCMS_ZEPHYR_API_URL="$ZS_URL/v2" "$TCMS" "${ZSYNC[@]}" jira sync --product PROD-ZSCALE-ACC --apply --as "$ACTOR" --state-dir state --only tests
ZS_CASES_AFTER=$(zs_cases)
ZS_CYCLES_AFTER=$(curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectKey=PAY" "$ZS_URL/v2/testcycles" | grep -o '"name":"RUN-ACC-1"' | wc -l | tr -d ' ')
ZS_EXECS_AFTER=$(zs_execs "$ZS_CYCLE" | grep -o '"id":' | wc -l | tr -d ' ')
if [[ "$ZS_CASES_AFTER" -eq "$ZS_CASES_BEFORE" && "$ZS_CYCLES_AFTER" -eq 1 && "$ZS_EXECS_AFTER" -eq "$ZS_EXECS_BEFORE" ]]; then
    record_pass "25. the second sync creates no Zephyr Scale test case, cycle or execution"
else
    record_fail "25. the second sync creates no Zephyr Scale object (cases $ZS_CASES_BEFORE -> $ZS_CASES_AFTER, cycles $ZS_CYCLES_AFTER, executions $ZS_EXECS_BEFORE -> $ZS_EXECS_AFTER)"
fi

# 26. audit entries name the acting user; the token appears nowhere in the Zephyr Scale run
Z_AUDIT_COUNT=0
Z_AUDIT_OK=0
for f in "$Z"/state/audit/*/*.yaml; do
    [[ -f "$f" ]] || continue
    Z_AUDIT_COUNT=$((Z_AUDIT_COUNT + 1))
    grep -qF "who: $EXPECT_WHO" "$f" && Z_AUDIT_OK=$((Z_AUDIT_OK + 1))
done
if [[ $Z_AUDIT_COUNT -gt 0 && $Z_AUDIT_OK -eq $Z_AUDIT_COUNT ]]; then
    record_pass "26. all $Z_AUDIT_COUNT Zephyr Scale audit entries name the acting user ($EXPECT_WHO)"
else
    record_fail "26. Zephyr Scale audit entries name the acting user $EXPECT_WHO ($Z_AUDIT_OK of $Z_AUDIT_COUNT do)"
fi
Z_SECRET_HITS=$(grep -rl -- "$TOKEN" "$Z" "$WORK_DIR"/z*.out "$WORK_DIR/zscale-case.json" 2>/dev/null | wc -l | tr -d ' ')
if [[ "$Z_SECRET_HITS" -eq 0 ]]; then
    record_pass "26. the token is in no Zephyr Scale record, state, audit file or output"
else
    record_fail "26. the token is in $Z_SECRET_HITS Zephyr Scale file(s)"
fi

# ================================================================ product 5: Zephyr Squad
section "Product 5: Jira requirement source, Zephyr Squad test entities (assumed API, mock 4)"
start_mock "$WORK_DIR/zsquad-mock.out"
SQ_URL="$LAST_MOCK_URL"
info "mock Jira for Zephyr Squad at $SQ_URL"
Q="$WORK_DIR/zsquad"
mkdir -p "$Q/testcases" "$Q/test-runs" "$Q/problem-reports"
sed "s|__MOCK_JIRA_URL__|$SQ_URL|" "$FIXTURES/zephyr_squad_product.yaml" > "$Q/integration-config.yaml"
cp "$FIXTURES/jira/testcases/"*.yaml "$Q/testcases/"
cp "$FIXTURES/jira/test-runs/"*.yaml "$Q/test-runs/"
cp "$FIXTURES/jira/problem-reports/"*.yaml "$Q/problem-reports/"
QSYNC=(--config integration-config.yaml --schemas-root "$SCHEMAS_ROOT")

run_in "$Q" 0 "27. test-connection succeeds with the service account" "$WORK_DIR/q27.out" \
    "$TCMS" "${QSYNC[@]}" jira test-connection --as "$ACTOR"
expect_text "$WORK_DIR/q27.out" "accessible" "27. the project PAY is accessible"

run_in "$Q" 0 "28. requirements import from Jira" "$WORK_DIR/q28.out" \
    "$TCMS" "${QSYNC[@]}" requirements import --out requirements --as "$ACTOR"
if [[ -f "$Q/requirements/PAY-1.yaml" && -f "$Q/requirements/PAY-2.yaml" ]]; then
    record_pass "28. one record per feature (PAY-1, PAY-2)"
else
    record_fail "28. one record per feature (PAY-1, PAY-2)"
fi

run_in "$Q" 0 "29. a dry run writes nothing" "$WORK_DIR/q29.out" \
    "$TCMS" "${QSYNC[@]}" jira sync --product PROD-ZSQUAD-ACC --as "$ACTOR" --state-dir state --only tests
expect_text "$WORK_DIR/q29.out" "dry run: nothing written" "29. the dry run says it wrote nothing"
if [[ ! -e "$Q/state" ]]; then record_pass "29. the dry run wrote no state"; else record_fail "29. the dry run wrote no state"; fi

run_in "$Q" 0 "30. sync --apply of the Zephyr Squad product" "$WORK_DIR/q30.out" \
    "$TCMS" "${QSYNC[@]}" jira sync --product PROD-ZSQUAD-ACC --apply --as "$ACTOR" --state-dir state --only tests
expect_text "$WORK_DIR/q30.out" "write(s) applied" "30. the summary reports the applied writes"
SQ_KEY=$(sed -n '/TC_TRACE_001:/,/key:/s/.*key: //p' "$Q/state/state.yaml" | head -1)
if [[ -n "$SQ_KEY" ]]; then record_pass "30. the Squad test issue key is recorded in the sync state ($SQ_KEY)"; else record_fail "30. the Squad test issue key is recorded in the sync state"; fi
expect_text "$Q/testcases/TC_TRACE_001.yml" "system: zephyr_squad" "30. the test case record holds the Squad entity (system zephyr_squad)"
if [[ -n "$SQ_KEY" ]]; then
    expect_text "$Q/testcases/TC_TRACE_001.yml" "key: $SQ_KEY" "30. the record holds the same key as the sync state"
fi

# the cycles and executions of the mock: the cycle listing is keyed by cycle id; the run of the
# fixture is RUN-ACC-1 (one test case, TC_TRACE_001)
sq_cycles() {
    curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectId=10000" "$SQ_URL/rest/zapi/latest/cycle" \
        | grep -o '"name":"RUN-ACC-1"' | wc -l | tr -d ' '
}
sq_executions() {
    local cycle_id
    cycle_id=$(curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "projectId=10000" "$SQ_URL/rest/zapi/latest/cycle" \
        | grep -o '"id":[0-9]*,"name":"RUN-ACC-1"' | head -1 | grep -o '[0-9]*' | head -1)
    curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "cycleId=$cycle_id" "$SQ_URL/rest/zapi/latest/execution" \
        | grep -o '"issueId":"[0-9]*"' | wc -l | tr -d ' '
}
SQ_CYCLES=$(sq_cycles)
SQ_EXECS=$(sq_executions)
if [[ "$SQ_CYCLES" -eq 1 && "$SQ_EXECS" -eq 1 ]]; then
    record_pass "31. the mock holds one cycle for the run with one execution per case (1)"
else
    record_fail "31. the mock holds one cycle with one execution per case (cycles $SQ_CYCLES, executions $SQ_EXECS)"
fi
SQ_ISSUES_BEFORE=$(curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "jql=project = \"PAY\" AND issuetype = \"Test\"" "$SQ_URL/rest/api/3/search/jql" | grep -o '"key":"PAY-[0-9]*"' | wc -l | tr -d ' ')

# 32. a second sync creates no test issue, cycle or execution
run_in "$Q" 0 "32. a second sync --apply succeeds" "$WORK_DIR/q32.out" \
    "$TCMS" "${QSYNC[@]}" jira sync --product PROD-ZSQUAD-ACC --apply --as "$ACTOR" --state-dir state --only tests
SQ_ISSUES_AFTER=$(curl -s -G -H "Authorization: Bearer $TOKEN" --data-urlencode "jql=project = \"PAY\" AND issuetype = \"Test\"" "$SQ_URL/rest/api/3/search/jql" | grep -o '"key":"PAY-[0-9]*"' | wc -l | tr -d ' ')
SQ_CYCLES_AFTER=$(sq_cycles)
SQ_EXECS_AFTER=$(sq_executions)
if [[ "$SQ_ISSUES_AFTER" -eq "$SQ_ISSUES_BEFORE" && "$SQ_CYCLES_AFTER" -eq 1 && "$SQ_EXECS_AFTER" -eq 1 ]]; then
    record_pass "32. the second sync creates no Squad test issue, cycle or execution"
else
    record_fail "32. the second sync creates no Squad object (issues $SQ_ISSUES_BEFORE -> $SQ_ISSUES_AFTER, cycles $SQ_CYCLES_AFTER, executions $SQ_EXECS_AFTER)"
fi

# 33. audit entries name the acting user; the token appears nowhere in the Squad run
Q_AUDIT_COUNT=0
Q_AUDIT_OK=0
for f in "$Q"/state/audit/*/*.yaml; do
    [[ -f "$f" ]] || continue
    Q_AUDIT_COUNT=$((Q_AUDIT_COUNT + 1))
    grep -qF "who: $EXPECT_WHO" "$f" && Q_AUDIT_OK=$((Q_AUDIT_OK + 1))
done
if [[ $Q_AUDIT_COUNT -gt 0 && $Q_AUDIT_OK -eq $Q_AUDIT_COUNT ]]; then
    record_pass "33. all $Q_AUDIT_COUNT Zephyr Squad audit entries name the acting user ($EXPECT_WHO)"
else
    record_fail "33. Zephyr Squad audit entries name the acting user $EXPECT_WHO ($Q_AUDIT_OK of $Q_AUDIT_COUNT do)"
fi
Q_SECRET_HITS=$(grep -rl -- "$TOKEN" "$Q" "$WORK_DIR"/q*.out 2>/dev/null | wc -l | tr -d ' ')
if [[ "$Q_SECRET_HITS" -eq 0 ]]; then
    record_pass "33. the token is in no Zephyr Squad record, state, audit file or output"
else
    record_fail "33. the token is in $Q_SECRET_HITS Zephyr Squad file(s)"
fi

# ================================================================ summary
echo ""
echo "Jira integration acceptance: $PASSED passed, $FAILED failed"
if [[ $FAILED -gt 0 ]]; then
    log_error "Jira integration acceptance FAILED"
    exit 1
fi
pass "Jira integration acceptance passed"
exit 0
