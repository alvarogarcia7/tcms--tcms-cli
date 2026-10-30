# Feature Landscape

**Domain:** Traceability and test-verification reports for regulated V&V evidence (DO-178C-style: HLR/LLR, normal-range and robustness tests, problem reports, reviewer independence)
**Researched:** 2026-10-30
**Overall confidence:** MEDIUM. The domain conventions come from training knowledge of DO-178C (section 6.4 test coverage, section 6.5 verification results, section 11.14 Software Verification Results) and common requirements-management tooling (DOORS, Polarion, Jama, codebeamer, StrictDoc). No web verification was done in this session. The data-model facts below were read directly from the vendored schemas and are HIGH confidence.

## What the Input Data Already Supports (HIGH confidence, read from schemas)

Source: `schemas/tcms-schemas/tcms/trace-matrix.schema.json` and `test-run.schema.json`.

- **Trace matrix link:** `requirement_id`, `level` (HLR or LLR), `test_case_ids[]`, `normal_range_covered`, `robustness_covered`, `run_ids[]`. Matrix-level `id` and `baseline_id`.
- **Test run:** `run_id`, `plan_id`, `baseline_id`, `build_id`, `environment` and `environment_id`, `tester(s)`, `reviewer`, `witness`, `started_at` and `finished_at`, optional `signature`.
- **Run results:** `test_case_id`, `test_case_version`, `verdict` (pass, fail, not_executed, error), `verification_result_ref`, `problem_report_id`, `notes`.
- **Constraint:** A matrix link has `run_ids` only, not per-test verdicts. Verdicts live in the run records. The report therefore needs a join: requirement to test cases (matrix), then to results (runs), keyed by `test_case_id` and `run_id`.
- **Caveat:** Normal-range and robustness flags on a link say a linked test case has that `test_type`. They do not say that case passed. Coverage must be distinguished from verified (covered and passing).

## Table Stakes

Missing any of these makes the output unusable as V&V evidence.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Report header / identification block | Verification evidence must say what it describes. Auditors expect matrix id, baseline_id, run_id(s), build_id, environment, tester, reviewer, start/finish time, and generation timestamp. | Low | All fields exist in the schemas. Missing optional fields should render as "not recorded", not be omitted silently. |
| Requirement-to-test-case-to-result matrix (forward trace) | This is the core deliverable. One row per requirement, listing linked test cases and the verdict of each. | Medium | Needs the join of matrix links with run results. Show requirement id, level (HLR/LLR), test case id, verdict and run id. Group by level. |
| Per-requirement rolled-up status | Reviewers scan for the requirement outcome. Suggested rule: fail or error in any linked case gives FAILED, all pass gives VERIFIED, otherwise INCOMPLETE. The rule must be documented in the report. | Low-Medium | The roll-up rule is a design decision. A requirement is never "passed" if any linked case is not_executed. |
| Four-way verdict taxonomy shown explicitly | Counts for pass, fail, not_executed and error must be separate. Merging not_executed or error into fail, or hiding them, is a known evidence defect. | Low | Use the schema enum exactly. |
| Test-run summary counts | Total cases and counts per verdict for the run, plus pass rate. Conventional "SVR" summary content. | Low | Pass rate denominator must be stated (all cases vs executed cases). |
| Per-requirement verdict counts | Counts per requirement as well as per run, so it is visible which requirement drives failures. | Low | Same aggregation, grouped by requirement. |
| Coverage gap: requirements with no test cases | Core value says gaps are shown, not hidden. Requirements with empty `test_case_ids` must be listed first and prominently. | Low | Whether a requirement with an empty link array can appear depends on the matrix generator. See the dependencies and open questions below. |
| Coverage gap: missing normal-range or robustness tests | DO-178C requires both normal-range and robustness test coverage of requirements. Use `normal_range_covered` and `robustness_covered` flags. | Low | Flags exist. Report per requirement and as a total. |
| Requirements with tests but no recorded result | Covered on paper but never run. This is the most common hidden gap. | Low-Medium | Requires the join. Shows as not_executed, or as no result in any run. |
| Problem report list | Table of every fail or error result with `problem_report_id`, test case, requirement(s), run, and verdict. | Low | Join from result to requirement via the matrix. |
| Flag fail or error results lacking a problem_report_id | A failing case with no problem report is an audit finding. | Low | A simple check, high value. Make it visible rather than silent. |
| Deterministic, reproducible output | Same inputs must give byte-identical output (stable sort, no wall-clock in the body other than a labelled generation time). Needed so evidence is diffable and reviewable in git. | Low-Medium | Sort by requirement_id and test_case_id. |
| Input validation with clear errors | Inputs are validated against the vendored schemas. Dangling references (a test_case_id in a run not in the matrix, or a matrix test case absent from all runs) must be reported, not crash. | Medium | Uses the existing `load_and_validate_yaml` convention. |
| Human-readable output (Markdown and AsciiDoc at minimum) | Matches the existing documentation pipeline. | Medium | Reuse the TPDG path per PROJECT.md. HTML comes from AsciiDoc. |
| Non-zero exit for evidence-defects option | CI use: the command should be able to fail the build on gaps or failures (opt-in flag). | Low | Pairs with machine-readable output. |

