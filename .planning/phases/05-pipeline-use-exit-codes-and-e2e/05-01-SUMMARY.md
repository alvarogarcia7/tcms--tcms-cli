# Phase 5 Plan 01: Findings from gaps and problems views Summary

One library rule, `collect_findings`, gives the gap-entry and failed-result counts from the same `build_gaps` and `build_problems` views the reports print, so the exit status for `--fail-on-gaps` (plan 05-02) cannot disagree with the report (D-01, CLI-02 groundwork).

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (red) | 394b703 | test(05-01): failing tests for findings from gaps and problems views |
| 1 (green) | 1269aa3 | feat(05-01): collect findings from the gaps and problems views |
| 2 | 172b343 | feat(05-01): test that findings match the rendered gaps and problems reports [ci skip] |

The two commits from Task 1 were made without the `[ci skip]` suffix in their subject line. They are local and unpushed; the history was not rewritten.

## Red run (Task 1, before the implementation)

`collect_findings` was a stub returning zeros. Six of nine tests failed on the non-clean cases (`requirement_without_tests_counts_three_gap_entries`, `each_gap_list_alone_makes_findings_not_clean`, `decided_fail_and_error_count_as_failed_results`, `only_not_executed_record_is_a_no_result_gap_not_a_failure`, `fail_of_unlinked_test_case_still_counts`, `shuffled_inputs_give_equal_findings`). The clean, superseded and summary-text tests passed against the stub, as expected.

## Gate results

| Gate | Result |
|------|--------|
| `env -u CARGO_TARGET_DIR cargo fmt -p testcase-reporting -p test-executor -- --check` | exit 0 |
| `env -u CARGO_TARGET_DIR cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | exit 101: pre-existing `if` with identical blocks at `crates/testcase-execution/src/executor.rs:2351`, a dependency this plan does not touch |
| `... cargo clippy -p testcase-reporting -p test-executor --all-targets --no-deps -- -D warnings` | exit 0 |
| `env -u CARGO_TARGET_DIR cargo test -p testcase-reporting -p test-executor` | exit 0: 415 passed, 0 failed (10 of them in `findings`) |

`make test`, `make acceptance-test` and `make coverage-e2e` were not run, per instruction.

## Criteria

- Findings has `gap_entries` (sum of the four gap lists), `failed_results` (problem rows), `is_clean()` and `summary()` returning `N gap entries, M failed results`. Tested.
- Clean fixture gives 0 and 0. One requirement without tests gives exactly 3 gap entries (it sits in three lists). Each of the four gap lists alone makes the result not clean. A decided fail or error gives 1 failed result.
- A fail superseded by a later pass is clean. A not-executed-only case is one no-result gap, not a failure. A fail of an unlinked test case still counts.
- Shuffled inputs give equal findings.
- Table test `collect_findings_agrees_with_rendered_reports` parses the four gap headings and the `Failed or errored results: N` line from the rendered Markdown and asserts equality with `collect_findings` for clean, gap only, fail only, error only and gap plus fail (expected pairs pinned as literals: (0,0), (3,0), (0,1), (0,1), (3,1)).
- No kind or format parameter exists; no clock, file access or HashMap in the module.
- Only `findings.rs` and `lib.rs` changed.

## Deviations

- Renamed the agreement test from `findings_agree_with_rendered_reports` to `collect_findings_agrees_with_rendered_reports` to match the plan's grep criterion.
- Factored the scenario tuple into a `Scenario` type alias to satisfy clippy `type_complexity`.
- The problems count line is required to be present in the rendered text, not treated as absent-means-zero; the renderer always prints it.

## Self-Check: PASSED

Created file `crates/testcase-reporting/src/findings.rs` present; commits 394b703, 1269aa3, 172b343 present in `git log`.

CLI-02 is not marked complete; plan 05-02 wires the exit status.
