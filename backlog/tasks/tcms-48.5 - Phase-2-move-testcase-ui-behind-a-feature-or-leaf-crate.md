---
id: TCMS-48.5
title: 'Phase 2: move testcase-ui behind a feature or leaf crate'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
testcase-ui (8.7k lines, no tests) is rebuilt whenever a lower crate changes. Move it behind a cargo feature or into a leaf crate so that changes to testcase-execution do not rebuild it.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
