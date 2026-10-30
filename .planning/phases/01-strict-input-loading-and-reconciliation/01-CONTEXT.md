# Phase 1: Strict Input Loading and Reconciliation - Context

**Gathered:** 2026-10-30
**Status:** Ready for planning
**Source:** Decisions taken in discussion on 2026-10-30 (not an interactive discuss-phase run)

<domain>
## Phase Boundary

`test-executor report` loads a trace matrix and test-run records, fails closed on bad input,
selects one result per test case, and prints what it used and skipped. No report kinds are
rendered in this phase beyond the command skeleton (CLI-01). Verdict roll-up and rendering are
Phase 2 and later.

</domain>

<decisions>
## Implementation Decisions

### Run selection (LOAD-03)
- **D-01:** For each test case, the latest run (by `started_at`, RFC 3339) that has an executed
  result for it decides.
- **D-02:** A later `pass` overrides an earlier `fail` or `error`. A later `fail` or `error`
  overrides an earlier `pass`.
- **D-03:** A run that omits the test case, or records `not_executed` for it, does not change the
  result.
- **D-04:** Runs are not restricted to one build. The report lists every build it used.
- **D-05:** The report lists the runs it used and the runs it excluded.

### Input validation (LOAD-01, LOAD-02, LOAD-04)
- **D-06:** Every matrix and run input is read through `testcase_common::load_and_validate_yaml`.
  A schema failure fails the command and names the file.
- **D-07:** A reconciliation block lists inputs discovered, loaded, and skipped with a reason. No
  input is dropped silently.
- **D-08:** Two run records with the same `run_id` fail the command, naming both files.

### Crate and command placement (CLI-01)
- **D-09:** New Layer 3 crate `testcase-reporting`. Only `test-executor` depends on it.
- **D-10:** The `report` subcommand sits next to `trace-matrix` in `test-executor`.
- **D-11:** Loaders move out of `crates/test-executor/src/main.rs` into `testcase-reporting`
  behaviour-preserving. The existing `trace_matrix_cli` tests must still pass.

### Zero-test requirements (research flag, closed)
- **D-12:** `build_matrix` emits a row for every requirement in the requirements input, including
  requirements with no test cases (`test_case_ids: []`, both coverage flags false). Verified on a
  demo. The gap report therefore needs no separate input, provided the requirement list comes
  from the requirements source.

### Assumptions to confirm
- **D-13 (assumed):** In the roll-up, `ERRORED` ranks above `FAILED` when a requirement's linked
  cases disagree. The owner has not confirmed this.

### Resolved during research (2026-10-30)
- **D-14:** D-11 is refined. `trace-matrix` keeps its current lenient loaders (`read_test_runs`,
  `load_test_cases_strict`) unchanged, and they move to `testcase-reporting` as-is. The `report`
  command uses a new strict loader that validates every input and records each file as loaded
  or skipped with a reason (D-06, D-07). `read_matrix_requirements` stays in `test-executor`,
  because moving it would pull `requirement-import` and `jira-client` into the Layer 3 crate.
- **D-15:** A YAML file in a runs directory without a `type` field is skipped and listed with a
  reason. A YAML syntax error fails the command.
- **D-16:** Runs with equal `started_at` are ordered by `run_id` ascending; the later `run_id` wins.
- **D-17:** `--runs` is required. A duplicate `test_case_id` inside one run fails the command.
- **D-18:** Results for run IDs the matrix names but that were not loaded, and results for test
  cases missing from the matrix, appear as reconciliation notes, not failures.
- **D-19:** The `report` command accepts `--kind traceability|summary|gaps|problems`. Phase 1 prints
  a "renderer not yet available" line for each kind instead of a report body.

### Claude's Discretion
- Exact error wording and the reconciliation layout, provided D-07 and D-08 are met.
- Internal module split inside `testcase-reporting`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Planning
- `.planning/PROJECT.md` - project context, Key Decisions
- `.planning/REQUIREMENTS.md` - LOAD-01..04, CLI-01 are this phase's requirements
- `.planning/ROADMAP.md` - Phase 1 goal and success criteria
- `.planning/research/SUMMARY.md` - architecture (LOAD -> MODEL -> RENDER), pitfalls, decided policies

### Code
- `crates/testcase-verification/src/trace_matrix.rs` - `build_matrix`, `read_requirements`
- `crates/testcase-verification/src/test_run.rs` - `TestRunRecord`, `TestRunResult`, `Verdict::of`
- `crates/test-executor/src/main.rs` - `read_test_runs`, `load_test_cases_strict`, `trace_matrix`
- `crates/test-executor/tests/trace_matrix_cli.rs` - behaviour that must not change

### Schemas and docs
- `schemas/tcms-schemas/tcms/trace-matrix.schema.json`
- `schemas/tcms-schemas/tcms/test-run.schema.json`
- `docs/TRACEABILITY.md`, `docs/RUN_METADATA.md`
- `AGENTS.md` (layering, bash 3.2 rules), `crates/README.md` (layer table)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build_matrix` and `MatrixFindings` in `trace_matrix.rs` already compute links, coverage flags,
  run IDs and unknown-reference findings.
- `TestRunRecord::build` already validates a record before writing.

### Established Patterns
- `load_and_validate_yaml` is the required loader (AGENTS.md).
- Strict loaders fail closed: `load_test_cases_strict` is the pattern to follow.

### Integration Points
- `test-executor` gains a `report` subcommand beside `trace-matrix`.
- `testcase-reporting` depends on `testcase-verification`, `testcase-common` and `testcase-models`.

</code_context>

<specifics>
## Specific Ideas

- Reconciliation block printed in the report header, not only on stderr.
- Demo to exercise D-01 to D-03: one test case with pass in RUN-1, fail in RUN-2, not_executed in
  RUN-3 should resolve to fail, and the report should list RUN-3 as not affecting the result.

</specifics>

<deferred>
## Deferred Ideas

- Verdict roll-up and status rules: Phase 2.
- Staleness check against `source_hashes`: v2 (STALE-01).
- TPDG or HTML rendering: v2 (FOUT-01).
- Adding source hashes to the run record: schema change, out of scope for this milestone.
- Reviewer / independence report: not selected.

</deferred>
