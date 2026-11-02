---
phase: 06-requirement-lookup-tcms-requirements-locate-verifies-that-re
verified: 2026-11-02T00:00:00Z
status: passed
score: 8/8 truths verified (goal, LOOK-01..07)
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 7/8
  gaps_closed:
    - "A StrictDoc ID is found in a real StrictDoc export given with --input (LOOK-03, LOOK-07 StrictDoc path, goal clause 'StrictDoc export')"
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "Make-level gates (make test, make acceptance-test, make coverage-e2e) recorded before commit"
    addressed_in: "Process gate, not a Phase 6 requirement (CLAUDE.md pre-commit workflow; backlog tcms-45, tcms-46 open)"
    evidence: "07-05-SUMMARY.md: make test, make coverage-e2e, make acceptance-test not run by instruction; tcms-45 and tcms-46 open"
---

# Phase 6: Requirement lookup Verification Report

**Phase Goal:** `tcms requirements locate <ID>...` shows, for each ID, whether it is a requirement in the configured Jira backlog or a StrictDoc export, with non-zero exit when any ID is missing.
**Verified:** 2026-11-02
**Status:** passed
**Re-verification:** Yes, after Phase 7 (07-01 to 07-05) closed the StrictDoc real-layout gap

Method: goal-backward. Phase 7 summaries and DECISIONS.md D-07-1..D-07-9 were read only to locate claims. Each claim was checked in source and by running `cargo test -p tcms` and `cargo fmt --check -p tcms`.

## Commands run

| Command | Result |
|---------|--------|
| `env -u CARGO_TARGET_DIR cargo test -p tcms` | All test binaries `ok`, 0 failed. Counts: unit 64, ci_template 7, jira_cli 5, locate_cli 10, pr_cli 5, ref_cli 5, requirements_cli 7, requirements_export_ids_cli 18, requirements_export_ids_strictdoc_e2e 18, requirements_locate_cli 23, requirements_locate_strictdoc_e2e 21, sync_cli 15, vv_metrics_cli 19 |
| `env -u CARGO_TARGET_DIR cargo fmt --check -p tcms` | Exit 0 |
| make test, make acceptance-test, make coverage-e2e | Not run, per instruction |
| clippy | Not run here (not requested) |

## Goal and LOOK requirements

| Req | Status | Evidence |
|-----|--------|----------|
| Goal | VERIFIED | Jira and StrictDoc (real export) both report per-ID found or not-found; exit 1 when any ID is missing (`locate.rs:184` exit_code, `locate.rs:315` run). |
| LOOK-01 | VERIFIED | One batch Jira search per Jira system via `jql_for_keys` (`crates/requirement-import/src/jira.rs:75`); StrictDoc export read once. Tests: `locate_mixed_sources_one_search` (`requirements_locate_cli.rs:635`), `locate_jira_batch_finds_any_issue_type_cloud/_datacenter` (`:306`, `:311`), one-search assertions at `requirements_locate_cli.rs:278`, `:343`, `requirements_export_ids_cli.rs:541`, `:583`. |
| LOOK-02 | VERIFIED | Exact key match, no issue-type filter (`locate.rs:85` jira_matches; `jira.rs:232` `jql_for_keys_has_no_project_or_type_clause`). Tests: `jira_matches_the_exact_key` cloud and datacenter (`requirements_locate_cli.rs:315`, `:347`, `:352`). |
| LOOK-03 | VERIFIED against real StrictDoc 0.20.0 export | Reader now reads `DOCUMENTS` / `NODES` (`crates/requirement-import/src/strictdoc.rs:36-44`, `read_export` at `:178`, refusal message naming `DOCUMENTS` at `:181`). Checked-in real exports: `crates/tcms/tests/fixtures/strictdoc/{locate,refused-level,no-level}/export.json`. E2E tests on the real files: `requirements_locate_strictdoc_e2e.rs` `locate_finds_an_hlr`, `locate_finds_an_llr` (nested two sections deep), `locate_finds_a_system_item`, `locate_reports_an_id_that_is_not_in_the_export`, `locate_batch_gives_one_result_per_id_in_command_line_order`, `locate_json_has_the_expected_objects`, `locate_writes_nothing`. Level is shown in the line (`locate.rs:120` render_line). |
| LOOK-04 | VERIFIED | Constants `locate.rs:22-26`. Exit 1: `locate_reports_an_id_that_is_not_in_the_export`. Exit 2 (real export): `locate_without_input_exits_2`, `locate_with_a_missing_input_file_exits_2`, `locate_with_a_refused_level_exits_2`, `locate_with_an_item_without_level_exits_2`, `locate_with_the_old_lowercase_layout_exits_2`, `locate_with_a_non_json_input_exits_2`, `locate_an_id_matching_no_system_format_exits_2`. Jira credentials: `locate_jira_needs_the_token`. |
| LOOK-05 | VERIFIED | `locate.rs:147` render; `locate_cli.rs` format tests; `locate_format_json`, `locate_format_plaintext`, `locate_rejects_an_unknown_format`. |
| LOOK-06 | VERIFIED | `export_ids.rs:30-37` (`deny_unknown_fields`), `:107` export_ids uses `locate::lookup`. Default `requirements.json` and `--requirements`: `export_ids_default_path_all_valid_cloud/_datacenter` (`requirements_export_ids_cli.rs:284`, `:289`), `export_ids_reads_requirements_json_of_the_current_directory_by_default` (strictdoc e2e). StrictDoc real layout: `export_ids_an_id_of_each_level_is_valid_and_ids_not_in_the_export_are_not`. |
| LOOK-07 | VERIFIED | `export_ids.rs:50` check_file (product mismatch refused before lookup), `:74` render_output (found IDs only), `:125` run (invalid IDs to stderr, exit 1 with valid IDs still written). Real export: `export_ids_one_invalid_id_exits_1_and_still_writes_the_valid_ones`, `export_ids_every_id_invalid_exits_1_with_an_empty_list`, `export_ids_product_mismatch_exits_2_and_writes_nothing`, `export_ids_with_a_refused_export_exits_2`. Jira: `export_ids_mixed_valid_and_invalid_exits_1_cloud/_datacenter`. |

