---
id: TCMS-43.4.2
title: Local report schema; JSON and YAML output validated before write
status: To Do
assignee: []
created_date: '2026-10-30T16:21:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.4
priority: medium
type: task
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: OUT-02. Local report schema using the type and schema envelope, in schemas/. Shared schemas are not changed. Output validated before write; a failure writes no file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Local schema added; shared schemas unchanged
- [ ] #2 Validation failure writes no file
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
