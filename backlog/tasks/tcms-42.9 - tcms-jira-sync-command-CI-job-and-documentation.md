---
id: TCMS-42.9
title: tcms jira sync command, CI job and documentation
status: Done
assignee:
  - '@worker-c3'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T17:05:00Z'
labels: []
dependencies:
  - TCMS-42.3
  - TCMS-42.5
  - TCMS-42.6
  - TCMS-42.7
  - TCMS-42.8
parent_task_id: TCMS-42
priority: medium
type: docs
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One command runs import of requirements and all write-backs for a product, usable by people and by the CI job that follows a committed run. Documentation explains the mapping, the configuration, the roll-up rules and the resolver.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tcms jira sync --product <id> runs requirement import, test status, feature status, milestone and defect sync in that order and summarises writes
- [x] #2 A --dry-run prints the plan of writes; --only <step> limits the scope
- [x] #3 A GitLab CI job template runs the sync after a results merge
- [x] #4 Docs cover integration-config with the four example configs, the roll-up rules and tcms ref
- [x] #5 Acceptance suite covers a full cycle: import features, run, sync, verify Jira mock state for plain Jira and Xray
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. tcms jira sync: steps import, tests, features, milestones, defects in order; --only; dry run by default, --apply writes. 2. State (state.yaml in --state-dir) and per-run audit files with the acting user. 3. Tests against the jira-client mock through the binary; secret hygiene on outputs. 4. CI job template, docs, standalone mock-jira binary, validate_jira_integration.sh wired as stage 9. 5. Leftover wiring (a)-(e) with tests; tick only what is proven.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (TCMS-42.9, worker C3):
(D-42.9a) Dry run is the default; --apply writes. --dry-run is the explicit form and conflicts with --apply. A dry run writes no file and sends no write; reads still run so the plan is real.
(D-42.9b) State is state.yaml in --state-dir (default .tcms/jira-sync): test case entity keys and the values last written (hand-edit detection), plan Fix Version ids. Not written into test case or requirement records (a YAML rewrite of a test case could lose content); problem-report keys ARE in the report external_refs (schema field, written by tcms pr).
(D-42.9c) Audit: one audit-trail-entry per changed record per run in <state-dir>/audit/<run-id>/, written by the sync step (not per HTTP request). who = acting user. Comments go through jira-client, whose audit entries are merged.
(D-42.9d) Acting user: --as, else TCMS_USER, else USER. It goes into every audit entry and into every comment body (jira-client) and into the defect description.
(D-42.9e) One summary comment per feature whose roll-up was written, only when write_policy.comments is on.
(D-42.9f) Milestones: plan progress over non-obsolete cases from the run records; features = every requirement of the matrix; release date --release-date or today.
(D-42.9g) state.yaml names the local type jira_sync_state; there is no vendored schema for it, so it is not validated (limit). Audit entries and the problem report are validated by hand against the vendored schemas.
(D-42.9h) Test entity lookup, latest verdicts and the roll-up are fed from the trace matrix built by testcase_verification::build_matrix; validation status is read from requirement records that have it.
(D-42.9i) audit entity_type 'test-case' (the schema enum); an earlier 'test-case-main' failed schema validation and was fixed.
(D-42.9j) Mock (owned crate) gained: a default 'To Do' status for created issues; a GET fallback message with method and segments. Standalone binary mock-jira (crates/jira-client, feature mock) for the acceptance script.
(D-42.9k) CI: template .tcms-jira-sync and job tcms:jira-sync (default branch on success; manual when TCMS_PRODUCT is set). The state is not pushed back by the job (limit).
ASSUMPTIONS: Jira REST shapes as in 42.2-42.8 (verified against the mock only). Xray and Zephyr shapes are not exercised by this task.
Evidence: cargo test -p tcms: sync_cli 6 pass (dry run writes nothing; apply writes the cycle; every audit entry names the user; second apply finds the same entities and creates nothing; defect step; comment names the user; product mismatch and unknown step refused), pr_cli 5 pass, validation-read unit test pass. cargo test -p test-backends --all-features and -p jira-client --all-features: all pass. Clippy -D warnings clean on tcms, test-backends, jira-client. test-acceptance/validate_jira_integration.sh: 24 of 24 checks pass; with JIRA_ACCEPTANCE_EXPECT_WHO=bob it fails (exit 1). Audit files and problem report validated against tcms-schemas.
NOT run: the CI job (cannot run GitLab CI here). The full acceptance suite (make acceptance-test) cannot run here: test-plan-documentation-generator is not installed (stage 6). Xray is not covered by the acceptance script.

