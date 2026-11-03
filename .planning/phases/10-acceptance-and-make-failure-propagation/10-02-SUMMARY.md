# Phase 10 Plan 02: Make test-step failures fail the make run (TCMS-46) Summary

**Runner for E2E scripts and exit-status fixes in the verifier and report scripts; the failing mechanism is fixed, but the green-tree criteria are NOT MET on this checkout (see Criteria).**

## Status

- Commits: `373eb2d` (runner, Makefile wiring, tests), `9d11579` (report and verifier script exit status)
- `make test` and `make coverage-e2e` were NOT run by the executor (per instructions); their exit codes are DEFERRED to the orchestrator checkpoint, and the expected result is non-zero (see Checkpoint).
- Nothing pushed.

## Criteria (TCMS-46 acceptance criteria and plan must_haves)

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| AC1 | A failing E2E script or failing verifier step makes make test and make coverage-e2e exit non-zero | NOT MET (partial) | Mechanism MET: runner exits 1 on failure (tests B, planted failure); verifier steps end in `|| exit 1` (test D1). NOT MET for 6 listed scripts that print fail() and exit 0 (deferred item D-1). |
| AC2 | "Verification output not generated" warning is fixed or fails the run | MET (fails the run; cause recorded) | Reproduced in generate_documentation_reports.sh and run_all_samples_and_generate_reports.sh. Both now exit 1 with the real cause. Cause recorded below. Not fixed at cause (cause is out of scope, D-2). |
| AC3 | Existing passing runs stay green | NOT MET on this checkout | make test is expected red: D-2 (stale container_config.yml), D-3 (expect missing), D-4 (.env breaks make). These failures were hidden before this plan. coverage-e2e UNVERIFIED (see below). |
| MH-1 | make test/coverage-e2e exit non-zero when a listed E2E script exits non-zero | MET by test, not by make run | test B, planted failure exit 1. make not run. |
| MH-2 | ...when test-verifier-edge-cases or run_verifier_and_generate_reports.sh fails in the affected branch | MET by static test; run_verifier direct run exit 1 | test D1; run_verifier after fix exit 1 with 7 failed steps. |
| MH-3 | A listed E2E script that does not exist fails the run | MET | test C. Stale entry test_audit_logging_demo_e2e.sh removed (never existed in git history). |
| MH-4 | Every E2E script reached from test or coverage-e2e that calls fail() exits non-zero when it has failures | NOT MET | D-1: 6 out-of-scope scripts (outside files_modified, not edited). In-scope test_documentation_generation.sh now MET (test E). |
| MH-5 | Warning cannot appear on a green run, or fails the run; cause recorded | MET | See warning table. |
| MH-6 | A green tree still runs make test and make coverage-e2e to exit 0 | NOT MET / UNVERIFIED | Not run. Expected red for D-2, D-3, D-4. |

### Coverage-e2e (UNVERIFIED)

`coverage-e2e` was NOT run. Its SKIPPED branches (Makefile lines 483-487: SKIP_COVERAGE=1, or cargo-llvm-cov absent) exit 0 with "SKIPPED". If the orchestrator sees that, the 70% gate was NOT checked. It must not be recorded as a passed gate.

## Warning sites (TCMS-46 AC 2)

| Site | Reproduced | Exit before fix | Exit after fix | Cause |
|------|-----------|-----------------|----------------|-------|
| scripts/generate_documentation_reports.sh:233 | Yes (direct run) | 1 | 1 | Verifier aborts on the stale `container_config.yml` schema URI (D-2). The `if eval ... \| while` pipeline reported the loop's status, so "Verifier completed successfully" printed for a failed verifier. Now the verifier status is captured and the verifier's error is printed. |
| crates/testcase-manager/tests/integration/test_documentation_generation.sh:152 | No (direct run exit 0, before and after) | 0 | 0 | Not reproduced. In a make run the warning already fails via `exit 1` in this loop. The coverage-log occurrence cannot be confirmed without the log. |
| scripts/run_all_samples_and_generate_reports.sh:272 | Yes (direct run, before step-2 fix) | 1 | 1 (not re-run) | Step 2 `cargo build --release --bin test-orchestrator` ran at workspace root and failed (bin is in a non-default member). The pipe hid the failure, so the script said "built". Step 3 then got exit 127 for every sample, no execution logs were produced, the verifier failed on "No execution log files", and the warning followed. Fix: `-p test-orchestrator` and captured status. Not re-run end to end (see Deviations). |

## Changes

- `scripts/run-e2e-scripts.sh` (new): runs each argv script with bash from repo root, prints pass/fail per script, treats a missing script as a failure, exit 1 if any failed.
- `Makefile` test-e2e: all E2E scripts go through the runner; the affected branch's `test-verifier-edge-cases` and `run_verifier_and_generate_reports.sh` end in `|| exit 1`; check_environment.sh is no longer a bare call; stale audit entry removed. `validate-output-schemas` stays last.
- `tests/integration/test_run_e2e_scripts.sh` (new): tests A to E. Fixtures in `tests/integration/fixtures/e2e-runner/`.
- `scripts/generate_documentation_reports.sh`, `scripts/run_all_samples_and_generate_reports.sh`: verifier and build status captured directly; failures exit 1.
- `scripts/run_verifier_and_generate_reports.sh`: `set -e`-safe capture of the verifier status; failed scenarios, missing reports and doc-generation failures counted and exit 1 (the script previously ended with an unconditional `exit 0`).
- `crates/testcase-manager/tests/integration/test_documentation_generation.sh`: fail() counts failures; script exits 1 when any check failed (it previously printed "All tests passed!" regardless).

