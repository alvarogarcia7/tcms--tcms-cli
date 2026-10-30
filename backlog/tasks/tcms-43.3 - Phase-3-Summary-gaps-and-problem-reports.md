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
