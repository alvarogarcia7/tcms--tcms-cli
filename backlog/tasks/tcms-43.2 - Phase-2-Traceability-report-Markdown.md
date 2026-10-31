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
- [x] #7 Report shape A: each requirement row shows REQUIREMENT | list of test cases (ID, status) | final verdict for the requirement.
- [x] #8 Report shape B: REQUIREMENT | final verdict, and per-test-case rows TESTCASE ID | status for that test case.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Notes

- Plan 02-05 (gap closure) done: a blank baseline_id or matrix id now renders as "not recorded" (commits fd3a1f8, 300744f, 330e3ed, 0f75467; summary 9ac6776).
- Gates run for 02-05: cargo fmt --check, cargo clippy -D warnings, cargo test for testcase-reporting (87 passed) and report_cli (35 passed).
- Review: first review FAILED on an inlined blank check in model.rs; fixed in 0f75467; re-review PASSED.
- Still open for Phase 2: phase-level gates (make build, lint, test, coverage-e2e, acceptance-test), E2E subtask tcms-43.2.9, pipeline run.
- Acceptance criterion #5 (header not recorded) is covered by 02-05; criteria #1-#4 were verified in 02-VERIFICATION.md.
- Plan 02-06 done (report shapes A and B and --shape CLI): commits 03e7a26, c88f81e; summary e100f69. Review PASS. Acceptance criteria #7 and #8 ticked.
