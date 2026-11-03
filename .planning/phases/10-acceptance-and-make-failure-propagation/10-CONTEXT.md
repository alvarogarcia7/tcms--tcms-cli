# Phase 10 context: acceptance suite discovery and make failure propagation

Source of truth: the acceptance criteria of TCMS-45 and TCMS-46 in `backlog/tasks/`. This phase adds
no scope beyond them. Branch: `dev`. Nothing is committed by the planning step.

## Goal

1. TCMS-45: `make acceptance-test` finds every test case file (89 in `test-acceptance/00_test_cases`),
   fails when the suite has no inputs, exits non-zero on any stage failure, and stages 8 (glossary) and
   9 (Jira) either pass or are recorded as known failures with a backlog reference.
2. TCMS-46: `make test` and `make coverage-e2e` exit non-zero when an E2E step reports a failure, and
   the "Verification output not generated" warning either fails the run or is fixed. Green runs stay
   green.

## Plans

| Plan | Backlog | Wave | Depends on | Summary |
|------|---------|------|-----------|---------|
| 10-01 | TCMS-45 AC 1 | 1 | - | One discovery function for both `.yml` and `.yaml`, used by Stage 1 and Stage 2; zero inputs is a failure; test-first count guard |
| 10-02 | TCMS-46 AC 1-3 | 1 | - | Make recipes stop on failing steps; audit scripts that print failures but exit 0; investigate and resolve the verification-output warning; test-first with a planted failing step |
| 10-03 | TCMS-45 AC 2-3 | 2 | 10-01, 10-02 | Acceptance suite exit contract; stages 8 and 9 fixed or recorded as known failures with a backlog id |

Wave 2 because 10-03 touches `run_acceptance_suite.sh` (also edited by 10-01) and the `acceptance-test`
recipe in `Makefile` (also edited by 10-02). The same-wave file-overlap rule forces it to wave 2.

## Findings that shape the plans (verified on branch `dev`)

- TCMS-45 description says Stage 1 searches `*.yaml`. On `dev`, HEAD already searches `*.yml` only
  (`test-acceptance/run_acceptance_suite.sh` lines 298 and 348; commit 428fb36 changed it). On `main`
  the `*.yaml` search is still there (line 285). So the zero-count bug is fixed on `dev` for the
  extension, but two gaps remain: a `.yaml` file would be silently ignored, and an empty discovery
  is still a passing stage. Both are addressed in 10-01.
- Stage 1 and Stage 2 each run their own `find`. They must share one discovery function so a file
  that validates is also generated.
- `Makefile` `acceptance-test` already captures the suite exit status through a pipe
  (`{ ...; echo $$? > "$$STATUS_FILE"; } | tee`). Its exit code is therefore correct for any non-zero
  status the suite returns. The gap is in the suite: zero inputs and empty stages return success, and
  Stage 1 through Stage 7 counters are the only failure signal.
- `run_acceptance_suite.sh` does not use `set -e`, so `((x++))` from zero is safe.
- Stages 8 and 9 run `validate_glossary_entities.sh` and `validate_jira_integration.sh`. Stage 9 needs
  `cargo build -p tcms -p jira-client --features mock` (mock-jira). `tcms`, `test-executor` and
  `test-plan-documentation-generator` were not on PATH when this context was gathered, so the failure
  cause of stages 8 and 9 is unknown until the suite is run. 10-03 records the cause first.
- TCMS-46 root cause candidate (to confirm in 10-02): in `Makefile`, `test-e2e` (affected-crate branch)
  runs `${MAKE} test-verifier-edge-cases;` and `./scripts/run_verifier_and_generate_reports.sh;`
  separated by `;` inside one `if`. Only the last command's status reaches make, and that last command
  is `${MAKE} validate-output-schemas`. A failure in the verifier steps is therefore swallowed. The
  no-changes branch has the same shape for `check_environment.sh`. The per-script loop
  (`bash "$$test_script" || exit 1`) does propagate, but it silently skips a listed script that is
  missing (`if [ -f ... ]`).
- Scripts called by the loop report with `fail()` from `scripts/lib/logger.sh`. That helper only prints
  and does not set an exit status, so each script must `exit 1` itself. 56 scripts in the integration,
  scripts and audit-verifier folders call `fail "`. The audit is part of 10-02.
- `coverage-e2e` prints "SKIPPED" and exits 0 when `SKIP_COVERAGE=1` or when `cargo llvm-cov` is
  missing. That is a skipped gate, not a failing test step. It is out of TCMS-46 scope and is listed as
  an open item below.

## Decisions (locked for this phase)

- D-45-1: Discovery matches both `*.yml` and `*.yaml` (lower-risk option named in TCMS-45 AC 1). No file
  renames. One shared function, used by Stage 1 and Stage 2.
- D-45-2: Zero discovered files is a stage failure, not a pass. A count guard test fails if the count
  of files on disk and the count the stage processes differ.
- D-46-1: A failing step must make the make target exit non-zero. Fix the recipe separators (`;` to
  `|| exit 1` or equivalent) and make each E2E script exit non-zero when it reports a failure. No
  grep-of-output heuristic as the primary mechanism.
- D-46-2: The "Verification output not generated" warning is fixed at its cause if the cause is a
  script bug; if the warning is a real missing output, the run fails. Either way the run must not
  print the warning and exit 0.
- D-46-3: Existing green runs stay green. No test is weakened or removed to make the run pass.
- D-45-3: Stages 8 and 9 are fixed where the cause is local to tcms-cli. A stage that cannot be fixed in
  this phase is recorded in `test-acceptance/known_failures.txt` with a backlog task id. The suite
  prints it as KNOWN FAILURE. Any failure not listed exits non-zero. Every listed id must exist in
  `backlog/tasks/`, and the suite checks this.

## Open items (decide at checkpoints, do not block planning)

- O-1: Should `coverage-e2e` fail rather than print SKIPPED when `cargo llvm-cov` is missing? Default in
  this phase: unchanged (not a test-step failure). Revisit if the user wants the 70% gate enforced.
- O-2: If stages 8 or 9 need a backlog task that does not exist yet, the plan creates it through the
  backlog MCP tools, with the user's confirmation.

## Out of scope

- Renaming the 89 `.yml` files.
- Changing test-case schemas or the verifier's output format.
- Stage 10 (report exit codes) beyond keeping it green.
- Any change to `tcms-web` or Verdict.
