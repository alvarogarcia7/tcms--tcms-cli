---
phase: 05-pipeline-use-exit-codes-and-e2e
verified: 2026-11-02T00:00:00Z
status: passed
score: 3/3 success criteria verified; 13/13 phase and backlog acceptance criteria verified, 2 NOT VERIFIED (make test, not run by scope)
overrides_applied: 0
re_verification: No
---

# Phase 5: Pipeline Use: Exit Codes and E2E Verification Report

**Phase Goal:** A user can run `test-executor report` in a script or CI job and rely on its exit status, with the report always written first.
**Verified:** 2026-11-02, branch `dev`, working tree clean at start.
**Status:** passed (no BLOCKER, no WARNING gaps; see NOT VERIFIED list)

## Goal Achievement

### ROADMAP Success Criteria

| # | Criterion | Status | Evidence (file:line or test) |
|---|-----------|--------|------------------------------|
| SC1 | Exit 0 on success, 1 on input or validation error, 2 with `--fail-on-gaps` when gaps or failed results exist | VERIFIED | `crates/test-executor/src/main.rs:1705-1707` is the only `exit(2)` on the report path, reached only when `report()` returns `breached`. `main.rs:860-861` computes `breached = fail_on_gaps && !findings.is_clean()`. `main.rs:940-951` `parse_cli` exits 1 for report clap errors. Tests in `crates/test-executor/tests/report_exit_codes_cli.rs`: `clean_exits_0_with_and_without_flag` (202), `gaps_exit_0_without_flag_and_2_with_flag` (216), `failed_result_exits_2_with_flag` (230), `error_result_exits_2_with_flag` (244), `errors_win_over_the_flag` (414), `usage_errors_exit_1` (526). E2E sections A, B, D, E, G pass. |
| SC2 | Report file is written before any non-zero exit caused by gaps or failures | VERIFIED | `main.rs:855-870` writes the Markdown/format output (stdout or `-o` file, flushed in `write_stdout` at 967-973) inside `report()`. Exit 2 happens only after it returns (`main.rs:1705-1707`). `finish_status` (927-938) prints only to stderr. Tests: `report_written_before_exit_2_stdout` (270), `report_written_before_exit_2_file` (300), `every_kind_and_format_writes_then_exits_2` (322). E2E section C compares exit-0 and exit-2 output with `cmp -s`. |
| SC3 | Bash 3.2 and BSD/GNU safe E2E exercises exit codes, mapped in `mk/incremental.mk` and the acceptance suite | VERIFIED | Script: `crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh` (executable; sources `find-binary.sh` and `logger.sh`, lines 36-37). Mapped in `mk/incremental.mk:215` and `:219`, Make target and E2E lists in `Makefile:314`, `:355`, `:446`. Acceptance Stage 10 in `test-acceptance/run_acceptance_suite.sh:160` and `:1174-1178`, wrapper `test-acceptance/validate_report_exit_codes.sh`. Portability grep for `declare -A`, `mapfile`, `readarray`, `grep -P`, `sed -i`, `sed -r`, `readlink -f`, `stat -c`, `date -d` returns 0 hits. |

**Score:** 3/3 success criteria verified.

### Plan 05-01 (findings)

| Item | Status | Evidence |
|------|--------|----------|
| `Findings`, `is_clean`, `collect_findings` | VERIFIED | `crates/testcase-reporting/src/findings.rs:20`, `:30`, `:44-54` (same `build_gaps` and `build_problems` views on one `ViewInput`; gap entries = sum of four lists, failed results = problem rows) |
| Test cases (13 tests) | VERIFIED | `findings.rs` tests: `nothing_wrong_is_clean` (130), `requirement_without_tests_counts_three_gap_entries` (162), four `gap_list_*_alone_makes_findings_not_clean` (190-218), `decided_fail_and_error_count_as_failed_results` (224), `fail_superseded_by_later_pass_is_clean` (243), `only_not_executed_record_is_a_no_result_gap_not_a_failure` (286), `fail_of_unlinked_test_case_still_counts` (297), `shuffled_inputs_give_equal_findings` (318), `summary_text_is_fixed` (360), `collect_findings_agrees_with_rendered_reports` (407) |

### Backlog Acceptance Criteria

