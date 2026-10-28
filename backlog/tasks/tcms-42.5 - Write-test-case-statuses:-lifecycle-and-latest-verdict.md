---
id: TCMS-42.5
title: 'Write test case statuses: lifecycle and latest verdict'
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.4
  - TCMS-40.4
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Jira shows two values per test case: the lifecycle status (draft, in_review, approved, obsolete) and the latest verdict on the plan's current baseline, mapped to not_started, in_progress, passed, failed, blocked as defined in the mapping doc, plus the baseline id the verdict refers to.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After a run is committed, every executed test case's Jira entity shows the mapped verdict and baseline
- [ ] #2 A status change by review updates the lifecycle field at the next sync
- [ ] #3 A value edited by hand in Jira is overwritten and the overwrite is logged
- [ ] #4 Test cases without external_refs are created on first sync when dedicated_issues is allowed, otherwise reported as unmapped
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