## Verification run by the executor

| Command | Result |
|---------|--------|
| `bash tests/integration/test_run_e2e_scripts.sh` before runner exists | Exit 1: A, B, C, D1, D2, E failed (RED as intended; E also showed missing listed script) |
| `bash tests/integration/test_run_e2e_scripts.sh` after changes | Exit 0: 6 passed, 0 failed |
| `./scripts/run-e2e-scripts.sh tests/integration/fixtures/e2e-runner/fail_script.sh` | Exit 1 |
| same with pass_script.sh | Exit 0 |
| same with a missing path | Exit 1 |
| `bash crates/testcase-manager/tests/integration/test_documentation_generation.sh` | Exit 0, 0 "✗" lines |
| `bash scripts/generate_documentation_reports.sh` | Exit 1 with true cause |
| `bash scripts/run_verifier_and_generate_reports.sh` | Exit 1, 7 steps failed, 1 warning (true cause) |
| `bash scripts/run_all_samples_and_generate_reports.sh` (before step-2 fix) | Exit 1, warning, exit 127 for all samples |
| `make -n test-e2e` | Not usable: `.env` include breaks make (D-4). Parse check on a scratch copy with the include removed: make ran check_environment.sh, which failed on missing `expect` (D-3). |

Static audit results (test E): 28 listed scripts; 1 in-scope flagged (fixed); 6 out-of-scope flagged (D-1).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] run_verifier_and_generate_reports.sh and generate_documentation_reports.sh reported success on failure**
- **Found during:** Task 3
- **Issue:** Verifier status came from a `| tail` or `| while` pipeline; failed scenarios were skipped with `continue`; the script ended with `exit 0`.
- **Fix:** Captured status; counted failures; exit 1 on any failure. Under `set -e` the capture uses `|| VERIFIER_EXIT=$?`.
- **Commit:** 9d11579

**2. [Rule 1 - Bug] run_all_samples_and_generate_reports.sh build failure hidden; wrong cargo package**
- **Found during:** Task 3 (warning investigation)
- **Fix:** Captured the build status; `cargo build --release -p test-orchestrator --bin test-orchestrator`.
- **Commit:** 9d11579

**3. [Rule 1 - Bug] Listed audit script does not exist**
- **Issue:** test-e2e entry for `test_audit_logging_demo_e2e.sh` pointed at a file that never existed and was silently skipped.
- **Fix:** Removed the Makefile entry. `mk/incremental.mk` still lists it (D-5).
- **Commit:** 373eb2d

**4. [Rule 2 - Missing critical] test_documentation_generation.sh printed "All tests passed!" with failing checks**
- **Fix:** fail() counter and finish_run gate on both exit paths.
- **Commit:** 9d11579

### Scope deviations (not auto-fixed)

**5. Scope boundary: six listed scripts still exit 0 after fail()**
- The plan's Task 3 Step 3 asked to add `exit 1` to every audit-listed script that calls fail(). Six of those scripts are outside `files_modified` (see D-1), and the executor was instructed to edit only `files_modified`. Not edited. Recorded as deferred.

**6. Test E audit definition changed during Task 1**
- The plan's test E (`grep -q 'exit 1'` anywhere in the file) passed trivially, so it could not detect the defect. Replaced with a flag rule: calls `fail "`, last top-level exit is `exit 0`, and no failure counter gates it. Files in `files_modified` fail the test; out-of-scope flagged files are printed as DEFERRED. This narrows the hard failure to in-scope files. It does not weaken any pre-existing test.

**7. Run-verifier fixes are not covered by test E**
- run_verifier_and_generate_reports.sh and generate_documentation_reports.sh call no `fail "`, so test E does not check them. They were verified by direct runs only.

### Process notes

- A stray `cargo build --release --bin test-orchestrator` at workspace root was run once by mistake. It failed at resolution before compiling. No release artifacts were produced.
- Running run_all_samples_and_generate_reports.sh regenerated 8 files under testcases/generated_samples/. They were reverted with `git checkout` (D-8).
- Investigation logs are in `reports/tcms-46/` (gitignored; not committed).
- The release-build step in run_all_samples was not re-run end to end (release builds are not allowed on this disk).

## Checkpoint (plan Task 4) for the orchestrator: DEFERRED

Run on a checkout where `.env` is valid make syntax, with `expect` installed and container_config.yml fixed (D-2, D-3, D-4):

1. `make test > reports/tcms-46/make_test.log 2>&1; echo $?`
2. `make coverage-e2e > reports/tcms-46/coverage_e2e.log 2>&1; echo $?` (a SKIPPED line means UNVERIFIED)
3. `grep -c "Verification output not generated" reports/tcms-46/make_test.log reports/tcms-46/coverage_e2e.log` (expected 0)

On the current checkout, step 1 is expected to be non-zero, and make cannot run at all until `.env` is fixed.

## Open items

- D-1 to D-9 in `.planning/phases/10-acceptance-and-make-failure-propagation/deferred-items.md`.
- O-1 from 10-CONTEXT.md (coverage-e2e SKIPPED behaviour) is unchanged and still UNVERIFIED.

## Self-Check

Files (scripts/run-e2e-scripts.sh, tests/integration/test_run_e2e_scripts.sh, both fixtures, deferred-items.md, this SUMMARY) and commits 373eb2d and 9d11579 exist.

## Self-Check: PASSED
