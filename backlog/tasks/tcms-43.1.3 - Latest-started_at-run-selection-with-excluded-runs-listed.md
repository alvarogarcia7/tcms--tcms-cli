---
id: TCMS-43.1.3
title: Latest-started_at run selection with excluded runs listed
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies: []
parent_task_id: TCMS-43.1
priority: high
type: task
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirement: LOAD-03. When several runs contain a result for the same test case, the latest started_at (RFC 3339) wins, and the report lists the runs it excluded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Latest run wins per test case
- [ ] #2 Excluded runs are listed
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
