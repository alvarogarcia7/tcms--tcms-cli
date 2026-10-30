---
id: TCMS-43.4.3
title: E2E tests for Phase 4: AsciiDoc and JSON/YAML output
status: To Do
assignee: []
created_date: '2026-10-31T08:00:00Z'
labels:
  - e2e
milestone: m-0
dependencies:
  - TCMS-43.4
parent_task_id: TCMS-43.4
priority: high
type: task
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run after the phase implementation is finished and before its verification. A new agent writes and runs the end-to-end tests for this phase against the real `test-executor report` binary. Scope: AsciiDoc escaping, JSON and YAML output validated before write, identical bytes with --generated-at.
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
