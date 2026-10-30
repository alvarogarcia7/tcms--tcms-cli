---
id: TCMS-43.1.3
title: Latest-started_at run selection with excluded runs listed
status: In Progress
assignee: []
created_date: '2026-10-30T16:20:00Z'
updated_date: '2026-10-30T19:14:00Z'
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
Requirement: LOAD-03. Run selection per test case: the latest run (by started_at, RFC 3339) with an executed result decides. A later pass overrides an earlier fail or error; a later fail or error overrides an earlier pass. Runs that omit the test case or record not_executed do not count. The report lists runs used and excluded.
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
