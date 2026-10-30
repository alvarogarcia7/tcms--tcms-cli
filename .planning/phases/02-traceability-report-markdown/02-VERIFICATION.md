---
phase: 02-traceability-report-markdown
verified: 2026-10-30T21:20:11Z
status: gaps_found
score: 13/16 must-haves verified
overrides_applied: 0
deferred:
  - truth: "TRC-03: document grouping of requirements from a real StrictDoc export (--requirements-export)"
    addressed_in: "Phase 7"
    evidence: "Phase 7 goal: 'The StrictDoc reader first reads the layout StrictDoc really writes (DECISIONS.md Q1-Q3)'; plan 07-01 rewrites requirement_import::strictdoc::export_items, which is the reader the report uses (crates/test-executor/src/main.rs:584)"
gaps:
  - truth: "Header shows 'not recorded' for every missing value (OUT-04, roadmap SC5)"
    status: failed
    reason: "A blank (empty or whitespace-only) baseline_id or matrix id is rendered as an empty cell, not as 'not recorded'. A missing baseline shows 'not recorded' only when the YAML key is absent (None). The inputs summary shows '(baseline: )'."
    artifacts:
      - path: "crates/testcase-reporting/src/model.rs"
        issue: "Line 261 copies matrix.baseline_id with no blank filter (Some(\"\") passes through)"
      - path: "crates/testcase-reporting/src/render_markdown.rs"
        issue: "Lines 53 and 55-59: Matrix cell is cell(&matrix_id) with no blank check; Baseline cell maps Some(\"\") to an empty string"
      - path: "crates/testcase-reporting/src/lib.rs"
        issue: "Lines 17-22: input summary uses unwrap_or(\"not recorded\"), so Some(\"\") prints '(baseline: )'"
    missing:
      - "Treat a blank (trim().is_empty()) baseline_id as None in build_model (model.rs:261), and render a blank matrix id as 'not recorded'"
      - "Make input_summary (lib.rs:19-21) use the same blank check"
      - "Add a CLI test with baseline_id: \"\" that expects '| Baseline | not recorded |'"
---

# Phase 2: Traceability Report (Markdown) Verification Report

**Phase Goal:** A user can generate a Markdown traceability report showing, for every requirement, its test cases, their verdicts, and an honest verification status.
**Verified:** 2026-10-30T21:20:11Z
**Status:** gaps_found
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
| - | ----- | ------ | -------- |
| SC1 | Report lists each requirement with its linked test cases, each case's verdict and the run it came from; HLR and LLR appear distinctly | VERIFIED | `render_markdown.rs:102-141` (row and cell `ID (outcome, run)`); test `verified_requirement_shows_each_passing_case` (`report_cli.rs:554`); `hlr_and_llr_levels_are_shown_distinctly` (`report_cli.rs:769`); `RequirementLevel` has only `Hlr`/`Llr` (`trace_matrix.rs:24-31`). Real binary run: `T2 (pass, RUN-1)`, `HLR-1 ... HLR`, `LLR-2 ... LLR` |
| SC2 | VERIFIED only if every linked case is `pass`; otherwise FAILED, ERRORED, NOT RUN, PARTIAL or NO TESTS per the documented rule | VERIFIED | Rule `roll_up` at `rollup.rs:84-98` (error > fail > all-pass > some-pass > none-pass; empty = NO TESTS); exhaustive test over all slices of length 0-4 and a permutation test in `rollup.rs` tests; CLI tests `failed_requirement_shows_failed` (571), `errored_ranks_above_failed` (586), `partial_when_some_pass_and_the_rest_have_no_result` (625), `requirement_without_cases_shows_no_tests` (641); rule table in `docs/TRACEABILITY.md:205-225` |
| SC3 | Coverage (normal-range, robustness) shown separately from status; a linked case with no result shows NOT RUN | VERIFIED | Yes/no columns `render_markdown.rs:107-108`, status is a separate column; `coverage_is_shown_apart_from_status` (660); missing or `not_executed` result maps to NotRun at `rollup.rs:104-111`; `not_run_case_is_never_a_pass` (607); real binary: `LLR-3 ... NOT RUN ... T4 (NOT RUN, no result)` |
| SC4 | Obsolete test cases excluded from roll-up and counted in the header | VERIFIED | Filter `model.rs:130-136` and `151-159`; header count `model.rs:267`; `obsolete_case_is_excluded_and_counted_with_test_cases` (`report_cli.rs:678`) asserts `1` with `--test-cases` and `not recorded` without it (design: count unknown without the case list) |
| SC5 | Same inputs and `--generated-at` give byte-identical output; header shows matrix, baseline, run, build, environment, tester, generation time, with "not recorded" for missing values | FAILED (partial) | Byte identity: `same_inputs_and_generated_at_give_identical_bytes` (884), pipeline snapshot test in `render_markdown.rs`, library has no clock (grep: 0 hits in model, render, rollup); real binary run: `cmp` identical. Header fields: `header_records_inputs_and_generation_time` (851). **Gap:** blank `baseline_id` or matrix id renders as an empty cell, not "not recorded" (see gaps) |

