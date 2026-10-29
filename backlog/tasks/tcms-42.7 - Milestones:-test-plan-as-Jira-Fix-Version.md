---
id: TCMS-42.7
title: 'Milestones: test plan as Jira Fix Version'
status: Done
assignee:
  - '@worker-c2'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T16:21:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A test plan is the milestone. The sync creates or updates a Fix Version named after the plan, writes plan state, executed/total cases, pass rate, baseline and TCMS link into its description, assigns it to the features in scope when feature_fields is allowed, and marks it released when the plan completes and release_on_plan_complete is set.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A plan without external_refs gets a Fix Version created and its id stored
- [x] #2 Progress in the description matches the plan's runs after every sync
- [x] #3 Completing the plan releases the Fix Version with the release date; reopening never un-releases automatically but reports it
- [x] #4 milestone.target epic and issue are accepted by the config and documented as not yet implemented, failing clearly
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Plan = Fix Version: create once (POST version with projectId), keep its id; description rewritten each sync (state, executed/total, pass rate, baseline, TCMS link).
2. Release on completion when release_on_plan_complete; a reopened plan is reported, never un-released.
3. Features in scope get fixVersions when feature_fields is on; the guard allows fixVersions only for the fix_version target.
4. milestone.target epic or issue: accepted by the config, refused at sync with a clear message.
5. Tests against the in-process fake; guard tests for version writes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.7, worker C2):
(D-42.7a) A milestone write is allowed only when milestone.target is fix_version (WriteKind::Milestone). The schema has no milestones policy flag, so the milestone block is the switch; no schema edit.
(D-42.7b) fixVersions on a feature is a system field, allowed only for WriteKind::FeatureFields with the fix_version target and feature_fields on (FIX_VERSIONS_FIELD in jira-client guard). Summary and description still refused.
(D-42.7c) feature_fields off: features are not assigned and the sync says so in its notes; the sync does not fail.
(D-42.7d) Released state is read from the version (GET version) before a release is written; a complete plan that is already released is not released again.
(D-42.7e) The release date is passed in by the caller (YYYY-MM-DD).
(D-42.7f) The Fix Version id is returned in MilestoneReport.version_id for the caller to store in test-plan.external_refs; storing it in the record is TCMS-42.9.
Evidence: cargo test -p test-backends --test milestones: 7 pass (description content, created once and description follows every sync, release with date and reopen reported not undone, features assigned only when feature_fields allows and summary untouched, dry run sends nothing, epic and issue refused, guard rules). cargo test -p jira-client --all-features: 22 + 1 pass. test-backends total 30 pass. Clippy -D warnings clean.
Not done: milestone.target epic and issue are documented in docs/JIRA_INTEGRATION.md only in TCMS-42.9; AC4 stays open until that doc exists.

AC 1 not ticked: the Fix Version is created and its id returned, but storing that id in the test plan record is TCMS-42.9. AC 4 not ticked: the refusal is tested, the documentation is TCMS-42.9. AC 2 and AC 3 are proven by tests/milestones.rs.

Review round 3 (reviewer, branch tcms-review-r3): cargo test -p test-backends --test milestones: 7 pass, including version_is_created_once_and_its_description_follows_every_sync (AC #2), completing_the_plan_releases_with_the_date_and_reopening_is_reported_not_undone (AC #3: a reopened plan reports 'stays released', released_now false), dry_run_sends_nothing, epic_and_issue_targets_are_accepted_by_the_config_and_refused_clearly, and the guard test for version writes. Dry run and guard checked: milestone writes go through Ctx::write (check_write first); fixVersions on features only with fix_version and feature_fields. Status stays In Review: AC #1 (store the Fix Version id in the test-plan record) and AC #4 (docs/JIRA_INTEGRATION.md has no milestone.target section; the documentation is TCMS-42.9) remain open. Note: the test-backends tests run here after the summary/description guard fix in 5edda19.

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-backends and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.

Leftover wiring (TCMS-42.9, worker C3): AC 4 proven by crates/test-backends/tests/milestones.rs (epic and issue targets accepted by the config and refused at sync with a clear message) and documented as not implemented in docs/JIRA_INTEGRATION.md (Limits). AC 1 NOT ticked: the Fix Version id is returned by the sync and kept in state.yaml under plans.<id>.version_id, not in the test plan's external_refs; no sync test exercises a milestone create (the sync_cli fixture has no milestone block).

Decisions (TCMS-42.9 round, worker C4):
(D-42.4i) The entity key is taken from the record first when the sync state has none: the external_refs entry of the backend's system (test case) or of system jira (plan). The state stays as a fallback and a mirror; a sync without state finds the same entities.
(D-42.4j) Records are changed by a targeted text edit (crates/tcms/src/extrefs.rs): only the external_refs block changes; every byte before it is kept. The same system and key again: file not touched (idempotent). A new key for the same system replaces that entry; no entry adds one at the end. synced_at is written with the entry, not on every sync. url is never stored.
(D-42.4k) The edited text must parse, hold the entry, leave every other top-level key equal, and validate against its schema before it is written. A non-empty flow list is refused with an error (not rewritten).
Evidence (AC 1, plan): crates/tcms/tests/sync_cli.rs plan_without_external_refs_gets_one_fix_version_and_its_id_in_the_plan: one POST /version on the mock (jira-client mock now serves /version, GET/PUT /version/{id}, project id), the Fix Version id ('10101') lands in test-plans/PLAN-1.yaml as external_refs system jira, the plan validates against tcms-schemas test-plan, a second apply creates nothing and leaves the file byte-identical, and so does a third apply after the state directory is deleted (record alone). Mutation check: disabling the store fails this test and the test-case test. Verified against the mock only.

Review (round 4, reviewer). AC1: sync_cli plan_without_external_refs_gets_one_fix_version_and_its_id_in_the_plan passes: one Fix Version created, its id stored in the plan record external_refs (system jira), a second apply creates nothing. AC2: milestones.rs description_carries_the_plan_state_and_progress and version_is_created_once_and_its_description_follows_every_sync. AC3: milestones.rs completing_the_plan_releases_with_the_date_and_reopening_is_reported_not_undone. AC4: milestones.rs epic_and_issue_targets_are_accepted_by_the_config_and_refused_clearly; docs/JIRA_INTEGRATION.md Limits states it. Verified against the mock only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Test plan as Jira Fix Version: created once with its id stored in the plan record, description with progress, release on completion, epic and issue targets refused. Verified by cargo test (sync_cli, test-backends milestones) against the mock. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
