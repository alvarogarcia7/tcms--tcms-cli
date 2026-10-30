---
id: TCMS-43.3.2
title: Coverage gaps report; confirm zero-test requirements
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.3
priority: high
type: task
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: GAP-01. Lists requirements with no test cases, missing normal-range, missing robustness, and test cases with no result. Research flag: check in trace_matrix.rs whether requirements with zero test cases are emitted; if not, a requirements input is needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each of the four gap kinds listed
- [ ] #2 Zero-test requirement source confirmed and documented
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
