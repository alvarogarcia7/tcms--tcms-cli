# Phase 10 Plan 01: Shared test case discovery for Stage 1 and Stage 2 Summary

Shared discovery of `*.yml` and `*.yaml` test case files used by Stage 1 and Stage 2, with zero discovered files failing Stage 1.

## Metadata

- **Phase:** 10-acceptance-and-make-failure-propagation
- **Plan:** 01
- **Requirement:** TCMS-45 AC 1
- **Type:** tdd
- **Fix commit:** 66a1704 `fix(10-01): share test case discovery for *.yml and *.yaml; fail on zero inputs`

## What Changed

- `test-acceptance/lib/discover_test_cases.sh` (new): `discover_test_case_files DIR` prints NUL-separated, sorted, unique paths of `*.yml` and `*.yaml` under DIR. `require_test_case_files COUNT` returns 1 for zero or non-integer counts.
- `test-acceptance/run_acceptance_suite.sh`: sources the library. Stage 1 and Stage 2 both call `discover_test_case_files`. Stage 1 calls `require_test_case_files` after counting and returns 1 on zero. The return feeds `overall_success` in `main`, so the run exits non-zero.
- `test-acceptance/tests/test_stage1_discovery.sh` (new): count guard and extension tests (A1-A3, B1-B3, C1-C3, D1, E1-E2).
- `Makefile`: `test` target runs the guard test after `test-e2e`. The acceptance-folder tests were not run by `make test` before.

## TDD Evidence

- RED: before the library existed, the guard test exited 1 (`discover_test_cases.sh: missing`).
- GREEN: after the library was added, all 13 checks passed (exit 0).

## Gate Results (run in this plan)

- `bash test-acceptance/tests/test_stage1_discovery.sh`: exit 0, 13 checks pass.
- `bash -n` on `run_acceptance_suite.sh` and `discover_test_cases.sh`: pass.
- `shellcheck` on the two new files: only SC1091 info (sourced path not followed). No warnings.
- Stage 1 discovery via the same library call on the current tree: `Found 89 test case YAML files`. `require_test_case_files 89`: pass.
- `make test` dry-run wiring: the guard line appears between `test-e2e` and `verify-testcases`. Verified with `MAKE=echo` on a scratch copy without `-include .env`.
- Not run, per instructions: `make test`, `make acceptance-test`, `make coverage-e2e`, release builds.

## Criteria Status

- **Stage 1 and Stage 2 share one discovery function:** MET
- **Discovery returns every `*.yml` and `*.yaml` under `00_test_cases`, sorted, no duplicates:** MET (tests B, D, E)
- **Current tree gives 89 files and 0 `.yaml` files:** MET at the library level (tests A2, A3; Stage 1 call returns 89). The full suite log line is DEFERRED to the 10-03 checkpoint.
- **Zero discovered files makes Stage 1 fail:** MET at the function and code-path level (test C2, `return 1` in Stage 1). End-to-end run DEFERRED to 10-03.
- **Count guard fails when discovered count differs from files on disk:** MET (test A1)
- **TCMS-45 AC 1 overall:** MET for the discovery fix. The end-to-end Stage 1 run is DEFERRED to 10-03.

## Deviations

- **[Rule 3 - Blocking] Gitignored directory.** `.gitignore` line 117 (`lib/`) ignores `test-acceptance/lib/`. The library was force-added with `git add -f`. `.gitignore` was not changed because it is outside this plan's files. Follow-up: add a `!test-acceptance/lib/` exception, or the next new file in that directory will be silently ignored.
- **[Rule 3 - Environment] `make -n test` cannot parse locally.** The gitignored local `.env` (shell syntax) is loaded by `-include .env` in the Makefile and fails with `missing separator`. This is pre-existing and not caused by this change. The wiring was checked with a scratch copy and `MAKE=echo`. `.env` was not modified.

## Deferred Issues

- **Stages 3 to 7 still look up test cases by `.yml` name.** `run_acceptance_suite.sh` lines around 460, 607 and 765 use `find ... -name "${basename}.yml"`. A `.yaml` test case would pass Stages 1 and 2 but would not be found by later stages. There are no `.yaml` files on disk today, so this has no current effect. Out of scope for this plan, which covers Stage 1 and Stage 2 only. Suggested follow-up: route these lookups through `discover_test_case_files` in the same way.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- `test-acceptance/lib/discover_test_cases.sh`: FOUND
- `test-acceptance/tests/test_stage1_discovery.sh`: FOUND
- Commit `66a1704`: FOUND
