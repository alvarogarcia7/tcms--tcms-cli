---
id: TCMS-40.4
title: Produce a test run record per execution
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.3
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Runs must group logs and results and give a per-test-case verdict (pass, fail, not_executed, error) for audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each execution writes a test-run YAML that validates against the test-run schema
- [ ] #2 A failed case and a not-executed case appear with the right verdict
- [ ] #3 The reviewer field can be set
- [ ] #4 A problem_report_id can be attached to a fail or error result
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
