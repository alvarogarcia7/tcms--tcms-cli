---
id: TCMS-40.11
title: Document and acceptance-test the glossary entities
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.2
  - TCMS-40.3
  - TCMS-40.4
  - TCMS-40.5
  - TCMS-40.6
  - TCMS-40.7
  - TCMS-40.8
  - TCMS-40.9
  - TCMS-40.10
parent_task_id: TCMS-40
priority: medium
type: docs
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users need to know how glossary terms map to commands and files, and the acceptance suite must prove the new behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs describe the glossary-to-file mapping and each new command
- [ ] #2 The acceptance suite covers runs with metadata, the traceability matrix, regression selection and the independence check
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
