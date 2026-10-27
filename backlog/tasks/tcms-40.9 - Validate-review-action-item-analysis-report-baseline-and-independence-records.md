---
id: TCMS-40.9
title: Validate every new record type from tcms-schemas
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New record types must be recognised by schema auto-resolution (see TCMS-17, TCMS-24, TCMS-35) instead of failing as unknown. Auto-resolution reads the `schema` envelope field of the document (testcase-common::envelope); it does not look at the folder name. The tcms-schemas samples carry no envelope, so they can only pass with an explicit schema argument unless TCMS-40.1 makes the envelope optional or adds a folder-based fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A document whose envelope names one of the new schemas resolves and validates; the envelope `type` enum lists every schema in tcms-schemas (test_run, trace_matrix, baseline, review, action_item, independence_assignment, analysis_report, coverage_report, product, plan_document, requirement, design_element, code_unit, environment, problem_report, change_request, qa_record, objective, audit_trail_entry)
- [ ] #2 An unknown type or schema still errors as today
- [ ] #3 The sample files from tcms-schemas all pass when validated with an explicit schema, and, if TCMS-40.1 adds a fallback, also without one
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
