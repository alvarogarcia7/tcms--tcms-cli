---
phase: 07-e2e-tests-for-locating-requirements-by-id-in-strictdoc
verified: 2026-11-02T00:00:00Z
status: passed
score: 9/9 must-haves verified
overrides_applied: 0
---

# Phase 7: E2E tests for locating requirements by ID in StrictDoc Verification Report

**Phase Goal:** The real `tcms` binary is tested end to end on exports made by the real StrictDoc tool: `tcms requirements locate` finds HLR, LLR and `system` items by ID, reports missing IDs, and exits 0/1/2 as specified; `tcms requirements export-ids` writes the valid IDs. The StrictDoc reader first reads the layout StrictDoc really writes (DECISIONS.md D-07-3).
**Requirements:** LOOK-03, LOOK-04, LOOK-07
**Verified:** 2026-11-02 against branch dev, HEAD 675e1bed

## Goal Achievement

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | Reader reads the real StrictDoc layout (DOCUMENTS, NODES, uppercase fields) | VERIFIED | `crates/requirement-import/src/strictdoc.rs:36-70` (serde renames); `strictdoc_fixture_imports_with_path_uid_and_commit` (`crates/requirement-import/tests/import.rs:283`); `items_map_to_requirement_records`; `path_is_the_document_title` |
| 2 | Only REQUIREMENT nodes are items; SECTION walked; TEXT ignored | VERIFIED | `strictdoc.rs:205-209`; `nested_sections_are_walked_and_other_nodes_ignored` (ok) |
| 3 | Old lowercase layout refused with a message naming DOCUMENTS (D-07-3) | VERIFIED | `strictdoc.rs:181`; `old_lowercase_layout_is_refused`; `locate_with_the_old_lowercase_layout_exits_2` (`crates/tcms/tests/requirements_locate_strictdoc_e2e.rs:437`) |
| 4 | Checked-in exports are the output of the real StrictDoc tool; --check passes | VERIFIED | `bash crates/tcms/tests/fixtures/strictdoc/regenerate.sh --check` exit 0, "All exports match a fresh StrictDoc export" (locate, refused-level, no-level up to date); `fixture_is_the_output_of_the_real_strictdoc_tool` (`requirements_locate_strictdoc_e2e.rs:121`) |
| 5 | `locate` finds HLR, LLR and system items by ID | VERIFIED | `locate_finds_an_hlr` (:142), `locate_finds_an_llr` (:155), `locate_finds_a_system_item` (:168); output lines assert level HLR/LLR/system |
| 6 | `locate` reports missing IDs and exits 0/1/2 as specified | VERIFIED | Exit 1: `locate_reports_an_id_that_is_not_in_the_export` (:181), `locate_text_format_marks_the_missing_id` (:281). Exit 0: `locate_batch_all_found_exits_0_in_text_format` (:258). Exit 2: `locate_an_id_matching_no_system_format_exits_2` (:372), `locate_without_input_exits_2` (:392), `locate_with_a_missing_input_file_exits_2` (:401), `locate_with_a_refused_level_exits_2` (:412), `locate_with_an_item_without_level_exits_2` (:425), `locate_with_a_non_json_input_exits_2` (:453), `locate_unknown_output_format_exits_2` (:465). Mapping in `crates/tcms/src/locate.rs:184-190, 314-330` |
| 7 | `export-ids` writes the valid IDs (stdout or --output) | VERIFIED | `export_ids_all_valid_writes_every_id_to_stdout` (`requirements_export_ids_strictdoc_e2e.rs:129`); `export_ids_output_option_writes_the_file_and_leaves_stdout_empty` (:284); `export_ids_output_file_has_the_valid_ids_on_exit_1` (:313) |
| 8 | export-ids: invalid IDs on stderr, exit 0/1/2, product mismatch exits 2 and writes nothing | VERIFIED | `export_ids_one_invalid_id_exits_1_and_still_writes_the_valid_ones` (:175; asserts code 1, stderr, stdout lacks HLR-99); `export_ids_product_mismatch_exits_2_and_writes_nothing` (:371); exit 2 cases :403, :433, :463, :493, :506, :529, :545, :573. Mapping `crates/tcms/src/export_ids.rs:123-140` |
| 9 | No e2e test calls StrictDoc, skips, is ignored, or uses the network (D-07-4) | VERIFIED | Only `Command::new(env!("CARGO_BIN_EXE_tcms"))` in both files (`requirements_locate_strictdoc_e2e.rs:62`, `requirements_export_ids_strictdoc_e2e.rs:72`); grep for `#[ignore]`, `skip`, `uv`, `python` returns nothing in either file. `https://pages.example.com/srs` is a config string only. Run shows 0 ignored in every suite |

