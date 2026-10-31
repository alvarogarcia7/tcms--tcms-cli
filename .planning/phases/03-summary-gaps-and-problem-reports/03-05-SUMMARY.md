# Phase 3 Plan 05: Wire summary, gaps and problems into `test-executor report` - Summary

**`test-executor report --kind summary|gaps|problems` renders from one model; 14 new CLI tests cover SUM-01, GAP-01, PRB-01 and PRB-02; the report and trace suites pass; the make gates `test`, `coverage-e2e` and `acceptance-test` did not pass in this environment (details below).**

## Tasks and commits

| Task | Commit | Files |
|------|--------|-------|
| 1. Render the three new kinds from one model (D-06, D-07) | `e18f652` | `crates/test-executor/src/main.rs` |
| 2. CLI tests for summary, gaps and problems | `2b43ee9` | `crates/test-executor/tests/report_cli.rs` |
| 3. Document the three kinds | `5b668b3` | `docs/TRACEABILITY.md` |

Commit order followed the plan (Task 1 wiring, then Task 2 tests, then Task 3 docs). The plan puts wiring first, so the new tests were written after the code, not before it.

### Task 1 details
- `fn report` builds one `ReportModel` (`build_model(` count stays 1) and one `ViewInput`, then calls `build_summary`, `build_gaps` and `build_problems` (count 3 in the file). Traceability still uses `render_markdown`.
- The `renderer not yet available` text and its code path are gone (count 0).
- No new clock read: `Utc::now` count is 4 before and after.
- `ReportKind` and `Commands::Report` doc comments updated.

### Task 2 details
- 14 new tests (report_cli: 35 before, 49 after; trace_matrix_cli unchanged at 11). Names include `summary_buckets_sum_to_total`, `summary_superseded_result_not_counted`, `problems_flags_missing_problem_report`, `problems_missing_flag_clears_when_id_is_given`, `views_are_deterministic`, `consistency_across_default_kinds`, `new_kinds_fail_before_write`.
- A `scenario()` fixture writes the 03-01 data to disk. `run_yaml_pr()` is a new helper that sets `problem_report_id`; the existing `run_yaml` is unchanged.
- `grep -c 'MISSING problem_report_id'` in the test file: 2.

## Changed existing assertions (Phase 1/2 tests)

Only the renderer-placeholder assertions changed, in `report_kinds_default_to_all_and_can_be_selected`:
- `Report kind {kind}: renderer not yet available` became `Report kind {kind}: rendered below` (summary, gaps, problems).
- `summary: renderer not yet available` and `gaps: renderer not yet available` became `  summary: rendered below\n` and `  gaps: rendered below\n`.

`git diff` shows only these three assertion lines removed. No other Phase 1 or Phase 2 assertion was edited.

### Task 3 details
- `docs/TRACEABILITY.md`: the placeholder sentence is replaced. New sections: `### Run summary`, `### Coverage gaps`, `### Problem reports`. The required strings are present: `Without problem report` (1), `MISSING problem_report_id` (1), `Run summary`/`Coverage gaps`/`Problem reports` (3), `None` (2).

## Exact gate results

Plan verify and extra gates (all run, all pass):

| Command | Result |
|---------|--------|
| `CARGO_HOME=$HOME/.cargo-user cargo build -p test-executor` | exit 0 |
| `CARGO_HOME=$HOME/.cargo-user cargo clippy -p test-executor --all-targets -- -D warnings` | exit 0 |
| `cargo fmt -p test-executor -- --check` | exit 0 |
| `CARGO_HOME=$HOME/.cargo-user cargo test -p test-executor --test report_cli` | 49 passed, 0 failed |
| `CARGO_HOME=$HOME/.cargo-user cargo test -p test-executor --test trace_matrix_cli` | 11 passed, 0 failed |
| Task 3 grep checks (docs) | pass |
| `cargo fmt --check -p testcase-reporting -p test-executor` (extra) | exit 0 |
| `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` (extra) | exit 0 |
| `cargo test -p testcase-reporting -p test-executor` (extra) | exit 0, no `FAILED` lines |
| `cargo build --workspace` | exit 0 |
| `make build` | exit 0 |
| `make lint` | exit 0 (workspace was already fmt-clean, so `cargo fmt` changed nothing) |

