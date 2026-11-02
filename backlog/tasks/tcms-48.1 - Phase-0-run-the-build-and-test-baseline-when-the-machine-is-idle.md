---
id: TCMS-48.1
title: 'Phase 0: run the build and test baseline when the machine is idle'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run scripts/dx/measure-baseline.sh on a warm and, if disk allows, a cold target/. Record the report location and the top compile units, test binaries and slow test executables in this task. Needs about 17 GB free for a cold build.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