## Differentiators

Valued, not expected. Order roughly by value for cost.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Machine-readable report output (JSON or YAML alongside the human report) | Lets CI, dashboards and downstream tools consume the same computed data. It also makes the report testable. | Low-Medium | Compute once into a report model, then render. Do not add new upstream schemas (out of scope). Define a local report schema only if needed. |
| Multi-run aggregation with "latest result wins" or full run history per test case | Real campaigns have reruns. Show the latest verdict plus the history, so regressions and reruns are visible. | Medium-High | Requires an explicit policy (latest by `finished_at`). Policy ambiguity is the risk. Keep single-run and explicit-run-list modes first. |
| Reverse trace (test case to requirements) | Standard bidirectional traceability. Detects orphan test cases that trace to no requirement. | Medium | Derived from the same links. Orphans in the run but not in the matrix are the useful signal. |
| Traceability breakdown by level (HLR vs LLR) with per-level coverage percentages | Regulators review HLR and LLR coverage separately. | Low | `level` is in the schema. |
| Coverage percentages with stated denominators | Summary numbers: requirements with tests, requirements with normal-range and robustness, requirements verified. | Low | Always print numerator and denominator, not just a percent. |
| Stale / version-mismatch detection | Run result records `test_case_version`. Warn if it differs from the current test case version, or if run `baseline_id` differs from matrix `baseline_id`. | Medium | Baseline mismatch check is cheap and high value. Version check needs the loaded test case. |
| Reviewer / signature / independence presence in the header | Show reviewer, witness and whether the run is signed. Presenting the data is cheap. | Low | Does not re-implement the independence rule (TCMS-40.8), and the reviewer report is out of scope. Display only. |
| Links to evidence (`verification_result_ref`) from each result row | One click from verdict to actual values and logs. | Low | Render as relative link or path. |
| Problem report summary grouped by problem_report_id | One problem report can affect several cases and requirements. Show the impact fan-out. | Low-Medium | Grouping on top of the problem report list. |
| Notes passthrough | Show `notes` from results, which often explain a waiver or a not_executed reason. | Low | |
| Single combined "verification results" document | One document containing header, summary, matrix, gaps, problem reports. It resembles a DO-178C Software Verification Results artifact. | Medium | Composition of the individual reports. Still offer each section as a standalone report as required by PROJECT.md. |
| CSV export of the matrix | Auditors and managers work in spreadsheets. | Low | Optional, cheap once the report model exists. |

