---
id: TCMS-42.5
title: 'Write test case statuses: lifecycle and latest verdict'
status: Done
assignee:
  - '@worker-c2'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T16:21:00Z'
labels: []
dependencies:
  - TCMS-42.4
  - TCMS-40.4
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Jira shows two values per test case: the lifecycle status (draft, in_review, approved, obsolete) and the latest verdict on the plan's current baseline, mapped to not_started, in_progress, passed, failed, blocked as defined in the mapping doc, plus the baseline id the verdict refers to.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After a run is committed, every executed test case's Jira entity shows the mapped verdict and baseline
- [x] #2 A status change by review updates the lifecycle field at the next sync
- [x] #3 A value edited by hand in Jira is overwritten and the overwrite is logged
- [x] #4 Test cases without external_refs are created on first sync when dedicated_issues is allowed, otherwise reported as unmapped
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Lifecycle is written on every sync (sync_case); verdict and baseline come from sync_run with the latest_verdict mapping (42.4).
2. Hand edits: the caller passes the values TCMS last wrote (CaseStatus.last_written, from the record, persisted in 42.9). Backends read current values before writing; a difference is an Overwrite event and an audit-trail entry (action sync); no last-written value means nothing is logged.
3. Unmapped: creation refused by write_policy.dedicated_issues becomes a typed Unmapped error, reported by sync_case_or_unmapped with no request sent.
4. Tests: lifecycle at next sync on all four backends; hand edit overwritten and logged on plain Jira and Zephyr Scale; unmapped vs found with creation off.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.5, worker C2):
(D-42.5a) Hand-edit detection needs the value TCMS last wrote; the caller supplies it (CaseStatus.last_written, RunCase.last_written). The record that holds it is written by the sync command (TCMS-42.9). Without it, a difference cannot be told from a first write, so nothing is logged.
(D-42.5b) An overwrite is returned by take_overwrites() as an Overwrite and built into an audit-trail entry (action sync, before and after, reason) by overwrite_audit_entry(). Writing those entries to a file is TCMS-42.9.
(D-42.5c) Creation with write_policy.dedicated_issues off is an Unmapped error, reported by sync_case_or_unmapped; no request is sent. An existing entity is still found (a read) and mapped; writing its status needs dedicated_issues, so that write is refused and reported as an error.
(D-42.5d) Verdict and baseline use the 42.4 latest_verdict mapping; lifecycle is written as is whenever it is given.
Evidence: cargo test -p test-backends: scenario 4 + refused write + unknown backend (6), plain Jira over HTTP (1), statuses 4 (lifecycle on four backends, hand edit on plain Jira and Zephyr Scale, unmapped vs found). All pass; clippy -D warnings clean; fmt applied. Scenario tests from 42.4 also prove AC1 (verdict and baseline on each executed case after a run).

AC 3: the overwrite is returned as an event and as an audit-trail entry; persisting the audit entry to disk is TCMS-42.9. AC 1 proven by the 42.4 scenario test (verdict fields and baseline on each executed case, Jira-based and Zephyr Scale).

Review round 3 (reviewer, branch tcms-review-r3): all four AC re-verified by cargo test -p test-backends (statuses.rs: lifecycle_change_is_written_at_the_next_sync on four backends; hand_edit_is_overwritten_and_the_overwrite_is_logged on plain Jira; hand_edit_is_overwritten_on_zephyr_scale; unmapped_when_creation_is_not_allowed_and_found_when_it_exists; scenario.rs for AC #1 verdict and baseline): all pass. Status stays In Review, because the operational path depends on TCMS-42.9: hand-edit detection needs last_written from the record (the caller supplies it in the tests), and the audit-trail entry of an overwrite is built in memory and is not written to a file until 42.9. Library-level proof only. Verdict mapping checked against the mapping doc (jira-integration.md section 2): pass/fail/error/not_executed(finished or not)/none. Tests: cargo test -p test-backends (statuses 4 pass).

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-backends and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.

Review (round 4, reviewer). AC1: scenario.rs plain_jira_scenario and xray_scenario and zephyr_scenario check verdict and baseline per executed case; acceptance check 6 reads the verdict passed from the mock. AC2: statuses.rs lifecycle_change_is_written_at_the_next_sync passes. AC3 sync level, checked by hand with the real binary: the lifecycle was hand-edited on the mock to hand-edited; the next apply printed 'hand-edited, overwritten' as a difference, wrote an audit entry (reason: hand edit in the system overwritten by the TCMS sync) and restored draft on the mock. Also statuses.rs hand_edit_* on plain Jira and Zephyr Scale. AC4: statuses.rs unmapped_when_creation_is_not_allowed_and_found_when_it_exists passes. Verified against the mock only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Lifecycle and latest verdict written at each sync, hand edits overwritten and logged as audit entries (checked on the mock with the real binary), unmapped cases reported. Verified by cargo test -p test-backends and the Jira acceptance script (mock). DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
