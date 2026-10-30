---
phase: 01-strict-input-loading-and-reconciliation
plan: 02
subsystem: reporting
tags: [rust, loaders, schema-validation, reconciliation, fail-closed]
requires:
  - phase: 01-01
    provides: testcase-reporting crate, LoadedRun with parsed started_at
provides:
  - load_inputs: strict matrix and run loader (fail closed, every bad file named in one error)
  - Reconciliation: every discovered input loaded or skipped with a reason, plus D-18 notes
  - check_unique_run_ids (D-08) and matrix_notes (D-18)
affects: [01-03, 01-04]
tech-stack:
  added: []
  patterns: [collect all failures before bailing, BTreeMap/BTreeSet for deterministic output, schema check before typed parse]
key-files:
  created: []
  modified:
    - crates/testcase-reporting/src/load.rs
key-decisions:
  - "Schema URI is checked against RUN_SCHEMA / MATRIX_SCHEMA before load_and_validate_yaml runs, so a lenient schema cannot be selected by the document"
  - "Directory candidates are deduplicated by canonical path; a file named explicitly wins over the same file found through a directory"
  - "RunOutcome::Loaded is boxed to keep the enum small (clippy large_enum_variant)"
patterns-established:
  - "Each failure is one line that starts with the offending path; multi-line library errors are collapsed to one line"
requirements-completed: [LOAD-01, LOAD-02, LOAD-04]
duration: 40min
completed: 2026-10-30
---

# Phase 1 Plan 02: Strict report input loader and reconciliation Summary

`load_inputs` loads one trace matrix and run records from files and directories through the schema-validated loader, rejects bad inputs all at once with each failure naming its file, fails duplicate `run_id` and duplicate test cases within a run, and returns a Reconciliation that lists every discovered file as loaded or skipped with a reason, followed by matrix/run mismatch notes.

## Tasks
1. RED then GREEN for load_inputs (failures, dedupe, skip reasons, duplicate checks): fef9088 (test), 598733d (feat)
2. Matrix notes and reconciliation block layout: adfdaca (test), 84d5576 (feat)

TDD gate sequence: test commits precede feat commits for both tasks.

## Verification
- `cargo test -p testcase-reporting`: 23 passed (5 from plan 01-01, 18 new in load::tests)
- `cargo clippy -p testcase-reporting --all-targets -- -D warnings`: clean
- `cargo fmt -p testcase-reporting -- --check`: clean
- `cargo test -p test-executor --test trace_matrix_cli`: 11 passed (legacy path unchanged)
- Acceptance greps: `load_and_validate_yaml::<TraceMatrix` and `::<TestRunRecord` present; zero references to `legacy::` and zero typed `serde_yaml::from_str` of the two record types in load.rs

## Deviations from Plan

**1. [Rule 1 - Test assertion matched the wrong layer] Non-RFC 3339 `started_at` is rejected by the schema first**
- **Found during:** Task 1 GREEN
- **Issue:** The test-run schema declares `started_at` with `format: date-time`, so `load_and_validate_yaml` rejects `"yesterday"` with a message naming the file and `date-time`, before `LoadedRun::new` runs. The behaviour asked for an error containing "RFC 3339".
- **Fix:** The test accepts either wording, with the file path still required. `LoadedRun::new` remains as the second check for values the schema's format check does not catch.
- **Files modified:** crates/testcase-reporting/src/load.rs
- **Commit:** 598733d

**2. [Spec reconciliation] Failure line layout**
- **Found during:** Task 1 design
- **Issue:** Step (4) of the plan asks for failures indented two spaces; the must_haves ask for each line to start with the offending path. These conflict.
- **Fix:** Followed must_haves: header line, then one unindented line per failure starting with the path. Multi-line library errors are collapsed to one line so that no continuation line escapes the path prefix.
- **Commit:** 598733d

**3. [Rule 3 - Lint] Clippy fixes**
- Test calls used `&[x.clone()]`, changed to `std::slice::from_ref(&x)`.
- `RunOutcome::Loaded` boxed because of `large_enum_variant`.
- **Commit:** 598733d

**Not run:** The full CLAUDE.md pre-commit set (`make build`, `make lint`, `make test`, `make coverage-e2e`, `make acceptance-test`) was not run, same as plan 01-01. The plan's verification commands (`cargo test -p testcase-reporting`, clippy, fmt, and the legacy trace_matrix CLI tests) were run. The build used the OpenSSL environment variables from the execution context.

## Known Stubs
None. `select.rs` remains doc-comment-only and is owned by plan 01-03, as planned in 01-01.

## Threat Flags
None. The changes add no new network, auth, or filesystem trust surface beyond the planned loader. The directory scan is non-recursive (T-01-09 accepted in the plan).

## Self-Check: PASSED
- FOUND: crates/testcase-reporting/src/load.rs
- FOUND commits: fef9088, 598733d, adfdaca, 84d5576
- Worktree: STATE.md and ROADMAP.md were not modified by this plan; pre-existing unrelated edits to STATE.md and backlog task files were left unstaged
