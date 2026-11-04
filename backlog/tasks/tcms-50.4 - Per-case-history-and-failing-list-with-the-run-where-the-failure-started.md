---
id: TCMS-50.4
title: Per-case history and failing list with the run where the failure started
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
labels:
  - vv-metrics
  - feature
dependencies:
  - TCMS-47.3
parent_task_id: TCMS-50
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict needs, from the CLI, two things it currently computes in Python (scripts/vv/shim/vv_history.py and vv_report.py):
1. The status of one test case in every run of a run root, oldest to newest (PASSED, FAILED, BLOCKED, SKIPPED, UNTESTED, or absent from the run). See docs/VV_CASE_HISTORY_GAP.md in tcms-web.
2. The failing list for a run, with the run where each failure started. Rule (tcms-web D-98): the oldest run of the consecutive FAILED streak that ends at the selected run. PASSED ends the streak; BLOCKED, SKIPPED, UNTESTED and runs without the case are skipped.

Proposed output: a new `tcms vv history --case <ID>` and a `failingCases` list in `tcms vv metrics`, each entry with the run id where the failure started.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms vv history --case <ID> prints one row per run, oldest first, with the status or absent
- [ ] #2 tcms vv metrics lists every failing case with its failure-start run; a case that flips PASSED, FAILED, PASSED, FAILED starts at the last FAILED run
- [ ] #3 A test covers a case with BLOCKED-only results and a case missing from some runs
- [ ] #4 The output shape is documented in docs/VV_METRICS.md
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
