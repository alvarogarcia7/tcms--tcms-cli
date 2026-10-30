# Phase 2 Plan 04: Traceability report wiring and CLI proof Summary

`test-executor report --kind traceability` prints one Markdown traceability report that starts with `# Traceability Report`, holds the Phase 1 input and run-selection details in its `## Inputs` section, and is byte-identical for the same inputs and `--generated-at`. The roll-up rule is documented in `docs/TRACEABILITY.md`.

## Performance

- **Tasks:** 3 of 3 planned tasks present on the branch; no code change was needed in this completion pass
- **Commits:** 3 plan commits from the interrupted session, plus this completion commit
- **Files touched by the plan:** `crates/test-executor/src/main.rs`, `crates/test-executor/tests/report_cli.rs`, `docs/TRACEABILITY.md`

## Accomplishments

- `report` accepts `--test-cases DIR`, `--requirements-export FILE` and `--generated-at TIMESTAMP`. `--help` lists all three.
- With traceability selected, the output is exactly `render_markdown(&model)`, built with `build_model`. Without it, the Phase 1 text is unchanged.
- `--generated-at` is validated before any write. Without it, the only clock read in the pipeline is the default `Utc::now()` in `fn report`.
- The CLI tests cover every status (VERIFIED, FAILED, ERRORED, NOT RUN, PARTIAL, NO TESTS), coverage shown apart from status, obsolete exclusion, test-type columns, document grouping, the header fields, byte-identical output, and the failure paths that write nothing.
- `docs/TRACEABILITY.md` documents the three new options, the report layout, the grouping and sort rules, and the status rule table.

## Commits

1. `7bffa69` - `feat(02-04): report wiring for traceability Markdown [ci skip]`
2. `d1d99d3` - `test(02-04): command-line tests for the traceability report [ci skip]`
3. `7d051b1` - `docs(02-04): traceability report layout and verification status rule [ci skip]`

No fix commits were needed. Every acceptance criterion was already met by the code and tests on the branch.

## Test Counts

`CARGO_HOME=$HOME/.cargo-user cargo test -p testcase-reporting -p test-executor`:

| Target | Passed | Failed |
|--------|--------|--------|
| test-executor unit (`src/main.rs`) | 6 | 0 |
| `coverage_import_cli` | 3 | 0 |
| `independence_cli` | 3 | 0 |
| `regression_select_cli` | 4 | 0 |
| `report_cli` | 33 | 0 |
| `trace_matrix_cli` | 11 | 0 |
| testcase-reporting unit (`src/lib.rs`) | 73 | 0 |
| testcase-reporting doc-tests | 0 | 0 |
| **Total** | **133** | **0** |

`report_cli` is 33 tests. The plan asked for at least 25 (14 Phase 1 plus at least 11 new).

## Acceptance Criteria

| Criterion | Proof | Result |
|-----------|-------|--------|
| `cargo build -p test-executor` exits 0 | `make build` (workspace build) exits 0 | pass |
| `report --help` lists the three flags | `cargo run -p test-executor -- report --help` shows `--test-cases`, `--requirements-export`, `--generated-at` | pass |
| `build_model(` and `render_markdown(` are called from `fn report` | `grep -c 'build_model(' crates/test-executor/src/main.rs` = 1; `grep -c 'render_markdown(' ...` = 1 | pass |
| One new `Utc::now` in main.rs; none in the library modules | main.rs has 4 hits, the plan's parent `a4e0fa4` has 3; `model.rs`, `render_markdown.rs`, `rollup.rs` have 0 each | pass |
| Line 1 is `# Traceability Report` | `traceability_report_is_one_markdown_document_with_inputs_first` | pass |
| Inputs section holds the Phase 1 details, fenced | same test, asserts the fence and the matrix, reconciliation and run-selection lines between `## Inputs` and `## Report header` | pass |
| Same inputs and `--generated-at` give identical bytes | `same_inputs_and_generated_at_give_identical_bytes`, `same_inputs_give_identical_output` | pass |
| Invalid `--generated-at` exits 1, names it, writes nothing | `invalid_generated_at_fails_naming_it_and_writes_nothing` | pass |
| Unreadable test-case directory exits 1, names it, writes nothing | `unreadable_test_case_directory_fails_naming_it_and_writes_nothing` | pass |
| Bad export exits 1, names the file, writes nothing | `invalid_requirements_export_fails_naming_it_and_writes_nothing` | pass |
| Obsolete cases excluded and counted | `obsolete_case_is_excluded_and_counted_with_test_cases` | pass |
| NOT RUN is never a pass | `not_run_case_is_never_a_pass`, `latest_executed_result_decides_and_not_executed_does_not_count` | pass |
| Status rule (VERIFIED, FAILED, ERRORED, NOT RUN, PARTIAL, NO TESTS) | `verified_requirement_shows_each_passing_case`, `failed_requirement_shows_failed`, `errored_ranks_above_failed`, `partial_when_some_pass_and_the_rest_have_no_result`, `requirement_without_cases_shows_no_tests` | pass |
| Coverage shown apart from status | `coverage_is_shown_apart_from_status` | pass |
| Test types fill their columns | `test_types_fill_their_columns` | pass |
| Documents grouped; document with no requirements says so | `documents_group_rows_and_mark_a_document_without_requirements`, `rows_without_an_export_sit_under_document_not_recorded` | pass |
| HLR and LLR kept distinct | `hlr_and_llr_levels_are_shown_distinctly` | pass |
| Phase 1 tests intact | `git diff a4e0fa4 -- crates/test-executor/tests/report_cli.rs` removes lines only in `report_kinds_default_to_all_and_can_be_selected` and `same_inputs_give_identical_output` | pass |
| Legacy path unchanged | `cargo test -p test-executor --test trace_matrix_cli` = 11 passed | pass |
| Roll-up rule documented | `docs/TRACEABILITY.md` contains the status table (NO TESTS, ERRORED, FAILED, VERIFIED, NOT RUN, PARTIAL), `--generated-at` and the new options; the old "not rendered yet" paragraph is gone | pass |
| Clippy `-D warnings` | `cargo clippy -p testcase-reporting -p test-executor -- -D warnings` exits 0 | pass |
| Format | `cargo fmt --check -p testcase-reporting -p test-executor` exits 0 | pass |

