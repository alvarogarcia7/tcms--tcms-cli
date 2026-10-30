# Architecture Patterns

**Domain:** Traceability reporting layer on top of an existing TCMS (trace matrix + test-run records)
**Researched:** 2026-10-30
**Overall confidence:** MEDIUM-HIGH (code facts verified in repo; TPDG integration fit is LOW, see below)

## Findings from the code (verified)

- `testcase-verification` (Layer 3) already owns the two inputs and their types:
  - `trace_matrix.rs`: `TraceMatrix`, `TraceLink { requirement_id, level, test_case_ids, normal_range_covered, robustness_covered, run_ids }`, `MatrixFindings { untraced, gaps, unknown_references }`, `build_matrix(...)`, `read_requirements`.
  - `test_run.rs`: `TestRunRecord { run_id, build_id, environment, tester, reviewer, results[] }`, `TestRunResult { test_case_id, verdict, problem_report_id, verification_result_ref }`, `Verdict` (pass/fail/not_executed/error).
- Critical data fact: `TraceLink.run_ids` lists runs only. It carries NO verdicts. A matrix alone cannot answer "REQ -> PASS/FAIL". The report must JOIN the matrix (or rebuild links) with `TestRunRecord.results`. `run_ids` also excludes runs where the case was `not_executed`, so a requirement can have test cases but zero runs, which must render as NOT RUN, not as a gap.
- `MatrixFindings` (untraced, gaps) is produced by `build_matrix` but is NOT serialized into the matrix YAML. The coverage-gaps report can recompute gaps from `TraceMatrix` links alone (flags + empty `test_case_ids`), which preserves the "consume the artifact" rule.
- Glue code sits in `crates/test-executor/src/main.rs`: `read_test_runs`, `read_matrix_requirements`, `load_test_cases_strict`, `trace_matrix(...)`. These are binary-private. A report command needs the run-loading part, so it should move down into a library (see build order step 1).
- Doc generation: TPDG is an external binary, invoked only from shell/Make (`TEST_PLAN_DOC_GEN`), consuming container YAML (test results). No Rust crate in the workspace renders AsciiDoc/Markdown. `tpdg-compat` only checks container compatibility. Traceability data (requirement-centric, not test-result-centric) is not a container, so reuse of TPDG is uncertain (LOW confidence; needs a spike).
- `schemas/tcms-schemas/tcms/` has `trace-matrix`, `test-run`, `problem-report`, `analysis-report` schemas. No report schema exists. `analysis-report` (id, analysis_type, date, conclusion required) is a plausible envelope for a machine-readable report but is shared with other tools; do not change it.

## Recommended Architecture

Add one new Layer 3 library crate, `testcase-reporting`, with a strict three-stage pipeline: LOAD -> MODEL -> RENDER. Expose it through a new subcommand on the existing `test-executor` binary (next to `trace-matrix`), not a new binary.

```
Layer 5  test-executor  `report` subcommand (clap args, file/dir IO, exit codes)
            |
Layer 3  testcase-reporting  (NEW)
            |  load::  read trace matrix + test-run files (schema-validated)
            |  model:: ReportModel (pure data, no IO, no formatting)
            |  render:: markdown | asciidoc | json | yaml  (pure fn(&ReportModel)->String)
            |
Layer 3  testcase-verification  (existing: TraceMatrix, TestRunRecord, Verdict, build_matrix)
Layer 2  testcase-common (load_and_validate_yaml, to_yaml_string)
Layer 1  testcase-models (TestCase, ids)
```

Optional later hop, off the critical path:
`ReportModel -> container-shaped YAML -> TPDG` only if the spike shows TPDG can render it. Native renderers are the default.

### Why a new crate, not a module in testcase-verification

- Verification is about comparing logs to expectations; reporting is presentation over finished artifacts. Keeping them apart stops `testcase-verification` (already holds coverage, regression, independence, templates) growing further.
- Renderers add no heavy deps, but a separate crate keeps report formatting changes from rebuilding everything that depends on verification (`cargo tree -p testcase-verification --invert` shows verifier, test-executor, orchestration, manager).
- Dependency direction stays legal: `testcase-reporting -> testcase-verification -> testcase-common -> testcase-models`. Nothing lower imports it. It must NOT depend on `testcase-manager` or `testcase-orchestration` (Layer 4); that would repeat the known `validate-yaml -> testcase-manager` deviation.

