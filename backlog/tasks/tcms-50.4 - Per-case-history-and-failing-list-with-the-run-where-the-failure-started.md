---
id: TCMS-50.4
title: Per-case history and failing list with the run where the failure started
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:51:00Z'
labels:
  - vv-metrics
  - feature
dependencies:
  - TCMS-47.3
  - TCMS-50.1
  - TCMS-50.2
parent_task_id: TCMS-50
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict needs two things from the CLI that it currently computes in Python (scripts/vv/shim/vv_history.py and vv_report.py):
1. The status of one test case in every run of a run root, oldest to newest. See docs/VV_CASE_HISTORY_GAP.md in tcms-web.
2. The failing list for a run, with the run where each failure started. Rule (tcms-web D-98): the oldest run of the consecutive FAILED streak that ends at the selected run.

Rules this task relies on:
- Result per run and run order: TCMS-50.1 (one decided result per case, runs ordered by started_at, then path).
- Verdict to status: TCMS-50.2 (error counts as FAILED, not_executed as SKIPPED). The status set is PASSED, FAILED, SKIPPED, or absent. BLOCKED and UNTESTED are no longer produced for the history.
- Streak: PASSED ends the streak. FAILED extends it. SKIPPED and absent runs are passed over: they neither end nor extend the streak.

Proposed output:
- `tcms vv history --case <ID>`: one row per run, oldest first, with the status or absent (null in JSON).
- `failingCases` in `tcms vv metrics`: every case whose result in the selected run is FAILED, sorted by case id, each with `failureStartRunId`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms vv history --case <ID> prints one row per run, oldest first, with the status or absent
- [ ] #2 tcms vv metrics lists every case whose result in the selected run is FAILED, sorted by case id, each with its failure-start run; a case that flips PASSED, FAILED, PASSED, FAILED starts at the last FAILED run
- [ ] #3 Tests cover a case with SKIPPED-only results, a case with error-only results (counted as FAILED), and a case missing from some runs
- [ ] #4 The output shape and sort order are documented in docs/VV_METRICS.md
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