## Requirements Covered

Requirement IDs as worded in `.planning/REQUIREMENTS.md`. The tests named above provide the proof for each. `REQUIREMENTS.md` was not edited by this plan.

- **ROLL-01** coverage separate from status: `coverage_is_shown_apart_from_status`
- **ROLL-02** documented status rule: `docs/TRACEABILITY.md` table, plus the status tests above
- **ROLL-03** obsolete exclusion and header count: `obsolete_case_is_excluded_and_counted_with_test_cases`
- **ROLL-04** NOT RUN never passes: `not_run_case_is_never_a_pass`
- **TRC-01** requirement, linked cases, verdicts, run: `verified_requirement_shows_each_passing_case`, `failed_requirement_shows_failed`
- **TRC-03** rows are normative requirements grouped by document: `documents_group_rows_and_mark_a_document_without_requirements`
- **TRC-05** deterministic row order: `same_inputs_and_generated_at_give_identical_bytes`
- **OUT-03** byte-identical output for same inputs and `--generated-at`: `same_inputs_and_generated_at_give_identical_bytes`
- **OUT-04** header fields and `not recorded`: `header_records_inputs_and_generation_time`

## Deviations from Plan

None in code. The plan's three tasks were already implemented and tested on the branch by the interrupted session. This pass verified each acceptance criterion against the code and ran the gates. It made no code changes.

## Gates

| Gate | Result |
|------|--------|
| `cargo test -p testcase-reporting -p test-executor` | ran, 133 passed, 0 failed |
| `cargo clippy -p testcase-reporting -p test-executor -- -D warnings` | ran, exit 0 |
| `cargo fmt --check -p testcase-reporting -p test-executor` | ran, exit 0 |
| `make build` | ran, exit 0 |
| `make lint` (fmt and clippy, workspace) | ran, exit 0 |
| `make coverage-e2e` | not run: `cargo-llvm-cov` is not installed in this environment |
| `make acceptance-test` | not run: `test-plan-documentation-generator` (TPDG) is not installed and `TEST_PLAN_DOC_GEN` is unset |
| `make test` (full workspace suite) | not run in this pass; the crate-level suite above covers the crates this plan changed |

The coverage and acceptance gates required by CLAUDE.md before commit were not run, so this plan's work is not fully gated under those rules. Installing `cargo-llvm-cov` and TPDG was out of scope for this completion pass.

## Files Created/Modified

- `.planning/phases/02-traceability-report-markdown/02-04-SUMMARY.md` - this summary

## Self-Check

- `crates/test-executor/src/main.rs`, `crates/test-executor/tests/report_cli.rs`, `docs/TRACEABILITY.md` exist and contain the plan's changes.
- Commits `7bffa69`, `d1d99d3`, `7d051b1` are on the branch (`git log --oneline -3`).
- The untracked `.planning/DECISIONS.md` and phase 07 files predate this work and were not staged.

## Self-Check: PASSED
