---
id: TCMS-43.3
title: 'Phase 3: Summary, gaps and problem reports'
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies:
  - TCMS-43.2
parent_task_id: TCMS-43
priority: high
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: SUM-01, GAP-01, PRB-01, PRB-02. Goal: run summary, coverage-gaps report and problem-report list, all views over one ReportModel so numbers cannot disagree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Run summary shows pass, fail, not_executed, error and total per run and per requirement; buckets sum to total
- [ ] #2 Gaps report lists requirements with no tests, missing normal-range, missing robustness, and test cases with no result
- [ ] #3 Problem list shows each fail and error with its problem_report_id
- [ ] #4 Fail or error result without problem_report_id is flagged
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Notes

- Plan 03-01 done (shared foundation): header_markdown and cell made public with byte-identical output (deab74b); ViewInput, ResultCounts, linked_cases and shared fixture (5a15353); summary 3-01 SUMMARY at efd1502.
- Gates: fmt and clippy clean; cargo test for testcase-reporting and test-executor: 149 passed.
- Review: PASS (fresh Haiku reviewer, no findings).
- Still open: plans 03-02 to 03-05, phase gates, E2E subtask tcms-43.3.4, pipeline run.
- Plan 03-02 done (run summary view): commits a4defbb, fa7d96d; summary 22f9476. Review PASS.
- Non-blocking follow-up from review: the requirement and linked-case bucket-sum assertion is tautological (view.rs sets total = outcomes.len()). Fix in a later pass by comparing against an independently computed total.
- Plan 03-03 done (coverage gaps): commits 8f8d60c, 0dd4ec1; summary ea43f9e. Review PASS.
- Non-blocking from 03-03 review: golden fixture never renders a real document name; whitespace-only document name is untested. Follow up in a later pass.
- Plan 03-04 done (problem report list): commits 6be87b4, bc6ba27; summary ea00cb3. Review PASS.
- Non-blocking from 03-04 review: plan's clippy --all-targets and cargo fmt --check gates were not run for testcase-reporting in the executor pass; 03-05 runs them.
- Plan 03-05 done (CLI wiring for summary, gaps, problems; 14 new CLI tests; docs): commits e18f652, 2b43ee9, 5b668b3; review fixes fa97442 and 29048f8 (determinism rule per D-03-G3). Review of 29048f8 PASS.
- Gates: make build PASS; make lint PASS; make test FAIL (pre-existing: test_json_escape_e2e.sh expects a --config flag that test-executor does not have; tracked in tcms-44; make test exits 0 despite the failure); coverage-e2e and acceptance-test results: see run log.
- Blocker recorded: D-03-G1 (CARGO_TARGET_DIR moved into repo target/), D-03-G2 (pre-existing --config failure), D-03-G3 (determinism rule).
