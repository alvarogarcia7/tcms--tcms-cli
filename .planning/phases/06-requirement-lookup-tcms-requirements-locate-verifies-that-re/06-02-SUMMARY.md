# Phase 6 Plan 02: tcms requirements locate Summary

**`tcms requirements locate <ID>...` resolves every ID first, looks Jira IDs up in one batch search and StrictDoc IDs in one `--input` export, prints text, plaintext or JSON, and exits 0, 1 or 2.**

## Performance

- **Tasks:** 2 of 2 completed (committed together as one commit, see Task Commits)
- **Files modified:** 3 (1 created)

## Accomplishments

- `locate.rs` holds `LocateArgs`, `Format`, `LocateResult`, `run`, and the pure helpers `dedup_ids`, `jira_matches`, `strictdoc_index`, `render_line`, `render`, `exit_code`.
- Every ID passes `Resolver::resolve_id` before any lookup. An unresolvable ID returns exit 2, names the ID, and prints nothing on stdout.
- Jira: all IDs of one system go in one `jql_for_keys` search with `LOCATE_FIELDS`. Each returned issue counts only for the ID equal to its key. Any issue type counts.
- All Jira clients are built before the first search, so a missing token (`auth.secret_ref`) sends nothing.
- StrictDoc: the `--input` export is read once. Every item counts as found, and its level is shown. A StrictDoc ID without `--input` exits 2.
- A duplicate UID in the export is refused (exit 2).
- Output: `--format text` (default, with `✓`/`✗` markers, optional colour when on a TTY and `NO_COLOR` is unset, and a summary line), `plaintext` (bare lines only), `json` (one pretty array).
- `RequirementsCommand::Locate` added. `reqcmd::run` and `main::run` return an exit code. Every other command still exits 1 on error.

## Task Commits

1. **Task 1: locate.rs types and pure helpers with unit tests** - `26abea7` (feat, combined with Task 2)
2. **Task 2: wire the lookup, Locate variant, exit codes** - `26abea7` (feat, combined with Task 1)

Task 1's tests and helpers were written in one file write, not in a separate red step, so no failing-test run was recorded before the implementation. The 15 unit tests were run once and passed. Both tasks share one commit because the file was written whole.

## Files Created/Modified

- `crates/tcms/src/locate.rs` - new module: arguments, result type, lookup, rendering, exit codes, 15 unit tests.
- `crates/tcms/src/reqcmd.rs` - `Locate` variant; `run` returns `Result<i32>`.
- `crates/tcms/src/main.rs` - `mod locate;`; exit-code plumbing (`Ok(0)`, `Ok(code)` exits with it, errors exit 1).

## Decisions Made

None beyond the locked CONTEXT and DECISIONS entries. `ExportItem.level` is a `String` in the 06-01 reader, so a found StrictDoc item always shows its level.

## Deviations from Plan

None in behaviour. Notes:
- The plan's `CARGO_HOME=$HOME/.cargo-user` prefix was not used; gates ran with `env -u CARGO_TARGET_DIR` as instructed.
- Task 1 and Task 2 were committed as one commit (see Task Commits).

## Verification

- `cargo fmt -p tcms -- --check`: clean.
- `cargo clippy -p tcms --all-targets -- -D warnings`: clean (after one test assertion was changed from `get(..).is_none()` to `!contains_key(..)`).
- `cargo build -p tcms`: no warnings.
- `cargo test -p tcms`: all suites pass, including `requirements_cli` (7 tests) and the 15 `locate::` unit tests.
- Manual runs with the binary:
  - `ZZZ-9` (no system matches): stderr `error: id 'ZZZ-9' matches no system id_pattern ...`, exit 2, empty stdout.
  - `HLR-1` with no `--input`: exit 2, names the ID.
  - `--config /nonexistent.yaml requirements locate PAY-1`: exit 2. `requirements import` with the same bad config: exit 1 (unchanged).
  - `SYS-4 HLR-12 HLR-999 --input <fixture>`: two found, one not found, exit 1, `not found: 1 of 3 requirement IDs` on stderr.
  - `--format json` and `--format plaintext` output match the contract.
- The Jira path has no manual run; it is covered by the mock tests planned in 06-03.

Not run: `make test`, `make acceptance-test`, `make coverage-e2e` (not requested for this plan).

## Requirements

LOOK-01 to LOOK-05 are implemented here but not marked complete in REQUIREMENTS.md. Plan 06-03 adds the mock-based and fixture tests that verify them.

## Self-Check: PASSED

- `crates/tcms/src/locate.rs` exists.
- Commit `26abea7` exists on branch `dev`.
