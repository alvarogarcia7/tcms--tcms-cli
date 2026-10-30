---
phase: 01-strict-input-loading-and-reconciliation
plan: 03
subsystem: reporting
tags: [rust, run-selection, policy, pure-function, tdd]
requires:
  - phase: 01-01
    provides: testcase-reporting crate, LoadedRun with parsed started instant
  - phase: 01-02
    provides: strict loader (load_inputs) that feeds LoadedRun values
provides:
  - select_results: pure per-test-case run selection (D-01..D-05, D-16)
  - RunSelection with used()/excluded() and a fixed Display text block
  - SelectedResult, RunUse, RunReport public types for plan 01-04
affects: [01-04]
tech-stack:
  added: []
  patterns: [sort by parsed instant then run_id, last-write-wins over ordered runs, per-run counters indexed by selection position]
key-files:
  created: []
  modified:
    - crates/testcase-reporting/src/select.rs
key-decisions:
  - "Selection order is (started instant ascending, run_id ascending); the result map is overwritten in that order, so no verdict ranking is used (D-02)"
  - "Excluded reasons are derived from counters: no results, only not_executed results, or every executed result superseded by a later run"
  - "Builds are taken from used runs only (D-04 as written in must_haves)"
  - "RunSelection and RunReport derive PartialEq/Eq so order independence is asserted with assert_eq on whole selections"
patterns-established:
  - "Tie notes name both run IDs in ascending order and the winning run, with the shared started_at text"
requirements-completed: [LOAD-03]
duration: 35min
completed: 2026-10-30
---

# Phase 1 Plan 03: Run selection policy Summary

`select_results` decides each test case from the latest run (by parsed `started_at` instant, then `run_id`) that has an executed result for it. Every loaded run is listed as used or excluded with a reason, every build of a used run is listed, and a fixed text block prints the whole selection.

## Tasks
1. Run selection policy (D-01..D-05, D-16): RED eaf118a (test), GREEN e9754de (feat)
2. Run-selection block text: RED a3c923e (test), GREEN eb4e11c (feat)

TDD gate sequence: a `test(01-03)` commit precedes the `feat(01-03)` commit for both tasks. Each RED step was confirmed to fail before its GREEN step.

## Verification
- `cargo test -p testcase-reporting`: 32 passed (23 existing, 9 new in `select::tests`)
- `cargo clippy -p testcase-reporting --all-targets -- -D warnings`: clean
- `cargo fmt -p testcase-reporting -- --check`: clean
- `cargo test -p test-executor --test trace_matrix_cli`: 11 passed (legacy path unchanged)
- `cargo tree -p testcase-reporting --invert`: only test-executor depends on the crate (D-09, unchanged in this plan)
- Acceptance greps in `select.rs`: no `std::fs`, `std::io::Read` or `Utc::now` (pure function); no string comparison of timestamps; the four required reason and tie strings are present
- The demo case (RUN-1 pass, RUN-2 fail, RUN-3 not_executed) resolves to T1 = fail from RUN-2, RUN-3 excluded with "only not_executed results", RUN-1 excluded with "every executed result superseded by a later run". The full Display output is asserted with `assert_eq!`.

## Coverage of decisions
- D-01: runs sorted by `DateTime<Utc>`; `+02:00` offsets compared as instants (table row "offsets compare as instants")
- D-02: pass/fail/error each override the other in the table; no verdict ranking
- D-03: not_executed results never decide; a run that omits a test case leaves its result alone
- D-04: `builds` is the set of build IDs of used runs
- D-05: every run has a `RunReport` with counts; mixed run stays Used with decided/superseded/not_executed = 1/1/1
- D-16: equal instants ordered by run_id; the later run wins and a note names both runs

## Deviations from Plan

**1. [Spec reconciliation] Builds line in the sample layout**
- **Found during:** Task 2 (writing the exact-layout test)
- **Issue:** The interfaces sample shows `Builds used: b1, b2` next to `Runs used: 1` with only RUN-2 (build b2) used and RUN-1 (build b1) excluded. This contradicts the must_haves rule that builds are those of the used runs, and it contradicts the plan's own build test (D-04 with RUN-1 b1 deciding T1 and RUN-3 excluded).
- **Fix:** Implemented the rule. For the demo the builds line reads `  Builds used: b2`. Every other line matches the sample layout exactly.
- **Files modified:** crates/testcase-reporting/src/select.rs
- **Commit:** eb4e11c (test asserting the layout in a3c923e)

**2. [Rule 3 - Blocking] RED stubs for `used()`/`excluded()`**
- **Found during:** Task 1 RED
- **Issue:** `todo!()` bodies for functions returning `impl Iterator` do not compile (the hidden type falls back to `()`).
- **Fix:** RED stubs return `std::iter::empty()`, so the tests compile and fail at runtime on the stubbed `select_results`.
- **Commit:** eaf118a

**3. [Minor] Extra derives**
- `RunSelection` and `RunReport` also derive `PartialEq`, `Eq`. This is needed for the order-independence assertions. `RunUse` derives `PartialEq`, `Eq` (used by `used()` and `excluded()`).

**Not run:** The full CLAUDE.md pre-commit set (`make build`, `make lint`, `make test`, `make coverage-e2e`, `make acceptance-test`) was not run, same as plans 01-01 and 01-02. `cargo-llvm-cov` and the test-plan-documentation-generator binary were not found on PATH in this session, so coverage and acceptance cannot run here. The plan's verification commands (crate tests, clippy, fmt, and the legacy trace_matrix CLI tests) were run.

## Known Stubs
None. `select.rs` is fully implemented. `load.rs` and `run.rs` are unchanged in this plan.

## Threat Flags
None. The module is an in-memory pure function with no I/O, no new endpoints and no new trust boundaries. T-01-10 (timestamp ordering) is mitigated by instant comparison with run_id tie-break; T-01-11 (hidden run choice) is mitigated by per-run used/excluded reports and tie notes.

## Self-Check: PASSED
- FOUND: crates/testcase-reporting/src/select.rs
- FOUND commits: eaf118a, e9754de, a3c923e, eb4e11c
- STATE.md and ROADMAP.md were not modified by this plan; the pre-existing unstaged edits to STATE.md and backlog task files were left unstaged
