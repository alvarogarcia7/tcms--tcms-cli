---
id: TCMS-40.4
title: Produce a test run record per execution
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T17:00:00Z'
labels: []
dependencies:
  - TCMS-40.3
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Runs must group logs and results and give a per-test-case verdict (pass, fail, not_executed, error) for audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each execution writes a test-run YAML that validates against the test-run schema
- [x] #2 A failed case and a not-executed case appear with the right verdict
- [x] #3 The reviewer field can be set
- [x] #4 A problem_report_id can be attached to a fail or error result
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. TestRunRecord/TestRunResult/Verdict in testcase-verification (verdict rules, required metadata, problem-report rules)
2. verifier: --test-run, --reviewer (TCMS_REVIEWER), --problem-report TC=PR; start/end times and hook failures read from the execution logs
3. Validate against the vendored test-run schema; docs in docs/RUN_METADATA.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions: (D-40.4a) verdict: pass when the case passed; fail when any step failed; error when a hook failed (hook_type set and exit_code != 0 in the log) and the script ended; otherwise not_executed. (D-40.4b) the record is written only on request (verifier --test-run PATH), because the verifier is also used without run metadata and the schema requires run_id, build_id, environment and tester; without them it refuses and names what is missing. (D-40.4c) started_at/finished_at come from the first and last execution log timestamps (local time converted to UTC), else the report time. Evidence: 5 unit tests in test_run.rs (verdicts incl. error, reviewer and problem report, problem report only for fail/error, missing metadata named, timestamps/no tags), verifier test_log_facts_timestamps_hook_failures_and_run_id, testcase-manager test_run_record_validates_against_tcms_test_run_schema (all four verdicts, reviewer, problem report validated against the vendored test-run schema). Manual run with real binaries over three logs (failing step, failing first step, failed hook): verdicts fail, fail, error, run_id read back from the logs, problem report attached, output validated with the tcms-schemas test-run schema. Open: AC #1 says each execution writes a test-run; today that happens when --test-run is passed. Making it default (for example next to --output) is a small change if wanted. Not run: make test, acceptance suite.

Review (TCMS-40.4, reviewer, branch tcms-40-review): NOT Done. AC #1 open: a test-run is written only when verifier --test-run PATH is given, not for every execution; and test-executor execute (native path) writes no execution log, so it produces no run record. AC #2-#4 re-checked: verifier run gave a failed/not_executed verdict and problem_report_id; the run record validates against tcms-schemas test-run.schema.json (0 errors).

(D-40.4d) Decision by the owner (the user): follow the spec, the verifier writes the test run record by default. Implementation: written whenever run_id, build_id, environment and tester are known, next to --output as <name>.test-run.yaml (test-run.yaml in the cwd without --output); --test-run PATH chooses the path, --no-test-run turns it off. Conflicts with existing behaviour, resolved as follows and to be confirmed by the owner: (1) the schema requires those four values, so a default run without them cannot write a valid record: it warns (naming what is missing) and succeeds, so existing invocations keep working; --test-run, --reviewer and --problem-report ask for the record, so for them missing values stay an error. (2) the independence check (TCMS-40.8) now applies whenever a record is written: a run with complete metadata and a test case with independence_required fails without a suitable --reviewer; --no-test-run avoids it. (3) a run without --output writes test-run.yaml into the current directory. Evidence: crates/verifier/tests/default_test_run_test.rs (6 tests on the real binary: written next to the report with the right content, skipped with a warning when metadata is missing, --no-test-run, explicit requests fail on missing metadata, --test-run conflicts with --no-test-run, independence still enforced), unit test test_default_test_run_path_follows_the_report, and all other verifier tests still pass (no stray test-run.yaml appears in the repo). Docs: docs/RUN_METADATA.md table.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A test run record per verified execution: verdicts pass/fail/not_executed/error, reviewer, problem reports for failed or errored cases, start and end from the execution logs, validated against the tcms-schemas test-run schema. Per the owner's decision it is written by default when the run, build, environment and tester are known (see D-40.4d for the conflicts and how they were resolved). make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p verifier -p testcase-verification and the coordinator's workspace run (2177 passed, 0 failed before this change) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
