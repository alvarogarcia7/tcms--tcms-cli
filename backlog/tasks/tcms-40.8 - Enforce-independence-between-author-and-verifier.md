---
id: TCMS-40.8
title: Enforce independence between author and verifier
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.2
  - TCMS-40.3
  - TCMS-40.4
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Some objectives require that the checker is not the author. The test case records its `author`; the person who checks its results is `test-run.reviewer`. The tester may be the author; the reviewer may not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test-run covering a test case with independence_required is refused when its reviewer is missing or equals the test case's author, with a clear message
- [ ] #2 independence-assignment records can be created and validated, and a record with verifier_is_independent true but verifier equal to author is rejected (JSON Schema cannot compare the two fields)
- [ ] #3 Test cases without independence_required are unaffected
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
