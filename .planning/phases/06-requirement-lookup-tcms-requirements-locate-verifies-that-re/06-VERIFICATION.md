---
phase: 06-requirement-lookup-tcms-requirements-locate-verifies-that-re
verified: 2026-11-02T00:00:00Z
status: gaps_found
score: 7/8 truths verified (LOOK-01..07 plus goal; StrictDoc real-layout truth not verified)
overrides_applied: 0
gaps:
  - truth: "A StrictDoc ID is found in a real StrictDoc export given with --input (LOOK-03, LOOK-07 StrictDoc path, goal clause 'StrictDoc export')"
    status: failed
    reason: "The reader implements the assumed layout (top-level 'documents' array), not the layout StrictDoc writes. DECISIONS.md D-09 is still 'Not verified against a real StrictDoc export'. A real export has no 'documents' key and would be refused with exit 2. Tests use a hand-written fixture only. Phase 7 (D-07-3) is the planned fix and is listed as deferred below."
    artifacts:
      - path: "crates/requirement-import/src/strictdoc.rs"
        issue: "read_export (line 132) and struct Export (line 25) assume the 'documents' layout"
    missing:
      - "Phase 7 reader fix (DOCUMENTS/NODES layout) and e2e tests on real strictdoc exports, or an explicit owner decision to accept the assumed layout for Phase 6"
deferred:
  - truth: "StrictDoc real export layout read correctly"
    addressed_in: "Phase 7"
    evidence: "ROADMAP Phase 7 goal: 'The StrictDoc reader first reads the layout StrictDoc really writes'; DECISIONS.md D-07-3 'Supersedes Phase 6 D-09; 07-01'"
human_verification:
  - test: "Run tcms requirements locate and export-ids against a live Jira Cloud or Data Center instance"
    expected: "Found and not-found lines match the real backlog; exit codes 0/1/2 as specified"
    why_human: "Tests use the mock only (06-03, 06-05 summaries)"
---

# Phase 6: Requirement lookup Verification Report

**Phase Goal:** `tcms requirements locate <ID>...` shows, for each ID, whether it is a requirement in the configured Jira backlog or a StrictDoc export, with non-zero exit when any ID is missing.
**Verified:** 2026-11-02
**Status:** gaps_found
**Re-verification:** No

Method: goal-backward, from LOOK-01..07 and the goal text. Plan summaries were used only to locate claims; each claim was checked in code and by running the tests. ROADMAP Phase 6 has no separate success-criteria list, so the goal and LOOK-01..07 are the contract.

## Commands run

| Command | Result |
|---------|--------|
| `env -u CARGO_TARGET_DIR cargo test -p tcms` | Exit 0. 64 unit, ci_template 7, jira_cli 5, locate_cli 10, pr_cli 5, ref_cli 5, requirements_cli 7, requirements_export_ids_cli 18, requirements_locate_cli 21, sync_cli 15, vv_metrics_cli 19. 0 failed. |
| `env -u CARGO_TARGET_DIR cargo fmt --check -p tcms` | Exit 0 |
| `env -u CARGO_TARGET_DIR cargo clippy -p tcms --all-targets -- -D warnings` | Exit 0. The known testcase-execution `executor.rs:2435` lint did not fire for `-p tcms`. |
| make test, make acceptance-test, make coverage-e2e | Not run, per instruction (tcms-45, tcms-46 open). |

## Goal and LOOK requirements

