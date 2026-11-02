#!/usr/bin/env bash
# End-to-end checks of `tcms vv metrics` through the binary (TCMS-47):
#   - the files it writes (--out) and its stdout are complete, valid JSON with the expected fields;
#   - with a fixed clock (TCMS_NOW) the output is byte-for-byte reproducible, over repeated runs;
#   - only `generatedAt` depends on the clock;
#   - the figures of every fixture match the hand-derived expectations (docs/VV_METRICS.md);
#   - bad input fails without leaving a partial file behind;
#   - the derived fixtures are exactly what scripts/vv/generate_derived_fixtures.py produces.
#
# Usage: scripts/vv/test_vv_metrics.sh [--no-build]
# Env:   TCMS_BIN   binary to test (default: target/debug/tcms, built unless --no-build)
#        REPEATS    repeated runs per fixture for the reproducibility check (default 5)
#        KEEP_TMP=1 keep the temporary directory for inspection

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1

BUILD=1
[ "${1:-}" = "--no-build" ] && BUILD=0
REPEATS="${REPEATS:-5}"
FIXTURES="$REPO_ROOT/crates/tcms/tests/fixtures/vv-metrics"
SCHEMAS="$REPO_ROOT/schemas/"

# A time with an offset, so the normalisation to UTC is exercised too.
FIXED_NOW="2026-11-02T10:00:00+04:00"
FIXED_GENERATED="2026-11-02T06:00:00Z"
OTHER_NOW="2031-01-15T23:59:59Z"

if [ "$BUILD" = 1 ] && [ -z "${TCMS_BIN:-}" ]; then
    (cd "$REPO_ROOT" && cargo build -q -p tcms) || { log_error "cargo build -p tcms failed"; exit 1; }
fi
TCMS="${TCMS_BIN:-$REPO_ROOT/target/debug/tcms}"
if [ ! -x "$TCMS" ]; then
    log_error "binary not found or not executable: $TCMS"
    exit 1
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/vv-metrics-e2e.XXXXXX")" || exit 1
if [ "${KEEP_TMP:-0}" != 1 ]; then
    setup_cleanup "$WORK"
else
    log_info "keeping $WORK"
fi

PASSED=0
FAILED=0

# check DESCRIPTION COMMAND... : run the command, count the result.
check() {
    local desc="$1"
    shift
    if "$@" >"$WORK/check.out" 2>"$WORK/check.err"; then
        PASSED=$((PASSED + 1))
        pass "$desc"
    else
        FAILED=$((FAILED + 1))
        fail "$desc"
        sed 's/^/      /' "$WORK/check.err" "$WORK/check.out" 2>/dev/null | head -20
    fi
}

# run_metrics ROOT OUT [extra args...] : run the binary with the fixed clock, writing --out to OUT.
run_metrics() {
    local root="$1" out="$2"
    shift 2
    TCMS_NOW="$FIXED_NOW" "$TCMS" --schemas-root "$SCHEMAS" vv metrics --root "$root" --out "$out" "$@"
}

# expect_figures FILE RUN VERDICT PASS_DISPLAY DELTA VV_DISPLAY STEP_DISPLAY
expect_figures() {
    jq -e --arg run "$2" --arg verdict "$3" \
        --argjson pass "$4" --argjson delta "$5" --argjson vv "$6" --argjson steps "$7" '
        .run.runId == $run
        and .gate.verdict == $verdict
        and .gate.passRate.display == $pass
        and .gate.deltaFromPrevious == $delta
        and .vvRun.vvPassRate.display == $vv
        and .vvRun.stepCoverage.display == $steps' "$1" >/dev/null
}

# expect_fields FILE: every top-level field of the documented output is present with the right type.
expect_fields() {
    jq -e '
        (.generatedAt | type == "string")
        and (.run | has("runId") and has("startedAt") and has("finishedAt") and has("provisional"))
        and (.gate | has("gateThresholdPercent") and has("denominator") and has("counts") and has("passRate")
                     and has("verdict") and has("provisional") and has("deltaFromPrevious") and has("previousRunId"))
        and (.vvRun | has("totalTestCases") and has("passedTestCases") and has("vvPassRate")
                      and has("totalSteps") and has("passedSteps") and has("stepCoverage"))
        and (.requirements.summary | has("total") and has("covered") and has("failing") and has("notRun")
                                     and has("noCases") and has("withDefect") and has("openDefects"))
        and (.stability | has("flakyThreshold") and has("flakyCount") and has("cases"))
        and (.loadSummary | has("distinctCases") and has("schemaErrorCount") and has("schemaErrors") and has("runsRecorded"))' "$1" >/dev/null
}

# same_bytes A B
same_bytes() { cmp -s "$1" "$2"; }

section "1. Output files for every fixture"
# ROOT (relative to fixtures) | RUN | VERDICT | PASS_DISPLAY | DELTA | VV_DISPLAY | STEP_DISPLAY
FIXTURE_TABLE='sut-1|RUN-32921|PASSED|100|0|100|100
derived/failing-run|RUN-DERIVED-FAIL-01|FAILED|50|-50|50|83
derived/no-data-run|RUN-DERIVED-NODATA-01|NO_DATA|null|null|0|null
derived/blocked-only-run|RUN-DERIVED-BLOCKED-01|FAILED|0|-100|0|0
derived/skipped-outside-denominator|RUN-DERIVED-SKIP-01|PASSED|100|0|50|50
derived/uncovered-requirement|RUN-DERIVED-UNCOVERED-01|PASSED|100|0|100|100
derived/missing-case-result|RUN-DERIVED-MISSING-01|PASSED|100|0|50|50'

