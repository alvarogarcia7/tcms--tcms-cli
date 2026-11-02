#!/usr/bin/env bash
# End-to-end test of the exit codes of `test-executor report` (plan 05-03, CLI-02, D-04).
#
# Runs the real `test-executor` on throw-away matrices and run records and checks:
#
#   A. clean run                        -> 0 with and without --fail-on-gaps
#   B. gaps, failed and errored results -> 0 without the flag, 2 with it (summary as JSON: 0)
#   C. write-before-exit (D-02)         -> for markdown kinds traceability, summary, gaps and
#                                          problems, and traceability in asciidoc, json and yaml,
#                                          to stdout and to -o FILE: the exit-2 output is
#                                          byte-identical to the exit-0 output, non-empty, has the
#                                          expected first line (json: first byte), and the
#                                          explanatory line is on stderr only
#   D. errors win (D-01)                -> invalid matrix, duplicate run_id, broken YAML run,
#                                          invalid --generated-at, unwritable -o, JSON schema
#                                          violation: exit 1 with and without the flag, no file
#   E. usage errors of report (D-03)    -> missing --runs, missing --matrix, unknown option,
#                                          invalid --kind, invalid --format: exit 1; --help: 0
#   F. other subcommands keep clap's 2  -> `generate` without argument, unknown subcommand
#   G. closed set                       -> every recorded code is 0, 1 or 2; 2 only with the flag
#
# Every invocation runs with --no-audit from the repository root. All fixtures and outputs live in
# a mktemp directory that is removed on exit (use --no-remove to keep it). The repository is not
# written.
#
# Usage: crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh [--no-remove]
# Env:   TEST_EXECUTOR_BIN overrides the binary (default: find_binary "test-executor")
#        The script exits 1 with an [ERROR] line when the binary is missing.
# Needs: nothing beyond coreutils, grep, cmp. bash 3.2 compatible.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

source "$PROJECT_ROOT/scripts/lib/find-binary.sh" || exit 1
source "$PROJECT_ROOT/scripts/lib/logger.sh" || exit 1

# The binary resolves schemas/ relative to the working directory, so every run starts here
cd "$PROJECT_ROOT"

TEST_EXECUTOR_BIN="${TEST_EXECUTOR_BIN:-$(find_binary "test-executor")}"
if [ -z "$TEST_EXECUTOR_BIN" ] || [ ! -x "$TEST_EXECUTOR_BIN" ]; then
    log_error "test-executor binary not found (cargo build -p test-executor, or set TEST_EXECUTOR_BIN)"
    exit 1
