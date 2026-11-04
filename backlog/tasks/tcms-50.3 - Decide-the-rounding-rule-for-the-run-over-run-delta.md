---
id: TCMS-50.3
title: Decide the rounding rule for the run-over-run delta
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:50:00Z'
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
The gate pass rate is displayed floored (round down), but the delta between runs is rounded half-up. On the real binary, a move from 0% to 12.5% shows a delta of 13 next to a displayed rate of 12, and the reverse shows -12. Verdict (tcms-web ORCH-087.11) cannot make the delta match the displayed rates until one rule is chosen. docs/VV_METRICS.md (D-96) currently states the exact difference rounded half up.

Decision (recommended: option (a); confirm before implementing): the delta is the difference of the two displayed (floored) rates, an integer. Null when either run has no displayed rate.
- Example: 0% to 12.5% gives displayed 0 and 12, delta +12. 12.5% to 0% gives delta -12.
- Reason: the display is floored, so an exact-then-rounded delta prints numbers that do not subtract (13 next to 12). Option (a) makes every printed delta equal the difference of the printed rates and leaves the displayed rate unchanged.
- Option (b) (exact delta, display rounded half up) would change the displayed rate that Verdict and the badge rely on, so it is not recommended.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/VV_METRICS.md states the rule (option (a)) with the reason, and the D-96 text is replaced
- [ ] #2 A test covers 0% to 12.5% (delta +12) and 12.5% to 0% (delta -12), and checks that each delta equals the difference of the printed rates
- [ ] #3 The same delta applies in the JSON deltaFromPrevious field and in the Markdown report (TCMS-47.1); a test covers the JSON field and the report shows the same value
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
