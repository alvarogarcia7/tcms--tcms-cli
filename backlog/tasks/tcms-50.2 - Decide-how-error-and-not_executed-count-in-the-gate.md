---
id: TCMS-50.2
title: Decide how error and not_executed count in the quality gate
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:50:00Z'
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
- not_executed: the derived fixtures say UNTESTED; tcms-web's recorded-run service counts it as SKIPPED. docs/VV_METRICS.md currently maps not_executed to UNTESTED.

TCMS-47.4 already defines the four verdicts; this task fixes their gate bucket.

Decision:
- error = FAILED
- not_executed = SKIPPED

Reason:
- error means the test could not reach a verdict. The run did not show the requirement is met, so it counts against the gate as a failure, not as a separate blocked state. tcms-web's recorded-run service already does this.
- not_executed means no run took place. That is a skip for the gate, not a failure. tcms-web's recorded-run service already does this.

Effect on the gate: error moves from Blocked to Failed. Both are in the denominator, so the pass rate does not change; only the Failed count changes. not_executed moves from Untested to Skipped; both are outside the denominator, so the pass rate does not change either.

Conflict to resolve in the change set: TCMS-47.4 AC #3 says both not_executed and error are outside the gate denominator. This decision keeps error inside it (as FAILED). TCMS-47.4 AC #3 must be amended to match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/VV_METRICS.md states the gate bucket of error (FAILED) and of not_executed (SKIPPED) with the reason, and its verdict vocabulary table is updated to match
- [ ] #2 tcms vv metrics counts error in Failed and not_executed in Skipped; a test per verdict checks the counts and that the gate pass rate is unchanged by the mapping
- [ ] #3 TCMS-47.4 AC #3 is amended so that error is inside the gate denominator as FAILED
- [ ] #4 The tcms-web recorded-run service and the Verdict shim are noted in the MR as follow-up, using the same buckets
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