fi
# Runs may change the working directory, so the binary path must be absolute
case "$TEST_EXECUTOR_BIN" in
    */*) TEST_EXECUTOR_BIN="$(cd "$(dirname "$TEST_EXECUTOR_BIN")" && pwd)/$(basename "$TEST_EXECUTOR_BIN")" ;;
esac

REMOVE_TEMP=1
while [ $# -gt 0 ]; do
    case $1 in
        --no-remove) REMOVE_TEMP=0; shift ;;
        *)
            log_error "Unknown option: $1"
            log_error "Usage: $0 [--no-remove]"
            exit 1
            ;;
    esac
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/report-exit-codes.XXXXXX")"
setup_cleanup "$WORK"
if [ "$REMOVE_TEMP" -eq 0 ]; then
    disable_cleanup
    echo "Keeping $WORK" >&2
fi

GEN="2026-10-31T09:00:00Z"
LAST_OUT="$WORK/last.stdout"
LAST_ERR="$WORK/last.stderr"
OUT="$WORK/out"
mkdir -p "$OUT"

PASSED=0
FAILED=0
RC=0
CODES=""
FLAG_VIOLATIONS=0
RUN_CWD="$PROJECT_ROOT"

# ---------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------

ok() {
    pass "$1"
    PASSED=$((PASSED + 1))
}

bad() {
    fail "$1"
    FAILED=$((FAILED + 1))
}

# expect_code DESCRIPTION EXPECTED: compares the exit status of the last run
expect_code() {
    if [ "$RC" -eq "$2" ]; then
        ok "$1 (exit $RC)"
    else
        bad "$1 (exit $RC, expected $2)"
        sed 's/^/      | /' "$LAST_ERR" | head -n 5 >&2
    fi
}

# run_cmd ARG...: runs the binary with the arguments; sets RC, LAST_OUT, LAST_ERR and CODES
run_cmd() {
    set +e
    (cd "$RUN_CWD" && "$TEST_EXECUTOR_BIN" "$@") >"$LAST_OUT" 2>"$LAST_ERR"
    RC=$?
    set -e
    CODES="$CODES $RC"
}

# run_report MATRIX RUNS GENERATED [ARG...]: `--no-audit report` with the given inputs.
# Exit 2 is only allowed with --fail-on-gaps; any other case is counted by FLAG_VIOLATIONS.
run_report() {
    local matrix="$1" runs="$2" generated="$3" flagged=0 arg
    shift 3
    for arg in "$@"; do
        if [ "$arg" = "--fail-on-gaps" ]; then
            flagged=1
        fi
    done
    run_cmd --no-audit report --matrix "$matrix" --runs "$runs" --generated-at "$generated" "$@"
    if [ "$RC" -eq 2 ] && [ "$flagged" -eq 0 ]; then
        FLAG_VIOLATIONS=$((FLAG_VIOLATIONS + 1))
    fi
}

# expect_usage DESCRIPTION ARG...: a usage error of report or another command (exit 1 for report)
expect_usage() {
    local desc="$1"
    shift
    run_cmd "$@"
    expect_code "$desc" 1
    if [ -s "$LAST_ERR" ]; then
        ok "$desc: message on stderr"
    else
        bad "$desc: no message on stderr"
    fi
}

# expect_error DESCRIPTION TARGET MATRIX RUNS GENERATED [ARG...]: exit 1 with and without the
# flag, and no output file in either case
expect_error() {
    local desc="$1" target="$2" matrix="$3" runs="$4" generated="$5"
    shift 5
    run_report "$matrix" "$runs" "$generated" "$@" -o "$target"
    expect_code "$desc" 1
    run_report "$matrix" "$runs" "$generated" "$@" -o "$target" --fail-on-gaps
    expect_code "$desc with --fail-on-gaps" 1
    if [ -e "$target" ]; then
        bad "$desc: an output file was created"
    else
        ok "$desc: no output file"
    fi
}

# check_header FILE KIND FORMAT: the first line of a traceability document
check_header() {
    case "$2:$3" in
        traceability:markdown)
            if [ "$(head -n 1 "$1")" = "# Traceability Report" ]; then
                ok "$2 $3: first line is the title"
            else
                bad "$2 $3: first line is not '# Traceability Report'"
            fi
            ;;
        traceability:asciidoc)
            if [ "$(head -n 1 "$1")" = "= Traceability Report" ]; then
                ok "$2 $3: first line is the level-0 title"
            else
                bad "$2 $3: first line is not '= Traceability Report'"
            fi
            ;;
        traceability:json)
            if [ "$(head -c 1 "$1")" = "{" ]; then
                ok "$2 $3: first byte is '{'"
            else
                bad "$2 $3: first byte is not '{'"
            fi
            ;;
    esac
}

# make_set NAME MATRIX VERDICT: fixture set in $WORK/sets/NAME
#   MATRIX  base: HLR-1 with T1 (both coverage flags true); gaps: also HLR-2 with T2 and no
#           robustness test, so T2 has no result (two gap entries)
#   VERDICT the T1 result in RUN-1: pass, fail (problem PR-1) or error (problem PR-2)
make_set() {
    local dir="$WORK/sets/$1" result
    mkdir -p "$dir/runs"
    cat > "$dir/matrix.yaml" <<EOF
type: trace_matrix
schema: tcms/trace-matrix.schema.json
id: TM-1
links:
  - requirement_id: HLR-1
    level: HLR
    test_case_ids: [T1]
    normal_range_covered: true
    robustness_covered: true
    run_ids: ["RUN-1"]
EOF
    if [ "$2" = "gaps" ]; then
        cat >> "$dir/matrix.yaml" <<EOF
  - requirement_id: HLR-2
    level: HLR
    test_case_ids: [T2]
    normal_range_covered: true
    robustness_covered: false
    run_ids: ["RUN-1"]
EOF
    fi
    case "$3" in
        pass) result='  - {test_case_id: T1, verdict: pass}' ;;
        fail) result='  - {test_case_id: T1, verdict: fail, problem_report_id: PR-1}' ;;
        error) result='  - {test_case_id: T1, verdict: error, problem_report_id: PR-2}' ;;
    esac
    cat > "$dir/runs/RUN-1.yaml" <<EOF
type: test_run
schema: tcms/test-run.schema.json
run_id: RUN-1
build_id: build-RUN-1
environment: e
tester: t
started_at: '2026-10-28T10:00:00Z'
results:
$result
EOF
}

CLEAN_M="$WORK/sets/clean/matrix.yaml"; CLEAN_R="$WORK/sets/clean/runs"
GAPS_M="$WORK/sets/gaps/matrix.yaml"; GAPS_R="$WORK/sets/gaps/runs"
FAILED_M="$WORK/sets/failed/matrix.yaml"; FAILED_R="$WORK/sets/failed/runs"
ERRORED_M="$WORK/sets/errored/matrix.yaml"; ERRORED_R="$WORK/sets/errored/runs"

make_set clean base pass
make_set gaps gaps pass
make_set failed base fail
make_set errored base error

# ---------------------------------------------------------------------------------------------
# A. clean run
# ---------------------------------------------------------------------------------------------
section "A. Clean run (exit 0)"
run_report "$CLEAN_M" "$CLEAN_R" "$GEN"
expect_code "clean without --fail-on-gaps" 0
run_report "$CLEAN_M" "$CLEAN_R" "$GEN" --fail-on-gaps
expect_code "clean with --fail-on-gaps" 0
if grep -q -- "--fail-on-gaps" "$LAST_ERR"; then
    bad "clean: the flag is named on stderr although nothing failed"
else
    ok "clean: no explanatory line on stderr"
fi

# ---------------------------------------------------------------------------------------------
# B. gaps and failed results
# ---------------------------------------------------------------------------------------------
section "B. Gaps and failed results (0 without the flag, 2 with it)"
run_report "$GAPS_M" "$GAPS_R" "$GEN"
expect_code "gaps without --fail-on-gaps" 0
run_report "$GAPS_M" "$GAPS_R" "$GEN" --fail-on-gaps
expect_code "gaps with --fail-on-gaps" 2
if grep -q "2 gap entries" "$LAST_ERR"; then
    ok "gaps: stderr reports 2 gap entries"
else
    bad "gaps: stderr does not report 2 gap entries"
fi

run_report "$FAILED_M" "$FAILED_R" "$GEN"
expect_code "failed result without --fail-on-gaps" 0
run_report "$FAILED_M" "$FAILED_R" "$GEN" --fail-on-gaps
expect_code "failed result with --fail-on-gaps" 2

run_report "$ERRORED_M" "$ERRORED_R" "$GEN"
expect_code "errored result without --fail-on-gaps" 0
run_report "$ERRORED_M" "$ERRORED_R" "$GEN" --fail-on-gaps
expect_code "errored result with --fail-on-gaps" 2

# The summary as JSON is a valid document, not an error (05-02 summary); its status follows the
# same rule as any other kind.
run_report "$FAILED_M" "$FAILED_R" "$GEN" --kind summary --format json
expect_code "failed result, summary as JSON, without --fail-on-gaps" 0
if [ "$(head -c 1 "$LAST_OUT")" = "{" ]; then
    ok "summary as JSON: stdout starts with '{'"
else
    bad "summary as JSON: stdout does not start with '{'"
fi

# ---------------------------------------------------------------------------------------------
# C. write-before-exit (D-02): the exit-2 output equals the exit-0 output
# ---------------------------------------------------------------------------------------------
section "C. Write before exit 2 (stdout and -o, all kinds and formats)"
C_OUT="$OUT/c"
mkdir -p "$C_OUT"
for spec in traceability:markdown summary:markdown gaps:markdown problems:markdown \
            traceability:asciidoc traceability:json traceability:yaml; do
    kind="${spec%%:*}"
    fmt="${spec#*:}"
    tag="$kind-$fmt"

    # stdout
    run_report "$FAILED_M" "$FAILED_R" "$GEN" --kind "$kind" --format "$fmt"
    expect_code "$tag to stdout without the flag" 0
    cp "$LAST_OUT" "$C_OUT/$tag.stdout.plain"
    run_report "$FAILED_M" "$FAILED_R" "$GEN" --fail-on-gaps --kind "$kind" --format "$fmt"
    expect_code "$tag to stdout with the flag" 2
    cp "$LAST_OUT" "$C_OUT/$tag.stdout.flag"
    if [ -s "$C_OUT/$tag.stdout.flag" ]; then
        ok "$tag to stdout: the document is written before exit 2"
    else
        bad "$tag to stdout: nothing on stdout before exit 2"
    fi
    if cmp -s "$C_OUT/$tag.stdout.plain" "$C_OUT/$tag.stdout.flag"; then
        ok "$tag to stdout: exit 2 bytes equal exit 0 bytes"
    else
        bad "$tag to stdout: exit 2 bytes differ from exit 0 bytes"
    fi
    if grep -q -- "--fail-on-gaps" "$C_OUT/$tag.stdout.flag"; then
        bad "$tag to stdout: the explanatory line is on stdout"
    else
        ok "$tag to stdout: the explanatory line is not on stdout"
    fi
    if grep -q -- "--fail-on-gaps" "$LAST_ERR"; then
        ok "$tag to stdout: the explanatory line is on stderr"
    else
        bad "$tag to stdout: no explanatory line on stderr"
    fi
    check_header "$C_OUT/$tag.stdout.flag" "$kind" "$fmt"

    # -o FILE
    target="$C_OUT/$tag.plain.out"
    run_report "$FAILED_M" "$FAILED_R" "$GEN" --kind "$kind" --format "$fmt" -o "$target"
    expect_code "$tag to -o without the flag" 0
    if [ -s "$LAST_OUT" ]; then
        bad "$tag to -o: stdout is not empty"
    else
        ok "$tag to -o: stdout is empty"
    fi
    target_flag="$C_OUT/$tag.flag.out"
    run_report "$FAILED_M" "$FAILED_R" "$GEN" --fail-on-gaps --kind "$kind" --format "$fmt" -o "$target_flag"
    expect_code "$tag to -o with the flag" 2
    if [ -s "$LAST_OUT" ]; then
        bad "$tag to -o with the flag: stdout is not empty"
    else
        ok "$tag to -o with the flag: stdout is empty"
    fi
    if [ -s "$target_flag" ]; then
        ok "$tag to -o: the file exists and is not empty after exit 2"
    else
        bad "$tag to -o: the file is missing or empty after exit 2"
    fi
    if cmp -s "$target" "$target_flag"; then
        ok "$tag to -o: exit 2 file equals exit 0 file"
    else
        bad "$tag to -o: exit 2 file differs from exit 0 file"
    fi
    if grep -q -- "--fail-on-gaps" "$LAST_ERR"; then
        ok "$tag to -o: the explanatory line is on stderr"
    else
        bad "$tag to -o: no explanatory line on stderr"
    fi
    check_header "$target_flag" "$kind" "$fmt"
done

# ---------------------------------------------------------------------------------------------
# D. errors win: exit 1 with and without the flag, and no output file
# ---------------------------------------------------------------------------------------------
section "D. Errors win (exit 1, no output file)"

make_set badmatrix base fail
printf '%s\n' 'type: trace_matrix' 'schema: tcms/trace-matrix.schema.json' 'id: TM-1' 'links: 5' \
    > "$WORK/sets/badmatrix/matrix.yaml"
expect_error "invalid matrix" "$OUT/d-matrix.md" \
    "$WORK/sets/badmatrix/matrix.yaml" "$WORK/sets/badmatrix/runs" "$GEN"

make_set dupe base fail
cp "$WORK/sets/dupe/runs/RUN-1.yaml" "$WORK/sets/dupe/runs/copy.yaml"
expect_error "duplicate run_id" "$OUT/d-dupe.md" \
    "$WORK/sets/dupe/matrix.yaml" "$WORK/sets/dupe/runs" "$GEN"

make_set broken base fail
printf '%s\n' 'a: [unclosed' > "$WORK/sets/broken/runs/broken.yaml"
expect_error "broken YAML run file" "$OUT/d-broken.md" \
    "$WORK/sets/broken/matrix.yaml" "$WORK/sets/broken/runs" "$GEN"

expect_error "invalid --generated-at" "$OUT/d-time.md" \
    "$FAILED_M" "$FAILED_R" "yesterday"

expect_error "-o into a directory that does not exist" "$WORK/no-such-dir/report.md" \
    "$FAILED_M" "$FAILED_R" "$GEN"
if [ -d "$WORK/no-such-dir" ]; then
    bad "-o into a missing directory: the directory was created"
else
    ok "-o into a missing directory: the directory was not created"
fi

# A JSON document that fails the report schema: the schemas are copied and the report schema is
# replaced, and the binary runs from that copy (absolute input paths)
mkdir -p "$WORK/schema-root"
cp -R "$PROJECT_ROOT/schemas" "$WORK/schema-root/"
printf '%s\n' '{"type":"object","required":["nonexistent_field"]}' \
    > "$WORK/schema-root/schemas/tcms/report.schema.v1.json"
RUN_CWD="$WORK/schema-root"
expect_error "JSON schema violation" "$OUT/d-schema.json" \
    "$FAILED_M" "$FAILED_R" "$GEN" --kind traceability --format json
RUN_CWD="$PROJECT_ROOT"

# ---------------------------------------------------------------------------------------------
# E. usage errors of report (D-03): clap parse errors exit 1
# ---------------------------------------------------------------------------------------------
section "E. Usage errors of report (exit 1)"
expect_usage "report without --runs" --no-audit report --matrix "$CLEAN_M"
expect_usage "report without --matrix" --no-audit report --runs "$CLEAN_R"
expect_usage "report with an unknown option" \
    --no-audit report --matrix "$CLEAN_M" --runs "$CLEAN_R" --bogus
expect_usage "report with an invalid --kind" \
    --no-audit report --matrix "$CLEAN_M" --runs "$CLEAN_R" --kind nosuchkind
expect_usage "report with an invalid --format" \
    --no-audit report --matrix "$CLEAN_M" --runs "$CLEAN_R" --format xml

run_cmd --no-audit report --help
expect_code "report --help" 0
if grep -q -- "--fail-on-gaps" "$LAST_OUT"; then
    ok "report --help documents --fail-on-gaps"
else
    bad "report --help does not mention --fail-on-gaps"
fi

# ---------------------------------------------------------------------------------------------
# F. other subcommands keep clap's exit code 2
# ---------------------------------------------------------------------------------------------
section "F. Other subcommands keep exit 2 for usage errors"
run_cmd generate
expect_code "generate without a test case argument" 2
run_cmd nosuchsubcommand
expect_code "unknown subcommand" 2

# ---------------------------------------------------------------------------------------------
# G. closed set of exit codes
# ---------------------------------------------------------------------------------------------
section "G. Closed set of exit codes"
BAD_CODES=""
for code in $CODES; do
    case "$code" in
        0|1|2) ;;
        *) BAD_CODES="$BAD_CODES $code" ;;
    esac
done
if [ -z "$BAD_CODES" ]; then
    ok "every recorded exit code is 0, 1 or 2"
else
    bad "codes outside 0, 1, 2:$BAD_CODES"
fi
if [ "$FLAG_VIOLATIONS" -eq 0 ]; then
    ok "report exits 2 only with --fail-on-gaps"
else
    bad "report exited 2 without --fail-on-gaps $FLAG_VIOLATIONS time(s)"
fi

# ---------------------------------------------------------------------------------------------
# summary
# ---------------------------------------------------------------------------------------------
section "Summary"
log_info "Checks passed: $PASSED, failed: $FAILED"
if [ "$FAILED" -ne 0 ]; then
    log_error "report exit-code checks failed: $FAILED"
    exit 1
fi
log_info "report exit-code checks passed"
exit 0
