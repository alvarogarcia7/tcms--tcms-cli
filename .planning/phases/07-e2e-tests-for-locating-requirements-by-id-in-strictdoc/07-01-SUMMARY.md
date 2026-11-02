# Phase 7 Plan 01: Real StrictDoc export reader Summary

**The StrictDoc reader now reads the real `strictdoc export --formats json` layout (DOCUMENTS, NODES, uppercase fields) and refuses the old lowercase layout; all three consumers (import, locate, trace-matrix) use it. One Phase 6 report test still uses the old layout, so the workspace is not green.**

## Status

Plan NOT complete: the must-have "whole workspace stays green" is NOT MET (see Blocker).

## Performance

- Tasks: 3 of 3 executed and committed
- Commits: 3 task commits, no push

## Accomplishments

- `parse_export`, `export_levels` and `export_items` read the real layout through one private reader `read_export`.
- Only `REQUIREMENT` nodes are items; `SECTION` nodes are walked to any depth; `TEXT` and other nodes are ignored.
- The item path is the document `TITLE`; the record source is `<document TITLE>#<UID>`.
- `parent_ids` are the VALUE of every `RELATIONS` entry with TYPE `Parent`.
- A `REQUIREMENT` without `LEVEL` refuses the whole export, naming the UID.
- The old lowercase layout is refused with a message that names `DOCUMENTS`.

## Task Commits

1. `1bbb7bf` feat(07-01): read the real StrictDoc export layout (requirement-import reader, fixture, import tests)
2. `7958c60` feat(07-01): trace-matrix reads the real StrictDoc layout (test-executor fixture and tests)
3. `57de018` feat(07-01): locate and import tests use the real StrictDoc layout (tcms tests, locate help text)

## Decisions Applied

- **D-07-3 (real layout only):** the old layout is dropped, not read in parallel. A file in that layout is refused with a message naming `DOCUMENTS`.
- **D-07-4 (no skip):** the tests read checked-in fixtures and never call StrictDoc.
- **Recorded in this plan:** a `REQUIREMENT` without a UID is skipped (it cannot be addressed by an ID); a requirement without a LEVEL is refused, and no level is derived from the UID prefix.

## Behaviour Changes to Note

- `tcms requirements import` (StrictDoc): `source` is now `<document TITLE>#<UID>` (was `<file path>#<UID>`), e.g. `Software Requirements#HLR-12`.
- StrictDoc imports now always set `safety_related: false` and `security_related: false`, because the real export carries no such flags.

## Criteria

| Criterion | Status | Evidence |
|---|---|---|
| Reader reads DOCUMENTS / recursive NODES | MET | requirement-import unit tests, 28 ok |
| Only REQUIREMENT items; SECTION walked; TEXT ignored | MET | `nested_sections_are_walked_and_other_nodes_ignored` ok |
| Uppercase item fields | MET | `items_map_to_requirement_records` ok |
| Path is document TITLE; source `<TITLE>#<UID>` | MET | `path_is_the_document_title`, `strictdoc_fixture_imports_with_path_uid_and_commit` ok |
| parent_uids only from TYPE Parent | MET | `only_parent_relations_become_parent_ids` ok |
| REQUIREMENT without LEVEL refuses export, naming UID | MET | `item_without_level_is_refused`; `a_strictdoc_export_item_without_level_is_refused`; `locate_strictdoc_item_without_level_exits_2` |
| Old lowercase layout refused, message names DOCUMENTS | MET | `old_lowercase_layout_is_refused`; `locate_strictdoc_old_lowercase_layout_is_refused` |
| Every old-layout test and fixture moved; whole workspace green | NOT MET | see Blocker |
| requirement-import: `cargo test` ok | MET | 28 unit and 6 integration tests ok |
| requirement-import: clippy `-D warnings` | MET | no warnings reported |
| test-executor trace_matrix tests ok | MET | 12 ok in `trace_matrix_cli`, 2 unit tests ok |
| test-executor: `cargo test -p test-executor` exit 0 | NOT MET | one failing test, see Blocker |
| test-executor: clippy `-D warnings` | MET with `--no-deps` | plain run stops on pre-existing `testcase-execution/src/executor.rs:2435:45` lint |
| tcms: `cargo fmt --check -p tcms` | MET | exit 0 |
| tcms: `cargo clippy -p tcms --all-targets -- -D warnings` | MET | exit 0 |
| tcms: `cargo test -p tcms` | MET | all binaries ok, exit 0 |
| `cargo build --workspace` | MET | exit 0 |
| `cargo fmt --all -- --check` | MET | exit 0 |
| `grep "assumed layout"` in strictdoc.rs and `ASSUMPTION` count 0 | MET | 0 and 0 |
| Plan Task 3 grep gate (no old layout outside the two excluded files) | NOT MET | see Blocker |

## Blocker

**1. `cargo test -p test-executor --all-features --no-fail-fast` fails in one test:**

```
test documents_group_rows_and_mark_a_document_without_requirements ... FAILED
thread 'documents_group_rows_and_mark_a_document_without_requirements' panicked at crates/test-executor/tests/report_cli.rs:865:5:
Error: /tmp/.tmpeNavFb/export.json is not a StrictDoc export
Caused by:
    0: StrictDoc export: expected the JSON written by 'strictdoc export --formats json' (an object with a 'DOCUMENTS' array)
    1: missing field `DOCUMENTS` at line 7 column 72
test result: FAILED. 55 passed; 1 failed
```

The cause is `crates/test-executor/tests/report_cli.rs:558`, `export_file()`, which writes an export in the old lowercase layout. The test also asserts the document heading `docs/srs.sdoc`, which under the real layout becomes the document TITLE, so it needs assertion changes as well as input changes. `report_cli.rs` is not in this plan's `files_modified`, so it was not edited, and the test was not weakened. Fix: add `crates/test-executor/tests/report_cli.rs` to this plan's `files_modified` and migrate `export_file()` and its assertions to the real layout.

**2. Old-layout residue outside `files_modified`:** the help strings still say "assumed layout" in `crates/tcms/src/export_ids.rs:20` and `crates/tcms/src/reqcmd.rs:22`. The plan's Task 3 action asked for `export_ids.rs` to change, but it is not in `files_modified`, so it was not edited. The plan's grep gate also matches `crates/testcase-reporting/*` and `crates/test-executor/tests/report_formats_e2e.rs` and `report_shapes_e2e.rs`. Those hits are the report schema's own `documents` key, not the StrictDoc layout, so the gate as written cannot pass without a narrower pattern.

## Deviations from Plan

- Task 3 action listed `crates/tcms/src/export_ids.rs` edits; this contradicted `files_modified` and was not applied (see Blocker 2).
- `crates/tcms/tests/requirements_cli.rs` and `crates/tcms/tests/requirements_locate_cli.rs` module doc lines still describe the fixture as assumed-format; not in the plan's changes, left as is.
- `cargo fmt` reformatted `strictdoc.rs` (this plan's own code).

## Self-Check

- Commits `1bbb7bf`, `7958c60`, `57de018` exist on `dev` and are not pushed.
- Files written in this plan exist (checked with `git show --stat`).

## Not Done (by rule)

- STATE.md, ROADMAP.md, ETA.md and backlog files not edited.
- Requirements LOOK-03 and LOOK-04 not marked complete.
- Nothing pushed.
