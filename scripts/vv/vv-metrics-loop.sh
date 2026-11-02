#!/usr/bin/env bash
# Repeat the V&V metrics checks and prove they are reliable (TCMS-47).
#
# One iteration, each step must pass before the next:
#   1. regenerate the derived fixtures into a temp directory and compare with the committed ones
#   2. build tcms
#   3. cargo unit and integration tests of tcms (vvmetrics, vvcmd, vv_metrics_cli)
#   4. the end-to-end suite scripts/vv/test_vv_metrics.sh (no rebuild)
#   5. hash every fixture's report (fixed clock): the hash must equal the one of the first iteration
#
# Iterations stop at the first failure. The loop exits 0 only when all ITERATIONS pass and the hashes
# never changed, so a flaky or clock-dependent output shows up as a failure, not as a lucky pass.
#
# Usage: scripts/vv/vv-metrics-loop.sh [ITERATIONS]     (default 3)
# Env:   STATE_DIR   where the hash baseline and logs go (default: a temp dir, printed at start)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
source "$REPO_ROOT/scripts/lib/logger.sh" || exit 1

ITERATIONS="${1:-3}"
case "$ITERATIONS" in
    ''|*[!0-9]*|0) log_error "ITERATIONS must be a positive integer, got '$ITERATIONS'"; exit 2 ;;
esac

STATE_DIR="${STATE_DIR:-$(mktemp -d "${TMPDIR:-/tmp}/vv-loop.XXXXXX")}"
mkdir -p "$STATE_DIR"
BASELINE="$STATE_DIR/baseline.sha256"
FIXTURES="$REPO_ROOT/crates/tcms/tests/fixtures/vv-metrics"
FIXED_NOW="2026-11-02T10:00:00+04:00"

# Each entry: fixture root (relative), run id.
REPORTS='sut-1|RUN-32921
derived/failing-run|RUN-DERIVED-FAIL-01
derived/no-data-run|RUN-DERIVED-NODATA-01
derived/blocked-only-run|RUN-DERIVED-BLOCKED-01
derived/skipped-outside-denominator|RUN-DERIVED-SKIP-01
derived/uncovered-requirement|RUN-DERIVED-UNCOVERED-01
derived/missing-case-result|RUN-DERIVED-MISSING-01'

# hash_reports OUTFILE: write "<sha256>  <fixture>/<run>" lines for every fixture report.
hash_reports() {
    local out="$1" bin="$2" rel run
    : >"$out"
    while IFS='|' read -r rel run; do
        local report
        report="$(mktemp "$STATE_DIR/report.XXXXXX")"
        TCMS_NOW="$FIXED_NOW" "$bin" --schemas-root "$REPO_ROOT/schemas/" vv metrics \
            --root "$FIXTURES/$rel" --run "$run" --out "$report" || { rm -f "$report"; return 1; }
        printf '%s  %s/%s\n' "$(sha256sum "$report" | cut -d' ' -f1)" "$rel" "$run" >>"$out"
        rm -f "$report"
    done <<EOF
$REPORTS
EOF
}

run_step() {
    local iteration="$1" name="$2"
    shift 2
    local log="$STATE_DIR/iter-$iteration-$name.log"
    log_info "[$iteration/$ITERATIONS] $name"
    if "$@" >"$log" 2>&1; then
        pass "[$iteration/$ITERATIONS] $name"
        return 0
    fi
    fail "[$iteration/$ITERATIONS] $name (log: $log)"
    tail -30 "$log" | sed 's/^/      /'
    return 1
}

log_info "state directory: $STATE_DIR"
for i in $(seq 1 "$ITERATIONS"); do
    section "Iteration $i of $ITERATIONS"
    DERIVED_TMP="$STATE_DIR/regen-$i"
    run_step "$i" "fixtures match generator" bash -c \
        "mkdir -p '$DERIVED_TMP' && python3 '$SCRIPT_DIR/generate_derived_fixtures.py' --dest '$DERIVED_TMP' >/dev/null && diff -r -x README.md '$FIXTURES/derived' '$DERIVED_TMP'" || exit 1
    run_step "$i" "build tcms" bash -c "cd '$REPO_ROOT' && cargo build -q -p tcms" || exit 1
    run_step "$i" "cargo tests tcms" bash -c "cd '$REPO_ROOT' && cargo test -q -p tcms --bins --test vv_metrics_cli" || exit 1
    run_step "$i" "end-to-end suite" env REPEATS=5 "$SCRIPT_DIR/test_vv_metrics.sh" --no-build || exit 1

    CURRENT="$STATE_DIR/hashes-$i.sha256"
    run_step "$i" "report hashes" hash_reports "$CURRENT" "$REPO_ROOT/target/debug/tcms" || exit 1
    if [ "$i" = 1 ]; then
        cp "$CURRENT" "$BASELINE"
        log_info "baseline recorded: $(wc -l <"$BASELINE" | tr -d ' ') reports"
    else
        if cmp -s "$BASELINE" "$CURRENT"; then
            pass "[$i/$ITERATIONS] hashes unchanged since iteration 1"
        else
            fail "[$i/$ITERATIONS] report hashes changed since iteration 1"
            diff "$BASELINE" "$CURRENT" | sed 's/^/      /'
            exit 1
        fi
    fi
done

section "Result"
log_info "$ITERATIONS of $ITERATIONS iterations passed; the reports are identical in every iteration"
exit 0
