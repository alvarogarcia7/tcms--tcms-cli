---
id: TCMS-42.4
title: Test backend adapters: plain Jira, Xray, Zephyr Scale, Zephyr Squad
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Where a test case and a run are represented in Jira depends on the test backend. Plain Jira: a TCMS-owned Test issue per test case (tcms_issue_types.test), found by the tcms_id field, linked to features with link_type. Xray: Test, Test Plan, Test Execution. Zephyr Scale: test case, cycle, execution through its own API. Zephyr Squad: Test issue and cycle. One trait, four implementations; the sync code never branches on the backend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each backend creates or finds the test entity for a test case and stores its key in external_refs
- [ ] #2 Each backend records a run as its execution entity (plain Jira: fields on the Test issue) with per-case verdict
- [ ] #3 Test lifecycle status and latest verdict are written to the configured fields on every backend
- [ ] #4 Switching test_backend in the config is the only change needed; the acceptance suite runs the same scenario on all four with mocks
- [ ] #5 A test case deleted in TCMS marks its Jira entity obsolete, never deletes it
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
