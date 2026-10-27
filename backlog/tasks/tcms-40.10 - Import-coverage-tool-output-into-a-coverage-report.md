---
id: TCMS-40.10
title: Import coverage tool output into a coverage-report
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: low
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Structural coverage (statement, decision, MC/DC) comes from tools. Gaps must be classified as missing test, dead code, deactivated code or tool limitation (the coverage-report enum) with a rationale, not just patched. A coverage-report needs requirements_based figures, structural figures or both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A documented input format converts to a coverage-report YAML that validates against its schema
- [ ] #2 Gaps can be annotated with classification and rationale
- [ ] #3 Unclassified gaps are reported
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
