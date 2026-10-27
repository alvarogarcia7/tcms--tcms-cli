---
id: TCMS-40.7
title: Select regression test cases from a baseline change impact
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.2
  - TCMS-40.5
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Regression scope must follow change impact analysis rather than re-running everything.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Given a baseline YAML with change_impact, only the listed test cases and those tracing to impacted requirements run
- [ ] #2 A plan with a regression block is honoured: the scope is the change_impact of the baseline named by the plan's baseline_id, measured from regression.previous_baseline_id
- [ ] #3 A missing or invalid baseline gives a clear error
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
