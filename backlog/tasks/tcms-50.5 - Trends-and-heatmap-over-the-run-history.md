---
id: TCMS-50.5
title: Trends and heatmap over the run history
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
labels:
  - vv-metrics
  - feature
dependencies:
  - TCMS-47.2
  - TCMS-50.1
  - TCMS-50.4
parent_task_id: TCMS-50
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict's dashboard shows the gate pass rate for the last N runs (trend bars) and a per-case stability label over the window (heatmap). Today tcms vv metrics reports only the selected run. Verdict's Python shim (scripts/vv/shim/vv_views_file.py) computes both.

Proposed: `tcms vv trends --runs N` returns one gate figure per run, oldest first, computed the same way as `tcms vv metrics` for that run (same gate rule, same display rounding from TCMS-50.3). A run with no results gives null, as NO_DATA does in the metrics output.

Stability label per case over the window, using the rule already in docs/VV_METRICS.md (D-97):
- Only PASSED and FAILED results are kept. Per TCMS-50.1 and 50.2, error is FAILED; SKIPPED and absent are ignored.
- A flip is a change between two neighbouring kept results.
- FLAKY when flips reach the flaky threshold (default 4). FLAKY outranks FAILING.
- FAILING when the last kept result is FAILED.
- Otherwise STABLE, including a case with no kept results.

The window and threshold come from vv-config and flags (TCMS-47.2). The per-case history used for the labels comes from TCMS-50.4.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tcms vv trends --runs N returns one gate figure per run, oldest first, with null for a run with no results; the figure for a run equals the one tcms vv metrics prints for that run
- [x] #2 Labels FLAKY, FAILING and STABLE follow the D-97 rule above; a test per label on the sut-1 and derived fixtures
- [ ] #3 The window and the flaky threshold come from vv-config and flags with the TCMS-47.2 precedence; a test covers the override
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New subcommand `tcms vv trends --root <DATA_ROOT> [--runs N]`: one gate figure per run for the last N runs, oldest first, {runId, passRate display|null, verdict}; same rules as metrics.
2. Heatmap section in trends: per case of the selected run, the D-97 label (FLAKY, FAILING, STABLE) over the window, reusing vvmetrics::stability on the per-case history (which comes from the 50.4 history logic).
3. Window and flaky threshold through vv-config and flags (TCMS-47.2 precedence).
4. Tests: a label per case on the derived fixtures; the trend figure equals metrics for the same run; docs/VV_METRICS.md updated.
Depends on 47.2, which has AC#7 blocked (no vv-config schema); code is not affected by that AC.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
tcms vv trends --root R [--runs N] [--plan P] [flags]: trend rows for the last N runs, oldest first (passRate display or null, verdict); heatmap of the newest run's cases using vvmetrics::stability over the window (D-97).

Tests: heatmap_labels_failing_on_the_derived_failing_run (FAILING), heatmap_flaky_and_failing_labels_follow_the_flaky_threshold (FAILING at default 4, FLAKY at 3 on a temp copy of sut-1), trends_show_the_last_runs_and_agree_with_metrics (STABLE on sut-1), trends_show_a_null_figure_for_a_run_without_data (NO_DATA null).

AC 3 partly verified: the --flaky-threshold flag override is tested through trends. The vv-config file precedence for trends uses the same vvconfig::resolve() that the metrics tests cover, but has no trends-specific test yet.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added tcms vv trends: one gate figure per run for the last N runs (null when a run has no data) and a per-case heatmap label (FLAKY, FAILING, STABLE) over the stability window. The labels and the trend figures are tested on sut-1, the derived failing run, and a temporary copy of sut-1 with three flips. The flag override is tested; vv-config file precedence for trends is covered only through the shared resolver.
<!-- SECTION:FINAL_SUMMARY:END -->