**Score:** 9/9 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/requirement-import/src/strictdoc.rs` | Real-layout reader | VERIFIED | Substantive; used by import, locate and trace-matrix |
| `crates/tcms/tests/fixtures/strictdoc/{locate,refused-level,no-level}/export.json` | Real StrictDoc exports | VERIFIED | Regenerate --check exit 0 |
| `crates/tcms/tests/fixtures/strictdoc/regenerate.sh` | Regenerate and check script | VERIFIED | Exit 0 on --check; exits 2 without uv; never run by cargo test |
| `crates/tcms/tests/requirements_locate_strictdoc_e2e.rs` | locate e2e | VERIFIED | 18 tests ok |
| `crates/tcms/tests/requirements_export_ids_strictdoc_e2e.rs` | export-ids e2e | VERIFIED | 21 tests ok |
| `docs/JIRA_INTEGRATION.md`, `docs/TRACEABILITY*` | Verified layout documented | VERIFIED | Per 07-05 criteria; grep for "assumed layout" in crates and docs now returns nothing |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| tcms locate / export-ids | requirement-import StrictDoc reader | `read_export` | WIRED | Exercised by the e2e tests on the real exports |
| e2e tests | tcms binary | `CARGO_BIN_EXE_tcms` | WIRED | Real binary run as a subprocess |
| regenerate.sh --check | strictdoc via `uv run` | `uv run strictdoc export --formats json` | WIRED | Fresh export diffed against checked-in JSON |

### Requirements Coverage

| Requirement | Source | Description | Status | Evidence |
| ----------- | ------ | ----------- | ------ | -------- |
| LOOK-03 | Phase 6, tested here | Item found by UID from local `--input`, level shown | VERIFIED | locate HLR/LLR/system tests; line format asserts `at level <LEVEL>` |
| LOOK-04 | Phase 6, tested here | Exit 0/1/2 as specified | VERIFIED | Truths 6 and 8 |
| LOOK-07 | Phase 6, tested here | export-ids writes valid IDs, invalid on stderr, exit 0/1/2, product mismatch exits 2 | VERIFIED | Truths 7 and 8 |

## Plan-level status (from SUMMARY tables, cross-checked)

- 07-01: Core criteria MET. NOT MET: literal grep wording for test-executor clippy `-D warnings` (pre-existing lint in testcase-execution). Re-run here with `--no-deps` on test-executor: clean.
- 07-02: All MET per summary; regenerate --check re-verified here.
- 07-03: Behaviour MET. NOT MET as literal grep text only: `status.code(), Some(0)` / `Some(1)` / `Some(2)` patterns (tests use a `code(&o)` helper). Behaviour asserted by tests that pass. WARNING-level wording gap, not a behaviour gap.
- 07-04: All MET per summary; tests re-run here.
- 07-05: Doc criteria MET. DEFERRED by the run's scope: `make test`, `make acceptance-test`, `make coverage-e2e`, `cargo test --all-features`, workspace-wide fmt and clippy. Not run in this verification.
- DECISIONS D-07-1 to D-07-9: consistent with the code and tests as checked above.

## Commands Run (this verification)

| Command | Result |
| ------- | ------ |
| `env -u CARGO_TARGET_DIR cargo test -p tcms -p requirement-import -p test-executor` | Exit 0; all suites ok, 0 failed, 0 ignored |
| `env -u CARGO_TARGET_DIR cargo fmt --check -p tcms -p test-executor -p requirement-import` | Exit 0 |
| `bash crates/tcms/tests/fixtures/strictdoc/regenerate.sh --check` | Exit 0 |
| `cargo clippy --no-deps -p tcms -p requirement-import -p test-executor --all-targets -- -D warnings` | Clean (no warnings) |

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `docs/strictdoc/software/requirements/SRD.sdoc` | 1856, 1972 | Still says the StrictDoc JSON layout "is an assumption" | WARNING | Stale project requirement text; contradicts verified layout in JIRA and TRACEABILITY docs. Not covered by the 07-05 grep pattern |
| `crates/requirement-import/tests/fixtures/strictdoc_export.json` | n/a | Provenance not stated as a real StrictDoc export | INFO | Reader unit fixture only; the real-tool claim rests on `crates/tcms/tests/fixtures/strictdoc` |

No debt markers (TBD, FIXME, XXX) found in the Phase 7 test or source files.

### Human Verification Required

None.

### Gaps Summary

No BLOCKER gaps. All Phase 7 success criteria and LOOK-03, LOOK-04, LOOK-07 are verified by passing tests on the real StrictDoc exports, and the regenerate --check drift gate passes.

Open WARNINGs for the developer:
1. SRD.sdoc lines 1856 and 1972 still call the StrictDoc layout an assumption.
2. The 07-03 literal grep patterns for exit codes are not met as text (behaviour is asserted).
3. Make-level and workspace-wide gates (`make test`, `make acceptance-test`, `make coverage-e2e`, `cargo test --all-features`) were not run here, per instruction.

---

_Verified: 2026-11-02_
_Verifier: Claude (gsd-verifier)_
