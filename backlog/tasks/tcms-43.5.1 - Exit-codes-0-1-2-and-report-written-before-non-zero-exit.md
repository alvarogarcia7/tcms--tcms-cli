---
id: TCMS-43.5.1
title: Exit codes 0, 1, 2 and report written before non-zero exit
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.5
priority: medium
type: task
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: CLI-02. 0 success; 1 input or validation error; 2 when --fail-on-gaps and gaps or failed results exist. The report file is written before any non-zero exit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Exit codes match the three states
- [ ] #2 Report written before non-zero exit
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
