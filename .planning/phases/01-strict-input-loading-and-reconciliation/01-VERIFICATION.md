---
phase: 01-strict-input-loading-and-reconciliation
verified: 2026-10-30T20:39:40Z
status: passed
score: 5/5 requirements verified; 4/4 success criteria verified
overrides_applied: 0
gaps: []
deferred: []
---

# Phase 1: Strict Input Loading and Reconciliation Verification Report

**Phase Goal:** A user can point `test-executor report` at a trace matrix and test-run records and see exactly which inputs were loaded, skipped, or rejected, and which run won for each test case.
**Verified:** 2026-10-30T20:39:40Z
**Status:** passed
**Re-verification:** No, initial verification

Verification was done against the code and tests in this worktree, not the SUMMARY files. The SUMMARY claims for plans 01-01 to 01-04 were used only as a list of where to look.

## Commands Run

All run from the worktree root with `CARGO_HOME=$HOME/.cargo-user`. No `OPENSSL_DIR` was set; the build used the system OpenSSL.

| Command | Result |
| ------- | ------ |
| `cargo test -p testcase-reporting -p test-executor` | exit 0; 75 passed, 0 failed, 0 ignored |
| `cargo clippy -p testcase-reporting -p test-executor -- -D warnings` | exit 0; no warnings |
| `cargo fmt --check -p testcase-reporting -p test-executor` | exit 0; no diff |

Per-target test counts from the `cargo test` run:

| Target | Passed | Failed |
| ------ | ------ | ------ |
| `testcase_reporting` unit tests (`crates/testcase-reporting/src`) | 34 | 0 |
| `test-executor` binary unit tests | 6 | 0 |
| `tests/coverage_import_cli.rs` | 3 | 0 |
| `tests/independence_cli.rs` | 3 | 0 |
| `tests/regression_select_cli.rs` | 4 | 0 |
| `tests/report_cli.rs` | 14 | 0 |
| `tests/trace_matrix_cli.rs` | 11 | 0 |
| **Total** | **75** | **0** |

Behavioural spot-check of the built binary (`./target/debug/test-executor --no-audit report`) on a fixture in `/tmp/v01-fixture`:

- Valid matrix plus three runs (RUN-1 pass, RUN-2 fail, RUN-3 not_executed): exit 0. Reconciliation printed `Inputs discovered: 4 (loaded: 4, skipped: 0)`. Selection printed `T1  fail  RUN-2`, RUN-1 excluded as "every executed result superseded by a later run", RUN-3 excluded as "only not_executed results".
- Duplicate `run_id` (second file with `RUN-2`): exit 1, stderr `duplicate run_id RUN-2 in /tmp/v01-fixture/dup.yaml and /tmp/v01-fixture/runs/run2.yaml`.
- Matrix missing `links`: exit 1, stderr names `/tmp/v01-fixture/badm.yaml` and the schema violation.

## Success Criteria

| # | Success criterion | Code | Proof | Status |
| - | ----------------- | ---- | ----- | ------ |
| 1 | `test-executor report` with a matrix and run records prints a reconciliation block listing inputs discovered, loaded, and skipped with reason | `crates/testcase-reporting/src/load.rs:126` (`load_inputs`), `Reconciliation` Display at `load.rs:71-106`; `input_summary` in `crates/testcase-reporting/src/lib.rs:17-27`; command in `crates/test-executor/src/main.rs:563-597` | `report_cli::reconciliation_lists_discovered_loaded_and_skipped_inputs` (report_cli.rs:68); `load::tests::reconciliation_block_layout_is_fixed` and `valid_inputs_are_reconciled_with_reasons`; manual run above | VERIFIED |
| 2 | A matrix or run record failing schema validation makes the command fail, naming the file | Matrix: `load.rs:276-304` (type and schema URI checked, then `load_and_validate_yaml`). Run: `load.rs:372-431` (type, schema URI, then `load_and_validate_yaml`). Failures collected and bailed at `load.rs:172-179` with each line starting with the path | `report_cli::an_invalid_matrix_fails_naming_the_file` (92), `an_invalid_run_fails_naming_the_file` (116), `a_yaml_syntax_error_in_a_runs_directory_fails` (135); `load::tests::run_missing_tester_fails_naming_the_run`, `matrix_missing_links_fails_naming_the_matrix`, `all_bad_inputs_are_reported_together`; manual bad-matrix run (exit 1) | VERIFIED |
| 3 | Latest executed result decides per test case; not_executed and omitted runs do not count; output lists runs used and excluded | `crates/testcase-reporting/src/select.rs:152-226` (`select_results`): runs sorted by parsed instant then run_id (154-158); each executed result overwrites the winner (184); not_executed skipped (167-168); exclusion reasons at 208-216; `RunSelection` Display (`select.rs:76-134`) prints used and excluded runs | `select::tests::policy_table` (281), `demo_case_resolves_to_fail_from_run_2` (415), `equal_started_at_is_noted_and_later_run_id_wins` (461), `run_without_results_is_excluded_with_reason` (488), `mixed_run_stays_used_and_counts_each_outcome` (538); `report_cli::latest_executed_result_decides_and_not_executed_does_not_count` (183), `a_later_pass_overrides_an_earlier_fail` (224); manual run above | VERIFIED |
| 4 | Two run records with the same `run_id` make the command fail | `crates/testcase-reporting/src/load.rs:242-260` (`check_unique_run_ids`), called at `load.rs:183` before the result is returned; the error propagates through `main.rs:570` | `report_cli::duplicate_run_ids_fail_naming_both_files` (149: exit 1, names both files); `load::tests::duplicate_run_id_names_both_files` (652); manual duplicate run (exit 1) | VERIFIED |

