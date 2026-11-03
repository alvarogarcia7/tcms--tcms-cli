# Deferred items (phase 10, plan 10-02)

Out-of-scope discoveries from TCMS-46 work. Each item is outside the plan's `files_modified`
or needs a decision from the user. Nothing here was changed.

## D-1: E2E scripts that print fail() but exit 0 (blocks TCMS-46 AC 1 for these scripts)

These scripts are listed in the `test-e2e` recipe of the Makefile. They call `fail "..."`, have no
failure counter, and end with an unconditional `exit 0`. A failing check in them does not change the
exit status, so `make test` and `make coverage-e2e` stay green when they report failures.

- crates/audit-verifier/tests/integration/test_audit_key_scenarios.sh (28 fail() calls, last exit 0)
- crates/audit-verifier/tests/integration/test_audit_verifier_e2e.sh (31 fail() calls, last exit 0)
- crates/testcase-manager/tests/integration/test_verifier_e2e.sh (43 fail() calls, last exit 0)
- crates/testcase-manager/tests/integration/test_verifier_edge_cases_e2e.sh (53 fail() calls, last exit 0)
- crates/testcase-manager/tests/integration/test_verifier_container_e2e.sh (62 fail() calls, last exit 0)
- crates/testcase-manager/tests/integration/smoke_test.sh (4 fail() calls, last exit 0; not individually reviewed)

Detection: test E in tests/integration/test_run_e2e_scripts.sh (flag = calls fail "..." and last
top-level exit is `exit 0` and no failure counter gates it). Fix: add a failure counter as done in
test_documentation_generation.sh (fail() override plus a finish_run gate) in each script. These files
are outside 10-02's files_modified, so they were not edited.

## D-2: container_config.yml is stale (blocks green verifier and doc steps)

`container_config.yml` (repo root) has `schema: tcms/testcase_results_container.schema.v1.json`.
That schema file does not exist. The nearest file is schemas/tcms/test-results-container.schema.v1.json.
With the URI corrected in a scratch copy, the verifier then fails schema validation of the config
("Schema constraint violations"), so this is more than a one-line fix.

Effect: run_verifier_and_generate_reports.sh and generate_documentation_reports.sh now correctly
report the failure and exit 1 (previously they hid it). make test's affected-verifier branch
therefore fails until this config is fixed.

## D-3: check_environment.sh needs `expect`, which is not installed here

check_environment.sh reports "expect NOT FOUND" and exits 1. The bare call in test-e2e previously
ignored that status. The call is now routed through run-e2e-scripts.sh and fails the run. Install
`expect` or change check_environment.sh's requirement list.

## D-4: local .env breaks every make invocation in this checkout

`.env` (gitignored, created 2026-11-03 10:16) is a bash script (`[[ -f ... ]] && set -a && source .env`).
The Makefile has `-include .env`, which make parses as makefile syntax, so every make run stops with
`.env:1: *** missing separator`. This affects the baseline too. Not changed here. The user must
keep .env in make syntax (or move the bash snippet elsewhere) before `make test` can run.

## D-5: stale reference to a missing audit script in mk/incremental.mk

crates/audit-verifier/tests/integration/test_audit_logging_demo_e2e.sh does not exist and has no git
history. The test-e2e entry was removed from the Makefile (10-02). mk/incremental.mk line 235 still
lists it for `test-from` and was outside files_modified.

## D-6: test-from / build-from path not reviewed

mk/incremental.mk (test-from) runs the same E2E scripts but was outside files_modified. It was not
reviewed for exit-status propagation.

## D-7: run_all_samples_and_generate_reports.sh Step 3 (sample execution) fails for every sample

Root cause fixed in 10-02: Step 2 built with `cargo build --release --bin test-orchestrator` at the
workspace root. That fails ("no bin target named test-orchestrator in default-run packages") because
the bin is in a non-default member. The fix uses `-p test-orchestrator`. The pipe that hid the build
failure was also fixed. The corrected build was NOT run end to end (it is a release build and the
disk is tight). Re-run the script with a free disk to confirm.

## D-8: side effect of running run_all_samples_and_generate_reports.sh

Step 1 regenerates testcases/generated_samples/*.yml. The 8 regenerated files were reverted with
`git checkout`. Re-running the script will modify them again.

## D-9: coverage-e2e cannot run here without cargo-llvm-cov

The coverage-e2e target prints SKIPPED and exits 0 when cargo-llvm-cov is absent or SKIP_COVERAGE=1.
This is recorded as UNVERIFIED in 10-02-SUMMARY.md, not as a passed 70% gate.

## Resolution (owner decision: D-1 and D-2 are in scope)

### D-1: partly done (commit efa6790)

The six scripts now count fail() calls and exit 1 from finish_run when any check failed. No check logic
was changed. Each script was run directly with bash (environment: verifier/audit binaries in target/debug,
jq present, expect absent, not needed by these scripts):

| Script | Direct run | Open reason |
|--------|-----------|-------------|
| crates/audit-verifier/tests/integration/test_audit_key_scenarios.sh | exit 1 (2 checks) | `verify-audit-signature --verbose` prints nothing: the binary never initializes a logger (`log::info!` is dropped). Product bug in audit-verifier, outside these files. |
| crates/audit-verifier/tests/integration/test_audit_verifier_e2e.sh | exit 1 (2 checks) | Same cause as above. |
| crates/testcase-manager/tests/integration/test_verifier_e2e.sh | exit 1 (7 checks) | Its temp config fixture has no `type`/`schema` fields, which CLAUDE.md requires; the verifier rejects it and the JSON and folder checks cascade. Fixture fix not made (minimal-edit instruction). |
| crates/testcase-manager/tests/integration/test_verifier_edge_cases_e2e.sh | exit 1 (21 checks) | total_test_cases is 20, expected 15; per-case step lookups return empty. Not investigated further. |
| crates/testcase-manager/tests/integration/test_verifier_container_e2e.sh | exit 0 | Open: none. |
| crates/testcase-manager/tests/integration/smoke_test.sh | exit 0 | Open: none. |

So D-1 is done as a mechanism (all six gate on failures), but four of the six still have failing checks
and are NOT green. The gate exposed these failures, which were previously hidden.

### D-2: done (commit 8d852a8)

container_config.yml: `schema` changed to `tcms/container-config.schema.v1.json` (the existing file under
schemas/tcms/), and `type` changed from `test_results_container` to `container_config` (the const that
schema requires). Nothing else changed. validate-yaml passes, and `verifier --config container_config.yml`
in single-file mode exits 0. The verifier prints a WARN about no test run record; this is not a failure.

### Still open: D-3, D-4 (unchanged; they block `make`, not these scripts)

### Test E after Task A

tests/integration/test_run_e2e_scripts.sh: 6 passed, 0 failed. Audit reports 0 deferred out-of-scope flags.

## Resolution (phase 10 close, branch feature/phase-10-acceptance-make-propagation)

- D-4: done in commit a0309df. The Makefile reads `.env` as NAME=value lines only (D-10-1). `.env` was not edited.
- D-3: NOT resolved, user action. No sudo on this machine. Run `sudo apt-get install -y expect`. check_environment.sh is unchanged (D-10-2).
- D-7/D-9: `make coverage-e2e` and the full `make test` were not run (disk fell to 340 MB during the build). See D-10-7.
