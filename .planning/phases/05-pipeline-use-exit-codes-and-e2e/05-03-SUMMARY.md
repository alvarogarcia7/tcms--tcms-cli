# Phase 5 Plan 03: report exit-code E2E, Make and acceptance wiring, exit contract docs Summary

A bash E2E (144 checks, all passing) proves the exit codes of `test-executor report` against the real binary, is mapped in Make and runs as acceptance Stage 10; one planned check (`--kind summary --format json` exits 1) is not met by the shipped binary and is not tested as an error.

## Performance

- **Tasks:** 3
- **Commits:** 2 code commits plus this summary

## Accomplishments

- `test_report_exit_codes_e2e.sh`: exits 0, 1 and 2 with the real binary; byte-identical report at exit 2 for stdout and `-o`, for markdown traceability, summary, gaps, problems and traceability in asciidoc, json and yaml; error precedence over the flag; usage errors of report (1) and of other subcommands (2); closed set of codes.
- `make test-report-exit-codes` target; script in `test-from` for `test-executor` (now 7 E2E tests) and a new `testcase-reporting` case; in `test-e2e` and `test-e2e-all-no-build`.
- Acceptance Stage 10 via `validate_report_exit_codes.sh` with `--skip-report-exit-codes`; Stages 1 to 9 and the summary format unchanged.
- `docs/TRACEABILITY.md`: "Exit status and pipeline use" replaces "The three-state exit code is a later phase"; `--fail-on-gaps` in the option table.

## Task Commits

1. **Task 1 (test):** `c165d61` test(05-03): bash E2E of report exit codes, write before exit 2, error precedence [ci skip]
2. **Tasks 2 and 3 (feat, docs):** `b8e72fd` feat(05-03): report exit-code E2E in make, acceptance stage 10 and exit contract docs [ci skip]

## Files Created/Modified

- `crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh` - new E2E (executable, bash 3.2, uses logger.sh and find-binary.sh).
- `test-acceptance/validate_report_exit_codes.sh` - new Stage 10 wrapper.
- `test-acceptance/run_acceptance_suite.sh` - `SKIP_REPORT_EXIT_CODES`, `--skip-report-exit-codes`, Stage 10 block, usage and config log lines (additions only).
- `test-acceptance/README.md` - the skip flag.
- `mk/incremental.mk` - test-executor mapping (7 tests), testcase-reporting case, header comment.
- `Makefile` - `test-report-exit-codes` target, lists, help comment.
- `docs/TRACEABILITY.md` - exit status section, `--fail-on-gaps` row.

## Verification of the E2E

- `bash -n` passes; `shellcheck -x` passes on both new scripts (without `-x`, only the usual SC1091 info note).
- Direct run (`TEST_EXECUTOR_BIN=$PWD/target/debug/test-executor ./crates/.../test_report_exit_codes_e2e.sh`): 144 checks passed, 0 failed, 0 lines with the fail mark.
- Portability grep for `declare -A`, `mapfile`, `readarray`, `grep -P`, `sed -i`, `sed -r`, `readlink -f`, `stat -c`, `date -d`: 0 matches. `logger.sh` used; `cmp -s` used.
- Mutation check: a copy of the script with the flagged `--generated-at` in section C changed to `2026-01-01T00:00:00Z` exits 1 with 14 failed checks (`exit 2 bytes differ from exit 0 bytes`). The repository copy was not modified.
- Wrapper run directly: `env -u CARGO_TARGET_DIR ./test-acceptance/validate_report_exit_codes.sh` exits 0. With `TEST_EXECUTOR_BIN=/nonexistent` it exits 1 with an `[ERROR]` line.
- Temp directory removed on exit; `git status` shows only the planned files.

## Deviations from Plan

### Unmet planned check

**1. Section D expected `--kind summary --format json` to exit 1; the shipped binary exits 0.**
- **Found during:** Task 1, probing the real binary.
- **Observed:** `test-executor --no-audit report --matrix M --runs R --generated-at 2026-10-31T09:00:00Z --kind summary --format json` exits 0 (2 with `--fail-on-gaps` and a failed result). The 05-02 summary records the same behaviour as deliberate ("not a usage error").
- **Handling:** the check is not in the E2E as an error case, and the script does not assert exit 1. Section B asserts exit 0 without the flag and a leading `{`. Making this exit 1 needs a change to `crates/test-executor/src/main.rs`, which is not in this plan's `files_modified`. **User decision needed:** either change the binary (and a new plan) or drop the item from 05-03.

### Auto-fixed Issues

**2. [Rule 3 - Blocking] `testcase-reporting` case added to test-from.** The plan asked for it only if missing; it was missing, so it was added with the same script.

## Environment Notes

- `CARGO_TARGET_DIR` is set in this shell to `$HOME/.cache/cargo-target`, which holds a `test-executor` built on Oct 31, before 05-02. `find_binary` prefers that directory, so the wrapper and the E2E run without `env -u CARGO_TARGET_DIR` fail 61 checks. The make target builds into the same directory, so it is consistent. All gates below were run with `env -u CARGO_TARGET_DIR`, as in D-03-G1.
- `.planning/DECISIONS.md` was already modified before this plan; it was not staged or committed here.

## Quality Gate Record (D-06)

Each gate ran with `CARGO_HOME=$HOME/.cargo-user` and `env -u CARGO_TARGET_DIR`.

| Gate | Result | Reason / evidence |
|------|--------|-------------------|
| `cargo fmt -p testcase-reporting -p test-executor -- --check` | ran-pass | exit 0 |
| `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | ran-fail | exit 101: `crates/testcase-execution/src/executor.rs:2435` "this `if` has identical blocks" (pre-existing, not in this plan) |
| `cargo clippy ... --all-targets --no-deps -- -D warnings` | ran-pass | exit 0 for the two crates |
| `cargo test -p testcase-reporting -p test-executor --no-fail-fast` | ran-pass | exit 0; every `test result` is `ok`, no `FAILED` |
| `make build` | ran-pass | exit 0 |
| `make lint` | ran-fail | exit 2: same pre-existing `executor.rs:2435` clippy error (clippy with dependencies) |
| `make test` | not-run | excluded by the run instructions (tcms-45 open) |
| `make test-report-exit-codes` | ran-pass | exit 0; 144 checks passed |
| `make coverage-e2e` | not-run | excluded by the run instructions (tcms-46 open) |
| `make acceptance-test` | not-run | excluded by the run instructions (tcms-45 open); Stage 10 wrapper run on its own: pass |

Backlog.md was not updated (no backlog task changes requested; no backlog file edited).

## Issues Encountered

- The planned `make lint` and the planned clippy gate fail on the same pre-existing `testcase-execution` lint; `--no-deps` passes for the two crates in scope.
- `make test-from` and `make test` were not run; `make list-affected-crates BASE_REF=HEAD` reports no affected crates because the only changes are untracked or outside `crates/`. The `test-executor` mapping was checked with `make -n test-report-exit-codes`, `make -n test-e2e-all-no-build` and by grep.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED (one planned check unmet, see Deviations)

- Created files exist: `crates/testcase-manager/tests/integration/test_report_exit_codes_e2e.sh`, `test-acceptance/validate_report_exit_codes.sh`.
- Commits exist: `c165d61`, `b8e72fd`.
- Requirement CLI-02 is not marked complete here (no requirement update was requested).
