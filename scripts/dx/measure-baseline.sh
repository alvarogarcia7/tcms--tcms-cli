#!/usr/bin/env bash
# Phase 0 of "DX: Optimizing compilation": measure where build and test time goes.
#
# Writes a report directory containing:
#   summary.txt          environment, totals, top offenders
#   build.log            output of the workspace build
#   timings/             cargo's HTML timing report (copied)
#   unit-times.tsv       per compilation unit: name, version, mode, seconds (from cargo's timing report)
#   crate-times.tsv      per crate: compile seconds summed over its units
#   test-build.json      cargo's JSON messages from `cargo test --no-run`
#   test-binaries.tsv    every test/bin executable: package, target, kind, bytes, path
#   test-runs.tsv        wall seconds and exit status of running each test harness (unless SKIP_RUN=1);
#                        plain bin targets are listed in test-binaries.tsv but not run
#
# Usage:  scripts/dx/measure-baseline.sh [OUT_DIR]     (default: reports/dx/<UTC timestamp>)
# Env:    SKIP_RUN=1   measure the build only; do not run the test executables
#
# The script does not run `cargo clean`. Whatever state target/ is in is what gets measured, so
# record that state (fresh clone, warm incremental, after a touch) when you compare runs.

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT" || exit 1

for tool in cargo python3; do
    command -v "$tool" >/dev/null 2>&1 || { echo "missing required tool: $tool" >&2; exit 1; }
done

STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_DIR="${1:-reports/dx/$STAMP}"
mkdir -p "$OUT_DIR/timings" || exit 1
SUMMARY="$OUT_DIR/summary.txt"
: > "$SUMMARY"

log() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }
now() { python3 -c 'import time; print(time.time())'; }
secs() { python3 -c "print(round($2 - $1, 2))"; }

log "DX Phase 0 baseline"
log "timestamp_utc: $STAMP"
log "git_commit: $(git rev-parse --short HEAD 2>/dev/null) uncommitted_files: $(git status --porcelain | wc -l)"
log "cargo: $(cargo --version)"
log "CARGO_BUILD_JOBS: ${CARGO_BUILD_JOBS:-<unset>}"
log "cores: $(nproc 2>/dev/null || echo unknown)"
log "dev profile: $(grep -A2 '^\[profile.dev\]' Cargo.toml | tr '\n' ' ')"
log ""

# ---------------------------------------------------------------------------
# 1. Whole-workspace build with cargo's timing report
# ---------------------------------------------------------------------------
log "== step 1: cargo build --workspace --timings"
t0=$(now)
cargo build --workspace --timings > "$OUT_DIR/build.log" 2>&1
build_status=$?
t1=$(now)
log "build_exit: $build_status"
log "build_wall_seconds: $(secs "$t0" "$t1")"
# how much was actually recompiled: 0 or a handful means a warm target/, not a cold build
log "crates_compiled_in_build: $(grep -c '^ *Compiling ' "$OUT_DIR/build.log")"

TIMING_HTML="$(ls -t target/cargo-timings/cargo-timing-*.html 2>/dev/null | head -1)"
[ -z "$TIMING_HTML" ] && TIMING_HTML="target/cargo-timings/cargo-timing.html"

if [ -f "$TIMING_HTML" ]; then
    cp "$TIMING_HTML" "$OUT_DIR/timings/cargo-timing.html"
    python3 "$SCRIPT_DIR/parse_timing.py" "$TIMING_HTML" "$OUT_DIR" | tee -a "$SUMMARY"
else
    log "warning: cargo produced no timing report (see build.log)"
fi
log ""

# ---------------------------------------------------------------------------
# 2. Test executables: compile and link only, and record their sizes
# ---------------------------------------------------------------------------
log "== step 2: cargo test --workspace --no-run (compile + link every test binary)"
t0=$(now)
cargo test --workspace --no-run --message-format=json > "$OUT_DIR/test-build.json" 2> "$OUT_DIR/test-build.log"
no_run_status=$?
t1=$(now)
log "no_run_exit: $no_run_status"
log "no_run_wall_seconds: $(secs "$t0" "$t1")"

python3 "$SCRIPT_DIR/parse_executables.py" "$OUT_DIR/test-build.json" "$OUT_DIR/test-binaries.tsv" | tee -a "$SUMMARY"
log ""

# ---------------------------------------------------------------------------
# 3. Run every test harness executable, timing each (skippable)
#    Only executables built with the test profile are run. Plain bin targets (servers such as
#    mock-jira, CLIs that need arguments) are not tests and would block or fail the run.
# ---------------------------------------------------------------------------
if [ "${SKIP_RUN:-0}" = "1" ]; then
    log "== step 3: skipped (SKIP_RUN=1)"
else
    log "== step 3: run each test harness executable from its package directory (as cargo does)"
    log "skipped_non_test_executables: $(tail -n +2 "$OUT_DIR/test-binaries.tsv" | awk -F'\t' '$6!="true"' | wc -l)"
    printf 'package\ttarget\twall_s\texit\n' > "$OUT_DIR/test-runs.tsv"
    run_start=$(now)
    tail -n +2 "$OUT_DIR/test-binaries.tsv" | awk -F'\t' '$6=="true"' | while IFS=$'\t' read -r pkg target kind bytes path is_test; do
        # the package directory is crates/<package> for workspace members; fall back to the repo root
        pkg_dir="$REPO_ROOT/crates/$pkg"
        [ -d "$pkg_dir" ] || pkg_dir="$REPO_ROOT"
        s=$(now)
        # cargo reports absolute executable paths
        ( cd "$pkg_dir" && "$path" >/dev/null 2>&1 )
        rc=$?
        e=$(now)
        printf '%s\t%s\t%s\t%s\n' "$pkg" "$target" "$(secs "$s" "$e")" "$rc" >> "$OUT_DIR/test-runs.tsv"
    done
    run_end=$(now)
    log "run_total_wall_seconds: $(secs "$run_start" "$run_end")"
    log "failing_executables: $(tail -n +2 "$OUT_DIR/test-runs.tsv" | awk -F'\t' '$4!=0' | wc -l)"
    log "slowest 10 executables (package, target, seconds, exit):"
    tail -n +2 "$OUT_DIR/test-runs.tsv" | sort -t$'\t' -k3,3gr | head -10 | tee -a "$SUMMARY"
fi

log ""
log "report written to: $OUT_DIR"
exit 0
