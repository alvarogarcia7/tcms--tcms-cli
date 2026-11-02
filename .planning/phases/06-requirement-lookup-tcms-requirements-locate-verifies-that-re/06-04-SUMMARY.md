# Phase 6 Plan 04: Document tcms requirements locate and run gates Summary

**Documented `tcms requirements locate` (routing, JQL, StrictDoc `--input`, three formats, exit codes 0/1/2) with verified and assumed limits; fmt, clippy and all tcms tests pass.**

## Performance

- **Tasks:** 2 (Task 1 docs, Task 2 gates)
- **Files modified:** 2

## Accomplishments

- `docs/JIRA_INTEGRATION.md`: crate row lists `requirements locate`; new section `## Requirement lookup: tcms requirements locate` before `## Limits`, with usage, routing, Jira and StrictDoc behaviour, output lines table, formats, exit-code table and a "Verified and not verified" list.
- Output lines, JSON keys and exit codes in the doc were checked against `crates/tcms/src/locate.rs` (`found in jira (`, `found in strictdoc (`, `not found in jira (`, `N of M requirement IDs found`, `not found: K of M requirement IDs`, keys id/found/source/system/level/title/status/url).
- `crates/tcms/Cargo.toml` description updated to "...requirement import and lookup".

## Task Commits

1. **Task 1 + docs/Cargo description**: `f86f589` docs(06-04): document tcms requirements locate and tcms description [ci skip]
2. **Summary**: committed separately as `docs(06-04): summary [ci skip]`

## Quality gates

Run with `env -u CARGO_TARGET_DIR`, as instructed for this plan. The plan's `CARGO_HOME=$HOME/.cargo-user` form and the `--workspace`/`-p requirement-import`/`--all-features` variants were replaced by the commands below.

| Command | Exit | Result |
|---------|------|--------|
| `cargo fmt -p tcms --check` | 0 | clean |
| `cargo clippy -p tcms --all-targets -- -D warnings` | 0 | clean (the pre-existing testcase-execution lint did not appear) |
| `cargo test -p tcms` | 0 | 51 unit + ci_template 7, jira_cli 5, locate_cli 10, pr_cli 5, ref_cli 5, requirements_cli 7, requirements_locate_cli 21, sync_cli 15, vv_metrics_cli 19; all passed |
| `make build`, `make lint`, `make test`, `make coverage-e2e`, `make acceptance-test` | not run | Not run per instruction: `make test`, `make acceptance-test` and `make coverage-e2e` are blocked by open tcms-45 and tcms-46; build and lint were not requested for this plan run. Not run, so no result is claimed. |

No backlog task was updated (Backlog.md MCP not used in this run).

## Deviations from Plan

- **[Scope, user instruction] Gate set narrowed.** The plan named `cargo test --all-features -p tcms -p requirement-import` and `make` targets; the run instruction limited gates to `cargo fmt -p tcms --check`, `cargo clippy -p tcms`, and `cargo test -p tcms`, with `env -u CARGO_TARGET_DIR`. `requirement-import` tests were not run here.
- No production code or test file was changed.

## Issues Encountered

None in the gates that were run.

## Self-Check: PASSED

- `docs/JIRA_INTEGRATION.md` contains exactly one `^## Requirement lookup` heading; `requirements locate` appears 5 times; `--format` 2 times; `plaintext` once.
- `crates/tcms/Cargo.toml` contains "requirement import and lookup".
- Commit `f86f589` exists on `dev`.
- Nothing pushed; STATE.md, ROADMAP.md, ETA.md, backlog and requirements left unchanged.
