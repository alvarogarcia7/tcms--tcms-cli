---
id: TCMS-42.6
title: Write per-feature verification and validation status and coverage
status: Done
assignee:
  - '@worker-c2'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T16:21:00Z'
labels: []
dependencies:
  - TCMS-42.3
  - TCMS-42.5
  - TCMS-40.5
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each feature issue gets verification_status, validation_status and coverage, rolled up from the requirements that trace to it (the feature itself when Jira is the source, StrictDoc requirements whose external_refs name it otherwise), the trace matrix, the coverage report, latest verdicts and open problem reports, following the roll-up rules in the mapping doc. Optional workflow transitions per verification status.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Roll-up produces not_started, in_progress, passed, failed, blocked exactly as the mapping doc defines; unit tests cover every rule
- [x] #2 Validation status follows requirement.validation_status; a validation review changes it
- [x] #3 Transitions configured in transitions are applied only when feature_transitions is allowed; a missing workflow transition is logged and skipped
- [x] #4 Features with no traced requirement are reported, not written
- [x] #5 A dry-run flag prints every intended write without sending it
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pure roll-up in test-backends/src/rollup.rs: verification, validation, coverage from RollupInput (requirements with covered flag and validation state, latest status per traced test case, open problem report flag), following the mapping doc section 2.
2. FeatureWriter writes the configured fields (verification_status, validation_status, coverage) through Ctx::write with WriteKind::FeatureFields; transitions through FeatureTransition only when feature_transitions is on and the workflow has the transition; dry run reads but sends nothing.
3. Tests: one unit test per rule; writer tests for validation following the requirement, transitions allowed/refused/missing, untraced feature reported, dry run, summary and description refused by the guard.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.6, worker C2):
(D-42.6a) Verification, validation and coverage are pure functions of RollupInput; the caller builds the input. Reading the trace matrix and test-run records into RollupInput is NOT done in this task (it belongs to the sync command, TCMS-42.9). The roll-up therefore does not yet read coverage-report or the trace matrix files directly.
(D-42.6b) Coverage is the integer percent of covered requirements, rounded half up; a feature without requirements writes no coverage value and is reported as unreported.
(D-42.6c) Transitions: the key of transitions is the TCMS verification value (passed, failed, ...); a missing transition in the workflow is a Skipped entry with a log warning; feature_transitions off is a Skipped entry.
(D-42.6d) Problem reports are a boolean input (open report linked to the feature); the rule does not read problem-report records yet.
(D-42.6e) Values are written through status_values (Jira option names), as in 42.5.
Evidence: cargo test -p test-backends: 7 unit tests (one per roll-up rule: not_started, failed, blocked, passed, in_progress, validation, coverage), 5 writer tests in tests/features.rs (validation follows the requirement and a review changes it; transitions applied, skipped when missing, skipped when off; untraced feature reported and not written; dry run writes nothing; the guard refuses summary and description). All pass. Clippy -D warnings clean; fmt applied.

Gap: the roll-up rules are proven on in-memory inputs. Feeding them from the trace matrix, test-run records and problem reports is part of the sync command (TCMS-42.9); AC 1 covers the rules, not the loader.

Review round 3 (reviewer, branch tcms-review-r3): roll-up rules checked line by line against the mapping doc (/home/orca/tcms-schemas/docs/jira-integration.md, section 2) in crates/test-backends/src/rollup.rs: verification (not_started when nothing traces or no result; failed on fail or error; blocked on not_executed of a finished run; passed when all covered, all pass, no open problem report; in_progress otherwise), validation (rejected -> failed; all validated -> passed; none set -> not_started; else in_progress), coverage (rounded half up); verdict mapping in lib.rs latest_verdict matches. Tests: cargo test -p test-backends (rollup 7 unit, features 5 + the guard test added in 5edda19 for summary/description on dedicated issues) all pass. Status stays In Review: the description says the roll-up is computed from the trace matrix, coverage report, latest verdicts and problem reports on the record; the code takes an in-memory RollupInput, so the real inputs are the TCMS-42.9 sync command. Also note: open problem reports are a boolean input, the problem-report records are not read yet (D-42.6d).

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-backends and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.

Leftover wiring (TCMS-42.9, worker C3): the roll-up is fed from the real inputs: the trace matrix (testcase_verification::build_matrix over --requirements-dir, --tests-dir, --runs-dir), the latest verdicts of the run records, the open problem reports (blocks_passed, TCMS-42.8) and requirement validation_status when a record has it. Exercised by test-acceptance/validate_jira_integration.sh step 4 (feature writes derived from the matrix and runs) and by sync_cli (features step writes and comments). The blocking rule itself is proven by unit tests (rollup.rs and tests/defects.rs). Limit: the Jira import does not set validation_status.

Sync-level proof (worker C4, item 5): see TCMS-42.8 notes; crates/tcms/tests/sync_cli.rs an_open_linked_defect_blocks_passed_through_a_sync_run runs the roll-up through tcms jira sync --apply (features step) with an open linked defect (not passed), the control (passed) and the closed defect (passed). Verified against the mock only.

Review (round 4, reviewer). AC1: rollup.rs has one unit test per rule (not_started, failed, blocked, passed, in_progress, validation, coverage) and they pass. AC2: features.rs validation_status_follows_the_requirement_and_a_review_changes_it. AC3: features.rs transition_applies_only_when_allowed_and_present_otherwise_skipped. AC4: features.rs untraced feature reported and not written. AC5: features.rs dry_run_prints_every_intended_write_and_sends_nothing (no request, checked by mock log in the sync dry run). Sync level: the features step gets its inputs from the trace matrix and runs (acceptance step 4; sync_cli). Verified against the mock only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Per-feature verification, validation and coverage roll-up with guarded writes, transitions when allowed, dry run. Verified by cargo test -p test-backends (rollup and features tests) and the Jira acceptance script (mock). DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
