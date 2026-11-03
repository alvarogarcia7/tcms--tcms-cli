# Phase 9 Plan 01: tcms vv combine (TCMS-47.3) Summary

`tcms vv combine --root <DATA_ROOT> --run <RUN_ID>` combines the per-case results of one run into
`runs/<RUN_ID>/test-run.yaml` and `verification-results.yaml`, which `tcms vv metrics` reads unchanged.

## Commits

| Commit | Scope |
|--------|-------|
| `e34cc07` | `test(09-01)`: 7 failing binary tests (RED) and the 63-file fixture |
| `7a00bbc` | `feat(09-01)`: combinator, `vv combine` subcommand, docs (GREEN) |

Not pushed. `.planning/STATE.md` and `ROADMAP.md` were not updated: the orchestrator's SDK state
handlers were not run, and this run was limited to the files listed in the plan.

## Red and green evidence

- RED: before the subcommand existed, all 7 tests failed with `error: unrecognized subcommand 'combine'`.
- GREEN: `cargo test -p tcms --test vv_combine_cli` 7 passed; `cargo test -p tcms --bin tcms vvcombine` 7 passed.

## Gates (run with `env -u CARGO_TARGET_DIR`)

| Gate | Result |
|------|--------|
| `cargo fmt -p tcms -- --check` | exit 0 |
| `cargo clippy --no-deps -p tcms -- -D warnings` | exit 0 |
| `cargo test -p tcms` | exit 0: 71 unit (incl. 7 vvcombine), 7 vv_combine_cli, 19 vv_metrics_cli, all other suites pass |
| `make test`, `make lint`, `make build` | DEFERRED: not run, per instruction for this run. CLAUDE.md requires them before commit |
| `make acceptance-test`, `make coverage-e2e` | DEFERRED: not run, per instruction |

## Acceptance criteria (TCMS-47.3)

| # | Criterion | Status | Evidence / reason |
|---|-----------|--------|-------------------|
| 1 | One `test-run.yaml`, one results entry per test_case_id, validated against the test-run schema | MET | Validated with `parse_and_validate_yaml_string` against `schemas/tcms-schemas/tcms/test-run.schema.json` before writing. Test `combine_reads_63_case_files_into_one_record` |
| 2 | One `verification-results.yaml` keyed by test_case_id, validated against its schema | NOT MET | No schema for this file exists in the repo (open item Q3). Only the fields `vv metrics` reads are checked (`check_steps`). The file's `schema:` header points at the test-results-container schema, but that schema was not applied |
| 3 | Duplicates: latest started_at wins; not_executed never overrides executed | MET | Unit tests `latest_started_executed_attempt_wins`, `not_executed_never_overrides_executed`; binary tests `duplicate_case_latest_started_at_wins`, `not_executed_never_overrides_executed` |
| 4 | Verdict mapping: overall_pass true=pass, false=fail, missing run=not_executed | NOT MET (literal "missing run" clause) | pass and fail are MET. "Missing run" is not implemented. A case with no per-case file is left out of the record, as the plan requires. not_executed means a case with steps and none executed, a per-case file that says so. The script has no missing-run marker, so the literal clause cannot be met from its output |
| 5 | Equivalent to the legacy plain-folder single-run model (ORCH-012.02) | DEFERRED | The legacy Java reader is not in this repository, so equivalence was not checked. The output has the same one-folder-per-run layout |
| 6 | A test reads the 63-file layout and checks combined counts against the per-case inputs | MET | Fixture `crates/tcms/tests/fixtures/vv-combine/` (63 files): 62 results, 49 pass, 10 fail, 3 not_executed. Caveat: the fixture is derived from the script's output contract. No recorded 63-file run exists in the repo (open item Q4) |

Plan must-haves (`truths`):

| Truth | Status |
|-------|--------|
| One `test-run.yaml` with one results entry per test_case_id, read from the per-case files of the run | MET |
| One `verification-results.yaml` keyed by test_case_id with total_steps and passed_steps | MET |
| Duplicates: latest started_at wins; not_executed never overrides executed | MET |
| Verdict mapping: overall_pass true=pass, false=fail, missing run=not_executed | NOT MET (same as AC 4) |
| Combined record read by `tcms vv metrics` without changes to the metrics code | MET. `vvmetrics.rs` is unchanged; `vv metrics` on the fixture gives passed 49 |
| Both outputs validated before written; failing validation writes nothing | MET for test-run (schema). verification-results is checked by fields only (AC 2). Tests `invalid_record_writes_nothing` and `unreadable_case_file_writes_nothing_and_names_the_file` |

## Decisions and deviations from the plan

- **Checkpoint resolved without stopping (plan Task 2/3, Q5):** the command name `tcms vv combine` is the
  planner's option-a. Option-a was implemented, not paused for review. Please confirm.
- **Timestamps come from the execution logs.** The per-case files written by the conversion script carry
  no `started_at` or `finished_at`. The combinator reads the `timestamp` entries of
  `runs/<RUN_ID>/execution_logs/<KEY>_execution_log.json`. The backlog names `10_test_results/execution_logs`
  as the source; this layout was not used.
- **Metadata flags added:** `--plan`, `--build-id`, `--environment`, `--tester`. The plan's interface listed
  only `--root` and `--run`. The test-run schema requires `build_id`, `environment` and `tester`, and the
  per-case files do not carry them. A missing one makes validation fail, and nothing is written.
- **`--now` not added:** `combine` writes no time stamp of its own, so the plan's `--now` would have no effect.
- **`finished_at` is always written** when `started_at` is. Any run with timestamps has a finish time, so the
  plan's "provisional" branch is not reachable.
- **Duplicate file keys:** the file key is the file name without `_result.yaml`. The execution log of an
  attempt is found by that key, so `TC_FIX_010__a1` and `TC_FIX_010__a2` have separate logs. The
  conversion script itself never writes two files for one case, so duplicates arise only from
  hand-merged or retried runs.
- **Fixture placement:** the execution logs sit in `fixtures/vv-combine/per-case/execution_logs/` (inside a
  listed path). The test copies them to `runs/<RUN_ID>/execution_logs/`.
- **Extra test:** `unreadable_case_file_writes_nothing_and_names_the_file` (7 binary tests, plan listed 6).
- **Extra unit tests** cover the rules and the empty-run and unsafe-run-id refusals. The plan's
  "finished_at omitted" unit test was dropped, per the point above.
- **Not updated:** `docs/RUN_RECORD_FORMAT.md` still says "Implementation: TCMS-47.3, later". It is not in
  `files_modified`, so it needs a follow-up edit.
- **Verdict `error` is not produced** by the combinator. The script output has no error state.

## Open items carried forward

- Q3: no schema for `verification-results.yaml`. AC 2 stays NOT MET until one exists.
- Q4: the 63-file layout is derived, not recorded. Re-check against a real run when one is available.
- AC 4 literal "missing run" clause: decide whether a case in the plan with no result should be emitted as
  `not_executed`. That needs the plan's case list, which this command does not read.
- AC 5: check against the legacy reader when it is available.
- `make test`, `make lint`, `make build`, `make coverage-e2e`, `make acceptance-test`: run before merge.
