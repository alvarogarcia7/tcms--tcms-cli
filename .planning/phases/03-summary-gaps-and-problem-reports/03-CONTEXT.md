# Phase 3: Summary, Gaps and Problem Reports - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning
**Source:** REQUIREMENTS.md (SUM-01, GAP-01, PRB-01, PRB-02), SUMMARY.md, and the decisions in 01-CONTEXT.md and 02-CONTEXT.md

<domain>
## Phase Boundary

Three more report kinds from `test-executor report`: `summary`, `gaps` and `problems`. All three are
views over the same ReportModel that Phase 2 builds, so their numbers agree with the traceability
report. Markdown only (AsciiDoc, JSON and YAML are Phase 4).

</domain>

<decisions>
## Implementation Decisions

- **D-01:** The ReportModel and the roll-up rule come from Phase 2 (02-CONTEXT D-02). Phase 3 does
  not change them; it only reads them.
- **D-02 (summary, SUM-01):** Counts of `pass`, `fail`, `not_executed` and `error` results, and the
  total, per run and per requirement. The four buckets sum to the total. Results are the decided
  results of Phase 1 selection (01-CONTEXT D-01..D-05), so a superseded result is not counted.
- **D-03 (gaps, GAP-01):** Four lists: requirements with no test cases; requirements missing a
  normal-range test; requirements missing a robustness test; test cases with no recorded result.
  The gap list comes from the requirement set, so a requirement with no tests is listed (confirmed in
  Phase 1 research, D-12).
- **D-04 (problems, PRB-01):** Every `fail` and `error` result with its `problem_report_id`.
- **D-05 (problems, PRB-02):** A `fail` or `error` result with no `problem_report_id` is flagged
  visibly in the row, not hidden.
- **D-06:** Each view prints the same header as Phase 2 (D-11), and is deterministic (D-12 of
  Phase 2: injected clock, sorted collections).
- **D-07:** `test-executor report --kind summary|gaps|problems` renders the view. Phase 2's
  "renderer not yet available" line is removed for these kinds.

### Claude's Discretion
- Table layout and column order within the stated fields.

</decisions>

<canonical_refs>
- `.planning/REQUIREMENTS.md` - SUM-01, GAP-01, PRB-01, PRB-02
- `.planning/ROADMAP.md` - Phase 3 goal and success criteria
- `.planning/phases/02-traceability-report-markdown/02-CONTEXT.md` - ReportModel and roll-up (planned in parallel; read its plans if present)
- `.planning/phases/01-strict-input-loading-and-reconciliation/01-CONTEXT.md` - run selection
- `crates/testcase-reporting/src/` - Phase 1 loader and selection
</canonical_refs>

<deferred>
- AsciiDoc, JSON, YAML: Phase 4.
- Multi-run history: v2.
</deferred>
