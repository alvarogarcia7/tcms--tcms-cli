# Phase 4 Plan 05: Shape A and B validation before write Summary

Traceability shapes A and B are built as ReportDocuments, checked against `schemas/tcms/report.schema.v1.json` before any byte is written, for markdown, asciidoc, json and yaml (gap closure for tcms-43.4 AC #5, decision D-04-6).

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (red) | 8427cd1 | test(04-05): failing tests for shape A and B validation, check_document and writes [ci skip] |
| 2 (green, library) | 2441ce7 | feat(04-05): shape A and B report documents validated by the local schema [ci skip] |
| 3 (green, CLI) | 6558812 | feat(04-05): validate shape documents before write for every report format [ci skip] |

## Red run (Task 1, recorded before any production change)

- `testcase-reporting` lib tests: did not compile. E0425 `build_shape_document` and `check_document` not found, E0433 `TraceShape` not found.
- `report_schema`: 6 shape tests failed, all at the accept step, panic at `report_schema.rs:52`: `shape_a_and_b_documents_are_accepted`, `shape_rejects_unknown_shape_letter`, `shape_rejects_outcome_outside_enum`, `shape_requirement_rejects_grid_only_keys`, `shape_case_rejects_column_key`, `shape_rows_without_shape_key_are_rejected`. The reject tests failed at their leading `assert_accepted(shape_base())`.
- `report_shapes_e2e`: 5 failed. Exit codes before the fix: json, yaml and asciidoc shape writes exit 2 (`--shape a|b requires --format markdown`); markdown with tampered schema exits 0 (expected 1); usage test stderr named the format guard for json; asciidoc differs-from-grid exits 2.

## Gate results (after Task 3)

| Gate | Result |
|------|--------|
| `env -u CARGO_TARGET_DIR cargo fmt --check -p testcase-reporting -p test-executor` | exit 0 |
| `env -u CARGO_TARGET_DIR cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | exit 101: pre-existing `if` with identical blocks at `crates/testcase-execution/src/executor.rs:2351`, a dependency this plan does not touch. Left open, not fixed. |
| `env -u CARGO_TARGET_DIR cargo clippy --no-deps -p testcase-reporting -p test-executor --all-targets -- -D warnings` | exit 0 (the in-scope crates are clean) |
| `env -u CARGO_TARGET_DIR cargo test -p testcase-reporting -p test-executor` | exit 0: 405 passed, 0 failed |

`make test`, `make acceptance-test` and `make coverage-e2e` were not run, per instruction.

## Criteria

- Shape A and B documents validated before any write, all four formats: `check_document` is called in `report()` before the input summary, `fs::write` and `write_stdout`. Tested by `shape_failed_validation_writes_no_file_for_every_format`.
- Failure writes no file, prints nothing on stdout, exits 1, existing file unchanged: tested.
- `--shape a|b` accepted with every format; non-traceability or no `--kind` exits 2 and writes nothing: tested.
- Grid JSON, YAML and Markdown unchanged: the existing report suites pass unedited.
- JSON and YAML shape documents carry kind traceability and `content.shape` a or b with the grid's requirement and case ids in the same order: tested.

OUT-01 and OUT-02 are not marked complete. They are verified only after the verifier re-runs against this plan.

## Deviations

1. **[Rule 1 - Bug] `oneOf` replaced by `if`/`then`/`else` in the schema.** The plan asked for `oneOf` of grid and shape content. With `oneOf`, the existing test `two_simultaneous_violations_give_two_messages_with_paths` lost its field-level messages (one generic error at `/content`). The `if` on the presence of `shape` keeps the grid's per-field messages and remains disjoint: a document with a `shape` key goes to shape content, one without goes to grid content. The existing test was not edited.
2. **Task 1 acceptance wording.** Its action adds red tests to test modules inside `src/` files, but its acceptance check says no `src` file changes. Read as: no production code changed in Task 1. Confirmed by the diff, which is test-only.
3. **Unused helper removed.** Clippy flagged an unused `report` helper in the new `report_shapes_e2e.rs`; removed (my file, my commit).
4. **Stale test name, not edited.** `report_formats_cli.rs::shape_with_new_format_is_a_usage_error` still passes. Its first three cases omit `--kind traceability`, so they exit 2 by the kind rule now, not the removed format rule. Its name and message check are stale. `report_formats_cli.rs` is outside this plan's `files_modified`, so it was left alone.

## Known Stubs

None.

## Threat Flags

None. No new network, auth or file-access surface; the shape writes go through the same schema gate and output path as the grid.

## Self-Check: PASSED

- Commits 8427cd1, 2441ce7 and 6558812 exist on `dev`.
- Files changed: `schemas/tcms/report.schema.v1.json`, `crates/testcase-reporting/src/{report_document,render_data}.rs`, `crates/testcase-reporting/tests/report_schema.rs`, `crates/test-executor/src/main.rs`, `crates/test-executor/tests/report_shapes_e2e.rs`. No edits to STATE.md, ROADMAP.md, REQUIREMENTS.md or backlog.
