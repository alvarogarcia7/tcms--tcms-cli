---
id: TCMS-43.3.4
title: E2E tests for Phase 3: summary, gaps and problem reports
status: To Do
assignee: []
created_date: '2026-10-31T08:00:00Z'
labels:
  - e2e
milestone: m-0
dependencies:
  - TCMS-43.3
parent_task_id: TCMS-43.3
priority: high
type: task
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run after the phase implementation is finished and before its verification. A new agent writes and runs the end-to-end tests for this phase against the real `test-executor report` binary. Scope: summary, gaps and problems kinds, flags for missing problem_report_id, consistency with the traceability report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 E2E tests run the real binary against fixture inputs and assert exact output lines and exit codes
- [ ] #2 Tests are bash 3.2 and BSD/GNU safe and are mapped in mk/incremental.mk
- [ ] #3 Results recorded in the phase SUMMARY before verification starts
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
