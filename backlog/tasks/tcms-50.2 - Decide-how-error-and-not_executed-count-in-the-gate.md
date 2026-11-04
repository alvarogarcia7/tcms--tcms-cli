---
id: TCMS-50.2
title: Decide how error and not_executed count in the quality gate
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
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
- [x] #1 docs/VV_METRICS.md states the gate bucket of error (FAILED) and of not_executed (NOT EXECUTED, in the denominator, not passed, not FAILED) with the reason, and its verdict vocabulary table is updated to match
- [x] #2 tcms vv metrics counts error in Failed and not_executed in its own count inside the gate denominator; tests cover each verdict and check the pass rate with a not_executed case
- [x] #3 skipped stays outside the gate denominator; a test covers it
- [x] #4 TCMS-47.4 AC #3 is amended to match this decision
- [ ] #5 The tcms-web recorded-run service and the Verdict shim are noted in the MR as follow-up, moving not_executed to the same bucket
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Implemented by TCMS-47.4's bucket change: error counts as Failed, not_executed counts in the denominator as not passed with its own count, skipped stays outside. Work here: tests per verdict and a pass-rate test with a not_executed case, docs/VV_METRICS.md verdict table and gate section updated.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Gate tests in vvmetrics (not_executed in the denominator as not passed; skipped outside; not_executed-only is FAILED not NO_DATA). Integration: vv_trends_history_cli.rs not_executed_is_in_the_denominator_as_not_passed_and_never_failed; derived fixture not-executed-run; testcases/tcms-vv/TC_TCMS_VV_C5_001.yml updated to FAILED over 2 cases.

AC 5 OPEN: tcms-web recorded-run service (SKIPPED) and the Verdict shim (UNTESTED) still need to move to NOT EXECUTED in the tcms-web change set. Not in this repository.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the TCMS-50.2 buckets in the CLI: error counts as Failed; not_executed counts in the gate denominator as not passed, with its own count; skipped stays outside. Documented in docs/VV_METRICS.md and tested per verdict and with a not_executed pass-rate case. The tcms-web follow-up is still open.
<!-- SECTION:FINAL_SUMMARY:END -->
