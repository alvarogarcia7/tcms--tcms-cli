# Phase 7 Plan 01: Real StrictDoc export reader Summary

**The StrictDoc reader now reads the real `strictdoc export --formats json` layout (DOCUMENTS, NODES, uppercase fields) and refuses the old lowercase layout; all three consumers (import, locate, trace-matrix) use it. The one Phase 6 report test that still wrote the old layout now writes the real layout, so `cargo test -p tcms` and `cargo test -p test-executor` both pass.**

## Status

Plan NOT complete: the workspace-green criterion is MET, and the `locate --input` help text is MET. The grep gate is DEFERRED to 07-05 because its hits are outside `files_modified`, and two help strings outside `files_modified` still say "assumed layout" (see Blocker 2).

Follow-up pass: all four gates re-run and green (fmt, clippy with `--no-deps`, `cargo test -p tcms`, `cargo test -p test-executor`). No file inside `files_modified` still had old-layout text, so no code commit was made in that pass.

## Performance

- Tasks: 3 of 3 executed and committed, plus one follow-up fix (`d76c0ed`)
- Commits: 4 commits (3 task commits and the `report_cli` fix), no push

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
4. `d76c0ed` fix(07-01): move report_cli old-layout test to real StrictDoc layout (test-executor `report_cli.rs`, per D-07-3)

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
| Every old-layout test and fixture moved | MET | `report_cli.rs` `export_file()` migrated in `d76c0ed`; the remaining old-layout inline JSON in `requirements_locate_cli.rs` is the refusal test and is meant to stay |
| Whole workspace green: `cargo test -p tcms` and `cargo test -p test-executor` both exit 0 | MET | tcms: all binaries ok; test-executor: 12 suites ok, exit 0 (after `d76c0ed`) |
| requirement-import: `cargo test` ok | MET | 28 unit and 6 integration tests ok |
| requirement-import: clippy `-D warnings` | MET | no warnings reported |
| test-executor trace_matrix tests ok | MET | 12 ok in `trace_matrix_cli`, 2 unit tests ok |
| test-executor: `cargo test -p test-executor` exit 0 | MET | exit 0 after `d76c0ed` (the `report_cli` test that failed now passes) |
| test-executor: clippy `-D warnings` | NOT MET | plain `cargo clippy -p test-executor --all-targets -- -D warnings` exits 101 on the pre-existing lint `crates/testcase-execution/src/executor.rs:2435` (`if` with identical blocks), outside this plan. The `--no-deps` run (`cargo clippy --no-deps -p tcms -p test-executor -p requirement-import -p testcase-reporting --all-targets -- -D warnings`) exits 0 |
| tcms: `cargo fmt --check -p tcms` | MET | exit 0 |
| tcms: `cargo clippy -p tcms --all-targets -- -D warnings` | MET | exit 0 |
| tcms: `cargo test -p tcms` | MET | all binaries ok, exit 0 |
| `cargo build --workspace` | MET | exit 0 |
| `cargo fmt --all -- --check` | MET | exit 0 |
| `grep "assumed layout"` in strictdoc.rs and `ASSUMPTION` count 0 | MET | 0 and 0 |
| `locate --input` help text describes the real export (`locate.rs`) | MET | `57de018`: "written by `strictdoc export --formats json`" |
| "assumed layout" help text in `export_ids.rs` and `reqcmd.rs` | DEFERRED to 07-05 | `export_ids.rs:20`, `reqcmd.rs:22`; see D-07-7 |
| Plan Task 3 grep gate (no old layout outside the two excluded files) | DEFERRED to 07-05 (narrowed) | the two hits above (`export_ids.rs`, `reqcmd.rs`). The `documents` hits are the report schema's own key, not StrictDoc residue. Narrowed pattern `'assumed layout\|"documents":\[{"path"'` assigned to 07-05 and added to its acceptance criteria |

## Deferred to 07-05 (D-07-7)

The help-text wording and the grep gate hits in `crates/tcms/src/export_ids.rs:20` and `crates/tcms/src/reqcmd.rs:22` ("assumed layout") are DEFERRED, not failed: wording only, outside 07-01 `files_modified`, owned by 07-05 (D-07-6), which now lists both files. The five `documents` hits in testcase-reporting and test-executor are the report schema's own key and are not StrictDoc residue.

## Review Fixes

Applied after review of 07-01. Comments, wording and this summary only; no behaviour change.

- Fixed: `crates/requirement-import/src/lib.rs` module doc now names the real StrictDoc export layout (`DOCUMENTS` and `NODES`) instead of an ASSUMPTION.
- Fixed: module doc line 2 of `crates/tcms/tests/requirements_cli.rs`, `crates/tcms/tests/requirements_locate_cli.rs` and `crates/tcms/tests/requirements_export_ids_cli.rs` now says "real-layout fixture" (was "assumed-format" or "assumed-layout").
- Gate table corrected: the test-executor clippy row is NOT MET (see the table). The `--no-deps` run passes.
- Grep gate narrowed and assigned to 07-05 (see the table). The acceptance line is in `07-05-PLAN.md`.
- Not changed: the review named always-false `safety_related` and `security_related` fields in `crates/testcase-reporting/src/strictdoc.rs`, which does not exist. The fields are in `crates/requirement-import/src/strictdoc.rs` (`Item`). `parse_export` reads them and copies them into `RequirementRecord`, so they stay, and the assertions at lines 283 and 288 stay. Open question: an export that carries no flags could map to `None` instead of `Some(false)`, but that changes the YAML output and is a behaviour change.
- Gates run for this fix: `cargo fmt --check` exit 0; `cargo clippy --no-deps ... -D warnings` exit 0; `cargo test` for tcms (178 passed), test-executor (156), requirement-import (34), testcase-reporting (279), all exit 0.

## Blocker

**1. RESOLVED in `d76c0ed`:** `documents_group_rows_and_mark_a_document_without_requirements` failed because `export_file()` in `crates/test-executor/tests/report_cli.rs` wrote the old lowercase layout. `export_file()` now writes `DOCUMENTS` / `NODES` / `REQUIREMENT` nodes with uppercase fields. The document TITLEs are `docs/srs.sdoc` and `docs/empty.sdoc`, so the path comes from the TITLE and every assertion is unchanged. The file was outside this plan's `files_modified`; it was edited per D-07-3.

**2. Old-layout residue outside `files_modified` (DEFERRED to 07-05, D-07-7):** the grep gate (`grep -rln '"documents"\|assumed layout' crates --include=*.rs --include=*.json`, minus the two excluded files) still lists these files, none of which is in `files_modified`:
- `crates/tcms/src/export_ids.rs:20` and `crates/tcms/src/reqcmd.rs:22`: help text still says "assumed layout". Task 3's action names `export_ids.rs`, but the file is not in `files_modified`, so it was left untouched.
- `crates/testcase-reporting/src/report_document.rs`, `crates/testcase-reporting/src/render_data.rs`, `crates/testcase-reporting/tests/report_schema.rs`, `crates/test-executor/tests/report_formats_e2e.rs`, `crates/test-executor/tests/report_shapes_e2e.rs`: these use the report schema's own `documents` key, not the StrictDoc layout. The gate as written cannot pass without a narrower pattern.

## Deviations from Plan

- Task 3 action listed `crates/tcms/src/export_ids.rs` edits; this contradicted `files_modified` and was not applied (see Blocker 2).
- `crates/tcms/tests/requirements_cli.rs`, `requirements_locate_cli.rs` and `requirements_export_ids_cli.rs` module doc lines still described the fixture as assumed-format or assumed-layout. Not in the plan's changes; reworded to real-layout in the review fix below (comments only).
- `cargo fmt` reformatted `strictdoc.rs` (this plan's own code).
- `safety_related` and `security_related` are not read from StrictDoc exports; they are always false until a real export carrying them is added to the fixtures (D-07-8). Follow-up: read the flags only when a real export carrying them is added to the fixtures, through a new plan.

## Self-Check

- Commits `1bbb7bf`, `7958c60`, `57de018` exist on `dev` and are not pushed.
- Files written in this plan exist (checked with `git show --stat`).

## Not Done (by rule)

- STATE.md, ROADMAP.md, ETA.md and backlog files not edited.
- Requirements LOOK-03 and LOOK-04 not marked complete.
- Nothing pushed.
