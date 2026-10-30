# Requirements: TCMS — Traceability Reports

**Defined:** 2026-10-30
**Core Value:** Every requirement can be traced to the test cases that verify it and to the recorded PASS/FAIL outcome of those tests, with gaps shown explicitly rather than hidden.

## v1 Requirements

Requirements for this milestone. Each maps to a roadmap phase.

### Input Loading (LOAD)

- [ ] **LOAD-01**: Report reads the trace matrix and test-run records, and fails the command if any input does not validate against its schema, naming the file
- [ ] **LOAD-02**: Report shows a reconciliation block (inputs discovered, loaded, skipped with reason) and never drops an input silently
- [ ] **LOAD-03**: Run selection: per test case, the latest run (by `started_at`) that has an executed result for it decides; a later pass overrides an earlier fail or error, a later fail or error overrides an earlier pass; a run that omits the test case or records `not_executed` does not change the result. The report lists the runs it used and the runs it excluded
- [ ] **LOAD-04**: Two test-run records with the same `run_id` cause the command to fail

### Verdict Roll-up (ROLL)

- [ ] **ROLL-01**: Each requirement shows test coverage (normal-range test present, robustness test present) separately from verification status
- [ ] **ROLL-02**: Verification status per requirement is derived from linked test-case verdicts by a documented rule (VERIFIED, FAILED, ERRORED, NOT RUN, PARTIAL, NO TESTS); a requirement is shown as VERIFIED only when every linked test case has verdict `pass`
- [ ] **ROLL-03**: Obsolete test cases are excluded from the roll-up and counted in the report header
- [ ] **ROLL-04**: A linked test case with no result in the selected runs is reported as NOT RUN, never as passed

### Traceability Report (TRC)