## Requirements Coverage

| Requirement | Source plan | Description | Code | Proof | Status |
| ----------- | ----------- | ----------- | ---- | ----- | ------ |
| LOAD-01 | 01-02 | Matrix and run records validated; command fails naming the file | `load.rs:276-304`, `load.rs:372-431`, `load.rs:126-180` | Tests in SC2 above | VERIFIED |
| LOAD-02 | 01-02 | Reconciliation block; no input dropped silently | `load.rs:31-106`, `load.rs:126-180`; directory skips carry a reason (`load.rs:378`, `388-390`, `400-410`) | `reconciliation_lists_discovered_loaded_and_skipped_inputs`, `valid_inputs_are_reconciled_with_reasons`, `reconciliation_without_notes_says_none` | VERIFIED (see Warning 1) |
| LOAD-03 | 01-03 | Latest executed result decides; not_executed and omitted runs do not count; runs used and excluded listed | `select.rs:152-226`, `select.rs:76-134` (Display) | Tests in SC3 above | VERIFIED |
| LOAD-04 | 01-02 | Two run records with the same `run_id` fail the command | `load.rs:242-260`, `load.rs:183` | Tests in SC4 above | VERIFIED |
| CLI-01 | 01-01, 01-04 | `test-executor report` exists and selects report kinds from matrix and run records | `crates/test-executor/src/main.rs:172-186` (subcommand and `--kind`), `main.rs:536-560` (`ReportKind`), `main.rs:563-597` (`report`) | `report_cli::report_kinds_default_to_all_and_can_be_selected` (289), `runs_flag_is_required` (356), `output_flag_writes_the_block_to_a_file` (327), `same_inputs_give_identical_output` (364) | VERIFIED (see Info 1) |

## Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `main.rs:570` `report()` | `load::load_inputs` | direct call with `"schemas/"` | WIRED | Errors returned with `?` before any output is written |
| `load_inputs` | `testcase_common::load_and_validate_yaml` | `load.rs:303` (matrix), `load.rs:428` (run) | WIRED | Schema validation runs before typed parse |
| `load_inputs` | `check_unique_run_ids` | `load.rs:183` | WIRED | |
| `main.rs:571` | `select::select_results` | direct call on `inputs.runs` | WIRED | |
| `main.rs:582` | `testcase_reporting::input_summary` | reconciliation plus selection printed | WIRED | Printed to stdout or `--output` file |

## Anti-Pattern Scan

Files scanned: `crates/testcase-reporting/src/{lib,load,run,select,legacy}.rs`, `crates/test-executor/tests/report_cli.rs`, and the `report` section of `crates/test-executor/src/main.rs`.

- No `TODO`, `FIXME`, `XXX`, `todo!`, `unimplemented!` or placeholder code in the new crate. The one `placeholder` match is a test name (`lib.rs:42`, "baseline placeholder"), not a stub.
- No debt markers in files changed by this phase, so the debt-marker gate passes.
- `load.rs` and `select.rs` were doc-comment-only stubs during plan 01-01 and are fully implemented now.

## Warnings and Information

**Warning 1: directory YAML with a non-`test_run` type is skipped, not failed.**
`crates/testcase-reporting/src/load.rs:400-404`: a YAML file found in a `--runs` directory whose `type` is some value other than `test_run` is skipped with the reason `type is '<value>', not test_run`. D-15 (01-CONTEXT.md:59-60) specifies only the case with no `type` field, which is skipped (`load.rs:409`); a YAML syntax error fails. So a run record with a mistyped `type` (for example `type: test-run`) in a runs directory is reconciled as skipped and does not fail the command. The skip is visible in the reconciliation block, so LOAD-02 holds, and SC2 is about files that are run records and fail schema validation, so it is not violated. The code goes further than D-15's wording, so the choice to fail or skip such files needs a human decision. Explicitly named files with the wrong type do fail (`load.rs:404-407`). This does not block the phase.

**Info 1: CLI-01 kinds are not rendered yet.** `crates/test-executor/src/main.rs:586` prints `<kind>: renderer not yet available` for each selected kind. The Phase 1 roadmap note says CLI-01 "is satisfied by the command and its report-kind selection existing from here; later phases add the kinds." The requirement is accepted in that scoped form. Phase 2 owns the traceability renderer.

**Info 2: schemas root is relative to the current directory.** `main.rs:570` passes `"schemas/"`, the same convention as the existing commands (`main.rs:420`). Running `report` from a directory without `schemas/` fails schema resolution. Consistent with the project; not a Phase 1 gap.

**Info 3: quality gates not run.** `make build`, `make lint`, `make test`, `make coverage-e2e` and `make acceptance-test` were not run here. Only the three cargo commands above were run, as the verification request specified. The 01-04 SUMMARY records that `cargo-llvm-cov`, the TPDG binary and shellcheck are not on PATH here.

## Gaps Summary

No gaps. All four success criteria and all five requirements (LOAD-01..04, CLI-01) are backed by code and passing tests, and the three cargo gates are clean. Warning 1 is a policy question for a human and does not change the status.

---

_Verified: 2026-10-30T20:39:40Z_
_Verifier: Claude (gsd-verifier)_
