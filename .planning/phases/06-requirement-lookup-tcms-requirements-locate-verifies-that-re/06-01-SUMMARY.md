# Phase 6 Plan 01: Batch JQL and StrictDoc export items Summary

**Batch `key in (...)` JQL with `\` and `"` escaping, and a StrictDoc export reader that returns every item with its level, both in `requirement-import`.**

## Performance

- **Duration:** about 10 min
- **Tasks:** 2 of 2 completed
- **Files modified:** 2

## Accomplishments

- `jira::jql_for_keys` builds one search for any number of keys, with no project or issue type clause (D-03, D-04).
- `jira::escape_jql_string` escapes `\` before `"`, so a key cannot end the JQL string literal.
- `jira::LOCATE_FIELDS` names the fields the lookup reads (`summary`, `status`).
- `strictdoc::ExportItem` and `strictdoc::export_items` return every item of an export with uid, level, title, status and document path (D-05, D-09). They use the same `read_export` checks as `parse_export`.

## Task Commits

1. **Task 1: Batch JQL for Jira keys with escaping** - `0dce251` (feat)
2. **Task 2: StrictDoc export items reader** - `302d0ed` (feat)

The tests for each task were written first and failed to compile before the implementation was added (red phase). They were not committed as separate `test(...)` commits.

## Files Created/Modified

- `crates/requirement-import/src/jira.rs` - `jql_for_keys`, `escape_jql_string`, `LOCATE_FIELDS`, and 6 unit tests.
- `crates/requirement-import/src/strictdoc.rs` - `ExportItem`, `export_items`, and 5 unit tests (one reads the fixture file).

## Decisions Made

None beyond the locked CONTEXT decisions (D-03, D-04, D-05, D-09). The `jql_for_keys` doc comment records that callers must match results by `key`, because the mock does not filter on key.

## Deviations from Plan

None. The plan's acceptance criteria were all checked.

## Issues Encountered

None.

## Verification

- `cargo test -p requirement-import`: 22 unit tests and 6 integration tests pass.
- `cargo clippy -p requirement-import --all-targets -- -D warnings`: clean.
- `cargo fmt -p requirement-import -- --check`: clean.
- `cargo test -p tcms --test requirements_cli`: 7 existing tests pass.
- `cargo build --workspace`: builds.

Only the `requirement-import` and `tcms` tests were run. The full `make test`, `make coverage-e2e` and `make acceptance-test` gates from CLAUDE.md were not run for this plan.

## Requirements

LOOK-02 and LOOK-03 are not marked complete. This plan only adds the library helpers. The `tcms requirements locate` command that uses them is in a later plan, so the requirement checkboxes in REQUIREMENTS.md were left unchanged.

## Next Phase Readiness

Plan 02 can call `jira::jql_for_keys` with `jira::LOCATE_FIELDS` and `strictdoc::export_items`. The plan also requires the lookup to match returned issues to IDs by `key`.

## Self-Check: PASSED

- `crates/requirement-import/src/jira.rs` exists and contains the new functions.
- `crates/requirement-import/src/strictdoc.rs` exists and contains `export_items`.
- Commits `0dce251` and `302d0ed` exist on branch `traceability-matrix-report`.