### Why `test-executor` subcommand, not a new binary or `tcms`

- `test-executor` already hosts `trace-matrix`, `coverage`, `regression`, `independence`; users run reports in the same workflow and with the same inputs (`--test-cases`, `--runs`).
- A new binary requires Cargo member, Make target, and `mk/incremental.mk` mapping (per STRUCTURE.md). A subcommand avoids that. Cost: main.rs is already large; keep the subcommand body to ~30 lines that call the library.
- `tcms` is the Jira/ref CLI; reports are not Jira-related. Do not put them there.
- Alternative if main.rs bloat is judged unacceptable: a thin `tcms-report` bin crate in Layer 5. Same library either way, so the decision is cheap to reverse.

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `testcase-reporting::load` | Read trace-matrix YAML and test-run YAML (files or dirs) via `load_and_validate_yaml`; return typed `TraceMatrix` + `Vec<TestRunRecord>`. Replaces binary-private `read_test_runs`. | testcase-common, testcase-verification types |
| `testcase-reporting::model` | Join matrix + runs into `ReportModel`: per-requirement rows, per-run summaries, gaps, problem list. All counting and verdict aggregation rules live here. Pure, deterministic, unit-testable. | testcase-verification types only |
| `testcase-reporting::render` | One pure function per format over `ReportModel`. No logic beyond layout. | `ReportModel` only |
| `test-executor report` | CLI args, IO, `--format`, `--output`, exit code policy (e.g. non-zero with `--fail-on-gaps`). | testcase-reporting |
| `testcase-verification` (existing, unchanged) | Source of truth for matrix and run types. | n/a |
| TPDG (external, optional) | Possible renderer for container-shaped output. | Make/shell only, not linked |

### ReportModel (sketch)

```rust
pub struct ReportModel {
    pub matrix_id: String,
    pub baseline_id: Option<String>,
    pub runs: Vec<RunSummary>,            // counts per verdict per run
    pub requirements: Vec<RequirementRow>,// REQ -> cases -> verdicts -> rollup
    pub gaps: Vec<GapRow>,                // untraced, missing normal_range/robustness
    pub problems: Vec<ProblemRow>,        // fail/error cases with problem_report_id
}
pub struct RequirementRow {
    pub requirement_id: String, pub level: RequirementLevel,
    pub cases: Vec<CaseOutcome>,          // test_case_id, verdict, run_id, problem_report_id
    pub status: ReqStatus,                // Passed | Failed | Errored | NotRun | Untraced
}
```

The four required reports (traceability, run summary, coverage gaps, problem list) are four VIEWS over one `ReportModel`, selected by `--report <kind>` (or all sections in one document). One model means the numbers cannot disagree between reports.

### Data Flow

```
test-executor trace-matrix  -> trace-matrix.yaml  ----\
verifier (--test-run)       -> *.test-run.yaml    -----+--> load (schema-validated)
                                                       |
                                                       v
                                       model::build(matrix, runs) -> ReportModel
                                                       |
                       render::{markdown,asciidoc,json,yaml} -> report file / stdout
                                                       |
                         (optional) asciidoctor / TPDG for HTML/PDF
```

Direction is one-way and file-based, matching the existing pipeline style. No re-execution, no writing back to matrix or run files.

## Patterns to Follow

### Pattern 1: Pure model, thin renderers
**What:** All aggregation in `model`; renderers are `fn(&ReportModel) -> String`.
**When:** Always. **Why:** snapshot-testable renderers, no IO in tests, new format = one new function.

### Pattern 2: Join on test_case_id with an explicit policy
**What:** For each requirement case, pick the outcome across runs by a documented rule (recommended: latest `started_at` run containing the case wins; a `not_executed` result never overrides an executed one). Requirement rollup: any fail/error -> Failed/Errored; all pass -> Passed; none executed -> NotRun.
**Why:** The matrix does not carry verdicts; without a stated rule the report is ambiguous when a case ran in several runs. Surface the rule in the report header.

