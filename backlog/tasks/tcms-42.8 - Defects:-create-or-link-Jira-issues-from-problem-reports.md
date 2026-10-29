---
id: TCMS-42.8
title: 'Defects: create or link Jira issues from problem reports'
status: Done
assignee:
  - '@worker-c3'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T16:21:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A failed result raises a problem-report; the sync creates a Jira issue of defect_issue_type from it or links an existing issue, stores the key in external_refs, and keeps the problem report status and the Jira status visible to each other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tcms pr push creates the Jira issue with summary, description, severity to priority mapping, and links to the feature and test entity
- [x] #2 tcms pr link <PR> <KEY> links an existing issue
- [x] #3 The feature's verification roll-up treats an open linked issue as blocking passed
- [x] #4 Closing in Jira is reflected in the problem report at the next sync as a reported difference, not an automatic status change
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. test-backends defects module: create a defect from a problem report (guard WriteKind::Defect, priority from severity_to_priority, links to feature and test entity), link an existing issue (GET verify, record external_refs), reconcile PR status vs Jira status as reported differences only. 2. Blocking rule: a problem report on a feature's requirement with open status or open linked issue blocks passed (rollup input). 3. tcms pr push / tcms pr link commands. 4. Tests: guard refusals, mapping, dry run, MockJira HTTP.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.8, worker C3):
(D-42.8a) Defects are created and linked through Ctx::write with WriteKind::Defect, so write_policy.defects and the project scope apply; the link's defect key is the issue the guard checks.
(D-42.8b) A problem report's Jira key is kept in its own external_refs (the problem-report schema has the field). tcms pr push and pr link write it; pr link refuses to re-link a report to another key; a report with a key is never pushed again.
(D-42.8c) pr push writes directly; --dry-run prints the plan and sends nothing. (Sync defaults to dry run: TCMS-42.9.)
(D-42.8d) Blocking rule: a feature is not passed while a linked problem report is open (any status outside fixed, verified, closed, rejected, unknown statuses count as open) or its linked Jira issue is not done.
(D-42.8e) Reconciliation reports a difference when a Jira status category disagrees with the report status; it never changes the report or Jira. Sync wiring of the difference report: TCMS-42.9.
(D-42.8f) Priority from severity_to_priority; a severity without a mapping leaves priority unset and the output says so. Description: problem text, report id, raised_by, and the pushing user.
(D-42.8g) Link type from tcms_issue_types.link_type; without it the defect is created unlinked with a note.
(D-42.8h) jira-client mock (owned crate) gained GET /issue/{key} and a statusCategory derived from the status name (Done, Closed, Resolved are done).
ASSUMPTIONS (from Jira REST documentation as remembered, verified against the mock only): POST /issue with project, issuetype, summary, description (ADF on Cloud), priority, labels; POST /issueLink with type.name, outwardIssue, inwardIssue; GET /issue/{key}?fields=status with fields.status.statusCategory.key.
Evidence: cargo test -p test-backends --features mock: all pass (tests/defects.rs: 8 tests over HTTP: creation with priority, links, actor in description, no second push, dry run, policy refusal, unmapped severity, link_existing and project scope, reconciliation sends nothing, open linked issue blocks passed). cargo test -p tcms --test pr_cli: 5 pass (dry run writes nothing, create and record key, refusal, link, token absent from record and output). cargo test -p jira-client --all-features: 22 + 1 pass. Clippy -D warnings clean on the three crates. Problem-report fixture validated against tcms-schemas problem-report schema.
Not run: any real Jira. AC 3 is proven at the roll-up function; the sync feeds it real data in TCMS-42.9 (42.6 leftover d).

Sync-level proof (worker C4, item 5): crates/tcms/tests/sync_cli.rs an_open_linked_defect_blocks_passed_through_a_sync_run. Control: every case passes, PR-007 fixed, nothing open against PAY-1: --apply writes verification 'passed'. Blocked: the same with PR-007 linked (external_refs) to PAY-500, an open Bug on the mock: --apply does not write 'passed' (dry run writes nothing). Closing PAY-500 on the mock and re-syncing writes 'passed'. Mutation (the sync ignores the linked issue state): the blocked assertion fails with 'got Some(passed)'. Verified against the mock only.

Review (round 4, reviewer). AC1: pr_cli push_creates_defect_and_records_its_key and defects.rs push_creates_defect_with_priority_links_and_actor pass. AC2: pr_cli link_records_an_existing_issue_and_refuses_a_second_one passes. AC3: sync_cli an_open_linked_defect_blocks_passed_through_a_sync_run passes (the sync does not write passed while the linked issue is open). AC4: defects.rs closed_in_jira_is_reported_and_the_report_is_not_changed passes. Known gap, recorded in TCMS-42.9: a fresh CI checkout creates a new defect for each open report again (reproduced on the mock: PAY-103 after PAY-102), because the report key is kept only in the record and the CI job does not commit it back. The ACs above are met; the duplication is an operational gap of the sync job. Verified against the mock only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tcms pr push and pr link for defects with priority mapping, links, and reconciliation as a reported difference; open linked issue blocks passed. Verified by cargo test (pr_cli, defects tests, sync_cli) against the mock. Gap (TCMS-42.9): defects are not deduplicated across CI runs without a commit-back of the report records. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
