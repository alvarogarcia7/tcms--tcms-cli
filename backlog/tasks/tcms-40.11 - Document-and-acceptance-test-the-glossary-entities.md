---
id: TCMS-40.11
title: Document and acceptance-test the glossary entities
status: Done
assignee:
  - '@worker-a'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:31:00Z'
labels: []
dependencies:
  - TCMS-40.2
  - TCMS-40.3
  - TCMS-40.4
  - TCMS-40.5
  - TCMS-40.6
  - TCMS-40.7
  - TCMS-40.8
  - TCMS-40.9
  - TCMS-40.10
parent_task_id: TCMS-40
priority: medium
type: docs
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users need to know how glossary terms map to commands and files, and the acceptance suite must prove the new behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs describe the glossary-to-file mapping and each new command
- [x] #2 The acceptance suite covers runs with metadata, the traceability matrix, regression selection and the independence check
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions: (D-40.11a) The acceptance scenarios are one standalone script, test-acceptance/validate_glossary_entities.sh, wired into run_acceptance_suite.sh as stage 8 with --skip-glossary. They run the real test-executor, verifier and validate-yaml on committed fixtures (test-acceptance/glossary_entities/fixtures/), so they need no TPDG and no 00_test_cases. (D-40.11b) Fixture test cases use echo steps only, so the scenarios do not depend on the output of other test cases. (D-40.11c) Stage 8 reports through its own exit status and does not feed the suite's *_FAILED counters or the summary report. (D-40.11d) The script looks binaries up in $CARGO_TARGET_DIR first (find_binary does not know it). run_acceptance_suite.sh verify_binaries still only searches the repo's target/; left unchanged as a pre-existing limitation. (D-40.11e) Terms with no CLI command (review gating, coupling, activities, software level, objectives) are marked 'schema only' in docs/GLOSSARY_ENTITIES.md rather than described as enforced. (D-40.11f) Coverage import is not among the four scenario areas named in AC 2, so it is not in the script (it is covered by its own tests, TCMS-40.10). Evidence: docs/GLOSSARY_ENTITIES.md maps every glossary term in tcms-schemas docs/entity-mapping.md to a schema, a code path and a command or 'schema only', and links TCMS_SCHEMAS, RUN_METADATA, TRACEABILITY, INDEPENDENCE, COVERAGE_IMPORT and REGRESSION. Each command it names was run during this work. Script: ./test-acceptance/validate_glossary_entities.sh exit 0, 48 checks passed (runs with metadata 14, traceability 8, regression 4 (plus JSON and plan-mismatch), independence 9). A mutated copy with one wrong expected line exits 1, so the checks are real. Suite: run_acceptance_suite.sh --skip-generation --skip-execution --skip-verification --skip-documentation --skip-consolidated-docs runs stage 8 and reports 'All glossary entity checks passed'; the suite exits 1 because of stage 1 (test-acceptance/00_test_cases is not in the repo, pre-existing) and verify_binaries (binaries are in CARGO_TARGET_DIR). For that run only, I linked target to CARGO_TARGET_DIR and removed the link afterwards. The run overwrote the tracked test-acceptance/30_documentation_source/acceptance_suite_summary.txt; restored from HEAD, not committed. NOT RUN: the full acceptance suite (TPDG, shellcheck and uv are not installed; stages 2 to 7 and the consolidated docs cannot run here); test-acceptance/tests/test_acceptance_suite_e2e.sh; make acceptance-test; make test; bash 3.2 (not installed here); shellcheck on the new script.

Review (round 2, branch tcms-review-r2): ran test-acceptance/validate_glossary_entities.sh with CARGO_TARGET_DIR set to the shared target: exit 0, 48 checks passed. Mutation check: a copy with the expected test-run verdict count changed from 2 to 3 and the incomplete-matrix expectation flipped to exit 0 exits 1 with two named failures (copy removed afterwards). Verified every CLI flag and subcommand the doc names exists in the built binaries (verifier --run-id/--build-id/--tester/--environment/--reviewer/--test-run/--problem-report; test-executor generate --json-log --run-id, trace-matrix, coverage-import, regression-select, independence-assignment create/check). AC1 (doc maps glossary terms to schema, code and command, and marks schema-only terms) and AC2 verified. Stage 8 wiring read in run_acceptance_suite.sh (--skip-glossary). Full acceptance suite not run here (TPDG, shellcheck, uv missing). Review fix in the same branch (commit ed6fcba, folder-mode test-case-dir, see TCMS-41) affects stage 7 only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Glossary-to-file mapping in docs/GLOSSARY_ENTITIES.md and a standalone acceptance script (validate_glossary_entities.sh, stage 8 of the suite, 48 checks on committed fixtures). Round 2 review re-ran the script (exit 0) and proved it fails on a broken expectation; every documented command flag exists. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test testcase-verification and test-executor and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