### Pattern 3: Schema-validated input, never hand-parsed
**What:** Load via `testcase_common::load_and_validate_yaml` (ARCHITECTURE.md anti-pattern: hand-parsing). Note `test-executor` currently deserializes the trace matrix with `serde_yaml::from_str` in the coverage path; the report must not copy that.

### Pattern 4: Deterministic output
**What:** Sort by requirement id and run id; no wall-clock in the body except an optional `--generated-at` header. **Why:** reports are verification evidence and are diffed/archived; also required for stable acceptance tests.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Rebuilding the matrix inside the report
**Why bad:** forks the data (explicitly rejected in PROJECT.md Key Decisions). **Instead:** consume `trace-matrix.yaml`; use `build_matrix` only in tests.

### Anti-Pattern 2: Putting report code in `testcase-manager` / `testcase-orchestration`
**Why bad:** Layer 4 dependency for a Layer 3 concern; deepens known layer deviations. **Instead:** `testcase-reporting` at Layer 3.

### Anti-Pattern 3: Treating missing run data as a gap, or hiding it
**Why bad:** "no test cases" (coverage gap) and "test cases exist but never ran" (NotRun) are different conclusions; conflating them misleads the verification reader. **Instead:** distinct statuses, both shown explicitly (Core Value: gaps shown, not hidden).

### Anti-Pattern 4: New schema or schema edits for report output
**Why bad:** schemas are shared (PROJECT.md constraint). **Instead:** human formats (md/adoc) have no schema; machine formats (json/yaml) are an internal, versioned `report_version` field first, and only get a schema if a consumer asks. Do not overload `analysis-report` without checking its consumers.

## Suggested Build Order (dependencies)

1. **Extract loaders**: move `read_test_runs` (and matrix loading with validation) from `test-executor/src/main.rs` into `testcase-reporting::load` (or `testcase-verification` if reused by others). Prerequisite for everything; behavior-preserving refactor, covered by existing `trace_matrix_cli.rs` tests.
2. **ReportModel + join policy** (`model`): requirement rollup, run counts, gaps, problems. Unit tests with fixtures built from `build_matrix`. Highest-risk logic; do it before any formatting.
3. **Renderers**: Markdown first (readable, diffable), then AsciiDoc (matches existing doc ecosystem), then JSON/YAML (for tooling/tests).
4. **CLI integration**: `test-executor report` subcommand, exit-code policy, Make target and `mk/incremental.mk` E2E mapping, shell E2E in `crates/testcase-manager/tests/integration/`.
5. **Acceptance/pipeline hook**: add a report step to the acceptance suite fixtures (needs executed-run artifacts; reuse existing verifier scenarios).
6. **TPDG/HTML spike (optional, last)**: decide whether any report goes through TPDG; fall back to `asciidoctor` on the generated `.adoc` for HTML (already a documented requirement for HTML).

Reports 2-4 (summary, gaps, problems) are all views in step 2/3, so they do not need separate phases; the traceability report itself is the first view delivered.

## Scalability Considerations

| Concern | 100 reqs | 10K reqs | 1M |
|---------|----------|----------|----|
| Load + join | trivial | in-memory BTreeMap index on test_case_id, O(runs x results) | out of scope |
| Output size | one file | markdown tables large; offer per-section output | n/a |

Index results once (`HashMap<test_case_id, Vec<(run, result)>>`) rather than scanning runs per requirement, which `build_matrix` currently does.

## Sources

- Repo: `crates/testcase-verification/src/trace_matrix.rs`, `test_run.rs`; `crates/test-executor/src/main.rs` (trace_matrix, read_test_runs); `schemas/tcms-schemas/tcms/{trace-matrix,test-run,analysis-report}.schema.json`; `.planning/codebase/ARCHITECTURE.md`, `STRUCTURE.md`, `.planning/PROJECT.md`. (HIGH)
- TPDG ability to render non-container (requirement-centric) YAML: not verified; no TPDG binary or docs inspected. (LOW)
