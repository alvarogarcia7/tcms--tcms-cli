---
id: TCMS-41
title: 'Verifier: add test cases, edge cases'
status: Done
assignee:
  - '@worker-b'
created_date: 2026-03-21 06:55
updated_date: '2026-10-28T15:33:00Z'
labels:
  - verifier
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cover the verifier binary and the testcase-verification library with edge-case tests that run the real binary: malformed and unusual execution logs, test case lookup problems, match strategies (including an invalid regex), source hash checking, exit codes, folder and single-file mode, output formats and schema validity, config file and run metadata error paths. Every defect these tests expose is fixed (smallest change) or recorded as a gap with an ignored test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log edge cases (empty array, truncated/invalid JSON, duplicate, out-of-order, missing, extra and foreign-test-case entries, hook-only log) each have a test asserting the verdict and exit code
- [x] #2 Output edge cases (very long output, unicode and special characters, values that look like dates/yes/0755) round-trip as strings in the YAML and JSON result and the result validates against both schemas
- [x] #3 Test case problems (unknown id, missing or invalid file, zero sequences or steps, manual steps) have tests with an explicit exit code and error message, or a non-silent verdict
- [x] #4 Every match strategy (exact, regex, invalid regex, contains, precomputed) and the invalid strategy name have a pass and a fail test
- [x] #5 --verify-source-hash match and mismatch, --success-on-completion and exit codes 0/1/other are tested in single-file and folder mode
- [x] #6 Folder mode (empty folder, nested folders, non-log files, two logs for one test case) and single-file mode argument errors are tested
- [x] #7 Config file errors and run metadata edge cases (blank values, env vars, --test-run and --problem-report error paths) are tested
- [x] #8 Each defect found is fixed with a regression test kept, or recorded in the task notes as a gap with an #[ignore] test
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Probe verifier behaviour for each edge case with a new tests/edge_cases_test.rs (real binary, tempdir).
2. Assert current correct behaviour; for defects fix minimally in verifier/testcase-verification.
3. Run cargo test/fmt/clippy for both crates, record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Not started. The task has no description or acceptance criteria. Needs a scope (which verifier edge cases) before work: for context, testcases/verifier_scenarios/edge_cases holds 40 scenarios and crates/verifier/tests/integration_test.rs the current coverage.

Worker B (continuation of the stopped worker). Tests: crates/verifier/tests/edge_cases_test.rs, 62 tests: 61 run and passing, 1 #[ignore] gap. Each runs the real binary (CARGO_BIN_EXE_verifier) on handwritten YAML and JSON in a tempdir.

Decisions:
- D-41a: scope is the verifier binary plus testcase-verification as the binary uses it. No new unit tests in testcase-verification.
- D-41b: a duplicated step keeps the LAST entry (HashMap insert in verify_test_case). A failed attempt followed by a passing retry therefore verifies as a pass. Not changed here: deciding whether retries count is a product question. Recorded as the ignored test a_failed_attempt_followed_by_a_passing_retry_must_not_pass (fails with exit 0 when run with --ignored). Identical duplicate entries are one step (test a_duplicated_step_with_identical_entries_is_one_step).
- D-41c: an empty folder is an error: exit 1, "No execution log files ... found", no report written. Before it exited 0 with an empty report, which reads as a pass in CI. Behaviour change for any script that runs folder mode on an empty folder.
- D-41d: the verifier does not special-case manual steps (schema field manual: true). A manual step needs its own log entry; without one it is NotExecuted and the case fails. Pinned by two tests. Open question for the owner: test-executor skips manual steps by default, so cases with manual steps fail unless the manual steps are logged.
- D-41e: a case errored by a failed hook (hook_type entry with non-zero exit) fails the run exit code (exit 1) unless --success-on-completion, as a failed verdict does. Before, the --test-run record said verdict error while the process exited 0 and the container reported the case as passed.

