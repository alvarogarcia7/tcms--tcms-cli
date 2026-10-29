---
id: TCMS-42.4
title: 'Test backend adapters: plain Jira, Xray, Zephyr Scale, Zephyr Squad'
status: Done
assignee:
  - '@worker-c2'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T17:05:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Where a test case and a run are represented in Jira depends on the test backend. Plain Jira: a TCMS-owned Test issue per test case (tcms_issue_types.test), found by the tcms_id field, linked to features with link_type. Xray: Test, Test Plan, Test Execution. Zephyr Scale: test case, cycle, execution through its own API. Zephyr Squad: Test issue and cycle. One trait, four implementations; the sync code never branches on the backend.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each backend creates or finds the test entity for a test case and stores its key in external_refs
- [x] #2 Each backend records a run as its execution entity (plain Jira: fields on the Test issue) with per-case verdict
- [x] #3 Test lifecycle status and latest verdict are written to the configured fields on every backend
- [x] #4 Switching test_backend in the config is the only change needed; the acceptance suite runs the same scenario on all four with mocks
- [x] #5 A test case deleted in TCMS marks its Jira entity obsolete, never deletes it
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New crate crates/test-backends: TestBackend trait (ensure_test, write_case_status, record_run, mark_obsolete) and four implementations (plain Jira, Xray, Zephyr Scale, Zephyr Squad); sync functions take &mut dyn TestBackend.
2. Transport trait Api: JiraApi wraps jira-client (retries, auth) through a new guard-free raw_json; ZephyrApi is reqwest with a bearer token from auth.secret_ref.
3. Every write goes through Ctx::write, which runs jira_client::check_write first.
4. In-process FakeApi (feature mock) models the assumed endpoints; scenario test runs the same sequence on all four configs loaded through the schema-validated loader.
5. Plain Jira also run over real HTTP against the jira-client mock (Cloud layout).
6. Verify: cargo test -p test-backends -p jira-client --all-features, clippy -D warnings, fmt. ACs ticked only where a test proves them.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.4, worker C2):
(D-42.4a) test_backend absent or 'none' selects plain Jira (the schema enum has 'none', not 'plain').
(D-42.4b) All writes pass one chokepoint, Ctx::write, which calls jira_client::check_write before any request; a refused write never reaches the transport (tested).
(D-42.4c) Zephyr Scale base URL comes from env TCMS_ZEPHYR_API_URL (default https://api.zephyrscale.smartbear.com/v2); the vendored integration-config schema has no field for it and must not be edited here. The token comes only from auth.secret_ref.
(D-42.4d) Plain Jira, Xray and Squad find a test entity by JQL (project and issue type) then match fields.tcms_id client-side; correct for small projects only. Links are created only when the entity is created: outwardIssue = test, inwardIssue = feature, type tcms_issue_types.link_type (required when a case has features).
(D-42.4e) Zephyr Scale writes no Jira links; features are not linked for that backend.
(D-42.4f) CaseStatus.lifecycle is optional: sync_run never rewrites a lifecycle value; mark_obsolete writes lifecycle obsolete and nothing else.
(D-42.4g) The trait has no delete operation; the scenario asserts no DELETE was sent.
(D-42.4h) Plain Jira: run verdicts are written on each Test issue (record_run returns None). Xray: a Test Execution issue per run. Zephyr Scale: a cycle per run. Squad: a cycle per run.
ASSUMPTIONS (not verified against any real product; from public documentation as remembered):
(A-42.4a) Xray: POST /rest/raven/1.0/api/testexec/{key}/test with add=[keys]; PUT /rest/raven/1.0/api/testrun/status with testExecKey, testKey, status; status names TODO, EXECUTING, PASS, FAIL, BLOCKED.
(A-42.4b) Zephyr Scale: GET /testcases?projectKey&query=labels = tcms:<id> finds a case by label; POST /testcases, PUT /testcases/{key} (statusName, customFields), POST /testcycles, POST /testexecutions (statusName Pass, Fail, Blocked, In Progress, Not Executed).
(A-42.4c) Zephyr Squad: POST /rest/zapi/latest/cycle returns id; POST /rest/zapi/latest/execution with issueId, cycleId, status 1 pass, 2 fail, 3 WIP, 4 blocked, -1 unexecuted.
Evidence: cargo test -p test-backends (6 scenario tests across the four backends, a refused-write test, an unknown-backend test) and --test plain_jira_http (plain Jira over HTTP against the jira-client mock): all pass. cargo test -p jira-client --all-features: 22 + 1 pass. Clippy -D warnings clean; fmt applied.
Not done: the key is produced as an external_refs entry (external_ref, known_entity), but not yet persisted into a TCMS record: that is the sync command (TCMS-42.9). Nothing was run against a real Xray, Zephyr or Jira instance.

AC 1 left open: the external_refs entry is built and round-trips, but persisting it to the record is TCMS-42.9. AC 2 to 5 proven by the scenario tests (verdict fields and execution entities per backend; obsolete written, never deleted; same config except test_backend).

Review round 3 (reviewer, branch tcms-review-r3): (1) Write guard audit: every write of the backends and milestones goes through Ctx::write, which calls check_write before Api::send; grep shows raw_json only reached from JiraApi::send. Guard refusals were tested before any request. Defect fixed (commit 5edda19, test first): the guard allowed summary and description for DedicatedIssue even with an issue key, so a status field id configured as summary would overwrite a TCMS-owned issue; now refused on update, creation unchanged. (2) AC #4 mutation check: three mutations in backends.rs (Xray PASS->PASSED, Zephyr Obsolete->Deprecated, Squad Passed->7) each failed exactly its own backend scenario test (xray_scenario, zephyr_scale_scenario, zephyr_squad_scenario); plain Jira still passed; file restored. The scenario shares run_on() and differs only in test_backend, so the claim holds, but it runs under cargo test (scenario.rs), NOT in the acceptance suite (test-acceptance does not reference test-backends), which is what AC #4 names: left open. (3) Documentation gap: docs/JIRA_INTEGRATION.md has no test-backend section, so the Xray, Zephyr Scale and Squad endpoint assumptions are labelled in code comments and task notes only; the docs belong to TCMS-42.9. (4) AC #1 stays open until the sync command persists external_refs (TCMS-42.9). AC #2, #3, #5 re-verified (scenario.rs, statuses.rs). Tests: cargo test -p test-backends -p jira-client: all pass (after the fix). Clippy -D warnings and fmt clean on both crates. Secrets: ZephyrApi has no Debug and redacts the token in every error path; JiraClient redacts via secret.redact. Status stays In Review.

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p test-backends -p jira-client and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.

Leftover wiring (TCMS-42.9, worker C3): AC 1 remains open. The sync (tcms jira sync, tests step) persists the entity key per test case in state.yaml (test_cases.<id>.key, system), not in the test case record's external_refs: the test case YAML is not rewritten by the sync (a rewrite of the whole record risks losing content). Proven by crates/tcms/tests/sync_cli.rs (state holds the keys; a second apply finds the same entities).

Persisting the entity key (worker C4, D-42.4i..k): the tests step stores the key of each test case in its record's external_refs with a targeted edit (crates/tcms/src/extrefs.rs; decisions and evidence in TCMS-42.7 notes). Proven at sync level for plain Jira: crates/tcms/tests/sync_cli.rs apply_stores_the_entity_key_in_the_test_case_record (record keeps every byte before the block, dry run does not touch it, second apply byte-identical with no new Test issue, the record loads and validates, url not stored). AC 1 stays unticked: the same store runs for Xray, Zephyr Scale and Zephyr Squad (the ext system comes from the backend kind), but only plain Jira has a sync-level run so far. Xray sync-level proof comes with TCMS-42.9 AC 5 (acceptance suite, worker C4 item 3); Zephyr Scale and Squad have no sync-level run.

Update (worker C4): Xray now has a sync-level proof of the key in external_refs (acceptance product 3, check 14: the test case record holds system xray with the key of the sync state); see TCMS-42.9 notes. Zephyr Scale and Zephyr Squad still have no sync-level run, so AC 1 stays unticked. Also D-42.4m: Xray record_run reuses the execution of the same run (test-backends scenario test). Zephyr Scale and Squad record_run still create a cycle or execution on every sync: open, see TCMS-42.9 notes.

Worker C5 (TCMS-42.4 AC 1 for all four backends; mocks only, nothing run against a real Jira, Xray, Zephyr Scale or Zephyr Squad).

Decisions:
(D-42.4o) Zephyr Scale and Zephyr Squad record_run reuse the cycle named by the run id (listing first, then create) and update the execution of a case already in that cycle (PUT) instead of creating one per sync. The same way as D-42.4m for Xray. Cycle and execution lookups are matched client-side on name and case, first page only (the same limit as the Jira search). ASSUMPTION endpoints, see docs/JIRA_INTEGRATION.md.
(D-42.4p) The tests step built a JiraApi for every backend, so `tcms jira sync --apply` could not reach Zephyr Scale at all. zephyr_scale now uses ZephyrApi (base from TCMS_ZEPHYR_API_URL, token from auth.secret_ref). Zephyr Squad stays on the Jira site.
(D-42.4q) Zephyr Squad record_run gets the Jira issue id by GET issue when the sync state has only the key (the state keeps no id). Before, a second sync failed with "no Jira issue id for the execution".
(D-42.4r) jira-client mock: each created issue gets its own id (it returned 20001 for every issue, which would have merged the Squad executions of different cases) and GET returns it. The same mock serves Zephyr Scale under /v2 and Zephyr Squad under /rest/zapi/latest, behind the same token. Zephyr keys: PROJ-T<n> for cases, PROJ-C<n> for cycles.
(D-42.4s) Acceptance products 4 (zephyr_scale) and 5 (zephyr_squad) each start a fresh mock, as products 1 to 3 do, with fixtures test-acceptance/jira_integration/fixtures/zephyr_scale_product.yaml and zephyr_squad_product.yaml. The fixture run is RUN-ACC-1 with one test case, so one execution per product.
(D-42.4t) The in-process FakeApi (feature mock) answers the new listings and updates, so the re-sync regressions run without HTTP.

Evidence:
- AC 1 per backend, sync level (crates/tcms/tests/sync_cli.rs): plain Jira apply_stores_the_entity_key_in_the_test_case_record; zephyr_scale_apply_stores_the_entity_key_and_a_second_apply_creates_nothing; zephyr_squad_apply_stores_the_entity_key_and_a_second_apply_creates_nothing. Each: dry run changes nothing; apply creates the entity and the cycle and executions; record keeps every byte before the external_refs block and holds system zephyr_scale / zephyr_squad with the state key; the record loads and validates; a second apply creates no object and leaves the record byte-identical.
- AC 1 for Xray at sync level and in the acceptance suite: acceptance product 3, checks 14 to 18.
- Acceptance (test-acceptance/validate_jira_integration.sh): products 4 and 5 added as checks 19 to 33 (Zephyr Scale: key in record, lifecycle Draft on the mock, one cycle, Pass execution, second sync creates no case/cycle/execution, audit and token). Full run: 82 passed, 0 failed.
- Regression tests for the reuse (fail without the fix; mutated and restored, each fails on its own test): test-backends tests/scenario.rs zephyr_scale_a_second_sync_of_the_same_run_reuses_its_cycle_and_executions and zephyr_squad_a_second_sync_of_the_same_run_reuses_its_cycle_and_executions. Mutations: no cycle reuse and no execution update, for each backend (4 runs, all fail). Sync level: removing the Zephyr Scale cycle reuse fails zephyr_scale_apply_*; routing zephyr_scale through JiraApi fails it too (mutated, restored).
- Acceptance mutation: the reuse removed in both backends makes checks 25 and 32 fail (mutated, restored, rebuilt, 82/82 again). Broken expectation JIRA_ACCEPTANCE_EXPECT_WHO=bob fails checks 5, 18, 26 and 33.
- Tests: cargo test -p tcms -p test-backends -p jira-client --all-features: all pass. clippy -D warnings (with -A clippy::useless_borrows_in_formatting) and cargo fmt: clean on the three crates. Fixture YAML validated against integration-config.schema.json.

Not verified / open:
- The Zephyr Scale and Zephyr Squad endpoint shapes are ASSUMPTIONS from documentation as remembered (listed in docs/JIRA_INTEGRATION.md); the mock is built from the same assumptions, so the proof is internal consistency, not the real API.
- A cycle or execution is found among the first page of a listing only; large projects need paging.
- The shared target dir held a tcms binary from another build during the work; the acceptance runs use a binary rebuilt from this tree immediately before each run.
- One early acceptance run had connection refused and reset errors against the mock in products 1 and 3; later runs did not show them. Cause not established.

Review round 5 (reviewer, worktree agent-a42311c, detached at 08173cf; fixes e370008 and c0b0831). Crate tests run: cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver --all-features: all pass (sync_cli 15, scenario 9, statuses 4, defects 9, plain_jira_http 1). AC 1 (mutation-checked): reuse off for Zephyr Scale fails scenario zephyr_scale_a_second_sync... and sync_cli zephyr_scale_apply_stores...; reuse off for Squad fails scenario zephyr_squad_a_second_sync... and sync_cli zephyr_squad_apply_stores...; key store off (extrefs write disabled) fails apply_stores_the_entity_key_in_the_test_case_record, zephyr_scale_apply_stores..., zephyr_squad_apply_stores... and plan_without_external_refs...; acceptance products 3, 4, 5 (checks 14-33) pass. AC 2-4: scenario.rs per backend pass; statuses.rs pass. AC 4 fixture check: test-acceptance/jira_integration/fixtures jira, xray, zephyr_scale, zephyr_squad products differ only in product_id, test_backend and the Xray test_run field. AC 5 GAP FOUND: the sync never called mark_obsolete, so a removed test case kept its lifecycle (new test a_test_case_removed_from_the_product_marks_its_entity_obsolete_and_deletes_nothing failed before the fix). Fixed in e370008: the tests step writes Obsolete to the entity of each sync-state entry whose case is gone (skipped when already recorded; dry run plans it; audited; no delete). Test passes after. Limits (docs c0b0831): only entities the sync state holds are marked (no state, no mark); run records of a removed case still write their verdict on each apply (existing rule). Assumptions unchanged: Xray, Zephyr Scale and Zephyr Squad endpoint shapes are not verified against any real product; the mock is built from the same assumptions. Acceptance: 82 passed in 5 consecutive runs after the rebuild, and 5 more on the final commit e370008 (binary cksum 1315700507). One earlier run failed 13 checks because my own mutation build had left a guard-mutated tcms binary in the shared target dir (labels refused); rebuilt, not a product defect. Clippy -D warnings and cargo fmt clean on tcms.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Each backend (plain Jira, Xray, Zephyr Scale, Zephyr Squad) creates or finds its test entity, stores its key in external_refs, records runs as execution entities with per-case verdicts, writes lifecycle and verdict to the configured fields, and marks a removed case obsolete without deleting. Switching test_backend is the only config change (fixtures differ only there). Found and fixed in review: the sync never marked a removed case obsolete (e370008). Verified with mutations and the acceptance script (products 1 to 5). Assumptions: Xray, Zephyr Scale and Zephyr Squad API shapes are from public documentation as remembered and are verified against mocks only. Known limits: see docs/JIRA_INTEGRATION.md (Limits). make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2177 passed, 0 failed at the last Jira commit) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
