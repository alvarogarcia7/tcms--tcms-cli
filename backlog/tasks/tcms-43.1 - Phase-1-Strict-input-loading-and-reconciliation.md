---
id: TCMS-43.1
title: 'Phase 1: Strict input loading and reconciliation'
status: In Progress
assignee: []
created_date: '2026-10-30T16:20:00Z'
updated_date: '2026-10-30T19:14:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43
priority: high
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: LOAD-01, LOAD-02, LOAD-03, LOAD-04, CLI-01. Goal: a user can point test-executor report at a trace matrix and test-run records and see which inputs were loaded, skipped or rejected, and which run won for each test case. Loaders move out of test-executor/src/main.rs into the new testcase-reporting crate, behaviour-preserving.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reconciliation block lists inputs discovered, loaded and skipped with reason
- [ ] #2 A matrix or run record that fails schema validation fails the command and names the file
- [ ] #3 When several runs contain a test case, the latest started_at wins and excluded runs are listed
- [ ] #4 Two run records with the same run_id fail the command
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
