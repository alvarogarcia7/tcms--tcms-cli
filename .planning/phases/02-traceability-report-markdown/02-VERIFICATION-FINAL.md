---
phase: 02-traceability-report-markdown
verified: 2026-10-31T00:00:00Z
status: passed
score: 5/5 roadmap success criteria verified; 7/8 acceptance criteria verified (AC #6 absent from task file, WARNING)
warnings:
  - "Header 'Obsolete test cases excluded' counts every obsolete case in the supplied --test-cases set (model.rs:130-134, 267), including obsolete cases that no requirement links to. The roll-up exclusion itself is correct (model.rs:146)."
  - "backlog/tasks/tcms-43.2 - Phase-2-Traceability-report-Markdown.md has no acceptance criterion #6 (list goes #5, #7, #8). Human to confirm whether #6 was dropped or renumbered."
  - "Phase-level gates not run by this verifier per instruction: make test, make coverage-e2e, make acceptance-test. DoD item 'make test' in tcms-43.2 remains open."
---

# Phase 2: Traceability Report (Markdown) - Final Verification

**Branch:** traceability-matrix-report (HEAD 8bfbdf0, clean tree at start)
**Scope:** SC1-SC5 from ROADMAP.md Phase 2; AC #1-#8 from tcms-43.2.

## Commands run

| Command | Result |
| --- | --- |
| `env -u CARGO_TARGET_DIR cargo test -p testcase-reporting -p test-executor` | EXIT 0. testcase_reporting lib: 137 passed. test-executor: unittests 6, coverage_import_cli 3, independence_cli 3, regression_select_cli 4, report_cli 56, report_kinds_e2e 14, report_traceability_e2e 13, trace_matrix_cli 11, all passed, 0 failed. |
| `env -u CARGO_TARGET_DIR cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | EXIT 0, no warnings. |

## Roadmap Success Criteria

| SC | Statement (abridged) | Status | Evidence |
| --- | --- | --- | --- |
| SC1 | Each requirement lists linked cases, verdict, run; HLR and LLR distinct | VERIFIED | Level column `render_markdown.rs:104-118`; cell `<id> (<outcome>, <run>)` `render_markdown.rs:121-141`; model run_id `model.rs:~140` (`selection.results`); test `grid_rolls_up_each_status_on_its_own_requirement` (report_traceability_e2e.rs:230) asserts `TV (pass, RUN-1)` and `| HLR-V | HLR |` |
| SC2 | VERIFIED only if all cases pass; else FAILED/ERRORED/NOT RUN/PARTIAL/NO TESTS | VERIFIED | `rollup.rs:84-96` roll_up (order: none->NoTests, Error->Errored, Fail->Failed, all Pass->Verified, Pass present->Partial, else NotRun); labels `rollup.rs:53-62`; rule documented `docs/TRACEABILITY.md:250-267`; E2E `grid_rolls_up_each_status_on_its_own_requirement`, `grid_scenario_rolls_up_failed_errored_no_tests_and_not_run`, `grid_error_outranks_fail_and_fail_outranks_a_missing_result` |
| SC3 | Coverage shown separately from status; linked case without result shows NOT RUN | VERIFIED | Coverage columns `render_markdown.rs:104-118` (yes/no, not folded into status); `model.rs` case_outcome->NotRun; `render_markdown.rs:~130` "no result" -> `TN (NOT RUN, no result)`; tests `grid_rolls_up...` (has_line `| HLR-N | ... TN (NOT RUN, no result)`) and report_cli.rs:656 `not_run_case_is_never_a_pass` |
| SC4 | Obsolete cases excluded from roll-up and counted in header | VERIFIED (with WARNING) | Exclusion `model.rs:130-146`; header count `model.rs:267`; unit test `obsolete_case_is_dropped_from_row_and_roll_up_and_counted` (model.rs ~422) and `obsolete_count_is_none_without_test_cases` (~455); CLI test `obsolete_case_is_excluded_and_counted_with_test_cases` (report_cli.rs:727). WARNING: count is over all supplied obsolete cases, not only linked ones. |
| SC5 | Byte-identical output for same inputs and --generated-at; header fields or "not recorded" | VERIFIED | Clock injected: `main.rs:630-638` (only default clock in `fn report`, `main.rs:677`); renderer has no clock/hash (grep of model.rs, render_markdown.rs, render_shapes.rs, rollup.rs: no Utc::now/HashMap). Header rows `render_markdown.rs:53-80`. E2E `same_inputs_and_generated_at_give_identical_bytes_in_every_shape` (report_traceability_e2e.rs:484), `different_generated_at_differs_only_in_the_generated_row` (:504), `blank_baseline_renders_not_recorded_in_every_shape` (:430), `blank_matrix_id_renders_not_recorded_in_grid_and_shape_b` (:455). Golden exact-text test `golden_layout_matches_exact_text` (render_markdown.rs:256). |

## Acceptance Criteria (tcms-43.2)

| AC | Statement (abridged) | Status | Evidence |
| --- | --- | --- | --- |
| #1 | Requirement lists cases, verdict, run; HLR/LLR distinct | VERIFIED | As SC1 |
| #2 | VERIFIED only when all pass; documented rule | VERIFIED | `rollup.rs:84-96`; `docs/TRACEABILITY.md:250-267`; tests in SC2 row |
| #3 | Coverage separate; linked case with no result is NOT RUN | VERIFIED | As SC3 |
| #4 | Obsolete excluded from roll-up and counted in header | VERIFIED (WARNING on count scope) | As SC4 |
| #5 | Byte-identical output; header fields or "not recorded" | VERIFIED | As SC5; AC #5 "not recorded" also covered by commits 0f75467 path, tests above |
| #6 | (no such criterion in task file) | UNCERTAIN (WARNING) | Task file numbering jumps #5 to #7. Nothing to verify against. Human decision needed. |
| #7 | Shape A: REQUIREMENT \| test cases (ID, status) \| final verdict | VERIFIED | `render_shapes.rs` `table_a` and `case_list`; test `shape_a_exact_rows_for_each_roll_up_status` (report_traceability_e2e.rs:310), `shape_a_scenario_rows_show_each_case_with_its_status` (:333) |
| #8 | Shape B: REQUIREMENT \| verdict, then TESTCASE ID \| status rows | VERIFIED | `render_shapes.rs` `tables_b`; test `shape_b_exact_case_status_and_verdict_rows` (report_traceability_e2e.rs:350) |

## Findings

- BLOCKER: none.
- WARNING 1: Obsolete header count is over-inclusive (model.rs:130-134, 267). Suggest counting only obsolete cases that appear in a matrix link, or relabel the field.
- WARNING 2: AC #6 missing from tcms-43.2. Confirm numbering.
- WARNING 3: `make test`, `make coverage-e2e`, `make acceptance-test` were not run here (per instruction). Phase-level gates remain open.

## Result

Phase goal achieved in code. All five roadmap success criteria and all present acceptance criteria are verified by code and by passing tests. Status: passed, with the warnings above.

_Verified: 2026-10-31_
_Verifier: Claude (gsd-verifier)_
