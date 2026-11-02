---
id: TCMS-48.7
title: 'Phase 3: validate every input and output against its schema'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reads use testcase_common::load_and_validate_yaml. Writers serialise, validate against the schema, then write atomically (as tcms vv metrics already does). Add a contract test per binary that validates its output against the schema.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