Make targets that did not pass:

| Command | Result |
|---------|--------|
| `make test` | **FAILED.** Run 1: 7 failures in `testcase-manager/tests/dependencies_test.rs` (`Failed to execute validate-yaml: NotFound`). The test looks for `<workspace>/target/debug/validate-yaml`, but this shell sets `CARGO_TARGET_DIR=/home/orca/.cache/cargo-target`. With the binary linked at the expected path, `dependencies_test` passed 55/55. Run 2: stopped at a linker crash, `ld terminated with signal 7 [Bus error]` while linking the `testcase-ui` test binary. The root filesystem is 99% full (929M free). Not rerun. |
| `make coverage-e2e` | **FAILED** (run 1, same `validate-yaml` path failures). Run 2 was killed during `cargo test` because of the full disk. Not completed. |
| `make acceptance-test` | **FAILED** (run 1): `test-plan-documentation-generator (TPDG) not found`. The Makefile looks for that name; `tpdg` is installed at `~/.cargo/bin/tpdg` but was not tried. Run 2 never started. |

Not run: `make test` e2e portion, `make coverage-e2e` coverage threshold, `make acceptance-test` stages. No gate is reported as passed that did not run.

`/home/orca/.cache/cargo-target/llvm-cov-target` holds 1.1G of coverage build output from this session. Freeing disk space is the user's decision, so it was left in place. Re-run `make test`, `make coverage-e2e` and `make acceptance-test` after space is freed.

## Acceptance criteria

- Task 1: all met.
- Task 2: all met (+14 tests against the 35 before; plan requires at least 13).
- Task 3: docs criteria met; `cargo test`, clippy and fmt for both crates pass.
- Make-target gates: not met (environment, see above).

## Deviations from Plan

### Interpretation

**1. Kinds list when traceability is selected.** The plan says rendered kinds are "listed as 'rendered below'". When traceability is selected, the output still starts with `# Traceability Report` (a Phase 2 requirement), so there is no `Report kinds` block before it. Each non-traceability kind is preceded by `Report kind <kind>: rendered below` and then its body. When traceability is not selected, the `Report kinds` block lists each kind as `  <kind>: rendered below`.

**2. Rendering order.** Summary, gaps and problems keep the `ReportKind` order from the plan (traceability, summary, gaps, problems).

### Auto-fixed issues

**1. [Rule 1 - Bug] Problems section slice in new tests.** My first version of the problems tests used the `## `-bounded `section()` helper. The problems document has its own `## Report header`, so the slice stopped there, and the `!contains` checks passed vacuously. Fixed by adding a `tail()` helper for the last document. Found by a failing test, before commit `2b43ee9`.

**2. [Rule 1 - Bug] Traceability slice in consistency test.** The consistency test also picked up summary rows. Fixed by slicing the traceability text before `# Run Summary`.

**3. Test renamed.** `default_kinds_are_consistent` became `consistency_across_default_kinds`, so the name contains `consistency` as the plan lists.

### Environment

- `cargo fmt -p test-executor` was run to apply formatting to my own changes; only `crates/test-executor/src/main.rs` and the test file changed.
- A temporary symlink `target/debug/validate-yaml` was created to check the `dependencies_test` finding. The `target/` directory it lived in was removed afterward. The working tree is clean apart from this summary file, which is not committed.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check

- Commits `e18f652`, `2b43ee9`, `5b668b3` exist on `traceability-matrix-report`.
- No push, no STATE.md, ROADMAP.md, ETA.md or backlog writes.
- This summary is not committed.
