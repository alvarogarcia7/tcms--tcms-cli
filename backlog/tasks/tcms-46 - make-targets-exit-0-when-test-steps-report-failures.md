---
id: TCMS-46
title: 'make test and coverage-e2e exit 0 when test steps report failures'
status: To Do
assignee: []
created_date: '2026-10-31T10:00:00Z'
labels:
  - bug
  - ci
milestone: m-0
dependencies: []
priority: high
type: bug
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`make coverage-e2e` and `make test` exit 0 even when a test step prints failures. Observed: test_json_escape_e2e.sh reported 7 failed cases and the make target still exited 0 (before tcms-44 was fixed). The "Verification output not generated" warning in the coverage log also did not fail the run. A green make run therefore does not prove the suite passed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A failing E2E script or failing verifier step makes make test and make coverage-e2e exit non-zero
- [ ] #2 The "Verification output not generated" warning is either fixed or fails the run
- [ ] #3 Existing passing runs stay green
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
