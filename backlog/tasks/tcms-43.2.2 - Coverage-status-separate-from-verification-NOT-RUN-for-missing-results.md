---
id: TCMS-43.2.2
title: Coverage status separate from verification; NOT RUN for missing results
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.2
priority: high
type: task
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: ROLL-01, ROLL-04. Normal-range and robustness presence is shown apart from verification status. A linked case with no result in the selected runs is NOT RUN, never passed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Coverage columns separate from status
- [ ] #2 Linked case without a result shows NOT RUN
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
