---
id: TCMS-48.3
title: 'Phase 1: consolidate test binaries to one per crate'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each file in tests/ is a separate link step. Phase 0 counts 39 test executables in testcase-manager (about 1.66 GB). Merge each crate's integration tests into one tests/main.rs with modules. Start with testcase-manager and check the test count is unchanged.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
