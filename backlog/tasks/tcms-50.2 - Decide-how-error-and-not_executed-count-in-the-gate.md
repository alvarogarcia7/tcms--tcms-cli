---
id: TCMS-50.2
title: Decide how error and not_executed count in the quality gate
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:56:00Z'
labels:
  - vv-metrics
  - decision
dependencies:
  - TCMS-47.4
parent_task_id: TCMS-50
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The verdict vocabulary disagrees between tools:
- error: tcms-web's recorded-run service counts it as FAILED; Verdict's shim (vv_history and the derived fixtures README) counts it as BLOCKED. docs/VV_METRICS.md currently maps error to BLOCKED.
- not_executed: the derived fixtures say UNTESTED; tcms-web's recorded-run service counts it as SKIPPED. docs/VV_METRICS.md currently maps not_executed to UNTESTED and keeps it outside the gate denominator.

TCMS-47.4 already defines the four verdicts; this task fixes their gate bucket.

Decision (follows Kiwi TCMS, which was checked against its source: FAILED, ERROR and BLOCKED count as failing; the idle/not-run state stays in the denominator as not passed):
- error = FAILED. Inside the gate denominator, counted as a failure.
- not_executed = NOT EXECUTED. Inside the gate denominator, counted as not passed, but never as FAILED. It has its own count in the output.
- skipped is unchanged: outside the denominator, as today.

Reason:
- error means the test could not reach a verdict. It counts against the gate as a failure, as in Kiwi.
- not_executed means the case did not run. The gate should not treat a case that never ran as passing, so it stays in the denominator, as Kiwi's idle state does. It is kept separate from FAILED so that a failure count still means a test that ran and failed.

Effect on the gate: a run with not_executed cases has a lower pass rate than it did before. The error mapping does not change the pass rate, because error was already in the denominator as BLOCKED. Counts change: Blocked goes away as a verdict, error appears under Failed, and not_executed appears under its own count.

Follow-up outside this repo (noted in the MR, not changed here): tcms-web's recorded-run service counts not_executed as SKIPPED, and the Verdict shim counts it as UNTESTED. Both must move to the same bucket.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/VV_METRICS.md states the gate bucket of error (FAILED) and of not_executed (NOT EXECUTED, in the denominator, not passed, not FAILED) with the reason, and its verdict vocabulary table is updated to match
- [ ] #2 tcms vv metrics counts error in Failed and not_executed in its own count inside the gate denominator; tests cover each verdict and check the pass rate with a not_executed case
- [ ] #3 skipped stays outside the gate denominator; a test covers it
- [ ] #4 TCMS-47.4 AC #3 is amended to match this decision
- [ ] #5 The tcms-web recorded-run service and the Verdict shim are noted in the MR as follow-up, moving not_executed to the same bucket
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
