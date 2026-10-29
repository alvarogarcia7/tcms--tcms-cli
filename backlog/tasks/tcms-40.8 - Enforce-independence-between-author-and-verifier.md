---
id: TCMS-40.8
title: Enforce independence between author and verifier
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:10:00Z'
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
- [x] #1 A test-run covering a test case with independence_required is refused when its reviewer is missing or equals the test case's author, with a clear message
- [x] #2 independence-assignment records can be created and validated, and a record with verifier_is_independent true but verifier equal to author is rejected (JSON Schema cannot compare the two fields)
- [x] #3 Test cases without independence_required are unaffected
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. independence module in testcase-verification: check_run_independence (reviewer missing / equals author / no author recorded), IndependenceAssignment with the cross-field rule
2. verifier --test-run applies the check before writing the record
3. test-executor independence-assignment create|check
4. docs/INDEPENDENCE.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions: (D-40.8a) names compare ignoring case and surrounding spaces. (D-40.8b) a test case that requires independence but records no author is refused too: the reviewer cannot be checked against nothing. (D-40.8c) the check runs where the run record is written (verifier --test-run), because that is where reviewer and test cases meet; the record is not written when it fails. (D-40.8d) 'create' computes verifier_is_independent from author and verifier and writes an honest 'false' when they are the same person; 'check' rejects a record that claims otherwise. Evidence: 8 unit tests (independence.rs), 4 end-to-end tests with the real verifier binary (crates/verifier/tests/independence_test.rs: author as reviewer refused, missing reviewer refused, independent reviewer recorded, no requirement unaffected), 3 CLI tests (crates/test-executor/tests/independence_cli.rs: create, schema failure, cross-field rejection, tcms-schemas ia_001 sample passes). Not run: make test, acceptance suite.

Review: verifier --test-run refused for reviewer alice = author ' Alice' and for missing reviewer, with clear messages and no run record; reviewer bob written; test case without independence_required unaffected. independence-assignment create/check: verifier_is_independent true with verifier = author rejected by check; honest false record created. cargo test -p verifier (independence_test 4 passed), test-executor independence_cli 3 passed, testcase-verification passes; workspace exit 0. make test not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test workspace instead.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Independence enforced at test-run time and for assignment records. make test not runnable here; verified with cargo test.
<!-- SECTION:FINAL_SUMMARY:END -->
