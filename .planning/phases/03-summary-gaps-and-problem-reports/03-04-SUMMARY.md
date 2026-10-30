# Phase 3 Plan 04: Problem Reports View Summary

Problem list of every decided fail and error result, with its problem_report_id, and a visible MISSING flag where the id is absent or blank (PRB-01, PRB-02).

## Tasks

| Task | Name | Commit | Files |
| --- | --- | --- | --- |
| 1 | Problem rows from decided results (D-04, D-05) | 6be87b4 | crates/testcase-reporting/src/problems.rs |
| 2 | Markdown rendering with a visible missing flag (D-04, D-05, D-06) | bc6ba27 | crates/testcase-reporting/src/problems.rs |

Both tasks were test-first: the tests were run against a stub and failed before the implementation was added (Task 1: 6 behaviour tests failed on `unimplemented!`; Task 2: 5 render tests failed on the stub, then the golden expectation was corrected to the fixture header, see Deviations).

## What was built

- `ProblemRow`, `ProblemsReport`, `build_problems`, `render_problems_markdown` in `problems.rs`.
- Rows come only from `selection.results`, filtered to Fail and Error. Superseded results never appear.
- `problem_report_id` is trimmed; absent or blank becomes `None`, is counted in `missing_count`, and renders as `**MISSING problem_report_id**`.
- Requirements come from `view::linked_cases`; an unlinked failing case is listed with no requirements (`-`).
- Header is the shared Phase 2 `header_markdown`. Every model-derived string goes through `render_markdown::cell`.

## Gate results

- `cargo fmt --check`: exit 0.
- `cargo clippy -p testcase-reporting -p test-executor -- -D warnings`: exit 0.
- `cargo test -p testcase-reporting -p test-executor`: exit 0. testcase-reporting lib: 120 passed, 0 failed (12 in `problems::`).
- `grep -c 'Utc::now\|SystemTime\|std::fs' problems.rs`: 0.

Not run, per instructions: `make acceptance-test`, `make coverage-e2e`.

## Acceptance criteria

- `cargo test -p testcase-reporting problems::` exits 0: met.
- Table-driven test over the eight (verdict, problem_report_id) rows with listed/flagged expectations: met (`verdict_and_report_id_decide_listing_and_flag`).
- Fixture rows exactly [T2, T3], `missing_count == 1`, T3 `is_missing()`, T2 id `PR-7`: met (`fixture_rows_are_t2_then_t3`).
- Superseded fail gives empty rows: met (`superseded_fail_gives_empty_rows`).
- Unlinked failing case listed with empty requirements: met (`unlinked_failure_is_listed_with_no_requirements`).
- Golden test contains the exact T2, T3, count lines: met (`fixture_renders_exact_golden_markdown`).
- Empty report says `No failed or errored results` and `Without problem report: 0`: met.
- Hostile id `PR|1\n9` escaped to one line: met (`hostile_problem_report_id_is_escaped_and_stays_on_one_line`).
- clippy and fmt exit 0: met.

## Deviations

- The golden header lists Environments `e` and Testers `t`, not "not recorded". These values come from the shared fixture records (`run.rs` test helper). My first golden draft assumed "not recorded"; I corrected the expectation to match the fixture. The renderer was not changed.
- Plan-listed test `selection_results_map_is_keyed_for_the_fixture` is a fixture guard, not a behaviour test; it passes on the existing fixture.
- Gate logs were written under `/tmp` rather than the session scratchpad. No project files affected.

## Self-Check: PASSED

Commits 6be87b4 and bc6ba27 exist on `traceability-matrix-report`. Only `problems.rs` was changed. This SUMMARY is uncommitted, as instructed.
