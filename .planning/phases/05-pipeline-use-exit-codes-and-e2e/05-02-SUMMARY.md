# Phase 5 Plan 02: report exit codes (0, 1, 2) and --fail-on-gaps Summary

`test-executor report` now exits 0 on success, 1 on any error, and 2 only with `--fail-on-gaps` when gaps or failed results exist, after the report is written and flushed; clap parse errors of `report` exit 1 (D-05-1).

## Performance

- **Tasks:** 2 (plus fixes to earlier-phase tests per D-05-1)
- **Commits:** 2 (tests, then code)

## Accomplishments

- `--fail-on-gaps` flag on `report`; the decision comes from `collect_findings` on the model, so it applies to every kind and format.
- Exit 2 only after the report is written and flushed; all errors return before any write and exit 1.
- Clap parse errors of `report` (missing `--runs`/`--matrix`, unknown option, invalid `--kind`/`--format`/`--shape` value) exit 1; other subcommands keep clap's 2.
- The `--shape` conflict with a non-traceability kind keeps exit 2 (D-02-6-2, D-05-1 (b)), raised with `clap::Error::raw(...).exit()` in `fn report`.
- Real-binary tests in `report_exit_codes_cli.rs` (16 tests) cover exits 0, 1 and 2, byte-identical output before exit 2 (stdout and `-o` file), every kind and format, and usage errors versus other subcommands.
- Earlier-phase tests that asserted exit 2 for parse errors now assert exit 1, with the assertions otherwise unchanged; shape-conflict tests still assert exit 2.

## Task Commits

1. **Tests:** `b514ef1` test(05-02): exit codes of report, clap errors exit 1, real-binary exit-code tests [ci skip]
2. **Code:** `4c6966f` feat(05-02): --fail-on-gaps and three-state exit status for report [ci skip]

## Files Created/Modified

- `crates/test-executor/src/main.rs` - `--fail-on-gaps`, `parse_cli`, `finish_status`, exit 2 after write, shape conflict kept at exit 2.
- `crates/test-executor/tests/report_exit_codes_cli.rs` - new, 16 real-binary tests.
- `crates/test-executor/tests/report_cli.rs`, `report_formats_e2e.rs`, `report_kinds_e2e.rs`, `report_traceability_e2e.rs` - exit code changes for clap parse errors only.

## Decisions Made

- D-05-1 (in `.planning/DECISIONS.md`): clap parse errors of `report` exit 1; the `--shape` conflict stays exit 2. The 05-02 must-have "usage error" is narrowed to clap parse errors.
- `--kind summary --format json` is not a usage error: it renders a valid JSON summary document and exits 0 (2 with `--fail-on-gaps` and a failed result). Verified on the real binary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Shape-conflict replaced by anyhow::bail in the previous executor's uncommitted work**
- **Found during:** Task 1 review
- **Issue:** The `--shape` conflict was changed to exit 1, which contradicts D-05-1 (b).
- **Fix:** Restored `clap::Error::raw(ErrorKind::ArgumentConflict, ...).exit()` (exit 2).
- **Commit:** `4c6966f`

**2. [Rule 1 - Bug] Shape-conflict case inside the exit-1 error table**
- **Found during:** Task 2 test run
- **Issue:** `errors_win_over_the_flag` included a `--shape a --kind summary` case that must exit 2.
- **Fix:** Removed it from that table; added `shape_conflict_exits_2_with_or_without_the_flag`.
- **Commit:** `b514ef1`

**3. [Rule 3 - Blocking] Clippy failure in pre-existing code**
- `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` fails in `crates/testcase-execution/src/executor.rs:2435` (not modified by this plan). The gate was run with `--no-deps`, which passes.

## Issues Encountered

- `cargo test` reports only the first failing test binary unless `--no-fail-fast` is used.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- `cargo fmt -p testcase-reporting -p test-executor -- --check`: exit 0
- `cargo clippy -p testcase-reporting -p test-executor --all-targets --no-deps -- -D warnings`: exit 0 (full clippy with deps fails on pre-existing `testcase-execution` executor.rs:2435)
- `cargo test -p testcase-reporting -p test-executor --no-fail-fast`: exit 0
- Commits `b514ef1` and `4c6966f` exist on `dev`
