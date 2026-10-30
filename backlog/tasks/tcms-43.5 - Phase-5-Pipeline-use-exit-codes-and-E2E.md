---
id: TCMS-43.5
title: 'Phase 5: Pipeline use: exit codes and E2E'
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies:
  - TCMS-43.4
parent_task_id: TCMS-43
priority: medium
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: CLI-02. Goal: test-executor report can be used in a script or CI job; exit status is reliable and the report is always written before a non-zero exit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Exit 0 on success, 1 on input or validation error, 2 with --fail-on-gaps when gaps or failed results exist
- [ ] #2 Report file is written before any non-zero exit caused by gaps or failures
- [ ] #3 Bash 3.2 and BSD/GNU safe E2E script covers exit codes; mapped in mk/incremental.mk and the acceptance suite
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
