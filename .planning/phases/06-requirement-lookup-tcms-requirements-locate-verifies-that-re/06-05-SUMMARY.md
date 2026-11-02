# Phase 6 Plan 05: tcms requirements export-ids Summary

**Added `tcms requirements export-ids`: reads `requirements.json` (or `--requirements`), refuses a bad file with exit 2 before any lookup, checks every distinct ID with the shared `locate` batch lookup, writes the valid IDs to stdout or `--output`, lists invalid IDs on stderr, exits 0/1/2. Self-check: all tcms tests, clippy and fmt pass; one literal grep criterion is not met (see Plan criteria).**

## Performance

- **Tasks:** 3 (Task 1 shared lookup and pure helpers, Task 2 run wiring, Task 3 binary tests)
- **Files modified:** 5 (`locate.rs`, `export_ids.rs` new, `main.rs`, `reqcmd.rs`, `tests/requirements_export_ids_cli.rs` new)

## Accomplishments

- `crates/tcms/src/locate.rs`: the body of `locate` moved into `pub(crate) fn lookup(cfg, ids, input, actor)`; `locate` loads the config and delegates. `render_line` and `dedup_ids` widened to `pub(crate)`. No output string, order or exit code changed; the 21 `requirements_locate_cli` tests and 7 `requirements_cli` tests stay green.
- `crates/tcms/src/export_ids.rs` (new): `ExportIdsArgs` (`--requirements` default `requirements.json`, `--output`, `--input`, `--as`); strict `RequirementsFile` / `RequirementEntry` (`deny_unknown_fields`); `parse_requirements`, `check_file` (product_id must equal the config, list must be non-empty), `file_ids`, `render_output`, `invalid_lines`, `export_ids`, `run`. Checks run in this order with nothing printed or written until they all pass: load config, read file, parse, product and empty checks, `locate::lookup` (one batch search per Jira system, duplicates checked once).
- `crates/tcms/src/reqcmd.rs`: `RequirementsCommand::ExportIds(crate::export_ids::ExportIdsArgs)`, clap name `export-ids`.
- `crates/tcms/src/main.rs`: `mod export_ids;` before `mod extrefs;`.
- `crates/tcms/tests/requirements_export_ids_cli.rs` (new, 743 lines after rustfmt): 18 binary tests, Cloud and Data Center through the Jira mock, StrictDoc through the assumed-layout fixture.

## Task Commits

1. **Task 1: shared lookup and export-ids parsing**: `60c4d8b` feat(06-05): shared locate lookup and export-ids file parsing [ci skip]
2. **Task 2: run wiring, ExportIds variant**: `7a4a2e3` feat(06-05): tcms requirements export-ids with exit codes 0/1/2 [ci skip]
3. **Task 3: binary tests**: `ac1aa24` test(06-05): binary tests for requirements export-ids [ci skip]
4. **Summary**: `docs(06-05): summary [ci skip]` (this file)

## Plan criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| Default path `./requirements.json`, or `--requirements <FILE>` | MET | `export_ids_default_path_all_valid_{cloud,datacenter}`, `export_ids_requirements_flag_reads_the_named_file`; `--help` shows the default |
| Unparseable file or foreign product_id: exit 2, nothing written, no Jira request | MET | `export_ids_unparseable_file_exits_2`, `export_ids_product_mismatch_exits_2`, `export_ids_output_file_not_created_on_error`; request_count 0 |
| Every ID checked with the locate lookup, one search per Jira system; StrictDoc via `--input` | MET | `export_ids_default_path_all_valid_*` (one search, `key in ("PAY-1", "PAY-2", "PAY-3")`), `export_ids_strictdoc_with_input` |
| Repeated ID checked once and reported once | MET | `export_ids_duplicate_id_is_reported_once` (one `"PAY-2"` in the search, one `invalid: PAY-99`) |
| Valid IDs written as `{"product_id","requirements":[{"id"}]}` to stdout or `--output` (stdout empty) | MET | `export_ids_output_file_instead_of_stdout`, `export_ids_default_path_all_valid_*` |
| Invalid IDs on stderr, one line each, plus count line | MET | `export_ids_mixed_valid_and_invalid_exits_1_*`, `export_ids_strictdoc_with_input` |
| Exit 0 all valid, 1 any invalid (valid IDs still written), 2 on error | MET | exit-code assertions in the 18 tests (exit 2: 9 assertions, exit 1: 4) |
| `locate` behaves as before (21+ and 7 tests green) | MET | `requirements_locate_cli` 21 passed, `requirements_cli` 7 passed |
| Task 1 criterion: `export_ids::` at least 10 unit tests ok | MET | 13 passed |
| Task 3 criterion: at least 18 export-ids binary tests ok | MET | 18 passed |
| Task 3 criterion: `grep -c "search_requests(&mock).len(), 1\|search_requests(&mock).len() == 1"` is at least 2 | NOT MET (literal count) | The grep finds 1 line. The one-search rule is asserted in three tests (`export_all_valid`, `export_mixed_valid_and_invalid`, `export_ids_duplicate_id_is_reported_once`), two of which use a local `searches.len()` binding. Assertions were not reworded to raise the grep count. |
| Task 2 criterion: `--help` lists `--requirements`, `--output`, `--input`, `requirements.json` | MET | checked by running `tcms requirements export-ids --help` |
| Task 2 criterion: missing `--config` file from `/tmp` exits 2 | MET | last line `2` |
| Task 2/3 criterion: `allow(dead_code)` count 0 in export_ids.rs | MET | 0 |
| `docs/JIRA_INTEGRATION.md`, `requirements_cli.rs`, `requirements_locate_cli.rs` unchanged | MET | not in any commit of this plan |

