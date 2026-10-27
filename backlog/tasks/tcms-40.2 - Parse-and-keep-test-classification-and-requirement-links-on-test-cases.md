---
id: TCMS-40.2
title: Parse and keep test classification and requirement links on test cases
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Normal-range vs robustness vs security, test level (hw_sw_integration, sw_integration, low_level, validation), objective, author and requirement IDs are needed for requirements-based coverage and independence checks. Today a test case carries only one free-text `requirement` string; that field stays required for compatibility but is not used for coverage. Traceability uses `requirements[]` only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Optional fields requirements, test_type, test_category, level, objective, environment, pass_fail_criteria, author and independence_required round-trip through load and save without loss
- [ ] #2 An invalid test_type, test_category or level value, or a test_category that does not belong to the test_type, gives a clear validation error naming the field
- [ ] #3 Test case files without the new fields behave exactly as before
- [ ] #4 Unit tests cover valid, invalid and absent fields
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