Acceptance full cycle (worker C4, item 3). test-acceptance/validate_jira_integration.sh now runs three products: 1 plain Jira (mock 1; checks 1-8: import, dry run, apply, audit names the actor, mock holds lifecycle and verdict 'passed', record holds the key in external_refs (4a), second sync creates nothing, defect pushed once, token absent); 2 StrictDoc (9-10); 3 Xray (mock 2; checks 11-18: import, dry run, apply records the Xray key in the test case record (system xray), the mock holds one Test Execution for the run found by JQL search with PASS for the passed case, the Test issue holds lifecycle and verdict, a second sync still has one Test Execution, audit names the actor, token absent). Run: ./test-acceptance/validate_jira_integration.sh gives 46 passed, 0 failed, exit 0. Mutations: JIRA_ACCEPTANCE_EXPECT_WHO=bob gives exit 1 (checks 5 and 18 fail); changing the Xray passed status name in backends.rs to PASSED gives exit 1 (check 15 fails; the other 45 pass). Source restored after each.
Assumptions, not verified against a real product: the Xray REST shapes (raven testexec/{key}/test, testrun/status, the Test Execution issue type) are from public Xray documentation as remembered, and the mock models them (jira-client mock, routes under /rest/raven/1.0). 'run' is the run record fixture (the verifier's output form); the script does not execute the test scripts. Verified against the mock only.
Decision (D-42.4m): Xray record_run reuses the Test Execution whose summary is the run id (JQL search in the project); before this every sync created a new execution. Regression test: test-backends tests/scenario.rs xray_a_second_sync_of_the_same_run_reuses_its_test_execution (mutation checked: fails without the reuse). Zephyr Scale and Zephyr Squad have the same per-sync creation of a cycle/execution (record_run); not changed here: open.

GitLab CI job template (worker C4, item 4): template statically validated; not executed by GitLab here. crates/tcms/tests/ci_template.rs parses .gitlab-ci.yml and asserts: the job tcms:jira-sync extends .tcms-jira-sync and is in a declared stage (e2e); its first rule is a default-branch pipeline ( == $CI_DEFAULT_BRANCH, after a results merge) with when on_success, and every when value is a GitLab value; the script builds tcms, runs jira sync with --product, --config, --state-dir from CI variables; no secret variable is set in the job and no literal token appears; the artifact path lies under the state dir; docs/JIRA_INTEGRATION.md names the same variables and modes. 7 tests pass; mutation (script always --apply) fails the mode test.
Defect fixed in the job (D-42.4n): the script passed --apply unconditionally, so the manual rule ran a write from any branch although the comment and the docs said dry run. Now MODE=--apply only when CI_COMMIT_BRANCH equals CI_DEFAULT_BRANCH, otherwise --dry-run. Records written by the job stay in the job workspace (no commit back); the next run finds entities by tcms_id, so no duplicates (documented).

Item 6 (worker C4): the flaky 'Unknown mock route GET /issue/PAY-101' was not reproduced. Loop: cargo test -p tcms -p test-backends -q, 30 iterations, all passed (88 s for the first 10 with a warm build; no failure). Static review: every GET path the clients send has a mock route (jira-client/src/mock.rs route table); the route arm ('GET', [issue, key]) cannot return 'Unknown mock route' for a path with two segments, so the message needs a path with a third segment (for example a remote link or a comment read), which no current client sends. The mock serves one connection at a time on its own port; each test starts its own mock, no static or env state is shared in these crates' tests (grep: no set_var outside a separate test binary). Mock fixes made on the way, both in jira-client mock: JQL matching is exact (before, 'Test' matched 'Test Execution', which made a sync see a Test issue as an execution) and Fix Version and Xray routes were added. If the message recurs, its full path identifies the route to add.

Review (round 4, reviewer, branch tcms-42-review-r4). AC1: manual apply with the real tcms binary against mock-jira prints the steps in order import, tests, features, milestones, defects and a summary of applied writes. AC2: dry run (default) sends only the import GET, writes no file and no state (checked with a logging proxy and md5 snapshots); --only tests is used by acceptance check 3. AC3: crates/tcms/tests/ci_template.rs (7 pass) and my reading of .gitlab-ci.yml: apply only on the default branch, dry run elsewhere, token from masked CI variables (no literal in the job), artifacts under the state dir. Not run on GitLab. AC4: docs/JIRA_INTEGRATION.md has the four examples (examples/integration-config), roll-up rules and tcms ref; ci_template and id-resolver every_example_config tests pass. AC5: test-acceptance/validate_jira_integration.sh: 46 passed; with JIRA_ACCEPTANCE_EXPECT_WHO=bob checks 5 and 18 fail. Idempotence: second apply from the same workspace sends no POST, rewrites the same values (4 PUTs), state.yaml and records unchanged, one new audit directory. Corrupt state: exit 1 before any request. Token: absent from state, audit, outputs and mock logs.
GAP (open, product decision): defects are not idempotent across CI runs. A fresh checkout (the CI job does not commit records back) creates a new defect for each open problem report on every default-branch run: reproduced on the mock (PAY-102 then PAY-103 for PR-007). Fix options: commit the report records back, or find defects by a label set at creation (needs a mock JQL clause). docs/JIRA_INTEGRATION.md Limits now says so (commit 5be21ce). Until fixed, keep write_policy.defects off in the CI configuration.
Minor, not fixed: the CI rule runs on any default-branch pipeline, not only a results-merge pipeline (the 'after a results merge' condition is by pipeline order, not checked). Dry run does not read test entities, so its plan says find or create. Verified against the mock only.

Defect idempotence (worker C6, item 1; reviewer's gap).
(D-42.9a) Every defect is created with the label tcms-pr-<problem report id> (labels: [tcms, tcms-pr-PR-007]). Before creating, push searches the first project for that label (JQL project = "P" AND labels = "tcms-pr-PR-007", a read, not guarded). One match: PushOutcome::Found, nothing created or linked; the sync and tcms pr push record the key (sync records and audits 'defect found by label' only on --apply). The label is a field write on create: 'labels' is already in the guard's CREATE_SYSTEM_FIELDS, so no config change was needed. The report id must be letters, digits, '-' and '_' (refused before any request otherwise).
(D-42.9b) More than one issue with the label is refused, naming the keys; nothing is guessed or written.
(D-42.9c) The search runs in dry runs too (reads are not writes), so the plan says 'existing PAY-n' or 'would create'. The external_refs key stays the fast path: a report that names a key is never searched.
Mock (jira-client, owned): JQL labels clause matched exactly (an issue must carry the exact label).
Evidence: cargo test -p tcms --test sync_cli: 14 pass; new tests a_fresh_checkout_finds_the_existing_defect_by_label_and_creates_nothing (no state, no key in the report, mock holds PAY-500 with the label: zero writes sent, key recorded), two_applies_from_fresh_checkouts_create_one_defect (the reproduction: two fresh workspaces from the same fixtures, one POST /issue, PAY-102 absent), two_labelled_defects_are_refused_and_nothing_is_written. Mutation (find_by_label disabled, Vec::new()): 3 failed; restored, 14 pass. test-backends tests/defects.rs label_search_is_exact_and_a_found_defect_is_not_created_again (tcms-pr-PR-1 does not match tcms-pr-PR-10 or PR-007): pass. Full: cargo test -p tcms -p test-backends -p jira-client --all-features all pass; clippy -D warnings clean on the three crates; validate_jira_integration.sh 82 passed, 0 failed. Verified against the mock only.

Status update (worker C6, item 2): the Zephyr Scale and Zephyr Squad cycle/execution reuse noted above as open is DONE: commit f227788 (TCMS-42.4, D-42.4o..t). record_run reuses the cycle named by the run and updates the execution of a case already in it; regression tests per backend (mutation checked); acceptance products 4 and 5 (checks 19-33). docs/JIRA_INTEGRATION.md (test backends table) already describes the reuse.
Docs (worker C6, item 2): the defect idempotence is described in the Defects section (label lookup, Found, two matches refused, dry-run plan, non-label ids refused); the temporary warning to keep write_policy.defects off in CI is removed; Limits now says what remains: defects created before the label existed (add the label by hand or record the key with tcms pr link), removed label or moved issue gives a second defect. Also corrected: Jira searches follow every page (search_issues); Zephyr listings are one page.

CI rule (worker C6, item 3). (D-42.9d) No precise 'results merge' condition exists: the repo has no results branch naming or results marker, GitLab has no predefined variable for a results merge, and the other jobs in .gitlab-ci.yml key only on tags (release) and CI_MERGE_REQUEST_IID. The job keeps the default-branch pipeline as its condition (it runs on every default-branch pipeline) and applies only there (the mode rule is unchanged, D-42.4n). Consequence documented: a default-branch pipeline without result changes re-sends the same roll-ups (writes are not diffed); nothing is created twice. Changed: .gitlab-ci.yml comment, crates/tcms/tests/ci_template.rs (test renamed the_job_runs_on_default_branch_pipelines_and_applies_only_there; asserts the same rule), docs/JIRA_INTEGRATION.md CI section. cargo test -p tcms --test ci_template: 7 pass; clippy clean. Not run on GitLab.

Intermittent mock failures (worker C6, item 4). Not reproduced. Runs: test-acceptance/validate_jira_integration.sh 25 times in a row, 82 passed, 0 failed each (exit 0 each; logs in the session scratchpad). cargo test -p tcms -p test-backends -p jira-client 20 times in a row: 20 of 20 exit 0, no FAILED or panicked line. No mock process left after a run.
Code review (no fixed ports anywhere): every mock binds 127.0.0.1:0 (jira-client mock.rs MockJira::start and the standalone mock-jira); the script starts one mock per product, each on its own port, and waits for the MOCK_JIRA_URL line, which is printed only after the listener is bound (no port race, no fixed sleep). The script's WORK_DIR is mktemp -d and its EXIT trap kills only the mocks it started. The mock serves one connection at a time with a 5 s read timeout and sends Connection: close, so no keep-alive reuse; the requests in the script are sequential. The only set_var is in jira-client tests/secret_hygiene.rs, a single test. 
Why the message cannot come from the current code: the route GET /issue/{key} exists (mock.rs route(), ('GET', ['issue', key])), and the Unknown route message is formatted with the Debug form of the segments (['issue', 'PAY-101']), never 'GET /issue/PAY-101'. A binary of another source version can produce a different text. The shared CARGO_TARGET_DIR is used by the session's worktrees (.claude/worktrees/agent-*), and their mock sources differ (two have the plain 'Unknown mock route' text). So the most likely cause of the report is a binary in the shared target directory replaced by a concurrent build during a run (or a stale binary), not a race in the mock.
Fix (hardening, not a proven root cause): validate_jira_integration.sh now runs private copies of tcms and mock-jira (WORK_DIR/bin), so a rebuild of the shared target directory cannot change a binary mid-run, and logs their cksum ('binaries under test (copies)'). Run after the change: 82 passed, 0 failed; loop of 25 (1 run after the edit plus the 25 above) not repeated after the edit. 
Mistake recorded, not a product bug: a loop run without CARGO_TARGET_DIR in the environment fails at once with 'Required binary not found' (the script finds binaries only where find_binary looks); the loop was rerun with the environment set.
Not done: a thread per connection in the mock (no evidence it is needed; a silent client would delay the others by up to 5 s).

Correction to the item 4 note above: the 25-run loop was before the script edit. After the edit (committed script, e333616) the loop ran again: 25 of 25 'Jira integration acceptance: 82 passed, 0 failed', exit 0 each, no mock process left. The crate-test loop (20 runs) ran after the edit's first build as recorded.

Review round 5 (reviewer, worktree agent-a42311c, detached at 08173cf; no defect in this task's code beyond the 42.4 obsolete fix e370008). Crate tests: all pass (see TCMS-42.4 notes for the list; ci_template 7, sync_cli 15, defects 9). DEFECT GAP (round 4) re-checked by hand with the real tcms binary and mock-jira: two applies from two fresh workspaces (committed fixtures only, no state, no external_refs on PR-007): the first created PAY-102; the second printed 'existing PAY-102 (found by label; nothing created)'; PAY-103 and PAY-104 return HTTP 404 on the mock; the report records PAY-102. Mutations: label search returns the first match when two exist -> sync_cli two_labelled_defects_are_refused_and_nothing_is_written fails (refusal is needed); labels dropped from the create allow-list -> 4 sync_cli tests and 2 jira-client tests fail (the label write goes through the guard). A known-token run (reviewer-secret-7f3a9c) left 0 hits in 32 files of the workspace (records, state, audit, mock output). AC 1: sync runs import, tests, features, milestones, defects in that order and prints the write summary (acceptance and the hand run). AC 2: dry run is the default; it prints 'dry run: nothing written' and writes no state (acceptance check 3); --only is used by checks 3 and 11 and others. AC 3 JUDGEMENT (kept ticked): the job tcms:jira-sync extends the hidden template .tcms-jira-sync and its first rule runs it on default-branch pipelines. A results merge to the default branch starts such a pipeline, so the job runs after a results merge; the AC does not say it runs only then (D-42.9d: the repo and GitLab have no marker that identifies a results merge). The job applies only on the default branch, otherwise a dry run; the token is named through auth.secret_ref and set as a masked CI/CD variable (masking is a project setting that the repo cannot check); ci_template asserts no literal secret in the job and no secret variable in it. Not run on GitLab. AC 4: docs/JIRA_INTEGRATION.md names the four examples in examples/integration-config (jira_cloud, jira_datacenter, strictdoc_html, strictdoc_server), the roll-up rules and tcms ref. AC 5: validate_jira_integration.sh 82 passed, 0 failed in 5 consecutive runs after the rebuild and 5 more on the final commit e370008; Jira (plain) and Xray products checked against the mock state. Intermittent failures: not reproduced in these 10 runs; the one failed run was my own stale binary. Remaining gaps (documented in Limits): writes not diffed (an unchanged roll-up is re-sent on each apply); state not pushed back by the CI job; defects made before the label existed need a manual label or tcms pr link.

Correction to the note above: --only is used by acceptance check 3 (plain Jira dry run) and by the Xray products (--only tests, and --only tests,features,defects); the exact list of checks was not re-counted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tcms jira sync runs the steps in order (import, tests, features, milestones, defects) and summarises writes; dry run by default with --only; a CI job template runs the sync on default-branch pipelines and applies only there (judged to meet 'after a results merge', as no results-merge marker exists); the docs cover the four example configs, roll-up rules and tcms ref; acceptance covers a full cycle for plain Jira and Xray (and StrictDoc and Zephyr products). Defects are found by label before any create, so a fresh CI checkout does not duplicate them (reproduced with the real binary). Verified against the mock only; the CI job is not run on GitLab; Xray endpoints are assumed. make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2177 passed, 0 failed at the last Jira commit) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
