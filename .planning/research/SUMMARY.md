# Project Research Summary

**Project:** TCMS (Test Case Manager), milestone: traceability reports
**Researched:** 2026-10-30
**Confidence:** MEDIUM-HIGH

## Executive Summary

The milestone adds four reports on top of existing, schema-validated artifacts: a traceability report (REQ → test cases → PASS/FAIL), a run summary (verdict counts per run and per requirement), a coverage-gaps report, and a problem-report list.

Recommended build: a pure Rust pipeline, LOAD → MODEL → RENDER. One `ReportModel` does all joining and counting; thin renderers produce Markdown, AsciiDoc and JSON/YAML. No new third-party dependencies. The four reports are views over one model, so their numbers cannot disagree.

Central fact (confirmed by three researchers): the trace matrix carries no verdicts. `normal_range_covered`, `robustness_covered` and `run_ids` mean "a test exists" or "a run touched it", not "a test passed". The reports are where PASS/FAIL first meets requirements.

Biggest risk: showing a requirement as covered or verified when its tests failed, never ran, or are stale. Mitigations: keep "test coverage" and "verification status" as separate concepts; document and table-test the roll-up rule; make run selection an explicit, printed policy; use strict loaders that fail closed.

Second risk: the rendering path. PROJECT.md says to reuse TPDG, but TPDG is an unpublished external binary, not installed in dev, and its documented input is a test-results container, not a requirement-centric matrix. Native in-process renderers are the primary path; TPDG/asciidoctor is an optional HTML step decided by a late spike.

## Key Findings

**Stack:** No new dependencies. Use `testcase_common::load_and_validate_yaml` for all inputs (never raw `serde_yaml::from_str`), `std::fmt::Write` renderers, `jsonschema` 0.17 pinned (bumping is workspace-wide churn), and a local JSON/YAML report schema with the `type`/`schema` envelope. Do not overload the shared coverage or analysis schemas.

**Features:**
- Table stakes: identification header (baseline, runs, build, environment, tester, reviewer, times; missing → "not recorded"); forward trace matrix with a documented per-requirement roll-up; four verdicts shown separately with stated denominators; run summary counts; coverage gaps (no tests, missing normal-range, missing robustness, tests with no result); problem-report list flagging failures without `problem_report_id`; dangling-reference reporting; deterministic output; strict validation; opt-in non-zero exit.
- Differentiators: JSON/YAML output from the same model (cheapest, do first); HLR/LLR breakdown with numerator/denominator; baseline-mismatch warning; `verification_result_ref` links.
- Defer: multi-run history, reverse trace / orphan detection, CSV export, combined document.
- Anti-features: re-executing tests, requirement import (owned by TCMS-42), reimplementing matrix generation, new template engines, dashboards, Jira lookups, an independence report.

**Architecture:** New Layer 3 crate `testcase-reporting` (`load`, `model`, `render` modules), depending only on `testcase-verification` and below. Exposed as a `report` subcommand of `test-executor`, next to `trace-matrix`. (STACK.md suggested a module inside `testcase-verification`; the new-crate choice is preferred because it avoids rebuilding reverse dependencies on formatting changes, and is cheap to reverse.)

**Critical pitfalls:**
1. "Covered" is not "passed" — use table-driven roll-up tests.
2. No run-selection policy — print runs used and excluded; one build per report; latest `started_at` wins (RFC 3339); duplicate `run_id` is an error.
3. Stale results after test-case edits — results carry no test-case version or digest. Flag STALE via git commit time or digest; list orphans. Needs research.
4. Silent drops of unreadable inputs — strict loaders, reconciliation block (discovered / loaded / skipped), invariant tests.
5. Hidden denominators and schema drift — count against the expected universe so buckets sum to the total; validate machine-readable output before writing.
- Also: non-determinism (sort everything, inject the clock); write the report before exiting non-zero; keep logic in pure library code so it runs under `cargo test` in this environment.

## Implications for Roadmap

Five phases, library-first:

1. **Strict loading and run selection** — new crate; loaders extracted from `test-executor/main.rs` (behaviour-preserving); run-selection policy; reconciliation counts; duplicate `run_id` errors. Avoids pitfalls 2, 4.
2. **ReportModel and verdict roll-up** — roll-up statuses (VERIFIED, FAILED, ERRORED, NOT RUN, PARTIAL, NO TESTS); counts, gaps, problem rows; staleness flag if research allows; table-driven tests. Avoids pitfalls 1, 3, 6, 8.
3. **Renderers and machine-readable output** — Markdown first, then AsciiDoc, then JSON/YAML with local schema; cell escaping; snapshot tests incl. hostile strings. Avoids pitfalls 5, 7, 10.
4. **CLI integration and E2E** — `test-executor report` with `--report`, `--format`, `--output`, `--generated-at`, `--fail-on-gaps`; three-state exit codes; Make target and `mk/incremental.mk` mapping; bash 3.2 / BSD-GNU safe E2E; acceptance-suite hook.
5. **HTML and TPDG spike (optional, last)** — decide TPDG vs asciidoctor for HTML; wrapper and docs.

Ordering: load → model → render → CLI follows the dependency chain. Summary, gaps and problem reports are views inside phases 2–3, not separate phases.

**Research flags:**
- Phase 2: what the verifier output and `verification_result_ref` record about the executed test-case version (staleness).
- Phase 5: whether TPDG can render requirement-centric data at all.
- Phase 1: whether `trace-matrix` emits requirements with zero test cases (if not, "no tests" needs a requirements input).
- Standard patterns (skip research): phases 3 and 4.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Workspace and crates.io verified; TPDG internals unverified |
| Features | MEDIUM | Schema facts HIGH; DO-178C and tool conventions from training knowledge |
| Architecture | MEDIUM-HIGH | Code facts verified; TPDG fit LOW |
| Pitfalls | HIGH (code) / MEDIUM (general) | |

**Gaps to address:**
- Run-selection and roll-up policy are product decisions. Suggested defaults: one build per report; latest `started_at` wins; `not_executed` never overrides an executed result.
- Zero-test requirements in the matrix (verify in `trace_matrix.rs`).
- TPDG capability (resolve in phase 5; native renderers are primary, so this only affects the "reuse TPDG" goal — consider updating that Key Decision in PROJECT.md).
- Staleness evidence (see research flags).
- Exact `TestRunRecord` and problem-report field names.
- Crate placement (new crate chosen; confirm in phase 1 planning).
- `make test` is not fully runnable here (uv, shellcheck, StrictDoc, TPDG missing); record which gates ran per phase.

## Sources

- Repo: `crates/testcase-verification/src/{trace_matrix,test_run}.rs`, `crates/test-executor/src/main.rs`, `schemas/tcms-schemas/tcms/*.schema.json`, workspace `Cargo.toml`, `AGENTS.md`, `.planning/PROJECT.md`, `.planning/codebase/*` (HIGH)
- crates.io API, 2026-10-30: jsonschema 0.60.2, serde_yaml 0.9.34 (deprecated), TPDG not published (HIGH)
- Traceability practice and DO-178C conventions: training knowledge, not re-verified (MEDIUM / LOW)

---
*Research completed: 2026-10-30*
*Ready for roadmap: yes*
