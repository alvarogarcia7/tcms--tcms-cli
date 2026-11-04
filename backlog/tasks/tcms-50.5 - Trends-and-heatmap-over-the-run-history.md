---
id: TCMS-50.5
title: Trends and heatmap over the run history
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:51:00Z'
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
- [ ] #1 tcms vv trends --runs N returns one gate figure per run, oldest first, with null for a run with no results; the figure for a run equals the one tcms vv metrics prints for that run
- [ ] #2 Labels FLAKY, FAILING and STABLE follow the D-97 rule above; a test per label on the sut-1 and derived fixtures
- [ ] #3 The window and the flaky threshold come from vv-config and flags with the TCMS-47.2 precedence; a test covers the override
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
