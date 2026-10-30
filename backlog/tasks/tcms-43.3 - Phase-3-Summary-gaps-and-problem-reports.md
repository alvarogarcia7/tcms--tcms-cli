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
