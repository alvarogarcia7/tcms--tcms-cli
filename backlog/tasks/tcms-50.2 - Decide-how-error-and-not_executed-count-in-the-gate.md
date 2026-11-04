---
id: TCMS-50.2
title: Decide how error and not_executed count in the quality gate
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T08:36:00Z'
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
- error: tcms-web's recorded-run service counts it as FAILED; Verdict's shim (vv_history and the derived fixtures README) counts it as BLOCKED.
- not_executed: the derived fixtures say UNTESTED; tcms-web's recorded-run service counts it as SKIPPED.

Decide the bucket for each and make tcms vv metrics follow it. TCMS-47.4 already defines the four verdicts; this task fixes their gate bucket.

--- 

Decision:
- Error = Failed
- not_executed = SKIPPED
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision entry states the gate bucket of error and of not_executed, with the reason, in docs/VV_METRICS.md
- [ ] #2 tcms vv metrics counts them in that bucket, and the gate pass rate follows from it (a test on each verdict)
- [ ] #3 The tcms-web recorded-run service and the Verdict shim use the same bucket after the decision is recorded (tcms-web follow-up noted in the MR)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