while IFS='|' read -r rel run verdict pass delta vv steps; do
    name="${rel//\//_}"
    root="$FIXTURES/$rel"
    dir="$WORK/$name"
    mkdir -p "$dir"
    out="$dir/nested/report.json"

    # The file is created in a directory that did not exist, and is valid JSON.
    if run_metrics "$root" "$out" --run "$run" >"$dir/stderr" 2>&1; then
        check "$name: --out creates the file and its parent directories" test -s "$out"
        check "$name: report is valid JSON" jq empty "$out"
        check "$name: report has every documented field" expect_fields "$out"
        check "$name: figures match the hand-derived values ($verdict, $pass, $delta)" \
            expect_figures "$out" "$run" "$verdict" "$pass" "$delta" "$vv" "$steps"
        check "$name: generatedAt is the fixed clock in UTC" \
            jq -e --arg t "$FIXED_GENERATED" '.generatedAt == $t' "$out"
        check "$name: no temporary file is left next to the report" \
            test "$(ls -A "$dir/nested" | wc -l | tr -d ' ')" = 1
    else
        FAILED=$((FAILED + 1))
        fail "$name: command failed"
        sed 's/^/      /' "$dir/stderr" | head -20
        continue
    fi

    # Stdout is the same bytes as the file.
    TCMS_NOW="$FIXED_NOW" "$TCMS" --schemas-root "$SCHEMAS" vv metrics --root "$root" --run "$run" >"$dir/stdout.json"
    check "$name: stdout and --out are byte-identical" same_bytes "$out" "$dir/stdout.json"

    # Reproducible: repeated runs with the same clock give the same bytes.
    i=1
    while [ "$i" -le "$REPEATS" ]; do
        run_metrics "$root" "$dir/repeat-$i.json" --run "$run" >/dev/null 2>&1
        i=$((i + 1))
    done
    repeats_same() {
        local n=1
        while [ "$n" -le "$REPEATS" ]; do
            same_bytes "$out" "$dir/repeat-$n.json" || return 1
            n=$((n + 1))
        done
    }
    check "$name: $REPEATS repeated runs with a fixed clock are byte-identical" repeats_same

    # Only the clock may change the output: a different time changes exactly the generatedAt line.
    TCMS_NOW="$OTHER_NOW" "$TCMS" --schemas-root "$SCHEMAS" vv metrics --root "$root" --run "$run" --out "$dir/later.json"
    changed_lines() { diff "$out" "$dir/later.json" | grep -c '^[<>]'; }
    only_generated_at() {
        [ "$(changed_lines)" = 2 ] && diff "$out" "$dir/later.json" | grep '^[<>]' | grep -q generatedAt
    }
    check "$name: a different clock changes only the generatedAt line" only_generated_at
done <<EOF
$FIXTURE_TABLE
EOF

section "2. Error paths leave no file behind"
ERR_DIR="$WORK/errors"
mkdir -p "$ERR_DIR"
SUT="$FIXTURES/sut-1"

no_file() { [ ! -e "$1" ]; }

if ! run_metrics "$SUT" "$ERR_DIR/gate49.json" --gate 49 >/dev/null 2>&1; then
    check "--gate 49 is refused and writes no file" no_file "$ERR_DIR/gate49.json"
else
    FAILED=$((FAILED + 1)); fail "--gate 49 was accepted"
fi
if ! run_metrics "$SUT" "$ERR_DIR/unknown.json" --run RUN-NOPE >/dev/null 2>&1; then
    check "unknown --run is refused and writes no file" no_file "$ERR_DIR/unknown.json"
else
    FAILED=$((FAILED + 1)); fail "unknown --run was accepted"
fi
if ! TCMS_NOW="yesterday" "$TCMS" --schemas-root "$SCHEMAS" vv metrics --root "$SUT" --out "$ERR_DIR/badnow.json" >/dev/null 2>&1; then
    check "a non-RFC-3339 TCMS_NOW is refused and writes no file" no_file "$ERR_DIR/badnow.json"
else
    FAILED=$((FAILED + 1)); fail "a non-RFC-3339 TCMS_NOW was accepted"
fi
EMPTY="$ERR_DIR/no-runs"
mkdir -p "$EMPTY"
if ! run_metrics "$EMPTY" "$ERR_DIR/empty.json" >/dev/null 2>&1; then
    check "a root without runs is refused and writes no file" no_file "$ERR_DIR/empty.json"
else
    FAILED=$((FAILED + 1)); fail "a root without runs was accepted"
fi
# --out below a regular file cannot be created: the command fails and the file is not there.
touch "$ERR_DIR/afile"
if ! run_metrics "$SUT" "$ERR_DIR/afile/report.json" >/dev/null 2>&1; then
    check "an unwritable --out path fails cleanly" no_file "$ERR_DIR/afile/report.json"
else
    FAILED=$((FAILED + 1)); fail "an unwritable --out path was accepted"
fi

section "3. Derived fixtures match the generator"
GEN_DIR="$WORK/generated"
mkdir -p "$GEN_DIR"
if python3 "$SCRIPT_DIR/generate_derived_fixtures.py" --dest "$GEN_DIR" >/dev/null 2>&1; then
    drift_free() { diff -r -x README.md "$FIXTURES/derived" "$GEN_DIR" >/dev/null; }
    check "committed derived/ folders equal the generator output (no drift)" drift_free
else
    FAILED=$((FAILED + 1)); fail "generate_derived_fixtures.py failed"
fi

section "Summary"
log_info "passed: $PASSED  failed: $FAILED"
if [ "$FAILED" -ne 0 ]; then
    log_error "vv metrics end-to-end checks FAILED"
    exit 1
fi
log_info "vv metrics end-to-end checks passed"
exit 0
