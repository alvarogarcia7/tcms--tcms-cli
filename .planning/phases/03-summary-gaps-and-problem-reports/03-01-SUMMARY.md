# Phase 3 Plan 01: Shared view foundation Summary

Shared view foundation: `ViewInput`, `ResultCounts` (total independent of buckets), `linked_cases`, one shared fixture, and `header_markdown` extracted from the Phase 2 renderer.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Extract shared header and cell helpers (D-06) | deab74b | crates/testcase-reporting/src/render_markdown.rs |
| 2 | ViewInput, ResultCounts, linked_cases, fixture, module stubs (D-01, D-02) | 5a15353 | crates/testcase-reporting/src/{view,summary,gaps,problems,lib}.rs |

## Gate results

- `cargo fmt -p testcase-reporting -- --check`: exit 0
- `cargo fmt -p test-executor -- --check`: exit 0
- `cargo clippy -p testcase-reporting -p test-executor -- -D warnings`: exit 0
- `cargo clippy -p testcase-reporting --all-targets -- -D warnings`: exit 0
- `cargo test -p testcase-reporting -p test-executor`: exit 0, 149 passed, 0 failed (testcase-reporting: 87 passed)

## Acceptance criteria

- `render_markdown::` tests pass, including the unchanged golden, byte-identical and shuffled-input tests: met
- No existing `assert_eq` removed from render_markdown.rs (`git diff -U0 | grep '^-' | grep -c assert_eq` = 0): met
- `pub fn header_markdown` count 1, `pub fn cell` count 1 in render_markdown.rs: met
- No `(03-` commits touching model.rs, rollup.rs, select.rs: met (count 0)
- Slices of length 0..=4 checked: `bucket_sum() == total` and `total == len`: met
- `linked_cases` of fixture returns T1, T2, T3, T4; T2 is Fail with run_id RUN-1: met
- Fixture statuses FAILED, ERRORED, NO TESTS, NOT RUN for HLR-1, LLR-2, HLR-3, LLR-4: met
- `grep -c 'Utc::now\|SystemTime\|std::fs' view.rs` = 0: met
- clippy `-D warnings` and fmt exit 0: met

## Deviations from Plan

**1. [Rule 3 - Blocking] Renamed call site in an existing test**
- **Found during:** Task 1
- **Issue:** The existing test `blank_matrix_id_and_baseline_render_not_recorded` called the old private `header_block`. Renaming the function broke it.
- **Fix:** Changed only the function name at that call site to `header_markdown`. No assertion was changed or removed.
- **Commit:** deab74b

**2. Trailing blank line interpretation**
- The plan said `header_markdown` ends "with a blank line". The Phase 2 section had no trailing newline; `render_markdown` joins blocks with a blank line. `header_markdown` returns the same text as before, so the Phase 2 output stays byte-identical. The separator remains in `render_markdown`.

**3. Fixture problem report ID set after `record()`**
- `run::test_support::record` always sets `problem_report_id` to None. The fixture sets `PR-7` on T2 by mutating the built record, so `run.rs` was not touched.

**4. Task 2 committed as one commit**
- The tests and implementation for view.rs were written together and committed once, not as a separate RED commit.

**5. Shared build lock**
- Another agent's cargo process held the build directory lock during the gate run. The gate ran after it finished. No change to the result.

## Scope

- `model.rs`, `rollup.rs`, `select.rs` untouched.
- `STATE.md`, `ROADMAP.md`, `ETA.md` not written.
- Nothing pushed.
- This SUMMARY.md is not committed.

## Self-Check: PASSED

- Commits `deab74b` and `5a15353` exist on `traceability-matrix-report`.
- Files created: view.rs, summary.rs, gaps.rs, problems.rs exist.
