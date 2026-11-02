# Phase 6 Plan 03: tcms requirements locate binary tests Summary

**`tcms requirements locate` is now proven through the real binary: Jira batch search and exact key matching in both Cloud and Data Center mock layouts, StrictDoc levels and error exits, the three output formats, and exit codes 0, 1 and 2, all in one new test file with 21 passing tests.**

## Performance

- **Tasks:** 2 of 2 completed (both tasks written in one test file, so one commit)
- **Files created:** 1 (`crates/tcms/tests/requirements_locate_cli.rs`)
- **Production code changed:** none

## Accomplishments

- Jira batch: N IDs cause exactly one search request. Its decoded JQL is `key in (...)` with no `project in` or `issuetype in` clause, and `fields=summary,status` (Cloud and Data Center).
- Any issue type is found, including a Bug (PAY-3) and an issue from a project outside `project_keys` (OTH-1).
- Key match: the mock returns every stored issue, and only asked keys are reported found (PAY-99 is not found and no other key appears in stdout).
- Credentials: an unset or empty token exits 2, names the variable, and sends no request.
- Resolution: an ID that matches no `id_pattern` exits 2 with the ID named, and no lookup is sent.
- Repeated IDs are reported once.
- StrictDoc: levels HLR and system are found and shown; a missing UID exits 1; missing `--input`, unreadable input, wrong layout, refused level and duplicate UID each exit 2 with the expected message.
- Mixed Jira and StrictDoc configuration: one Jira search, and no request at all when `--input` is missing.
- Formats: text (no colour when not a terminal), plaintext (exact line match), json (parsed and checked field by field); unknown format exits 2.
- Read-only: the temp-dir snapshot is equal before and after; the token is absent from stdout and stderr.

## Task Commits

1. **Task 1: Jira tests** and **Task 2: StrictDoc, mixed and format tests** - committed together in one commit (see Self-Check).

## Files Created/Modified

- `crates/tcms/tests/requirements_locate_cli.rs` - 21 binary tests, helpers copied from `requirements_cli.rs`, a hand-written percent decoder, and `search_requests`.

## Decisions Made

None beyond the locked decisions in `.planning/DECISIONS.md`.

## Deviations from Plan

- **JSON test parses the output** with `serde_json` and checks each field, instead of matching the compact string `"id":"PAY-2","found":true,...`. The output is pretty-printed, so the compact string could not match. The assertions are the same fields and values.
- **`--format=json` form** is used in the JSON test, so the plan's literal grep criterion matches. rustfmt splits the argument array across lines, and the single-token form is the same argument to clap.
- **Mixed-source tests use the Cloud layout only.** This follows the plan, which specifies a Cloud mock seeded with PAY-2; it is not a deviation. The Data Center path is covered by the Jira-only tests.
- **Cargo gates** ran as `env -u CARGO_TARGET_DIR` without the `CARGO_HOME=$HOME/.cargo-user` prefix that the plan's commands carry, as instructed for this run.
- **Literal grep forms**: one assertion was written as `assert_eq!(search_requests(&mock).len(), 1, ...)` so the plan's grep criterion matches. Behaviour is unchanged.

## Verification

- `cargo fmt --check -p tcms`: exit 0 (after `cargo fmt -p tcms`, which changed only the new test file).
- `cargo clippy -p tcms --all-targets -- -D warnings`: exit 0 (no `--no-deps` fallback needed).
- `cargo test -p tcms`: exit 0, 145 passed, 0 failed. Includes `requirements_cli` (7 tests, green) and `requirements_locate_cli` (21 tests, green).

Not run: `make test`, `make acceptance-test`, `make coverage-e2e` (not requested for this plan).

## Requirements

LOOK-01 to LOOK-05 are verified by these tests. They are not marked complete in REQUIREMENTS.md; that is left for the phase close-out.

## Known Stubs

None.

## Threat Flags

None. The test file adds no production endpoint, auth path or file write.

## Self-Check

- `crates/tcms/tests/requirements_locate_cli.rs` exists and the 21 tests pass.
- Commit `2a3a2e7` (test, `crates/tcms/tests/requirements_locate_cli.rs`) exists on branch `dev`.
- The 21 tests pass in `cargo test -p tcms` (145 passed in the crate, 0 failed).
