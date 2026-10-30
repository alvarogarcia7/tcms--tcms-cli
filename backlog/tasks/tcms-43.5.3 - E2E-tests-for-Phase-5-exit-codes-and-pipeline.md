---
id: TCMS-43.5.3
title: E2E tests for Phase 5: exit codes and pipeline use
status: To Do
assignee: []
created_date: '2026-10-31T08:00:00Z'
labels:
  - e2e
milestone: m-0
dependencies:
  - TCMS-43.5
parent_task_id: TCMS-43.5
priority: high
type: task
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run after the phase implementation is finished and before its verification. A new agent writes and runs the end-to-end tests for this phase against the real `test-executor report` binary. Scope: exit 0/1/2 behaviour, report written before a non-zero exit, --fail-on-gaps, usage errors exit 1.
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
