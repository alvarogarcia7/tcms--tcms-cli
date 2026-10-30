# Phase 2: Traceability Report (Markdown) - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning
**Source:** Requirements and decisions recorded in .planning/REQUIREMENTS.md and 01-CONTEXT.md (run selection, confirmed 2026-10-30)

<domain>
## Phase Boundary

`test-executor report` gets a Markdown traceability report. For every requirement it shows the
linked test cases, each case's verdict and source run, and an honest verification status. Other
report kinds (summary, gaps, problems) and AsciiDoc or JSON output are Phases 3 and 4.

</domain>

<decisions>
## Implementation Decisions

### Run selection (inherited from Phase 1, confirmed)
- **D-01:** Phase 1 selection is used unchanged: for each test case the latest executed result
  (by `started_at`) decides; `not_executed` and omitted cases do not count (01-CONTEXT D-01..D-05).

### Verification status roll-up (ROLL-02)
- **D-02:** A requirement's status is computed over its linked live test cases (obsolete cases are
  excluded, ROLL-03), using each case's decided result:
  - `NO TESTS`: no live test case is linked to the requirement.
  - `NOT RUN`: linked cases exist, but none has an executed result.
  - `VERIFIED`: every linked live case has an executed result and all are `pass`.
  - `ERRORED`: at least one linked case has an `error` result (checked before `FAILED`).
  - `FAILED`: at least one linked case has a `fail` result, and none is `error`.
  - `PARTIAL`: some linked cases have a `pass` result, and the rest have no executed result (no
  `fail` or `error`).
  - (Definitions of the mixed cases are the planning assumption; confirm.)
- **D-03:** `ERRORED` ranks above `FAILED` (carried over from 01-CONTEXT D-13, still unconfirmed).

### Coverage and status (ROLL-01, ROLL-04)
- **D-04:** Coverage (normal-range present, robustness present) is shown in its own columns, never
  folded into the status.
- **D-05:** A linked live case with no executed result is shown as `NOT RUN` on its row, never as
  passed.

### Obsolete cases (ROLL-03)
- **D-06:** Obsolete test cases are excluded from the roll-up and counted in the report header.

### Report layout (TRC-01..TRC-05, from REQUIREMENTS)
- **D-07:** Rows are normative requirements only, grouped under a heading per source document;
  a document with no normative requirement shows "No traceable content" (TRC-03).
- **D-08:** Columns are the test types in fixed order: `normal_range`, `robustness`, `other`. Each
  cell lists the linked test cases with their verdicts. An empty column is still shown (TRC-04).
- **D-09:** HLR and LLR requirements are kept distinct (TRC-02). Requirement IDs and levels come
  from the trace matrix.
- **D-10:** The traceability section shows, per requirement, its linked cases, each verdict and the
  run it came from (TRC-01).

### Header and determinism (OUT-03, OUT-04)
- **D-11:** The header records the matrix ID, baseline, run IDs, build IDs, environments, testers,
  and the generation time. A missing value is written as "not recorded".
- **D-12:** Output is byte-identical for the same inputs and the same `--generated-at`. The clock is
  injected; collections are sorted by document, requirement ID, then test case ID.

### Output (Phase 2 scope)
- **D-13:** The only output format in this phase is Markdown. AsciiDoc, JSON and YAML are Phase 4.

### Claude's Discretion
- Exact column widths and table syntax, provided D-07 to D-12 hold.
- Internal module split inside `testcase-reporting` (a `model` module and a `render_markdown`
  module are expected from the research in SUMMARY.md).

</decisions>

<canonical_refs>
## Canonical References

- `.planning/REQUIREMENTS.md` - ROLL-01..04, TRC-01..05, OUT-03, OUT-04
- `.planning/ROADMAP.md` - Phase 2 goal and success criteria
- `.planning/research/SUMMARY.md` - LOAD -> MODEL -> RENDER pipeline and roll-up pitfall
- `.planning/phases/01-strict-input-loading-and-reconciliation/01-CONTEXT.md` - run selection (D-01..D-05, D-13)
- `.planning/phases/01-strict-input-loading-and-reconciliation/01-0{2,3,4}-SUMMARY.md` - what Phase 1 built
- `crates/testcase-reporting/src/` - loader, selection and CLI from Phase 1
- `crates/testcase-verification/src/trace_matrix.rs` - `build_matrix` (links and coverage flags)
- `docs/TRACEABILITY.md` - documented report inputs
- `schemas/tcms-schemas/tcms/trace-matrix.schema.json`, `test-run.schema.json`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `testcase-reporting::select` (Phase 1): per-case decided result with reasons.
- `testcase-reporting::load` (Phase 1): strict loading and reconciliation block.
- `build_matrix` already computes linked cases, coverage flags and run IDs.

### Established Patterns
- Pure functions in `testcase-reporting`, CLI in `test-executor` (Phase 1 layout).
- Table-driven tests for rules; snapshot-style tests for rendered output.

### Integration Points
- `test-executor report --kind traceability` currently prints "renderer not yet available";
  Phase 2 replaces that line for `traceability` only.

</code_context>

<specifics>
## Specific Ideas

- A requirement with `VERIFIED` must show every linked case as `pass` in the table, so the reader
  can check the basis of the status.

</specifics>

<deferred>
## Deferred Ideas

- Summary, gaps and problem-report kinds: Phase 3.
- AsciiDoc, JSON and YAML output: Phase 4.
- Staleness (STALE-01) and multi-run history (HIST-01): v2.
