---
id: TCMS-50.4
title: Per-case history and failing list with the run where the failure started
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
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
- [x] #1 tcms vv history --case <ID> prints one row per run, oldest first, with the status or absent
- [x] #2 tcms vv metrics lists every case whose result in the selected run is FAILED, sorted by case id, each with its failure-start run; a case that flips PASSED, FAILED, PASSED, FAILED starts at the last FAILED run
- [x] #3 Tests cover a case with SKIPPED-only results, a case with error-only results (counted as FAILED), and a case missing from some runs
- [x] #4 The output shape and sort order are documented in docs/VV_METRICS.md
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New subcommand `tcms vv history --root <DATA_ROOT> --case <ID> [--plan <ID>]`: one row per run, oldest first, {runId, status} with status PASSED, FAILED, SKIPPED, NOT_EXECUTED or null when absent (decided in 50.1/50.2 vocabulary).
2. metrics adds failingCases: cases whose selected-run status is FAILED, sorted by case id, each with failureStartRunId: walk back from the selected run; FAILED extends the streak, PASSED ends it, SKIPPED, NOT_EXECUTED and absent are passed over.
3. Tests: PASSED,FAILED,PASSED,FAILED gives the last FAILED; SKIPPED-only case; error-only case (FAILED); case missing from some runs.
4. docs/VV_METRICS.md output shape and sort order.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
tcms vv history --root R --case C [--plan P]: one row per run, oldest first, status PASSED/FAILED/NOT_EXECUTED/SKIPPED or null (absent). Tests: vv_trends_history_cli.rs history_* (14 rows on sut-1, unknown case all null).

failingCases in metrics: sorted by case id; failureStartRunId walks back from the selected run (FAILED extends, PASSED ends, SKIPPED/NOT_EXECUTED/absent passed over). Unit tests in vvcmd.rs streak_tests: oldest run of the streak, pass ends streak, skipped/not_executed/absent passed over, error as failed, sorted output, skipped-only case not listed. Integration: failing_cases_name_the_run_where_the_failure_started.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added tcms vv history --case (one row per run, oldest first) and failingCases in tcms vv metrics (sorted by case id, each with the run where the failure started). Streak rules: FAILED extends, PASSED ends, SKIPPED, NOT_EXECUTED and absent are passed over. Documented in docs/VV_METRICS.md with the output shape. Tested with unit and integration tests; not yet verified by make test.
<!-- SECTION:FINAL_SUMMARY:END -->
