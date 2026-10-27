---
id: TCMS-40.3
title: Stamp run, build, environment and tester on execution logs and results
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A test result must state what ran, on which build, in which environment, by whom and when; execution logs and verification results currently cannot be grouped into a run or attributed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Execution log entries carry run_id and test_case_id
- [ ] #2 Verification results carry run_id, build_id, environment, tester and date
- [ ] #3 Values can be supplied by flag, config or environment variable; absent values are omitted rather than written empty
- [ ] #4 Output validates against the extended schemas
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