| Req | Status | Evidence |
|-----|--------|----------|
| Goal (per-ID found/not-found, non-zero exit when missing) | VERIFIED for Jira and for a StrictDoc export in the assumed layout. Real-layout StrictDoc is the gap above. | `locate.rs:184-190` (exit_code), `locate.rs:315-335` (run) |
| LOOK-01 resolve each ID to its system; one batch Jira search; one StrictDoc export | VERIFIED | `locate.rs:196-300` (`lookup`): `Resolver::resolve_id` for each deduped ID before any lookup (D-02); one `jira::jql_for_keys` search per Jira system (D-03); export read once (D-05). Tests: `locate_jira_batch_finds_any_issue_type_{cloud,datacenter}`, `locate_mixed_sources_one_search`, `locate_repeated_id_is_reported_once`. Pagination of the batch search is handled in `crates/jira-client/src/lib.rs:263-300`. |
| LOOK-02 Jira found by key, any issue type; no match = not found | VERIFIED | `locate.rs:85-104` (`jira_matches`: exact key match, no issue-type filter); `jira.rs:75-84` (`jql_for_keys`, no project/type clause); tests `jira_matches_the_exact_key` (cloud/datacenter), `locate_jira_batch_finds_any_issue_type_*` (Bug PAY-3 and OTH-1 outside project_keys). |
| LOOK-03 StrictDoc found by UID in local `--input` export; level shown | VERIFIED against the assumed layout only. NOT VERIFIED against a real StrictDoc export (see gap). | `locate.rs:108-118` (`strictdoc_index`), `locate.rs:120-146` (`render_line` shows level); `strictdoc.rs:113` (`export_items`), `strictdoc.rs:132` (`read_export`, assumed `documents` layout). Tests: `locate_strictdoc_reports_every_level`, `locate_strictdoc_missing_uid_exits_1`, `locate_strictdoc_wrong_layout`. Local file only: `locate.rs:232-238` (`std::fs::read_to_string`), no URL. |
| LOOK-04 exit 0 / 1 / 2 | VERIFIED | `locate.rs:22-26` constants; `locate.rs:184-190`; `locate.rs:315-321` (error to 2). 2: unknown ID format `locate_unknown_id_format_exits_2_naming_the_id`; StrictDoc without input `locate_strictdoc_needs_input`; credentials `locate_jira_needs_the_token`; duplicate UID `locate_strictdoc_duplicate_uid`. Missing config file exits 2 by the same `ctx.load_config()?` path (manual run recorded in 06-02; no dedicated test). |
| LOOK-05 text default, `--format=json`, `--format=plaintext` | VERIFIED | `locate.rs:147-183` (`render`); `locate_format_json`, `locate_format_plaintext`, `locate_text_has_no_colour_when_not_a_terminal`, `locate_rejects_an_unknown_format`. |
| LOOK-06 export-ids reads `requirements.json` (default) or `--requirements`; `product_id` + `requirements[{id}]`; same lookup | VERIFIED | `export_ids.rs:30-42` (strict structs, `deny_unknown_fields`); `export_ids.rs:44-47` (parse); `export_ids.rs:107-123` (`export_ids` uses `locate::lookup`); default `requirements.json` set by `default_value` in `export_ids.rs` (args struct); tests `export_ids_default_path_all_valid_{cloud,datacenter}`, `export_ids_requirements_flag_reads_the_named_file`, `export_ids_extra_fields_are_an_error`, `export_ids_missing_default_file_exits_2`, `export_ids_unparseable_file_exits_2`. |
| LOOK-07 valid IDs to stdout or `--output`; invalid on stderr; exit 0/1/2; product mismatch exit 2 | VERIFIED | `export_ids.rs:50-66` (`check_file`: product mismatch and empty list refused before any lookup); `export_ids.rs:74-87` (`render_output`, found IDs only); `export_ids.rs:125-161` (`run`: stdout via locked writer with error handling, `--output` file, invalid lines to stderr, exit 1 if any invalid, valid IDs still written); `reqcmd.rs:41,103`; `main.rs:63-69`. Tests: `export_ids_product_mismatch_exits_2`, `export_ids_output_file_instead_of_stdout`, `export_ids_output_file_on_exit_1_has_the_valid_ids`, `export_ids_output_file_not_created_on_error`, `export_mixed_valid_and_invalid`, `export_ids_mixed_valid_and_invalid_exits_1_{cloud,datacenter}`, `export_ids_strictdoc_with_input` (assumed layout). |

## Summary NOT MET items, re-checked

| Item | Blocks a requirement? | Finding |
|------|----------------------|---------|
| Make gates (`make build/lint/test/coverage-e2e/acceptance-test`) not run | No, for the requirements. CLAUDE.md requires them before commit. | Not run per instruction (tcms-45, tcms-46 open). Process gate, not a code defect. Owner decision needed before commit. |
| `eprintln!` residual in `export_ids.rs` (lines 129, 138, 149, 155) and `locate.rs` (319, 330) | No | Commit 27900fa fixed the stdout panic (`export_ids.rs:144-151` now uses `writeln` with error handling). `eprintln!` still panics if stderr is closed. Warning only; invalid IDs are listed on stderr as LOOK-07 requires. `locate.rs` still uses `print!` for stdout, so a broken pipe can panic there. |
| One-search grep in 06-05 (criterion "at least 2") | No. Now MET. | Summary said count 1. Current grep for `search_requests(&mock).len(), 1` matches 2 lines (`requirements_export_ids_cli.rs:541` and `:583`). The one-search rule is also asserted at `:267-271`. Resolved by 27900fa. |
| LOOK-01..07 checkboxes in REQUIREMENTS.md still `[ ]` / Pending | No | Close-out bookkeeping, left for the phase close. |

## Gaps

1. **StrictDoc real-layout (BLOCKER for the StrictDoc half of the goal; deferred to Phase 7).** Verified only against the assumed layout. D-09 is still unverified. A real StrictDoc export would fail to parse here (no `documents` key) and exit 2. Phase 7 is the planned fix (D-07-3). The owner must either accept this deferral or require the fix before Phase 6 is closed.

## Human verification

- Live Jira Cloud and Data Center run of `locate` and `export-ids`. The tests use the mock only.

## Anti-pattern scan

- Debt markers (TBD, FIXME, XXX, TODO, HACK, PLACEHOLDER) in all phase files: none.
- No `allow(dead_code)` in `export_ids.rs` (plan criterion).

---

_Verified: 2026-11-02_
_Verifier: Claude (gsd-verifier)_
