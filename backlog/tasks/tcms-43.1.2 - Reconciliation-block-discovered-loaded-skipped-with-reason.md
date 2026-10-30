---
id: TCMS-43.1.2
title: 'Reconciliation block: discovered, loaded, skipped with reason'
status: In Progress
assignee: []
created_date: '2026-10-30T16:20:00Z'
updated_date: '2026-10-30T19:14:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.1
priority: high
type: task
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: LOAD-02. Every input the report looks at is accounted for: discovered, loaded, or skipped with a reason. Nothing is dropped silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reconciliation block printed in every report header
- [ ] #2 Skipped inputs show a reason
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
