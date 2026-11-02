---
phase: 04-asciidoc-and-machine-readable-output
verified: 2026-11-02T13:00:00Z
status: passed
score: 3/3 roadmap success criteria verified; 2 backlog acceptance criteria NOT VERIFIED (tcms-43.4.2 #1, tcms-43.4.3 #2)
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/3 roadmap; 4 of 9 tcms-43.4 ACs NOT VERIFIED
  gaps_closed:
    - "Shape A and B validated before write (tcms-43.4 AC #5), via plan 04-05 commits 8427cd1, 2441ce7, 6558812"
    - "E2E results recorded in a SUMMARY (tcms-43.4.3 AC #3), via 04-E2E-SUMMARY.md (commit 2fac1a7)"
  gaps_remaining:
    - "tcms-43.4.2 AC #1: shared schema schemas/tcms-envelope.schema.json still edited by 57da2e7"
    - "tcms-43.4.3 AC #2: no mk/incremental.mk mapping for report_formats_e2e.rs"
  regressions: []
gaps:
  - truth: "Shared schemas are not modified (tcms-43.4.2 AC #1)"
    status: failed
    reason: "Commit 57da2e7 adds 'report' to the type enum of schemas/tcms-envelope.schema.json. Acceptance criterion says shared schemas unchanged."
    artifacts:
      - path: "schemas/tcms-envelope.schema.json"
        issue: "type enum gains 'report' (diff at 57da2e7)"
    missing:
      - "Revert the envelope enum change, or record a decision accepting it and amend tcms-43.4.2 AC #1"
  - truth: "Phase 4 E2E tests are mapped in mk/incremental.mk (tcms-43.4.3 AC #2)"
    status: failed
    reason: "crates/test-executor/tests/report_formats_e2e.rs has no mk/incremental.mk entry. 04-E2E-SUMMARY.md (D-04-8) says none is needed because cargo test runs it. The AC still requires the mapping."
    artifacts:
      - path: "mk/incremental.mk"
        issue: "No entry for report_formats_e2e.rs or report_shapes_e2e.rs"
    missing:
      - "Add the mapping, or amend AC #2 to match D-04-8 (Rust test covered by cargo test; bash 3.2 clause not applicable)"
deferred: []
human_verification: []
---

# Phase 4: AsciiDoc and Machine-Readable Output Verification Report

**Phase Goal:** A user can get every report as AsciiDoc and as JSON or YAML that has been validated against a local report schema.
**Verified:** 2026-11-02T13:00:00Z
**Status:** gaps_found
**Re-verification:** Yes, after plan 04-05 gap closure (HEAD 8d51cdb)

## Gate Results

| Gate | Command | Result |
| ---- | ------- | ------ |
| Tests | `env -u CARGO_TARGET_DIR cargo test -p testcase-reporting -p test-executor` | All suites report `ok`, no FAILED lines. report_formats_e2e 12/12, report_shapes_e2e 5/5 (per 04-05 red-to-green record; suite counts in run output). |
| Format | `env -u CARGO_TARGET_DIR cargo fmt --check -p testcase-reporting -p test-executor` | Exit 0 |
| Clippy (not fixed) | `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | Not re-run in this pass. Known pre-existing failure: `if` with identical blocks at crates/testcase-execution/src/executor.rs:2351 (dependency, out of scope). 04-05 records `--no-deps` clippy exit 0 for the two in-scope crates. |

## Roadmap Success Criteria

| # | Criterion | Status | Evidence |
| - | --------- | ------ | -------- |
| 1 | Markdown or AsciiDoc, all kinds, hostile strings escaped in cells | VERIFIED | `escape_adoc` at crates/testcase-reporting/src/render_asciidoc.rs:46; `hostile_strings_keep_structure_in_all_six_renderers` at render_asciidoc_shapes.rs:1006; E2E `asciidoc_hostile_requirement_ids_are_escaped_one_cell_per_line` (report_formats_e2e.rs:245) |
| 2 | JSON/YAML validated against local schema before write; failure writes no file | VERIFIED | `render_json`/`render_yaml` call `validate_document` (render_data.rs:44, :56). CLI validates shape documents before any write (crates/test-executor/src/main.rs:829-844) and `render_formatted` runs before `fs::write`. E2E `failed_validation_writes_no_file_and_keeps_an_existing_one` (report_formats_e2e.rs:475); `shape_failed_validation_writes_no_file_for_every_format` (report_shapes_e2e.rs:318) |
| 3 | Determinism for every format | VERIFIED | E2E `every_format_is_byte_identical_for_the_same_inputs` (report_formats_e2e.rs:437) |

## Requirements

| Requirement | Status | Evidence |
| ----------- | ------ | -------- |
| OUT-01 | Met by the code. REQUIREMENTS.md still shows Pending; not edited. | SC1 |
| OUT-02 | Met by the code. REQUIREMENTS.md still shows Pending; not edited. | SC2, SC3 |

## Shape A and B Validation Before Write (tcms-43.4 AC #5)

**VERIFIED.**
- Shape documents are built with `build_shape_document` (testcase-reporting `report_document`) for A and B (main.rs:829-838).
- They are checked against `schemas/tcms/report.schema.v1.json` via `check_document` (render_data.rs:113) at main.rs:840-844. This runs before the `format != Markdown` branch and before any write, so it covers json, yaml, asciidoc and markdown.
- Tests: `check_document_accepts_a_valid_shape_a_document` and `..._shape_b_...` (render_data.rs:984, :994), `check_document_rejects_a_schema_mismatch_with_messages` (:1004), `shape_a_and_b_json_and_yaml_write_valid_documents` (report_shapes_e2e.rs:135, validates written file), `shape_failed_validation_writes_no_file_for_every_format` (report_shapes_e2e.rs:318).
- Caveat (not a gap): for Markdown shapes the validated document is a separate build from the rendered `render_shape_a`/`render_shape_b` text (main.rs:859-862). The validated structure is the same model, but the bytes written are not themselves schema-checked.

## Backlog Acceptance Criteria

| Task | AC | Status | Evidence |
| ---- | -- | ------ | -------- |
| tcms-43.4 #1 | All kinds in Markdown and AsciiDoc, hostile strings escaped | VERIFIED | SC1 |
| tcms-43.4 #2 | JSON/YAML validated before write; failure writes no file | VERIFIED | SC2 |
| tcms-43.4 #3 | Determinism for every format | VERIFIED | SC3 |
| tcms-43.4 #5 | Both shapes schema-validated before write | VERIFIED | main.rs:829-844; report_shapes_e2e.rs:135, :318 |
| tcms-43.4.1 #1 | All kinds render in AsciiDoc | VERIFIED | render_asciidoc_shapes.rs:1006; report_formats_e2e.rs:293 |
| tcms-43.4.1 #2 | Hostile strings escaped in cells | VERIFIED | render_asciidoc.rs:46; report_formats_e2e.rs:245 |
| tcms-43.4.2 #1 | Local schema added; shared schemas unchanged | NOT VERIFIED | schemas/tcms/report.schema.v1.json added, but schemas/tcms-envelope.schema.json changed in 57da2e7 (type enum gains `report`) |
| tcms-43.4.2 #2 | Validation failure writes no file | VERIFIED | report_formats_e2e.rs:475 |
| tcms-43.4.3 #1 | E2E runs real binary, asserts exact lines and exit codes | VERIFIED | report_formats_e2e.rs (`CARGO_BIN_EXE_test-executor`, `assert_has_line`); report_formats_cli.rs |
| tcms-43.4.3 #2 | Bash 3.2 and BSD/GNU safe, mapped in mk/incremental.mk | NOT VERIFIED | Test is Rust, so the bash clause is not applicable. No mk/incremental.mk entry for report_formats_e2e.rs or report_shapes_e2e.rs. Decision D-04-8 in 04-E2E-SUMMARY.md says no mapping is needed. |
| tcms-43.4.3 #3 | Results recorded in phase SUMMARY before verification | VERIFIED | 04-E2E-SUMMARY.md records commit ca70196 and report_formats_e2e.rs (committed 2fac1a7) |

## Warnings

- ROADMAP.md Phase 4 text still says summary, gaps and problems are deferred. Plans 04-01 and 04-02 implement all four kinds. Roadmap text is stale.
- Clippy gate cannot pass at HEAD because of the pre-existing testcase-execution lint (executor.rs:2351). Not fixed, per instruction.

## Human Verification

None required for the criteria above.

## Gaps Summary

All three roadmap success criteria are met, and shape A/B validation before write (tcms-43.4 #5) is now verified. Two backlog criteria remain unmet: the shared envelope schema was edited (43.4.2 #1), and the Rust E2E tests have no mk/incremental.mk mapping (43.4.3 #2). Either fix or amend the criteria with a recorded decision; an override can be recorded in frontmatter if the deviation is accepted.

---

_Verified: 2026-11-02T13:00:00Z_
_Verifier: Claude (gsd-verifier)_
