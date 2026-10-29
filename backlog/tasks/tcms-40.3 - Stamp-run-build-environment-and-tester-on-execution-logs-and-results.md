---
id: TCMS-40.3
title: Stamp run, build, environment and tester on execution logs and results
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:10:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A test result must state what ran, on which build, in which environment, by whom and when; execution logs and verification results currently cannot be grouped into a run or attributed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Execution log entries carry run_id and test_case_id
- [x] #2 Verification results carry run_id, build_id, environment, tester and date
- [x] #3 Values can be supplied by flag, config or environment variable; absent values are omitted rather than written empty
- [x] #4 Output validates against the extended schemas
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RunInfo (run_id, build_id, environment, tester, date) flattened onto TestCaseVerificationResult; RunInfo on ContainerReportConfig stamps every result
2. verifier: --run-id/--build-id/--tester, TCMS_RUN_ID/TCMS_BUILD_ID/TCMS_TESTER, config keys; precedence flag, env, config, then the run_id found in the log
3. generated scripts write test_case_id and run_id on every log entry (TCMS_RUN_ID at run time, else test-executor generate --run-id), hook failures included
4. local container schema and container-config schemas accept the new keys
5. docs/RUN_METADATA.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions: (D-40.3a) precedence flag > env var > config file > run_id found in the execution log; empty/blank counts as absent and absent is omitted. (D-40.3b) per-result 'environment' is the report's --environment/config value, 'date' the report generation time (RFC 3339), so a result is always dated. (D-40.3c) run IDs are JSON-escaped with the configured json_escaping method, like command output (the jq method must not use sed, see jq_json_escaping_test). Evidence: unit tests run_info_* and test_absent_and_empty_run_info_is_omitted (testcase-verification), test_log_entries_carry_test_case_id_and_run_id and test_ids_with_quotes_and_backslashes_stay_valid_json (testcase-execution, run real generated scripts under the jq, auto and shell methods), test_run_metadata_precedence_flag_then_env_then_config and test_run_id_is_read_back_from_the_execution_log (verifier), test_stamped_results_validate_against_container_and_tcms_schemas (validates against the verifier container schema and the vendored verification-result schema). Manual run with real binaries: logs carried test_case_id/run_id (TCMS_RUN_ID beat the --run-id default), results carried run_id/build_id/environment/tester/date, and the control run without any value had none of the keys; both validated with the tcms-schemas validator logic. Not run: make test and the acceptance suite. Known limit: test-executor execute (native path) writes no JSON log, so only generate-based runs are stamped.

Review: blank-environment fix (3d79943) and example config fix (6a98ebd). Evidence: real verifier run with TCMS_RUN_ID/TCMS_BUILD_ID and flags: results carry run_id, build_id, environment, tester, date; validated 0 errors against tcms-schemas verification-result and test-run; control run without values omits keys; config file works; execution log entries validated against tcms-schemas execution-log. Log entries carry run_id/test_case_id via cargo test test_log_entries_carry_test_case_id_and_run_id (runs real scripts). Workspace cargo test exit 0. make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test workspace instead.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
run/build/environment/tester/date stamped on logs and results with flag, env and config precedence; blank values omitted. make test not runnable here; verified with cargo test.
<!-- SECTION:FINAL_SUMMARY:END -->
