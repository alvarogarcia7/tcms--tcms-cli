---
id: TCMS-50.1
title: Decide and apply one dedupe rule for per-case results
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
labels:
  - vv-metrics
  - decision
dependencies:
  - TCMS-47.3
parent_task_id: TCMS-50
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict (tcms-web) and the CLI pick one result per test case when a case has several results, and they disagree:
- TCMS-47.3 AC #3: the latest started_at wins; not_executed never overrides an executed result.
- Verdict (scripts/vv/shim/vv_dedupe.py): the result with the detail flag wins, then the newest date, then the path.

The two disagree when detail flags differ. This task records one rule, applies it in the CLI, and removes Verdict's Python dedupe.

Decision (supersedes the duplicate rule in TCMS-47.3 AC #3, which TCMS-47.3 now points here for, and the "Duplicates" paragraph in docs/VV_METRICS.md once applied):
- Order: attempts and runs are sorted by started_at, oldest first. Equal started_at is broken by the path (run folder name, then file path), greater path counts as later. "Last" means latest in this order.
- Walk from the last one back to the first, for one test case:
  1. The first pass, fail or error decides the case. pass gives PASSED; fail and error both give FAILED.
  2. not_executed and skipped never decide; go to the previous one.
  3. If nothing decides (only not_executed or skipped, or no result), the case has no decided result and is reported as not_executed.
- Reason: one rule removes the drift between the Java and Python implementations that the parent task describes, and it matches the "latest verdict by started_at" statement in docs/JIRA_INTEGRATION.md. error counts as a failure per TCMS-50.2.

Note for the change set (keep tcms-web changes minimal): only the expected values in the tcms-web parity fixture are regenerated from this rule. The Java parser logic is not rewritten for this; the Verdict switch to the CLI output is the only tcms-web change needed for dedupe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/VV_METRICS.md states the single rule, the order (started_at, then path), the reason, and that it replaces the rule in TCMS-47.3 AC #3; TCMS-47.3 points to it (done in 47.3)
- [ ] #2 tcms vv metrics applies the rule to the legacy duplicates fixture; the expected per-case results of that fixture are regenerated from the rule, and the tcms-web parity expectations (TcmsCliLegacyParityTest) are updated in the same change set with no change to the Java parser logic
- [x] #3 A test covers not_executed and skipped never overriding a pass, fail or error result
- [x] #4 A test covers error counting as FAILED
- [x] #5 A test covers the tie cases: equal started_at with different paths (greater path wins), and a case with no decided result reported as not_executed
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Implemented inside TCMS-47.3's winner() (see that task): newest-first walk, first pass/fail/error decides, not_executed and skipped never decide, ties broken by the greater path. Work here: decision entry in docs/VV_METRICS.md, a duplicates fixture under crates/tcms/tests/fixtures/vv-metrics/ (tie, skipped and error cases), and tests covering AC#3-AC#5. AC#2 parity with tcms-web Java cannot be run here (the Java parser and its fixture are in tcms-web); the expected values are generated from this rule.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rule implemented in vvcombine::winner (see TCMS-47.3). Tests: vvcombine unit tests (tie, newest decided, no decided) and vvmetrics status_mapping_from_verdicts (error = Failed).

AC 2 NOT MET here: the legacy duplicates fixture and the tcms-web Java parser are in tcms-web, not in this repository. Parity expectations there must be regenerated from this rule in the tcms-web change set (follow-up).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The single duplicate rule is implemented in the CLI and documented in docs/VV_METRICS.md and docs/RUN_RECORD_FORMAT.md. Tests cover not_executed and skipped never overriding a decided result, error counting as Failed, and the tie cases. The parity check against the tcms-web fixture is a follow-up in tcms-web and is not verified here.
<!-- SECTION:FINAL_SUMMARY:END -->