**Score:** 4 of 5 success criteria fully verified; SC5 failed on the blank-value sub-case only.

### Requirements Coverage

| Requirement | Description | Status | Evidence |
| ----------- | ----------- | ------ | -------- |
| ROLL-01 | Coverage separate from verification status | VERIFIED | `render_markdown.rs:107-108`; `coverage_is_shown_apart_from_status` |
| ROLL-02 | Documented status rule; VERIFIED only if all linked pass | VERIFIED | `rollup.rs:84-98`; `docs/TRACEABILITY.md:205-225`; rollup tests |
| ROLL-03 | Obsolete excluded from roll-up, counted in header | VERIFIED | `model.rs:130-159, 267`; `obsolete_case_is_excluded_and_counted_with_test_cases` |
| ROLL-04 | Linked case with no result is NOT RUN, never pass | VERIFIED | `rollup.rs:104-111`; `not_run_case_is_never_a_pass` |
| TRC-01 | REQ, linked cases, verdicts, run | VERIFIED | See SC1 |
| TRC-02 | Levels from matrix; HLR and LLR distinct | VERIFIED | `model.rs:90` (`level` copied from link); `hlr_and_llr_levels_are_shown_distinctly` |
| TRC-03 | Rows are normative requirements only; grouped by source document; "No traceable content" | UNCERTAIN (WARNING) | Rows come from matrix links, whose level type admits only HLR/LLR (`trace_matrix.rs:24-31`), so sections cannot become rows. "No traceable content" at `render_markdown.rs:92-93`; tested with a fixture (`documents_group_rows_and_mark_a_document_without_requirements`, `report_cli.rs:793`). The grouping input is read by the StrictDoc reader, whose layout `DECISIONS.md:68` says is wrong for real exports. Deferred to Phase 7 (see WARNING W1) |
| TRC-04 | Fixed test-type columns; cells list cases with verdicts; empty columns shown | VERIFIED | `render_markdown.rs:11, 111-113, 119-141` (header always lists the three columns, empty shows `-`); `test_types_fill_their_columns` (721) |
| TRC-05 | Deterministic row, column and case order | VERIFIED | `model.rs:148` (case IDs sorted), `191-233` (sections by name, rows by ID); `model.rs:754` shuffled-input test; `shuffled_inputs_render_the_same_bytes` in `render_markdown.rs` |
| OUT-03 | Same inputs and `--generated-at` give identical bytes | VERIFIED | `report_cli.rs:884`; `main.rs:618` is the only clock read (used only without the flag); library has none |
| OUT-04 | Header records matrix, baseline, runs, builds, environments, testers, generation time; missing shown as "not recorded" | FAILED | Fields present (`header_records_inputs_and_generation_time`). Blank-value sub-case fails: `model.rs:261`, `render_markdown.rs:53-59`, `lib.rs:19-21` |