## Phase 7 gap closure

| Former gap | Now | Evidence |
|------------|-----|----------|
| Reader assumed a `documents` layout | Closed. Real layout only; old layout refused naming `DOCUMENTS` | 07-01 commit `57de018`; `strictdoc.rs` tests `old_lowercase_layout_is_refused`, `wrong_layout_is_refused_naming_documents`; `requirements_locate_cli.rs:539`, `:587-597` |
| No real-export tests | Closed. 39 e2e tests on checked-in StrictDoc 0.20.0 exports | `requirements_locate_strictdoc_e2e.rs` (21), `requirements_export_ids_strictdoc_e2e.rs` (18); `fixture_is_the_output_of_the_real_strictdoc_tool` (`requirements_locate_strictdoc_e2e.rs:121`) |
| "assumed layout" wording | Closed. No hits in `crates/tcms/src` or `crates/requirement-import/src` | `grep -rn assumed` returned nothing; `docs/JIRA_INTEGRATION.md:135-156`, `docs/TRACEABILITY.md:40` |

Fixture provenance: the files are attested as StrictDoc 0.20.0 output by 07-02 and 07-05 (`regenerate.sh --check` reported "All exports match a fresh StrictDoc export"). That check was not re-run in this verification (needs uv and network). The tests read the checked-in files only.

## Deferred items (do not block the requirements)

| Item | Blocks a requirement? | Finding |
|------|----------------------|---------|
| make test, make acceptance-test, make coverage-e2e not run | No | Process gates before commit under CLAUDE.md. Not run here per instruction; tcms-45 and tcms-46 open. Owner must run them before commit. |
| Workspace-wide fmt, clippy, all-features test (07-05 deferred rows) | No | Per-crate `fmt --check -p tcms` passes here; `cargo test -p tcms` passes. |

## Documentation residue (non-blocking)

- 07-05-SUMMARY.md swaps the two e2e counts: it lists locate e2e as 18 and export-ids e2e as 21. The files show locate 21 and export-ids 18.
- 06-VERIFICATION.md previously cited `strictdoc.rs:132` and `:25`. Those lines are now `read_export` at `:178` and the `documents` field at `:36`.
- LOOK-01..07 checkboxes in `.planning/REQUIREMENTS.md` remain `[ ]` / Pending. Close-out bookkeeping for the phase owner.

## Human verification (not blocking)

- Live Jira Cloud or Data Center run of `tcms requirements locate` and `export-ids`. All Jira tests use the mock, as 06-03 specifies. The live check is an integration confirmation outside the phase's test contract, so it is recorded here and does not hold the status at human_needed.

## Anti-pattern scan

- Debt markers (TBD, FIXME, XXX) in the phase 6 and 7 source, test, and fixture files checked here (locate.rs, export_ids.rs, reqcmd.rs, strictdoc.rs, jira.rs, both strictdoc e2e test files, fixtures README): none.

---

_Verified: 2026-11-02_
_Verifier: Claude (gsd-verifier)_
