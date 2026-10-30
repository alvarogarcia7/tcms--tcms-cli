# TCMS — Test Case Manager

## What This Is

TCMS is a YAML-based test harness that turns declarative test case definitions into portable bash test scripts, executes them against a system under test (SUT), and verifies the captured results. It is a Rust workspace (`crates/`) with a Makefile-driven build, aimed at teams that need auditable, requirement-linked verification evidence.

The next capability adds **traceability reports**: after the user has executed the test cases against their SUT, TCMS analyses the samples and produces reports that show how requirements are covered by test cases and what those runs concluded — so that the results can be verified and validated.

## Core Value

Every requirement can be traced to the test cases that verify it and to the recorded PASS/FAIL outcome of those tests, with gaps shown explicitly rather than hidden.

## Requirements

### Validated

<!-- Inferred from the existing codebase (see .planning/codebase/). Shipped and in use. -->

- ✓ Declarative test cases in YAML, validated against JSON schemas before parsing — existing
- ✓ Bash script generation (bash 3.2+ compatible) from test cases, including hooks, variable capture, conditional verification — existing
- ✓ Script execution with JSON output capture (jq / json-escape / shell escaping fallback) — existing
- ✓ Verifier: compares execution logs with expectations and emits container YAML — existing
- ✓ Test-run record per verified execution: verdicts pass / fail / not_executed / error, reviewer, problem report IDs, validated against the test-run schema (TCMS-40.4) — existing
- ✓ Trace-matrix command: requirement → test case → run IDs, coverage flags for normal-range and robustness tests, schema-validated output (TCMS-40.5) — existing
- ✓ Documentation generation via test-plan-documentation-generator (AsciiDoc / Markdown / HTML) — existing

### Active

<!-- Current scope. Hypotheses until shipped and validated. -->

- [ ] Traceability report: a human-readable report of REQ → [test cases] → PASS/FAIL, built from the trace matrix and the test-run records
- [ ] Test-run summary report: counts per verdict (pass / fail / not_executed / error) for a run, and per requirement
- [ ] Coverage gaps report: requirements with no test cases, and requirements missing normal-range or robustness tests
- [ ] Problem report list: failed and errored cases with their attached problem_report_id
- [ ] Reports are generated from the executed-run artifacts (no re-execution), so the user can run them after testing their SUT

### Out of Scope

- Requirement ingestion from StrictDoc / Jira — handled by the separate requirement-import and Jira integration work (TCMS-42); the reports consume their output, they do not reimplement it — avoids duplicating an in-flight integration
- Reviewer/independence record report — not selected for this milestone; the independence check (TCMS-40.8) already enforces the rule during verification — can be added later if needed
- Executing test cases against the SUT as part of report generation — reports analyse recorded results only — keeps verification separate from execution
- Changing the schemas of test cases, test runs or trace matrices beyond what reports need — the upstream schemas are shared with other tools — avoids breaking consumers

## Context

- Existing crates relevant to this work: `testcase-verification` (test-run records, trace matrix), `test-executor` (`trace-matrix` command), `verifier` (`--test-run`, problem reports), `testcase-manager` (integration tests, doc generation).
- Documentation pipeline already exists: container YAML → test-plan-documentation-generator → AsciiDoc / Markdown / HTML. The new reports should reuse this path where it fits rather than add a second report renderer.
- Known gaps recorded in the codebase map: `.planning/codebase/CONCERNS.md`, `ARCHITECTURE.md` (layer deviations from `AGENTS.md`), `TESTING.md` (coverage thresholds 50% unit / 70% with E2E from `CLAUDE.md`).
- Owner's `CLAUDE.md` requires `make build`, `make lint`, `make test`, coverage, and acceptance tests before commit. `make test` was not runnable in previous work (uv, shellcheck, StrictDoc, TPDG missing).
- Backlog.md is the existing task tracker (`backlog/tasks/`); TCMS-40.5 and TCMS-40.4 are the completed tasks this work builds on.

## Constraints

- **Compatibility**: Shell scripts and generated bash must stay bash 3.2+ and BSD/GNU compatible (per `AGENTS.md`).
- **Schemas**: Reports consume the vendored test-run and trace-matrix schemas; output must validate against them — the schemas are shared with other tooling.
- **Layering**: New code follows the five-layer crate architecture in `AGENTS.md`; no new dependencies from lower layers onto higher ones.
- **Quality gates**: `make build`, `make lint`, `make test`, coverage ≥ 70% with E2E, and acceptance tests must pass before commit — per `CLAUDE.md`.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Reports build on the existing trace matrix and test-run records | Those outputs are schema-validated and already done (TCMS-40.4, TCMS-40.5); duplicating them would fork the data | — Pending |
| Reports consume recorded results; they do not execute tests | Keeps verification evidence separate from execution and reproducible from artifacts | — Pending |
| Reuse the test-plan-documentation-generator render path where it fits | One rendering pipeline for all human-readable reports | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-30 after initialization*