**Requirements score:** 13 of 16 truths verified (SC1-SC4, ROLL-01..04, TRC-01, TRC-02, TRC-04, TRC-05, OUT-03). Failed: SC5 and OUT-04 (same blank-value gap). Uncertain: TRC-03 (WARNING).

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/testcase-reporting/src/rollup.rs` | Roll-up rule and case outcome | VERIFIED | 293 lines; logic at 84-111; no stubs |
| `crates/testcase-reporting/src/model.rs` | ReportModel, header, rows, obsolete, ordering | VERIFIED (with gap) | 820 lines; blank-value gap at 261 |
| `crates/testcase-reporting/src/render_markdown.rs` | Markdown renderer | VERIFIED (with gap) | 553 lines; blank-value gap at 53-59 |
| `crates/testcase-reporting/src/lib.rs` | Input summary, module exports | VERIFIED (with gap) | Blank-baseline text at 19-21 |
| `crates/test-executor/src/main.rs` | `report` wiring | VERIFIED | `report` at 631-691; `generated_at_value` at 610-620; `read_requirement_documents` at 579-608 |
| `crates/test-executor/tests/report_cli.rs` | CLI tests | VERIFIED | 33 tests pass |
| `docs/TRACEABILITY.md` | Documented roll-up rule and layout | VERIFIED | Status table at 205-225 |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `main.rs report` | `load_inputs` / `select_results` | function call (`main.rs:635-636`) | WIRED | Real binary run loads and selects runs |
| `main.rs report` | `build_model` then `render_markdown` | calls at `main.rs:662, 669` | WIRED | Real binary output is the Markdown report |
| `build_model` | `roll_up` / `case_outcome` | `model.rs:156, 168` | WIRED | Verdicts from selection drive status |
| `main.rs report` | `read_requirement_documents` | `main.rs:646-648` | WIRED (input layout unverified, W1) | Uses `export_items` (`main.rs:584`) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `render_markdown.rs` table rows | `RequirementRow.cases`, `status` | `build_model` from matrix links and `select_results` over loaded run YAML | Yes (verified with a real run: statuses and verdicts match inputs) | FLOWING |
| Header | matrix, runs, builds, environments, testers | Loaded matrix and run records | Yes | FLOWING (blank-value gap as above) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Real binary produces the traceability report with correct statuses | `test-executor --no-audit report --matrix /tmp/p2verify/matrix.yaml --runs /tmp/p2verify/runs --generated-at 2026-10-31T09:00:00Z` | exit 0; HLR-1 VERIFIED, LLR-2 FAILED (T3 fail), LLR-3 NOT RUN | PASS |
| Two runs with the same inputs are byte-identical | same command run twice, `cmp` | IDENTICAL | PASS |
| Blank baseline shows "not recorded" | same command with `baseline_id: ""` | `| Baseline |  |` and `(baseline: )` | FAIL (gap) |

Fixture files were written under `/tmp/p2verify`, outside the repository. Nothing in the repository was modified by the spot-checks.

### Command Results

| Command | Result |
| ------- | ------ |
| `cargo test -p testcase-reporting -p test-executor` | 133 passed, 0 failed. test-executor unit 6; `coverage_import_cli` 3; `independence_cli` 3; `regression_select_cli` 4; `report_cli` 33; `trace_matrix_cli` 11; testcase-reporting unit 73; doc-tests 0 |
| `cargo clippy -p testcase-reporting -p test-executor -- -D warnings` | exit 0 |
| `cargo fmt --check -p testcase-reporting -p test-executor` | exit 0 |

Not run (out of the requested command set): `cargo test --all-features`, `make coverage-e2e`, `make acceptance-test`. CLAUDE.md requires these before commit; they are not part of this verification.

### Probe Execution

No probes are declared for this phase, and it is not a migration or tooling phase. Skipped.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/test-executor/src/main.rs` | 673, 683 | "renderer not yet available" lines appended after the traceability report when the default (all kinds) is used | INFO | With no `--kind`, the output is the Markdown report followed by three placeholder lines. The traceability part is correct. Clean up when Phase 3 lands |
| `crates/testcase-reporting/src/model.rs` | 261 | Blank baseline passed through | WARNING (part of gap G1) | See gaps |
| No `TBD`, `FIXME` or `XXX` markers in any Phase 2 file | - | Debt-marker gate | PASS | Phase 2 changed files: `rollup.rs`, `model.rs`, `render_markdown.rs`, `lib.rs`, `main.rs`, `report_cli.rs`, `TRACEABILITY.md` |

### Human Verification Required

None. All Phase 2 truths are checkable from code, tests and the real binary.

### Gaps Summary

**G1 (blocks SC5 and OUT-04, BLOCKER):** A blank `baseline_id` (or a blank matrix `id`) renders as an empty cell instead of "not recorded". The rule in OUT-04 is that a missing value shows "not recorded". A blank value in YAML is a missing value in practice. The fix is small: filter blank strings to `None` in `build_model` and render blank matrix IDs as "not recorded". Add a test. Everything else in SC1-SC4 and the requirements listed above is verified by code, tests and the real binary. If the team decides a blank string is not "missing", record an override in this file's frontmatter for OUT-04.

**W1 (WARNING, deferred to Phase 7):** `--requirements-export` grouping reads a StrictDoc layout that `.planning/DECISIONS.md:68` says does not match real StrictDoc exports. The TRC-03 document grouping, and the "No traceable content" section, are proven only against the assumed fixture. Phase 7 replaces this reader. Until it lands, do not rely on `--requirements-export` with real exports.

---

_Verified: 2026-10-30T21:20:11Z_
_Verifier: Claude (gsd-verifier)_
