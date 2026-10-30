# Phase 3 Plan 03: Coverage gaps view Summary

**Gap lists over the report model and their Markdown rendering (GAP-01, D-03, D-06).**

## Tasks

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Gap lists over the model (D-03) | 8f8d60c | crates/testcase-reporting/src/gaps.rs |
| 2 | Markdown rendering of the gaps report (D-03, D-06) | 0dd4ec1 | crates/testcase-reporting/src/gaps.rs |

## What was built

- `GapsReport`, `GapRequirement`, `GapCase`, `build_gaps(ViewInput)`: four lists (no tests, missing normal-range, missing robustness, no recorded result) plus the model header.
- Requirement lists read `VerificationStatus::NoTests` and the model's coverage flags as they are. Lists are sorted by requirement ID (plain string order) or test case ID.
- A requirement with no tests appears in all three requirement lists (decision recorded in the `build_gaps` doc comment, T-03-06).
- `render_gaps_markdown`: `# Coverage Gaps`, the shared `header_markdown`, four headed sections with counts, `None` for empty lists, all model strings through `cell` (T-03-08).

## Tests

- Table-driven `gap_lists_match_expected_for_every_scenario`: fixture, fully covered and run, all-obsolete requirement, case shared by two requirements and never run.
- Fixture asserts: no_tests `[HLR-3]`, missing_normal_range `[HLR-3, LLR-4]`, missing_robustness `[HLR-3, LLR-2]`, no_result `[T4 -> LLR-4]`.
- Shuffled links, test case IDs and runs give an equal `GapsReport`.
- A case decided by a later run is absent from `no_result`.
- Golden test of the fixture rendering; an all-empty report prints `None` four times with `(0)` headings; byte-identical output for identical inputs; exactly one trailing newline; hostile ID with `|` and newline stays on one line with the pipe escaped.

## Gate results

- `cargo fmt --check`: exit 0.
- `cargo clippy -p testcase-reporting -p test-executor -- -D warnings`: exit 0.
- `cargo test -p testcase-reporting -p test-executor`: all test binaries `ok`, 0 failed (testcase-reporting lib: 108 passed, of which 12 are `gaps::`).
- `grep -c 'Utc::now\|SystemTime\|std::fs' crates/testcase-reporting/src/gaps.rs`: 0.
- `make acceptance-test` and `make coverage-e2e` were not run (out of scope per instructions).

## Acceptance criteria

All criteria in both tasks are met.

## Deviations from Plan

**1. [Process] Task 1 test-first order not strictly followed**
- Task 1 tests and `build_gaps` were written in one file write, so I did not observe a failing run before the implementation existed. The tests were run once they compiled and all passed.
- Task 2 followed red-green: the golden and render tests were added first and failed to compile (`cannot find function render_gaps_markdown`) before the renderer was written.

**2. [Process] Task 1 commit amended to add `[ci skip]`**
- The first Task 1 commit (ded6e75) lacked the required `[ci skip]` subject suffix. It was local and unpushed, so it was amended to 8f8d60c. Content unchanged.

**3. [Observation] Concurrent docs commit on the branch**
- Commit 31f8a5b (`docs(03-02): record plan completion and refresh ETA`) landed on the branch between my commits. It was not made by this plan and was not touched.

## Self-Check

- Files: `crates/testcase-reporting/src/gaps.rs` present and the only changed source file.
- Commits 8f8d60c and 0dd4ec1 present in `git log`.
- STATE.md, ROADMAP.md, ETA.md and backlog files not written.
