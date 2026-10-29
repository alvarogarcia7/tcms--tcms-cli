---
id: TCMS-42.2
title: Jira client with service-account auth and write policy
status: Done
assignee:
  - '@worker-c'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T16:21:00Z'
labels: []
dependencies:
  - TCMS-42.1
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One Jira client for Cloud (REST v3) and Data Center (REST v2) with token auth from auth.secret_ref (environment variable or secret store), rate-limit handling, and a write guard that refuses any write not allowed by write_policy and any field not listed in fields. Every write produces an audit-trail-entry with action sync, the acting TCMS user and the external_refs written.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Connection test command reports site, user and the projects in project_keys
- [x] #2 A write to a field not in fields, or disallowed by write_policy, is refused before any request is sent
- [x] #3 Each sync run writes audit-trail entries naming the acting user; the Jira comment body names the same user
- [x] #4 Rate limiting (429) is retried with back-off; other errors fail the sync with the issue key in the message
- [x] #5 Integration tests run against a mocked Jira for both Cloud and Data Center URL layouts
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/jira-client: client (Cloud v3 / DC v2 layouts), token auth from env, 429 retry, error with issue key, write guard before any request, audit-trail entries. 2. feature 'mock': std TcpListener mock Jira (both layouts, scripted 429, request recording). 3. tcms jira test-connection. 4. Tests incl. secret grep. 5. docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence (verified against a mock, never a real Jira): cargo test -p jira-client --all-features: mock_jira 22 passed (Cloud and DC layouts), secret_hygiene 1 passed; cargo test -p tcms: jira_cli 5 passed (real tcms binary vs mock). clippy -D warnings clean for jira-client and tcms.
D-42.2a: Cloud sends Basic auth (service account from TCMS_JIRA_USER + token); Data Center sends Bearer (PAT) unless TCMS_JIRA_USER is set.
D-42.2b: The token is read only from the env variable named by auth.secret_ref; no secret store yet (not in this task's scope).
D-42.2c: 429 is retried up to 5 times with exponential back-off (500 ms base, 30 s cap), Retry-After seconds honoured; other HTTP errors are not retried and name the issue key.
D-42.2d: The write guard is a pure config check run before any request: project outside project_keys refused (also for creates), summary and description of features never written, fields must be listed in fields, transitions must be listed in transitions, each write kind needs its write_policy switch.
D-42.2e: Audit entries are produced in memory by each client and written by write_audit_entries as <dir>/<id>.yaml; the directory and the run wrapper belong to the sync command (42.9).
D-42.2f: tcms jira test-connection exits non-zero when any project_keys project is not accessible; it prints site, deployment, version, user and projects.
D-42.2g: Acting TCMS user for test-connection is --as, else TCMS_USER, else USER.
D-42.2h: Comment body (plain text on DC, one ADF paragraph on Cloud) ends with the sync run id and the acting user, and the audit entry carries the same user.
D-42.2i: The mock is a model of Jira built from public REST documentation, not a recording of a real instance; every claim here is verified against that model only.
Not met yet: AC3 wording 'each sync run writes audit-trail entries' needs the sync command (TCMS-42.9); the entries and the naming of the actor are proven at client level only.

Review (round 2, branch tcms-review-r2): cargo test -p jira-client --all-features (mock_jira 22, secret_hygiene 1) and -p tcms (jira_cli 5) passed. Real tcms binary: test-connection against a closed port exits 1 naming the connection error, and a missing secret variable is named without echoing any value. AC1, AC2, AC4, AC5 confirmed by the mock tests (refused_writes_send_no_request_* asserts no request; rate_limit_retry asserts one 429 then success; rate_limit_exhausted asserts 3 attempts and the issue key; other_errors asserts the issue key and no retry). AC3 NOT MET, left unticked: the client yields audit-trail entries in memory and write_audit_entries writes them, and the Jira comment names the acting user, but no sync run exists yet. The 'each sync run writes audit-trail entries naming the acting user' part needs the tcms jira sync command (TCMS-42.9), which owns the run wrapper and the output directory. Stays In Review until 42.9 lands and a sync run is shown to write the entries.

Leftover wiring (TCMS-42.9, worker C3): AC 3 proven by crates/tcms/tests/sync_cli.rs: apply_writes_the_cycle_and_names_the_user_in_every_audit_entry (every audit entry of the run has who: alice) and comment_on_the_feature_names_the_acting_user (the Jira comment body contains 'by alice'). The acceptance script validate_jira_integration.sh check 5 repeats the audit check. Verified against the mock only.

Review (round 4, reviewer, branch tcms-42-review-r4). AC1: acceptance check 1 (test-connection, project PAY accessible) passes in validate_jira_integration.sh; jira_cli 5 pass. AC2: refused_writes_send_no_request_cloud/_datacenter and test-backends a_refused_write_never_reaches_the_transport pass; every JiraClient write method calls the guard before request(); backends write only through Ctx::write. AC3: sync_cli apply_writes_the_cycle_and_names_the_user_in_every_audit_entry and comment_on_the_feature_names_the_acting_user pass; manual run with the real tcms binary: every audit entry has who: carol. AC4: rate_limit_is_retried_* and other_errors_fail_with_issue_key_* pass. AC5: mock_jira 22 tests (Cloud and Data Center layouts) pass. Token: absent from state, audit files, outputs, mock logs (grep). No defect found in the client. Verified against the mock only.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Jira client: write guard before every request, audit entries with the acting user, 429 retry, Cloud and DC mock layouts. Verified by cargo test (jira-client, tcms, test-backends) and the Jira acceptance script against the mock. Mock only. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