## Quality gates

All commands run with `env -u CARGO_TARGET_DIR` and `CARGO_HOME=$HOME/.cargo-user`, as the run instruction set. The plan's `--workspace`, `--all-features` and `make` variants were not run (see Deviations).

| Command | Exit | Result |
|---------|------|--------|
| `cargo fmt --check -p tcms` | 1 first, then 0 | First run: rustfmt diffs in `export_ids.rs` and the new test file only. `cargo fmt -p tcms` applied; recheck exit 0. |
| `cargo clippy -p tcms --all-targets -- -D warnings` | 0 | clean; the pre-existing testcase-execution `executor.rs:2435` lint did not fire, so `--no-deps` was not needed |
| `cargo test -p tcms` | 0 | unit 64 (includes 13 export_ids and 15 locate unit tests), ci_template 7, jira_cli 5, locate_cli 10, pr_cli 5, ref_cli 5, requirements_cli 7, requirements_export_ids_cli 18, requirements_locate_cli 21, sync_cli 15, vv_metrics_cli 19; all passed |
| `make build`, `make lint`, `make test`, `make coverage-e2e`, `make acceptance-test`, `cargo test --workspace` | not run | Not in the run instruction's gate list; `make test`, `make acceptance-test` and `make coverage-e2e` were excluded explicitly. |

No backlog task was updated; the Backlog.md MCP was not used in this run. STATE.md, ROADMAP.md, ETA.md and requirements (LOOK-06, LOOK-07) were not edited or marked complete.

## Deviations from Plan

- **[Run instruction] Gate set narrowed.** The plan's Task 3 also asked for `cargo fmt --all -- --check`, `clippy --workspace`, `cargo test --all-features -p tcms -p requirement-import`, `make build`, `make lint`, `make test`, `make coverage-e2e` and `make acceptance-test`. The run instruction limited gates to fmt, clippy and test for `-p tcms`, and excluded `make test`, `make acceptance-test` and `make coverage-e2e`. The `requirement-import` tests were not run here.
- **[Process] Red step not observed for Task 1.** Plan Task 1 asks to write the unit tests first and watch them fail. The tests and the implementation were written in one edit, so no failing run was recorded before the implementation. The tests were run and passed (13 export_ids, 15 locate). The Task 2 and Task 3 tests were written after the code they exercise.
- **[Rule 2 formatting] rustfmt applied.** `cargo fmt -p tcms` reformatted `export_ids.rs` and the new test file before the Task 3 commit. No other file was touched.
- **[Commit subject] Task 1 commit amended** before any push, to add the `[ci skip]` suffix that was missing from the first subject line.

## Deferred Issues

None from this plan.

## Known Stubs

None.

## Self-Check

- Commits `60c4d8b`, `7a4a2e3`, `ac1aa24` exist on `dev` (not pushed).
- Files `crates/tcms/src/export_ids.rs` and `crates/tcms/tests/requirements_export_ids_cli.rs` exist.
- One plan criterion (literal grep count of the one-search assertion) is NOT MET; all other criteria listed above are MET.

## Self-Check: PARTIAL (one literal grep criterion NOT MET; all tests, clippy and fmt green)
