# Phase 10 Plan 03: Acceptance suite exit contract and stages 8-9 (TCMS-45 AC 2-3) Summary

**The suite now exits 1 on any unlisted stage failure; stages 8, 9 and 10 pass on this checkout, and no known-failure entries are needed.**

## Commits

- `810a4b4` fix(10-03): conditional manual branches are echo commands (stage 2 failure)
- `29be3db` fix(10-03): exit contract, stage results, known-failure list, contract tests, acceptance build targets
- `f44a4a8` fix(10-03): summary header wording (keeps the KNOWN FAILURE count honest)
- `bbdf01f` docs(10-03): decisions D-10-4..6

## Results

| Check | Result |
|-------|--------|
| `make acceptance-test` (run 1, before header fix) | exit 0. Stage 1: 89 files. Stages 1-10: PASS. Known failures: 0 |
| `test-acceptance/tests/test_suite_exit_contract.sh` | exit 0 (cases A-E; A-C exit 1, D exit 0 with KNOWN FAILURE, E exit 0 with 10 stage lines) |
| `test-acceptance/tests/test_stage1_discovery.sh` | exit 0 |
| `tests/integration/test_run_e2e_scripts.sh` | exit 0 (6 passed) |
| Baseline `make acceptance-test` (before this plan) | exit 2 (suite exit 1): stage 2 failed on TC_MANUAL_CONDITIONAL_001 |

## Stage 8 and 9 causes (Task 1 record)

- Stage 8 (glossary): passed on this checkout (48 checks).
- Stage 9 (Jira): passed (82 checks). It needs `tcms` and `mock-jira` built with `--features mock`; `build-acceptance-binaries` did not build them, so it now does.
- No known-failure entries. `test-acceptance/known_failures.txt` has only the header.

## Criteria (TCMS-45)

- AC 1 (89 files found): MET. Stage 1 PASS, 89 files.
- AC 2 (non-zero on any unlisted failure): MET at the suite level. Contract cases A, B and C exit 1. The make wrapper's exit path is unchanged and was not exercised with a planted failure through make.
- AC 3 (stages 8 and 9 pass or are listed): MET. Both pass.

## Deviations

- **[Rule 1 - Bug] TC_MANUAL_CONDITIONAL_001 fixture.** Branch entries were operator prompts used as shell commands. Changed to quoted `echo` lines (D-10-5). Seven entries.
- **[Rule 3 - Blocking] build-acceptance-binaries** did not build `tcms` and `mock-jira`. Added.
- The `*_FAILED` consistency block was removed; the stage functions return the counter state (D-10-6).

## Open

- Stage 10 uses `target/release/test-executor`. It passed here because the release binary already exists. A clean checkout without that binary will fail stage 10 (not checked here).
- `make test` and `make coverage-e2e` were not run (D-10-7). The 10-02 checkpoint is still DEFERRED.

## Self-Check: PASSED

- The contract test, discovery test and runner test pass (see Results).
- Commits 810a4b4, 29be3db, f44a4a8, bbdf01f exist.
