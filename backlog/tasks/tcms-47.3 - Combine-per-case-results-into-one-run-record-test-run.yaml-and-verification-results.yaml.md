---
id: TCMS-47.3
title: >-
  Combine per-case results into one run record (test-run.yaml and
  verification-results.yaml)
status: In Review
assignee:
  - Claude
created_date: '2026-11-03T05:07:00Z'
updated_date: '2026-11-04T10:15:00Z'
labels:
  - tcms-cli
  - vv
  - converter
dependencies: []
parent_task_id: TCMS-47
ordinal: 91100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context: tcms vv reads one run folder per run (runs/<RUN_ID>/test-run.yaml with a results list, plus an optional verification-results.yaml). The 63 per-case results (type: test_result, from convert_verification_to_result_yaml.py) are separate files, so the CLI cannot read them as one run. This task adds the combinator. Comparison with MR 31 of tcms-cli (TCMS-47.2: read the per-product vv-config.yaml and add a stability-window option to tcms vv metrics): TCMS-47.2 is the consumer side. It reads runs/<RUN_ID>/test-run.yaml, the case files and the config, and computes the metrics. TCMS-47.3 is the producer side: it writes the one runs/<RUN_ID>/test-run.yaml and verification-results.yaml that TCMS-47.2 and tcms vv metrics read. The two must agree on the run record layout, so TCMS-47.3 should land before TCMS-47.2 is tested end to end. Run records must follow the tcms-schemas test-run schema, as ORCH-033.01 requires. Steps: read the per-case files; take run_id and plan_id from the shared run metadata; start/finish from the earliest and latest case timestamps (execution logs in 10_test_results/execution_logs); map overall_pass to pass or fail and a missing run to not_executed; write one test-run.yaml with one results entry per test_case_id and one verification-results.yaml keyed by test_case_id; for duplicate attempts of a case, apply the duplicate rule of TCMS-50.1 (attempts ordered by started_at then path; the newest pass, fail or error decides; not_executed and skipped never decide; error counts as FAILED); validate both outputs against their schemas before writing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One runs/<RUN_ID>/test-run.yaml with one results entry per test_case_id, validated against the test-run schema
- [x] #2 One verification-results.yaml keyed by test_case_id, validated against its schema
- [x] #3 Duplicate case ids: the duplicate rule of TCMS-50.1 applies (newest pass, fail or error decides; not_executed and skipped never decide; error counts as FAILED); a test covers it
- [x] #4 Verdict mapping: overall_pass true=pass, false=fail, missing run=not_executed
- [x] #5 Equivalent to the legacy plain-folder single-run model (MR 31 ORCH-012.02): the folder is read as one run without history
- [x] #6 A test reads the 63-file layout and checks the combined counts match the per-case inputs
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/tcms/src/vvcombine.rs winner(): newest-first walk over attempts sorted by (started_at, key). The first pass or fail decides (error would count as fail); not_executed never decides; if nothing decides the result is not_executed. Replaces the current executed-pool logic; same outcome for the existing cases.
2. Tests: ties (equal started_at, greater key wins), not_executed never overriding, all-not_executed gives not_executed, error handling in the same walk.
3. docs/RUN_RECORD_FORMAT.md and docs/VV_METRICS.md "Duplicates" paragraph updated to the TCMS-50.1 rule.
4. Verify: cargo test -p tcms, clippy, fmt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented: vvcombine::winner walks attempts newest-first (order by started_at, then key); the first pass or fail decides, not_executed never does, otherwise not_executed. Unit tests: newest decided wins, tie with greater key wins, no decided attempt gives not_executed. Integration: existing duplicate and not_executed tests in vv_combine_cli.rs still pass.

Docs: docs/RUN_RECORD_FORMAT.md duplicate rule and the plan_id note; docs/VV_METRICS.md Duplicates paragraph.

Verified: cargo test -p tcms --all-features (299 tests across tcms), clippy -D warnings, cargo fmt --check. make test NOT run in this session (DoD item 1 open), so the task is In Review, not Done.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The combine step now applies the TCMS-50.1 duplicate rule: newest-first walk, first pass or fail decides, not_executed and skipped never decide, equal started_at broken by the greater key. Verdicts are validated against the schema before writing, as before. Verified by cargo test (tcms, all features), clippy and fmt. Not verified: make test (project DoD).
<!-- SECTION:FINAL_SUMMARY:END -->