## Anti-Features

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Re-executing tests from the report command | Violates PROJECT.md: reports analyse recorded results only. | Read executed-run artifacts only. |
| Requirement import / sync from StrictDoc or Jira | Owned by TCMS-42. Duplicates in-flight work. | Consume requirement ids as they appear in the trace matrix. |
| Reimplementing trace matrix generation or run recording | Forks data already schema-validated (TCMS-40.4, TCMS-40.5). | Call or read the existing outputs. |
| Reviewer independence report / independence enforcement | Out of scope this milestone. Enforced at verification already. | Show reviewer name only. |
| New or modified upstream schemas (test-case, test-run, trace-matrix) | Shared with other tools. | Keep any report model internal, or add a separate local schema. |
| Silent "best guess" handling of gaps (hiding not_executed as fail, dropping dangling references, defaulting missing verdict to pass) | Defeats the core value: gaps shown explicitly. | Show every anomaly in a dedicated section. |
| Overwriting or editing source run records | Evidence integrity. | Strictly read-only inputs. |
| A second report renderer / new template engine | PROJECT.md says reuse the TPDG path. | Feed container-style data through the existing pipeline where it fits. |
| Interactive dashboards, web UI, charts | Outside a CLI test harness. Large cost, little regulatory value. | Plain tables. Charts, if ever, come from the JSON output externally. |
| Structural code coverage (MC/DC, statement coverage) | A different evidence class from requirements-based traceability. | Out of scope. Note in docs that this report is requirements-based coverage only. |
| Issue-tracker lookups to resolve problem report status | Integration scope (Jira work). Network dependency breaks reproducibility. | Print the `problem_report_id` as recorded. |
| Waiver / risk-acceptance workflow | Process tooling, not reporting. | Display `notes` only. |

## Feature Dependencies

```
Report model loader (matrix + runs, schema-validated)
  -> Join: requirement -> test cases -> verdicts
       -> Traceability matrix report (forward trace)
       -> Per-requirement roll-up status
            -> Coverage gaps report (no tests / no normal-range / no robustness / no result)
            -> Per-requirement verdict counts
  -> Run summary counts (needs only runs)
  -> Problem report list (runs + join to requirements)
       -> Missing-problem-report check
       -> Problem report grouping (differentiator)
Report model
  -> Rendering via TPDG path (Markdown / AsciiDoc / HTML)
  -> Machine-readable output (differentiator)
  -> Combined verification-results document (differentiator; needs all four reports)
Multi-run aggregation policy -> latest-result roll-up (differentiator; changes roll-up and counts semantics)
Baseline/version mismatch detection -> needs matrix baseline_id, run baseline_id, optionally test case versions
```

Key dependency note: all four active report types share one join. Build it once in a lower-layer library crate (likely `testcase-verification`), and keep the binary layer thin.

## Open Questions for Requirements

1. **Can the matrix contain requirements with zero test cases?** The schema requires `test_case_ids` but allows an empty array. The "no tests" gap report only works if the matrix generator emits such requirements. Verify how `test-executor trace-matrix` builds links (likely from test case metadata, which would only list requirements that have at least one test). If so, requirements lacking tests need an input requirements list (from the TCMS-42 output). Needs a phase-level check of the code. LOW confidence.
2. **Single run or several?** Input shape (one run file, directory of runs, explicit list) drives the aggregation policy.
3. **Roll-up rule** for requirement status and the pass-rate denominator should be decided and documented once.
4. **Where `run_ids` in the matrix come from** and whether they are always populated.

## MVP Recommendation

Prioritize, in order:
1. Shared loader and join (requirement, test case, verdict) with explicit dangling-reference reporting.
2. Traceability matrix report with per-requirement roll-up (table stakes).
3. Test-run summary counts, per run and per requirement (table stakes, low cost).
4. Coverage gaps report, including no-result requirements (table stakes).
5. Problem report list with missing-problem-report flag (table stakes).
6. Rendering through the existing TPDG path, header block, deterministic ordering, opt-in non-zero exit.

Then add one cheap differentiator: machine-readable JSON output (same report model).

Defer: multi-run history and latest-wins policy, reverse trace and orphan detection, baseline/version mismatch warnings (cheap, so first in the deferred list), combined SVR-style document, CSV export.

## Sources

- Local schemas, HIGH: `schemas/tcms-schemas/tcms/trace-matrix.schema.json`, `schemas/tcms-schemas/tcms/test-run.schema.json`.
- `.planning/PROJECT.md` for scope and out-of-scope boundaries, HIGH.
- DO-178C / RTCA conventions (requirements-based test coverage with normal-range and robustness cases; software verification results containing test results, coverage analysis and problem reports; bidirectional traceability), from training knowledge, MEDIUM-LOW. Not re-verified against the standard text in this session.
- Common RM/TCMS tool report conventions (traceability matrix, coverage gaps, test-run summaries), from training knowledge, LOW-MEDIUM.
