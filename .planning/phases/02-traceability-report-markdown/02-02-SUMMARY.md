# Phase 2 Plan 02: Report model Summary

Pure `ReportModel` for the traceability report: one header, an inputs section carrying the Phase 1 text verbatim, document sections holding one row per matrix link with its roll-up status, coverage flags and per-case outcomes, and obsolete test cases excluded from rows and roll-up but counted in the header.

## Performance

- **Tasks:** 2 of 2 completed
- **Commits:** 2 task commits plus this metadata commit
- **Files touched:** `crates/testcase-reporting/src/model.rs` (new), `crates/testcase-reporting/src/lib.rs`

## Accomplishments

- `build_model` produces one `RequirementRow` per matrix link. Status comes from `roll_up` over the row's live cases. Coverage flags and level are copied from the link and kept apart from status (D-04, D-09).
- Obsolete cases (status `obsolete` in the supplied test cases) are dropped from `cases` and from the roll-up. The header counts distinct obsolete cases (`obsolete_excluded`), or `None` when no test cases were supplied (D-06, ROLL-03).
- Each case carries its `TestColumn` (normal_range, robustness, other), its `CaseOutcome` from `case_outcome`, and the run ID that decided it. A case with no executed result is NOT RUN with no run ID (D-05, D-10).
- Header lists run IDs, build IDs, environments and testers from used runs only, sorted and deduplicated. Baseline stays `None` when not recorded. `generated_at` is the injected string, verbatim. The model never reads the clock.
- Grouping under documents (D-07): one section per listed document, including documents with no requirements. A requirement whose document is not listed gets its own section. Rows without a mapped document go to a trailing unnamed section. Without a document list there is one unnamed section.
- Deterministic ordering (D-12, TRC-05): sections by name, rows by requirement ID, cases by test case ID. Shuffled matrix links or case IDs give an equal model. Identical inputs give identical `Debug` text.

## Task Commits

1. **Task 1: model types, header, rows, obsolete handling** - `0d4b351` (`feat(02-02): report model header, rows and obsolete handling [ci skip]`)
2. **Task 2: document grouping and deterministic ordering** - `4d0ec72` (`feat(02-02): group rows by document and sort deterministically [ci skip]`)

## Files Created/Modified

- `crates/testcase-reporting/src/model.rs` - `TestColumn`, `RequirementDocuments`, `ReportInputs`, `Header`, `CaseEntry`, `RequirementRow`, `DocumentSection`, `InputsSection`, `ReportModel`, `build_model`, and 19 unit tests.
- `crates/testcase-reporting/src/lib.rs` - adds `pub mod model;`.

## Decisions Made

- Tests and implementation are committed together per task, as in 02-01. Tests were written against the behavior list first and failed to compile before the implementation existed. This keeps every commit green, which CLAUDE.md requires.
- `obsolete_excluded` counts the distinct obsolete test cases in the supplied list, not only those that are linked to a requirement. This follows the plan text. The header therefore reports the same number whether or not an obsolete case is linked.
- A requirement with no document mapping and no document list at all is placed in the single unnamed section. When a document list is supplied, unmapped requirements form a trailing unnamed section only when such requirements exist.
- The header skips blank run IDs, build IDs, environments and testers. An empty list means "not recorded", and the renderer prints the wording (D-11).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Tests and implementation committed together per task**
- **Found during:** Task 1 and Task 2
- **Issue:** The plan asks for a RED commit before each GREEN commit. CLAUDE.md requires that all tests pass before any code change is committed, so a failing-test commit is not allowed.
- **Fix:** The tests were written from the behavior list and checked against the implementation before each commit. Both task commits are green.
- **Commits:** `0d4b351`, `4d0ec72`

**2. [Rule 3 - Blocking] rustfmt applied to the new module**
- **Found during:** Task 1
- **Issue:** `cargo fmt -p testcase-reporting -- --check` reports a diff in `model.rs`.
- **Fix:** Ran `cargo fmt -p testcase-reporting`. Only `model.rs` changed, and it is part of the task commit.

## Verification Gates

| Gate | Result |
| ---- | ------ |
| `cargo test -p testcase-reporting` | passed: 62 (43 earlier, 12 from Task 1, 7 from Task 2) |
| `cargo clippy -p testcase-reporting --all-targets -- -D warnings` | passed |
| `cargo fmt -p testcase-reporting -- --check` | passed |
| `grep -c 'Utc::now\|SystemTime' model.rs` | 0 |
| `make lint`, `make test`, `make coverage-e2e` | NOT RUN: the cargo equivalents above were run. `cargo-llvm-cov` is not installed and the environment forbids installing packages |
| `make acceptance-test` | NOT RUN: `test-plan-documentation-generator` (TPDG) is not installed and the environment forbids installing packages |

## TDD Gate Compliance

The plan is `type: tdd`, but no `test(02-02)` commit exists. The tests and implementation for each task landed together in one `feat(02-02)` commit, as in 02-01. The RED state was not recorded in git history.

## Issues Encountered

- None blocking.

## Known Stubs

None. The model is pure data. The renderer (plan 02-03 and later) decides how "not recorded" and "No traceable content" are written.

## Threat Flags

None. The model is built from in-memory values with no I/O, clock or new dependency. Threat T-02-03 (obsolete cases causing a FAILED roll-up) is covered by `obsolete_case_is_dropped_from_row_and_roll_up_and_counted`. Threat T-02-04 (rows dropped for lack of a document) is covered by `sections_are_sorted_by_document_with_unnamed_last`, which asserts that the total row count equals the link count.

## Next Phase Readiness

- `build_model` and `ReportInputs` are ready for the Markdown renderer. The renderer reads only `ReportModel` and writes all user-visible wording.
- The `RequirementDocuments` input still has to be produced by the CLI from the requirements source. That wiring belongs to a later plan.

## Self-Check: PASSED

- Created file: `crates/testcase-reporting/src/model.rs` FOUND.
- Commits: `0d4b351` FOUND; `4d0ec72` FOUND.
- Tests: `cargo test -p testcase-reporting` exits 0 with 62 passing.
