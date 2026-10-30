# Phase 3 Plan 02: Run Summary View Summary

Run summary view: per-run and per-requirement counts of pass, fail, error and not_executed plus the total, rendered as Markdown from the shared ViewInput. Buckets sum to totals; superseded results are never counted.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Summary model with per-run and per-requirement counts (D-02) | a4defbb | crates/testcase-reporting/src/summary.rs |
| 2 | Markdown rendering of the summary (D-02, D-06) | fa7d96d | crates/testcase-reporting/src/summary.rs |

## What was built

- `RunSummaryRow`, `RequirementSummaryRow`, `SummaryReport`, `build_summary(ViewInput) -> SummaryReport`.
- Per-run pass/fail/error come from decided `selection.results` for that run; `not_executed` from `RunReport.not_executed`; `total` from `RunReport.total - RunReport.superseded`, with a `debug_assert_eq!` on the bucket sum.
- Per-requirement counts from `ResultCounts::from_outcomes` over each model row's cases; status copied from the model.
- `linked` counts the distinct live linked test cases via `view::linked_cases`.
- `render_summary_markdown(&SummaryReport) -> String`: `# Run Summary`, shared `header_markdown`, run table, requirement table, linked line. Every model-derived string goes through `cell`.

## TDD gate

- Task 1 RED: the tests were written together with the implementation in one edit. The RED state was then observed by inserting a temporary `panic!` at the top of `build_summary`; all 4 tests failed with `RED: build_summary not implemented`. The panic was removed before GREEN (4 passed).
- Task 2 RED: a stub `render_summary_markdown` returning an empty string was used; 4 rendering tests failed (golden, hostile ID, no-runs, no-requirements). The byte-identical test passed against the stub, so it does not discriminate on its own; it still asserts equality of two independent renderings.
- GREEN for Task 2: 9 of 9 summary tests passed.

## Gate results

| Gate | Command | Result |
| ---- | ------- | ------ |
| Format | `cargo fmt -p testcase-reporting -- --check` | exit 0 |
| Lint | `cargo clippy -p testcase-reporting -p test-executor -- -D warnings` | exit 0 |
| Test | `cargo test -p testcase-reporting -p test-executor` | exit 0; testcase-reporting 9 summary tests passing, all suites ok |

Acceptance greps:
- `grep -c 'Utc::now\|SystemTime\|std::fs' summary.rs` = 0
- `grep -c 'HashMap\|Utc::now\|env::var' summary.rs` = 0

`make acceptance-test` and `make coverage-e2e` were not run, per the instruction for this plan.

## Acceptance criteria

- `cargo test -p testcase-reporting summary::` exits 0: met.
- Table-driven test covers the six fixture rows with exact five numbers, plus linked: met (`fixture_counts_match_the_expected_table`).
- `bucket_sum() == total` asserted for every run and requirement row in fixture, superseded and excluded scenarios: met (`assert_buckets_sum`).
- Superseded RUN-A has total 0 and pass 0 while remaining in `runs`: met.
- Golden test contains the required header lines and `pass 1, fail 1, not_executed 1, error 1, total 4`: met.
- Byte-identical test name and `assert_eq!`: met.
- Hostile `R|1\n` renders as `R\|1` on one row: met.
- clippy `-D warnings` and fmt check exit 0: met.

## Deviations from Plan

**1. [Rule 1 - Lint] `clone` on a Copy type**
- **Found during:** Task 1 clippy gate
- **Issue:** `row.level.clone()` on `RequirementLevel`, which is `Copy`, fails `clippy::clone_on_copy` under `-D warnings`.
- **Fix:** `level: row.level`.
- **Files modified:** crates/testcase-reporting/src/summary.rs (included in commit a4defbb)

**2. Formatting applied with rustfmt on the single file**
- `cargo fmt --check` flagged layout in summary.rs; applied `rustfmt --edition 2021` to `summary.rs` only, so no other file in the crate was touched.

No other deviations. No files outside `files_modified` were edited. Nothing was pushed. STATE.md, ROADMAP.md, ETA.md and backlog files were not written.

## Known Stubs

None.

## Threat Flags

None. All strings from user input pass through `cell` before entering a Markdown table (T-03-04); per-run counts use decided results only (T-03-03); total and buckets come from two sources with an equality check (T-03-05).
