---
id: TCMS-40.5
title: Generate a traceability matrix from test cases
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.2
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements-based coverage needs a requirement to test case to result view. Requirement IDs come from StrictDoc, not from the CLI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A command outputs a trace-matrix YAML that validates against its schema
- [ ] #2 Requirements with no test cases are listed
- [ ] #3 normal_range_covered and robustness_covered are computed from test_type; the legacy `requirement` string is ignored
- [ ] #4 Results are linked through run_ids (test-run.run_id); verification results have no ID of their own
- [ ] #5 Link level is HLR or LLR (the requirement's level), never integration
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
