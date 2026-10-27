---
id: TCMS-40.1
title: Consume the extended tcms-schemas
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-25
parent_task_id: TCMS-40
priority: high
type: chore
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CLI keeps its own copies of the test-case, execution-log and verification-result schemas (legacy schemas/*.schema.json plus the envelope versions schemas/tcms/test-case, test-execution and test-verification .schema.v1.json); it has no test-plan schema. These copies now drift from the extended versions in tcms-schemas (new optional fields plus new schemas test-run, trace-matrix, baseline, review, action-item, independence-assignment, analysis-report, coverage-report). Continues the schema unification of TCMS-25.

Two things must be decided here, not assumed: (1) how the CLI obtains tcms-schemas (git submodule, vendored copy at a pinned tag, or build-time download) and (2) how the envelope meta-schema (tcms-envelope.schema.json) fits: its `type` enum is closed and its `schema` URI pattern requires `tcms/<name>.schema.v<n>.json`, while tcms-schemas files are unversioned (`tcms/<name>.schema.json`) and declare no envelope fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The CLI uses the schema set that contains the new fields and new schemas
- [ ] #2 No duplicate local copy remains of test-case, test-plan, execution-log or verification-result
- [ ] #3 Existing sample test case files still validate
- [ ] #4 The mechanism by which the CLI obtains tcms-schemas (and the version pinned) is documented
- [ ] #5 The envelope `type` enum and `schema` URI pattern accept the new types and the tcms-schemas file names, or the envelope is made optional for them
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
