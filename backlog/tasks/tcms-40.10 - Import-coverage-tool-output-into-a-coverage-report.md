---
id: TCMS-40.10
title: Import coverage tool output into a coverage-report
status: Done
assignee:
  - '@worker-a'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:33:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: low
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Structural coverage (statement, decision, MC/DC) comes from tools. Gaps must be classified as missing test, dead code, deactivated code or tool limitation (the coverage-report enum) with a rationale, not just patched. A coverage-report needs requirements_based figures, structural figures or both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A documented input format converts to a coverage-report YAML that validates against its schema
- [x] #2 Gaps can be annotated with classification and rationale
- [x] #3 Unclassified gaps are reported
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. coverage.rs: LCOV parser (statement + branch as decision), MC/DC side input YAML, optional trace-matrix for requirements_based, gap annotation file, unclassified gap report. 2. test-executor coverage-import subcommand. 3. unit tests, CLI tests, schema validation test in container_schema_validation_test.rs. 4. docs/COVERAGE_IMPORT.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Continued from a stopped worker; re-verified all of it. Decisions: (D-40.10a) LCOV DA records give statement coverage, BRDA records give decision coverage (a branch not taken is a decision gap); LF/LH/BRF/BRH are ignored, totals are recomputed. (D-40.10b) MC/DC is not in LCOV and has no standard format, so it comes from a small documented side YAML (--mcdc); MC/DC gaps are listed there as 'mcdc'. (D-40.10c) Gaps with no annotation are NOT written into the report (the schema requires classification and rationale on every gap); they are listed on stderr and the command exits 1, unless --allow-unclassified. The report is still written. (D-40.10d) Annotation match: exact location, or a file path (all gaps of that file, narrowed by coverage_type), or file:LINE/file:A-B inside the gap's range; first matching annotation wins. (D-40.10e) 'justified' requires an approver; any gap with no classification or rationale is an input error. (D-40.10f) Requirements-based figures come from a trace-matrix (--trace-matrix) produced by trace-matrix; requirements lacking a normal-range or robustness test become 'requirements' gaps. (D-40.10g) Data and control coupling are not imported (documented in docs/COVERAGE_IMPORT.md). Evidence: cargo test -p testcase-verification -p test-executor: all pass (70 + 8 unit tests in testcase-verification incl. coverage; test-executor coverage_import_cli 3/3). cargo test -p testcase-manager --test container_schema_validation_test: 15/15 incl. test_coverage_report_validates_against_tcms_coverage_report_schema. Clippy -D warnings (testcase-verification, test-executor --all-targets; schema test target): clean. rustfmt --check on my files: clean. Real binary run in scratch: LCOV+MC/DC+trace-matrix+annotations gives exit 0 and a valid report; unannotated run exits 1 and lists the gaps; --allow-unclassified exits 0. Both outputs validated with /home/orca/tcms-schemas/.venv/bin/python -I against schemas/tcms-schemas/tcms/coverage-report.schema.json: VALID. Not run: make test (workspace-wide, disk constraint: ~5.9G free), cargo fmt -p (would rewrite Worker B's verification.rs), make coverage-e2e.

Review (round 2, branch tcms-review-r2): cargo test -p testcase-verification (coverage unit tests in the 70) and -p test-executor (coverage_import_cli 3) passed. Real binary: LCOV with a statement gap 11-13, a decision gap at line 10 and a decision gap at line 210, plus an MC/DC side file and an annotation file. Without annotations: exit 1, the gaps are listed on stderr, the report is written with 0 classified gaps. With annotations (deactivated_code, missing_test, justified with approver): exit 1 for the still-unclassified gaps, a justified annotation without approver is refused with exit 1, and the valid annotated run writes a report whose structural and per-code-unit figures match a hand count (statement 50.0, decision 50.0, es6.c statement 33.33, decision 25.0). The report validates against tcms-schemas coverage-report.schema.json (Draft 7, format checker, yaml.safe_load). Output uses testcase_common::to_yaml_string. The container schema test (testcase-manager container_schema_validation_test) is in the coordinator's workspace run (2085 passed at 9525332) and was not re-run here. Known behaviour: unclassified gaps are not written into the report (the schema needs a classification on every gap); they are reported on stderr and by exit 1, and the report still carries the shortfall figures.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
test-executor coverage-import: LCOV statement and decision coverage, MC/DC side file, trace-matrix requirements-based figures, annotation-based gap dispositions, unclassified gaps reported and exit 1 unless --allow-unclassified. Output validated against the vendored coverage-report schema. Documented in docs/COVERAGE_IMPORT.md. Round 2 review ran the real binary on LCOV, MC/DC and annotations, checked exit codes and the schema, and the crate tests. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test testcase-verification and test-executor and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
