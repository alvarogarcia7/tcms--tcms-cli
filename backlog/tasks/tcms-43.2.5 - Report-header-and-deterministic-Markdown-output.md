---
id: TCMS-43.2.5
title: Report header and deterministic Markdown output
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.2
priority: high
type: task
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: OUT-03, OUT-04. Header shows matrix id, baseline, runs, builds, environments, testers and generation time; missing values read not recorded. Sort every collection; inject --generated-at.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Header shows all identification fields or not recorded
- [ ] #2 Same inputs and --generated-at give byte-identical output
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