Defects found and fixed (source changes in crates/verifier/src/main.rs and crates/testcase-verification/src/verification.rs):
1. Log entries naming another test case were counted as evidence for this one. Now ignored with a warning. Test: entries_that_name_another_test_case_are_not_evidence_for_this_one.
2. Per-entry source_yaml_sha256 from the log was dropped (None), so --verify-source-hash refused every log that had hashes. Fixed by reading the hash per entry. Test: verify_source_hash_accepts_a_log_made_from_the_same_test_case.
3. A test case with zero steps got overall_pass true. Now false. Test: a_test_case_without_sequences_or_steps_is_not_a_silent_pass.
4. The report was written with log::info!, so it went to stderr at the default log level and stdout was empty. Now written to stdout (or --output). Tests: the_report_goes_to_stdout_and_stays_there_whatever_the_log_level, json_report_on_stdout_is_parseable.
5. Logs that could not be verified (unknown test case, unparsable, folder-mode hash mismatch) were skipped while the run exited 0 and the report silently lacked them. Now exit 1, even with --success-on-completion. Tests: skipped_logs_stay_an_error_with_success_on_completion, a_log_whose_test_case_is_unknown..., an_unparsable_log..., a_hash_mismatch_in_folder_mode...
6. D-41c and D-41e as above.

Verification run (shared target, after touching sources; see note below):
- cargo test -p verifier --test edge_cases_test: 61 passed, 1 ignored.
- cargo test -p verifier -p testcase-verification: testcase_verification unit 70 passed; verifier unit 3 passed; edge_cases_test 61 passed / 1 ignored; independence_test 4 passed; integration_test 3 passed / 4 ignored (the 4 ignores already exist in HEAD).
- cargo fmt --check -p verifier -p testcase-verification: clean.
- cargo clippy -p verifier -p testcase-verification --all-targets -- -D warnings -A clippy::useless_borrows_in_formatting: clean.

Not run: make test / make lint / make coverage-e2e / make acceptance-test (workspace-wide, disk budget). testcase-manager tests that run the verifier (verifier_cli_test, verification_test, report_generation_e2e_test) were not run; they were only read, and they accept both exit codes for an empty folder. Whoever runs make test should check them against D-41c and the stdout change.

Build note: cargo twice reused a stale verifier binary in the shared target (edge tests failed on fixes that were in the tree). touch on the changed sources and a rebuild cleared it. Rerun after touching when results look inconsistent.

Format note: cargo fmt -p testcase-verification ran over the whole crate; it may have reformatted worker A's uncommitted lib.rs (whitespace only). Not committed by this task.

AC evidence: #1-#8 are covered by the tests named in the notes above and their run results. DoD #1 (make test) is not ticked: only the two crates were run, as the shared-tree rules require.

Review (round 2, branch tcms-review-r2): cargo test -p verifier -p testcase-verification -p test-executor passed: verifier edge_cases_test 61 passed and 1 ignored (the retry gap), verifier unit 4, testcase-verification 70, independence_test 4, integration_test 3 passed and 4 ignored (already in HEAD). Real binary probes: an execution log whose test case cannot be found exits 1 even with --success-on-completion and the report is still written (unknown test case = not silently skipped); the eight hook scenario logs under testcases/verifier_scenarios/hooks exit 1 with a failed verdict or errored case, as their names say; the report goes to stdout. Acceptance scripts: the folder-mode consolidated runs (run_acceptance_suite.sh stage 7 and validate_stage7_consolidated_docs.sh) ran --folder without --test-case-dir, so they looked in repo testcases/ and would now exit 1; fixed in commit ed6fcba by passing 00_test_cases. The single-file --success-on-completion runs and run_precomputed_tests.sh (single-file, exit codes only) are not affected by the changes in this task. Retry gap (D-41b, ignored test a_failed_attempt_followed_by_a_passing_retry_must_not_pass): AC8 allows an ignored test for an uncovered defect, so it is accepted here, but it is a verdict-level gap: a failed attempt followed by a passing retry of the same step verifies as a pass (last entry wins). Recorded for a product decision on retries. Behaviour changes for the owner: empty folder exit 1, skipped logs exit 1, report on stdout, errored hooks fail the run, manual steps without a log entry fail. testcase-manager verifier_cli_test and verification_test (changed in eff5436) are in the coordinator's workspace run, not re-run here.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Edge-case tests for the verifier binary and testcase-verification (62 tests on the real binary, 61 passing and one ignored gap for retries), with five defects fixed: entries of another test case no longer count, per-entry source hashes kept, zero-step test case not a pass, report on stdout, unverifiable logs and empty folders and errored hooks fail the exit code. Round 2 review re-ran the crate tests and probed the real binary, and fixed the acceptance-script folder-mode calls that would have broken on the new exit codes (commit ed6fcba). DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test verifier, testcase-verification and test-executor and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
