---
id: TCMS-45
title: 'Acceptance suite finds no test cases: searches *.yaml, cases are *.yml'
status: To Do
assignee: []
created_date: '2026-10-31T09:00:00Z'
labels:
  - bug
  - acceptance
milestone: m-0
dependencies: []
priority: high
type: bug
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`make acceptance-test` reports `Found 0 test case YAML files` (test-acceptance/run_acceptance_suite.sh:285-288). The script uses `find "$TEST_CASES_DIR" -type f -name "*.yaml"`, but all 89 test case files under test-acceptance/00_test_cases are `.yml`. Stages 2 to 7 then have no inputs. Stages 8 (glossary) and 9 (Jira) also fail. The suite exits 0 and prints SUCCESS with Total Test Cases: 0, so the failure is hidden.

Found while running the Phase 3 gates on branch traceability-matrix-report. Present on main as well (89 .yml, 0 .yaml tracked). Not caused by Phase 3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stage 1 finds all 89 test case files (decide: rename to .yaml or match both extensions, and keep schema validation working)
- [ ] #2 make acceptance-test exits non-zero when any stage has failures
- [ ] #3 Stages 8 and 9 pass or are documented as known failures with a backlog reference
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
