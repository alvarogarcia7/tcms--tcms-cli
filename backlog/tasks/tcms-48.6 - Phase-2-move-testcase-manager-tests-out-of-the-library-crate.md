---
id: TCMS-48.6
title: 'Phase 2: move testcase-manager tests out of the library crate'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
testcase-manager has 13.5k lines of source and 46k lines of tests in one crate. Move the tests to their own crate or workspace member so the library does not rebuild when tests change, and so tests can run per domain.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
