---
id: TCMS-50.5
title: Trends and heatmap over the run history
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
labels:
  - vv-metrics
  - feature
dependencies:
  - TCMS-47.2
parent_task_id: TCMS-50
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict's dashboard shows the gate pass rate for the last N runs (trend bars) and a per-case stability label over the window (heatmap). Today tcms vv metrics reports only the selected run. Verdict's Python shim (scripts/vv/shim/vv_views_file.py) computes both.

Proposed: `tcms vv trends --runs N` (one gate figure per run, same rounding as the metrics output) and the stability label per case over the window (flaky when flips reach the flaky threshold, failing when the last kept result is FAILED, otherwise stable). The window and threshold come from vv-config (TCMS-47.2).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms vv trends --runs N returns one gate figure per run, oldest first, with null for a run without data
- [ ] #2 The heatmap labels for the sut-1 and derived fixtures match the rule above (a test per label)
- [ ] #3 The window and the flaky threshold come from the vv-config file and flags (TCMS-47.2 precedence)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