| Task | Criterion | Status | Evidence |
|------|-----------|--------|----------|
| tcms-43.5 | #1 Exit 0/1/2 as in SC1 | VERIFIED | See SC1 |
| tcms-43.5 | #2 Report written before non-zero exit | VERIFIED | See SC2 |
| tcms-43.5 | #3 E2E covers exit codes, mapped in mk/incremental.mk and acceptance suite | VERIFIED | See SC3 |
| tcms-43.5.1 | #1 Exit codes match the three states | VERIFIED | `main.rs:183-187` doc, `main.rs:1705-1707`, tests above |
| tcms-43.5.1 | #2 Report written before non-zero exit | VERIFIED | See SC2 |
| tcms-43.5.2 | #1 E2E script covers exit codes and passes | VERIFIED | Direct run: 144 checks passed, 0 failed, exit 0 (see Behavioral Spot-Checks) |
| tcms-43.5.2 | #2 Mapped in `mk/incremental.mk` | VERIFIED | `mk/incremental.mk:215`, `:219` |
| tcms-43.5.2 | #3 Acceptance suite runs it | VERIFIED (wiring) | `run_acceptance_suite.sh:1174-1178`; wrapper run alone exits 0. Full `make acceptance-test` NOT RUN (scope, tcms-45). |
| tcms-43.5.3 | #1 Real binary, exact output lines and exit codes | VERIFIED | E2E checks exit codes, byte equality of exit-0 and exit-2 output (`cmp -s`), and first lines (`# Traceability Report`, `= Traceability Report`, leading `{`) |
| tcms-43.5.3 | #2 Bash 3.2 and BSD/GNU safe, mapped in `mk/incremental.mk` | VERIFIED (static) | Portability grep 0 hits; mapping at `mk/incremental.mk:215`. Not run under a real bash 3.2 binary. |
| tcms-43.5.3 | #3 Results recorded in phase SUMMARY | VERIFIED | `05-03-SUMMARY.md` records 144 checks and the gate table |
| tcms-43.5 | DoD #1 "All tests passing, run make test" | NOT VERIFIED | `make test` not run per instruction (tcms-45 open). `cargo test` below passes. |

### Requirement CLI-02

Implementation evidence satisfies CLI-02 (SC1 and SC2). `.planning/REQUIREMENTS.md:55` is correct, but the traceability row at `:123` still reads `Pending`. Documentation only; left for the orchestrator.

### Decisions

- D-05-1 (clap parse errors exit 1, `--shape` conflict exit 2): implemented at `main.rs:805` (`clap::Error::raw(...).exit()`), `main.rs:940-951`. Verified by `shape_conflict_exits_2_with_or_without_the_flag` (378) and `usage_errors_exit_1` (526).
- D-05-2 (`--kind summary --format json` exits 0): code exits 0 and the E2E asserts it (script line 286-291). Plan 05-03 criterion removed in commit `c871b3d`. The 05-03 SUMMARY "User decision needed" note is stale; resolved by D-05-2 option (a). Not a gap.

## Automated Checks Run

| Check | Command | Result |
|-------|---------|--------|
| Unit and integration tests | `env -u CARGO_TARGET_DIR CARGO_HOME=$HOME/.cargo-user cargo test -p testcase-reporting -p test-executor --no-fail-fast` | Exit 0. All `test result` lines `ok`, 0 failed. test-executor includes `report_exit_codes_cli` 16 passed. |
| Format | `env -u CARGO_TARGET_DIR cargo fmt --check -p testcase-reporting -p test-executor` | Exit 0 |
| Clippy (known failure) | `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | Exit 101. Pre-existing: `crates/testcase-execution/src/executor.rs:2435` "this `if` has identical blocks". Reported, not fixed. |
| E2E exit codes | `env -u CARGO_TARGET_DIR bash crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh` | Exit 0. **144 checks passed, 0 failed.** |
| Acceptance stage 10 wrapper | `env -u CARGO_TARGET_DIR bash test-acceptance/validate_report_exit_codes.sh` | Exit 0 |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Report exit codes 0/1/2 with real binary | E2E script above | 144 pass, 0 fail | PASS |

Note: the shell in this environment exports `CARGO_TARGET_DIR=/home/orca/.cache/cargo-target`. Running the E2E without `env -u CARGO_TARGET_DIR` uses a stale Oct 31 binary and fails 60 checks. The script works only with the variable unset or with `TEST_EXECUTOR_BIN` set to the freshly built binary.

## Not Verified / Not Run

| Item | Reason |
|------|--------|
| `make test`, DoD of tcms-43.5 | Excluded by run instructions (tcms-45 open) |
| `make acceptance-test` (full suite) | Excluded (tcms-45 open). Stage 10 wrapper verified alone. |
| `make coverage-e2e` (70% threshold) | Excluded (tcms-46 open) |
| `make lint` / full clippy with deps | Fails on pre-existing `testcase-execution` `executor.rs:2435`; out of phase scope, not fixed per instruction |
| bash 3.2 execution | Only static checks; no bash 3.2 binary run |

## Human Verification Required

None.

## Gaps Summary

No gaps block the phase goal. All three ROADMAP success criteria and all phase-specific backlog acceptance criteria are verified by code, tests, and the real-binary E2E. The unrun make gates and the pre-existing clippy failure are recorded above and are not caused by this phase.

---

_Verified: 2026-11-02_
_Verifier: Claude (gsd-verifier)_
