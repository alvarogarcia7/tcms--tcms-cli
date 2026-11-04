---
id: TCMS-50.3
title: Decide the rounding rule for the run-over-run delta
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
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
- [x] #1 docs/VV_METRICS.md states the rule (option (a)) with the reason, and the D-96 text is replaced
- [x] #2 A test covers 0% to 12.5% (delta +12) and 12.5% to 0% (delta -12), and checks that each delta equals the difference of the printed rates
- [x] #3 The same delta applies in the JSON deltaFromPrevious field and in the Markdown report (TCMS-47.1); a test covers the JSON field and the report shows the same value
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. vvmetrics::delta: delta = floor(current rate) - floor(previous rate), null when either side has no data (option a, confirmed).
2. Tests: 0 to 12.5 gives +12, 12.5 to 0 gives -12, and each equals the difference of the printed rates.
3. docs/VV_METRICS.md D-96 text replaced. Markdown report (TCMS-47.1) is not in this repository; the JSON field is the verified output.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
vvmetrics::delta is now floor(current) - floor(previous). Unit tests: 0 to 12.5 gives +12, 12.5 to 0 gives -12, equal printed rates give 0, first run and no-data give None.

AC 3: the JSON deltaFromPrevious is the value tcms-vv-report prints in its 'Delta from previous run' row; tests/render.rs checks that row against the metrics JSON for every fixture (crates/tcms-vv-report, TCMS-47.1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The run-over-run delta is the difference of the two printed (floored) gate rates, so every printed delta equals the difference of the printed rates. Documented in docs/VV_METRICS.md (D-96 replaced). Tests cover 0 to 12.5 percent in both directions. The Markdown and AsciiDoc reports show the same field, and the report crate's tests check it.
<!-- SECTION:FINAL_SUMMARY:END -->