- [ ] **TRC-01**: Traceability report lists each requirement with its linked test cases, each test case's verdict, and the run it came from (REQ → test cases → PASS/FAIL)
- [ ] **TRC-02**: Requirement IDs and levels come from the trace matrix, and HLR and LLR requirements are kept distinct in the report
- [ ] **TRC-03**: Traceability matrix rows are the normative requirements only (sections and free text are not rows), grouped under a heading for their source document; a document with no normative requirements is shown as "No traceable content" (as in StrictDoc's traceability matrix screen)
- [ ] **TRC-04**: Matrix columns are the test types (normal_range, robustness, other) in a fixed order; each cell lists the linked test cases with their verdicts, and a column is shown even when empty so gaps are visible
- [ ] **TRC-05**: Matrix row and column order is deterministic (documents, then requirement IDs, then test case IDs), so the same inputs always produce the same table

### Run Summary (SUM)

- [ ] **SUM-01**: Run summary shows counts for each verdict (pass, fail, not_executed, error) and the total, per run and per requirement

### Coverage Gaps (GAP)

- [ ] **GAP-01**: Gaps report lists requirements with no test cases, requirements missing a normal-range test, requirements missing a robustness test, and test cases with no recorded result

### Problem Reports (PRB)

- [ ] **PRB-01**: Problem report list shows each fail and error result with its `problem_report_id`
- [ ] **PRB-02**: A fail or error result without a `problem_report_id` is flagged in the list

### Output (OUT)

- [ ] **OUT-01**: Reports render as Markdown and AsciiDoc
- [ ] **OUT-02**: Reports are also available as JSON or YAML with a local schema, validated before being written
- [ ] **OUT-03**: Output is deterministic: the same inputs and the same `--generated-at` value produce byte-identical output
- [ ] **OUT-04**: Report header records the matrix ID, baseline ID, run IDs, build IDs, environments, testers and generation time; a missing value is shown as "not recorded"

### Command Line (CLI)

- [ ] **CLI-01**: `test-executor report` produces the selected report kinds from a trace matrix and test-run records
- [ ] **CLI-02**: Exit status distinguishes success (0), input or validation error (1), and gaps or failed results when `--fail-on-gaps` is given (2); the report is written before any non-zero exit

### Requirement Lookup (LOOK)

- [ ] **LOOK-01**: `tcms requirements locate <ID>...` resolves each ID to its system through the integration configuration, and checks all Jira IDs in one batch search and all StrictDoc IDs against one export
- [ ] **LOOK-02**: A Jira ID is found when an issue with that key exists in the batch search result, whatever its issue type; a key that matches no issue is reported as not found
- [ ] **LOOK-03**: A StrictDoc ID is found when an item with that UID is in the export given with `--input` (a local file; no URL); the line shows the item's level (for example `HLR`, `LLR`, `system`)
- [ ] **LOOK-04**: The command exits 0 when every ID is found, 1 when any ID is not found, and 2 on an error (an ID that matches no system, a StrictDoc ID without `--input`, a configuration or credential problem)
- [ ] **LOOK-05**: Output is text by default, JSON with `--format=json`, and plain text with `--format=plaintext`
- [ ] **LOOK-06**: `tcms requirements export-ids` reads the product requirements file (`requirements.json` by default, or `--requirements <FILE>`) with a `product_id` and a `requirements` array of `{ "id": ... }` objects, and checks each ID with the same lookup as `locate`
- [ ] **LOOK-07**: `export-ids` writes the valid IDs (those the source finds) as JSON with `product_id` and `requirements: [{ "id": ... }]` to stdout or `--output <FILE>`; invalid IDs are listed on stderr; exit 0 when all are valid, 1 when any is invalid (the valid ones are still written), 2 on an error, including a file whose `product_id` differs from the configured product

## v2 Requirements

Deferred to a future release. Tracked but not in the current roadmap.

### History (HIST)

- **HIST-01**: Report aggregates multiple test campaigns over time (multi-run history, see TCMS-37)
- **HIST-02**: Report detects orphaned results and test cases that no longer exist

### Staleness (STALE)

- **STALE-01**: Report flags a result as STALE when its test case changed after the run, using git commit time or a digest — pending the staleness research in SUMMARY.md

### Further Output (FOUT)

- **FOUT-01**: HTML output via asciidoctor, or via test-plan-documentation-generator if the TPDG spike shows it can render requirement-centric data
- **FOUT-02**: CSV export of the traceability matrix

## Out of Scope

| Feature | Reason |
|---------|--------|
| Requirement import from StrictDoc or Jira | Owned by the separate TCMS-42 integration; reports consume its output |
| Re-executing test cases in report generation | Reports analyse recorded results; execution stays separate |
| Reviewer / independence record report | Not selected for this milestone; the independence check (TCMS-40.8) already enforces the rule during verification |
| Reimplementing trace-matrix or test-run generation | Those outputs exist and are validated (TCMS-40.4, TCMS-40.5) |
| Changes to the shared test-run and trace-matrix schemas | Shared with other tooling; derived fields live in the report's own schema |
| Dashboards | Not part of a V&V evidence deliverable |
| Structural (code) coverage | Different concern; the coverage tooling already exists |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| LOAD-01 | Phase 1 | Pending |
| LOAD-02 | Phase 1 | Pending |
| LOAD-03 | Phase 1 | Pending |
| LOAD-04 | Phase 1 | Pending |
| ROLL-01 | Phase 2 | Pending |
| ROLL-02 | Phase 2 | Pending |
| ROLL-03 | Phase 2 | Pending |
| ROLL-04 | Phase 2 | Pending |
| TRC-01 | Phase 2 | Pending |
| TRC-02 | Phase 2 | Pending |
| TRC-03 | Phase 2 | Pending |
| TRC-04 | Phase 2 | Pending |
| TRC-05 | Phase 2 | Pending |
| SUM-01 | Phase 3 | Pending |
| GAP-01 | Phase 3 | Pending |
| PRB-01 | Phase 3 | Pending |
| PRB-02 | Phase 3 | Pending |
| OUT-01 | Phase 4 | Pending |
| OUT-02 | Phase 4 | Pending |
| OUT-03 | Phase 2 | Pending |
| OUT-04 | Phase 2 | Pending |
| CLI-01 | Phase 1 | Pending |
| CLI-02 | Phase 5 | Pending |
| LOOK-01 | Phase 6 | Pending |
| LOOK-02 | Phase 6 | Pending |
| LOOK-03 | Phase 6 | Pending |
| LOOK-04 | Phase 6 | Pending |
| LOOK-05 | Phase 6 | Pending |
| LOOK-06 | Phase 6 | Pending |
| LOOK-07 | Phase 6 | Pending |

**Coverage:**
- v1 requirements: 30 total
- Mapped to phases: 30
- Unmapped: 0

---
*Requirements defined: 2026-10-30*
*Last updated: 2026-10-30 after roadmap creation*
