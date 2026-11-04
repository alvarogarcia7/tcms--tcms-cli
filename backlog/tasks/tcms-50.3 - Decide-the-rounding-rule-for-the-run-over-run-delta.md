---
id: TCMS-50.3
title: Decide the rounding rule for the run-over-run delta
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
labels:
  - vv-metrics
  - decision
dependencies:
  - TCMS-47
parent_task_id: TCMS-50
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The gate pass rate is displayed floored (round down), but the delta between runs is rounded half-up. On the real binary, a move from 0% to 12.5% shows a delta of 13 next to a displayed rate of 12, and the reverse shows -12. Verdict (tcms-web ORCH-087.11) cannot make the delta match the displayed rates until one rule is chosen.

Choose one rule for the delta. Options: (a) the delta is the difference of the two displayed (floored) rates, so it always matches the display; (b) the delta is exact and the display rounds half-up. Record the choice and apply it everywhere the CLI prints a rate or a delta.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision entry states the rule, with the reason, in docs/VV_METRICS.md
- [ ] #2 A test covers 0% to 12.5% and 12.5% to 0%: the printed delta equals the difference of the printed rates under option (a), or the rule as recorded under option (b)
- [ ] #3 The same rule applies to the Markdown report (TCMS-47.1) and the JSON output
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
