---
id: TCMS-43.2.1
title: ReportModel and documented verdict roll-up rule
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.2
priority: high
type: task
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: ROLL-02. Requirement verification status is derived from linked verdicts: VERIFIED, FAILED, ERRORED, NOT RUN, PARTIAL, NO TESTS. VERIFIED only when every linked case has verdict pass. Document the rule and cover it with table-driven tests. All report views read this one model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Roll-up rule documented in docs/
- [ ] #2 Table-driven tests cover every status
- [ ] #3 No status shows VERIFIED with any non-pass linked case
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
