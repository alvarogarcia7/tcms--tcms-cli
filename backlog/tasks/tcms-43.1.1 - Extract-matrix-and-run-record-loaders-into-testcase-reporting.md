---
id: TCMS-43.1.1
title: Extract matrix and run-record loaders into testcase-reporting
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.1
priority: high
type: task
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: LOAD-01. Move read_test_runs and matrix loading out of test-executor/src/main.rs into the new testcase-reporting crate. Use testcase_common::load_and_validate_yaml, never raw serde_yaml. Behaviour-preserving for existing trace-matrix command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Matrix and run records load through load_and_validate_yaml
- [ ] #2 Schema failure names the file
- [ ] #3 trace_matrix_cli tests still pass
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
