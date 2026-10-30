---
id: TCMS-43.2
title: 'Phase 2: Traceability report (Markdown)'
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies:
  - TCMS-43.1
parent_task_id: TCMS-43
priority: high
type: feature
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: ROLL-01, ROLL-02, ROLL-03, ROLL-04, TRC-01, TRC-02, OUT-03, OUT-04. Goal: a Markdown traceability report showing for each requirement its test cases, their verdicts and an honest verification status (REQ -> test cases -> PASS/FAIL).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each requirement lists linked test cases, each verdict, and the run it came from; HLR and LLR distinct
- [ ] #2 VERIFIED only when every linked test case has verdict pass; otherwise FAILED, ERRORED, NOT RUN, PARTIAL or NO TESTS per the documented rule
- [ ] #3 Test coverage shown separately from verification status; a linked case with no result shows NOT RUN
- [ ] #4 Obsolete test cases excluded from roll-up and counted in header
- [ ] #5 Same inputs and --generated-at give byte-identical output; header shows identification fields or not recorded
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
