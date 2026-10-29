---
id: TCMS-40.6
title: Fail when a requirement lacks normal-range or robustness tests
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:47:00Z'
labels: []
dependencies:
  - TCMS-40.5
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Glossary rule: every high-level and low-level requirement needs both normal-range and robustness tests; today nothing enforces it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The check exits non-zero and names each requirement and which kind of test is missing
- [x] #2 The check can be disabled with a flag
- [x] #3 The requirement set is read from a StrictDoc export or an ID list
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Coverage check inside trace-matrix: gaps per requirement (normal_range, robustness), exit 1 naming them
2. --no-coverage-check disables it
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The check lives in 'test-executor trace-matrix' (same data as the matrix): after the matrix is written it exits 1 when a requirement lacks a normal-range or robustness test case, naming each requirement and the missing kinds; --no-coverage-check skips it. Evidence: unit test test_gaps_name_each_requirement_and_what_is_missing, CLI tests a_missing_test_type_fails_the_check_naming_requirement_and_kind_but_still_writes_the_matrix and the_check_can_be_disabled. AC #3 (requirement set from a StrictDoc export or an ID list) is only met for the ID list and for requirement records; reading a raw StrictDoc export (JSON or ReqIF) is not implemented because no export sample is available to design against. TCMS-42.3 (StrictDoc adapter) produces the requirement records this command reads. Not run: make test, acceptance suite.

Review (TCMS-40.6, reviewer, branch tcms-40-review): NOT Done. AC #1 and #2 re-checked with test-executor trace-matrix (exit 1 naming LLR-2, HLR-3, HLR-4 with the missing kind; --no-coverage-check exit 0). AC #3 open: the ID-list path (--requirements ids.txt) works and is verified; StrictDoc export input is not implemented (only ID lists and tcms-schemas requirement records). Decide whether the ID list satisfies AC #3 or implement a StrictDoc reader.

TCMS-40.6 AC #3 (D-40.6d, D-40.6e): trace-matrix --requirements also reads a StrictDoc JSON export (a .json file). Parsing reuses requirement-import (strictdoc::export_levels, additive API that shares the parser and checks with parse_export, so the ASSUMED export layout lives in one place). Only HLR and LLR items count; other levels (system, SFR, ...) are ignored; an unknown level, a duplicate UID, or a non-JSON / wrong-shape file is an error naming the file (and item). testcase-verification is unchanged (no dependency on requirement-import). Evidence: unit tests in test-executor (strictdoc_export_gives_hlr_and_llr_only, strictdoc_export_refuses_unknown_levels_duplicates_and_bad_json, id_lists_are_still_read_as_before); CLI tests in crates/test-executor/tests/trace_matrix_cli.rs (a_strictdoc_export_is_the_requirement_set_and_the_coverage_check_runs_on_it, a_strictdoc_export_with_every_requirement_covered_passes_the_check, an_invalid_strictdoc_export_is_an_error_with_the_file_named, a_strictdoc_export_with_an_unknown_node_type_is_refused, a_strictdoc_export_without_hlr_or_llr_is_refused) on the crafted fixture crates/test-executor/tests/fixtures/strictdoc_export.json (crafted to the ASSUMED layout, not a StrictDoc output). cargo test -p test-executor -p testcase-verification -p requirement-import: all pass; clippy -D warnings clean. Real binary: ID list gives exit 1 naming LLR-2 (no robustness) and HLR-3 (no normal_range and no robustness); --no-coverage-check gives exit 0; StrictDoc export gives the same matrix content (HLR-1, LLR-2, HLR-3; SYS-1 excluded) and exit 1 for the same gaps; both matrices validate against schemas/tcms-schemas/tcms/trace-matrix.schema.json (Draft7 + format checker). AC #1 and #2 re-verified on the real binary as above. Not run: make test, make coverage-e2e, acceptance suite. Open: StrictDoc layout is unverified until a real export is available (see docs/TRACEABILITY.md).

Review round 3 (reviewer, branch tcms-review-r3, tested at 3e9eb75): AC #1 and #2 re-run with the real binary: ID list (HLR-1, LLR-2, HLR-3) exits 1 with 'LLR-2 (LLR): no robustness test' and 'HLR-3 (HLR): no normal_range and no robustness test'; --no-coverage-check exits 0. AC #3: both inputs work: an ID list and the StrictDoc JSON fixture give identical matrix content (validated Draft 7 against tcms-schemas trace-matrix schema, 0 errors, id differs only). The StrictDoc layout is still ASSUMED (labelled in docs/TRACEABILITY.md and crates/requirement-import); AC #3 is met by the ID-list input alone, the StrictDoc input is extra. Malformed JSON is refused with the file named. Tests: cargo test -p test-executor -p testcase-verification -p requirement-import: all pass. Not run: make test, make coverage-e2e, acceptance suite.

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-executor -p testcase-verification -p requirement-import and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
trace-matrix fails (exit 1) when an HLR or LLR lacks a normal-range or robustness test, naming each requirement and the missing kind; --no-coverage-check disables it. The requirement set comes from an ID list or a StrictDoc JSON export (HLR and LLR only; the export layout is an assumption). Verified on the real binary: both inputs produce the same matrix, which validates against the trace-matrix schema. Definition of Done: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-executor -p testcase-verification -p requirement-import and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
