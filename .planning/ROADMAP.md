# Roadmap: TCMS — Traceability Reports

## Overview

This milestone adds a `test-executor report` command that turns the existing trace matrix and test-run records into traceability, run-summary, coverage-gap and problem-report outputs. The work is library-first (new Layer 3 crate `testcase-reporting`: load, model, render) but every phase ends with something the user can run and read. Phase 1 gives a command that loads and reconciles inputs. Phase 2 adds the first real report (traceability, Markdown). Phase 3 adds the other three report kinds as views over the same model. Phase 4 adds AsciiDoc and schema-validated JSON/YAML. Phase 5 makes the command safe to use in a pipeline (exit codes, E2E, acceptance hook).

## Phases

- [ ] **Phase 1: Strict Input Loading and Reconciliation** - `test-executor report` loads matrix and run records, fails closed, selects runs, and prints what it used and skipped
- [ ] **Phase 2: Traceability Report (Markdown)** - REQ -> test cases -> PASS/FAIL with a documented verdict roll-up and a deterministic, identified report
- [ ] **Phase 3: Summary, Gaps and Problem Reports** - Run summary, coverage gaps and problem-report list as views over one model
- [ ] **Phase 4: AsciiDoc and Machine-Readable Output** - AsciiDoc rendering plus JSON/YAML output validated against a local schema
- [ ] **Phase 5: Pipeline Use: Exit Codes and E2E** - Three-state exit status, report-before-exit, E2E and acceptance coverage

## Phase Details

### Phase 1: Strict Input Loading and Reconciliation
**Goal**: A user can point `test-executor report` at a trace matrix and test-run records and see exactly which inputs were loaded, skipped, or rejected, and which run won for each test case.
**Mode:** mvp
**Depends on**: Nothing (first phase)
**Requirements**: LOAD-01, LOAD-02, LOAD-03, LOAD-04, CLI-01
**Success Criteria** (what must be TRUE):
  1. User runs `test-executor report` with a matrix and run records and gets a reconciliation block listing inputs discovered, loaded, and skipped with reason
  2. A matrix or run record that fails schema validation makes the command fail with an error naming the offending file
  3. When several runs contain the same test case, the latest executed result decides (a later pass overrides an earlier fail or error, and a later fail or error overrides an earlier pass); runs that omit the test case or record `not_executed` do not count; the output lists the runs used and excluded
  4. Two run records sharing a `run_id` make the command fail
**Plans:** 4 plans

Plans:
- [ ] 01-01-PLAN.md — New crate `testcase-reporting`, legacy loaders moved behaviour-preserving, `LoadedRun` (wave 1)
- [ ] 01-02-PLAN.md — Strict fail-closed loader, duplicate `run_id`, reconciliation block (wave 2, TDD)
- [ ] 01-03-PLAN.md — Run-selection policy D-01..D-05, D-16 with table-driven tests (wave 2, TDD)
- [ ] 01-04-PLAN.md — `test-executor report` subcommand, CLI tests, docs, quality gates (wave 3)
**Notes**: CLI-01 is satisfied by the command and its report-kind selection existing from here; later phases add the kinds. Research flag: check in `trace_matrix.rs` whether requirements with zero test cases are emitted; if not, "no tests" needs a requirements input (affects Phase 3 GAP-01). Loaders are extracted from `test-executor/main.rs` behaviour-preserving; use `load_and_validate_yaml`. Record which quality gates could run (uv, shellcheck, StrictDoc, TPDG missing locally).

### Phase 2: Traceability Report (Markdown)
**Goal**: A user can generate a Markdown traceability report showing, for every requirement, its test cases, their verdicts, and an honest verification status.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: ROLL-01, ROLL-02, ROLL-03, ROLL-04, TRC-01, TRC-02, TRC-03, TRC-04, TRC-05, OUT-03, OUT-04
**Success Criteria** (what must be TRUE):
  1. Report lists each requirement with its linked test cases, each case's verdict, and the run it came from; HLR and LLR requirements appear distinctly
  2. A requirement shows VERIFIED only if every linked test case has verdict `pass`; otherwise it shows FAILED, ERRORED, NOT RUN, PARTIAL or NO TESTS per the documented rule
  3. Test coverage (normal-range, robustness present) is displayed separately from verification status, and a linked test case with no result shows NOT RUN
  4. Obsolete test cases are excluded from the roll-up and counted in the header
  5. Running twice with the same inputs and `--generated-at` gives byte-identical output; the header shows matrix, baseline, run, build, environment, tester and generation time, with "not recorded" for missing values
**Plans:** 4 plans

Plans:
- [ ] 02-01-PLAN.md — Verification status roll-up rule and case outcome mapping, table-driven tests (wave 1, TDD)
- [ ] 02-02-PLAN.md — ReportModel: header, document sections, rows, obsolete handling, deterministic order (wave 2)
- [ ] 02-03-PLAN.md — Markdown renderer with golden and byte-identical snapshot tests (wave 3)
- [ ] 02-04-PLAN.md — `test-executor report` traceability wiring, CLI tests, docs (wave 4)
**Notes**: Roll-up rule must be documented and covered by table-driven tests. Research flag: staleness (verifier output and `verification_result_ref` versioning) is deferred to v2 (STALE-01); do not block this phase on it. Sort all collections and inject the clock for determinism.

### Phase 3: Summary, Gaps and Problem Reports
**Goal**: A user can generate the run summary, coverage-gaps report and problem-report list, all consistent with the traceability report.
**Mode:** mvp
**Depends on**: Phase 2
**Requirements**: SUM-01, GAP-01, PRB-01, PRB-02
**Success Criteria** (what must be TRUE):
  1. Run summary shows pass, fail, not_executed, error counts and the total, per run and per requirement, with buckets summing to the total
  2. Gaps report lists requirements with no test cases, requirements missing a normal-range test, requirements missing a robustness test, and test cases with no recorded result
  3. Problem report list shows every fail and error result with its `problem_report_id`
  4. A fail or error result lacking a `problem_report_id` is visibly flagged in the list
**Plans:** 5 plans

Plans:
- [ ] 03-01-PLAN.md — Shared foundation: header extraction, ViewInput, ResultCounts, fixture (wave 1)
- [ ] 03-02-PLAN.md — Run summary view and Markdown (SUM-01) (wave 2)
- [ ] 03-03-PLAN.md — Coverage gaps view and Markdown (GAP-01) (wave 2)
- [ ] 03-04-PLAN.md — Problem report list with missing flag (PRB-01, PRB-02) (wave 2)
- [ ] 03-05-PLAN.md — `report --kind summary|gaps|problems` wiring, CLI tests, docs (wave 3)
**Notes**: All four report kinds are views over one `ReportModel` so numbers cannot disagree. Depends on the Phase 1 zero-test-requirement finding for "no test cases".

### Phase 4: AsciiDoc and Machine-Readable Output
**Goal**: A user can get every report as AsciiDoc and as JSON or YAML that has been validated against a local report schema.
**Mode:** mvp
**Depends on**: Phase 3
**Requirements**: OUT-01, OUT-02
**Success Criteria** (what must be TRUE):
  1. User selects Markdown or AsciiDoc and gets all report kinds in that format, with hostile strings (pipes, newlines, markup) escaped in cells
  2. User selects JSON or YAML and gets the same data; the output is validated against a local schema before writing, and a validation failure aborts without writing a bad file
  3. Determinism (same inputs and `--generated-at` give identical bytes) holds for every format
**Plans:** 4 plans (scoped: traceability kind only; summary, gaps and problems formats deferred until Phase 3 is done)

Plans:
- [ ] 04-01-PLAN.md — AsciiDoc renderer for the traceability model with escaping and hostile-string tests (wave 1)
- [ ] 04-02-PLAN.md — Local report schema (`type`/`schema`/`kind` envelope), envelope entry, `ReportDocument` and `validate_document` (wave 2)
- [ ] 04-03-PLAN.md — `render_json` and `render_yaml`, validated before return; round-trip and determinism tests (wave 3)
- [ ] 04-04-PLAN.md — `--format` on `test-executor report`, validation before write, CLI tests, docs, quality gates (wave 4)
- [ ] Follow-up plans (after Phase 3): AsciiDoc, JSON and YAML for summary, gaps and problems; OUT-01 and OUT-02 stay open until then
**UI hint**: yes
**Notes**: Native in-process renderers are the primary path. HTML via asciidoctor or TPDG is v2 (FOUT-01) pending a TPDG spike; consider updating the "reuse TPDG" Key Decision in PROJECT.md. Local report schema uses the `type`/`schema` envelope; do not modify shared schemas.

### Phase 5: Pipeline Use: Exit Codes and E2E
**Goal**: A user can run `test-executor report` in a script or CI job and rely on its exit status, with the report always written first.
**Mode:** mvp
**Depends on**: Phase 4
**Requirements**: CLI-02
**Success Criteria** (what must be TRUE):
  1. Command exits 0 on success, 1 on input or validation error, and 2 when `--fail-on-gaps` is given and gaps or failed results exist
  2. The report file is written before any non-zero exit caused by gaps or failures
  3. A bash 3.2 and BSD/GNU safe E2E script exercises the exit codes and is mapped in `mk/incremental.mk` and the acceptance suite
**Plans:** 3 plans

Plans:
- [ ] 05-01-PLAN.md — `findings` in `testcase-reporting`: gaps or failed results decided from the gaps and problems views (wave 1, TDD)
- [ ] 05-02-PLAN.md — `--fail-on-gaps`, exit 2 after the write, `report` usage errors exit 1, real-binary tests (wave 2)
- [ ] 05-03-PLAN.md — Bash E2E, `mk/incremental.mk` and Makefile wiring, acceptance Stage 10, docs, quality gates (wave 3)
**Notes**: Add Make target and E2E mapping; record which of `make build`, `lint`, `test`, coverage (>= 70% with E2E) and acceptance could run in this environment.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Strict Input Loading and Reconciliation | 0/0 | Not started | - |
| 2. Traceability Report (Markdown) | 0/0 | Not started | - |
| 3. Summary, Gaps and Problem Reports | 0/0 | Not started | - |
| 4. AsciiDoc and Machine-Readable Output | 0/0 | Not started | - |
| 5. Pipeline Use: Exit Codes and E2E | 0/0 | Not started | - |
| 9. TCMS-47 V&V metrics refinement | 0/3 | Not started | - |
| 10. Acceptance and make failure propagation | 0/3 | Not started | - |

### Phase 6: Requirement lookup: tcms requirements locate verifies that requirement IDs exist in the configured Jira backlog or StrictDoc export

**Goal:** A user can run `tcms requirements locate <ID>...` and see, for each ID, whether it is a requirement in the configured Jira backlog or in a StrictDoc export, with a non-zero exit status when any ID is missing.
**Requirements**: LOOK-01, LOOK-02, LOOK-03, LOOK-04, LOOK-05, LOOK-06, LOOK-07
**Depends on:** Phase 5
**Plans:** 5 plans

Plans:
- [ ] 06-01-PLAN.md — requirement-import helpers: batch `key in (...)` JQL with escaping, StrictDoc `export_items` (wave 1)
- [ ] 06-02-PLAN.md — `tcms requirements locate`: resolution, one batch Jira search matched by key, StrictDoc export lookup, text/plaintext/json, exit 0/1/2 (wave 2)
- [ ] 06-03-PLAN.md — binary tests: mock Jira Cloud/DC batch and key match, StrictDoc fixture, formats, exit codes (wave 3)
- [ ] 06-04-PLAN.md — docs in JIRA_INTEGRATION.md and CLAUDE.md quality gates (wave 4)
- [ ] 06-05-PLAN.md — `tcms requirements export-ids`: read requirements.json, shared batch lookup, valid IDs to stdout/--output, invalid on stderr, exit 0/1/2, binary tests (wave 5)

### Phase 7: E2E tests for locating requirements by ID in StrictDoc

**Goal:** The real `tcms` binary is tested end to end on exports made by the real StrictDoc tool: `tcms requirements locate` finds HLR, LLR and `system` items by ID, reports missing IDs, and exits 0/1/2 as specified; `tcms requirements export-ids` writes the valid IDs. The StrictDoc reader first reads the layout StrictDoc really writes (DECISIONS.md Q1-Q3).
**Requirements**: LOOK-03, LOOK-04, LOOK-07 (Phase 6 IDs; this phase has none of its own)
**Depends on:** Phase 6
**Plans:** 5 plans

Plans:
- [ ] 07-01-PLAN.md — StrictDoc reader fix: read DOCUMENTS/NODES, REQUIREMENT nodes only, uppercase fields, path = document TITLE, Parent relations, refuse an item with no LEVEL; old layout dropped; every changed test listed (wave 1)
- [ ] 07-02-PLAN.md — real-tool fixture: small .sdoc sources, exports made by `strictdoc export --formats json`, regenerate script with --check, README (wave 2)
- [ ] 07-03-PLAN.md — e2e tests of `requirements locate` on the real export: found HLR/LLR/system, not found, exit 2 cases, batch rule (wave 3)
- [ ] 07-04-PLAN.md — e2e tests of `requirements export-ids` on the real export: valid/invalid, --output, product mismatch, strict mode (wave 3)
- [ ] 07-05-PLAN.md — docs updated from "assumed" to verified layout; quality gates run and recorded (wave 4)

### Phase 8: V&V data flow diagram and parameterised acceptance scenarios

**Goal**: The V&V data flow (test plan config -> test-executor run -> verifier -> test-run record -> trace matrix -> traceability report) is documented with a file map and example invocations, and five parameterised acceptance scenarios (full-two, full-one, all-but-one, half, none) run in CI with a determinism check.
**Requirements**: ACC-01, ACC-02, ACC-03, ACC-04, ACC-05
**Depends on:** Phase 7
**Spec**: `.planning/phases/08-vv-data-flow-acceptance-scenarios/08-SPEC.md`
**Plans:** 5 plans

Plans:
- [ ] 08-01-PLAN.md — V&V diagram in the mermaid source and DATA_FLOW.md, per-arrow example invocations, per-step file map, mermaid syntax check (wave 1)
- [ ] 08-02-PLAN.md — bash 3.2 scenario driver, five scenario folders with fake inputs, layout test written first (wave 2)
- [ ] 08-03-PLAN.md — per-scenario assertions: verdict roll-up and document completeness, test-first (wave 3)
- [ ] 08-04-PLAN.md — determinism meta test: two runs with fixed --generated-at and matrix --id, diff returns 0 lines (wave 4)
- [ ] 08-05-PLAN.md — CI wiring in .gitlab-ci.yml and .github/workflows/workspace.yml, Make target, quality gates recorded (wave 5)
**Notes**: The spec names `tcms report`; no such command exists. The report and trace-matrix commands are `test-executor report` and `test-executor trace-matrix` (crates/test-executor). Scenarios use `--no-coverage-check` on trace-matrix, since full-one and half would otherwise fail the normal-range and robustness check. mermaid-cli (mmdc) is not installed locally; the diagram check is recorded honestly.

### Phase 9: TCMS-47 V&V metrics refinement

**Goal**: The TCMS-47 family is finished in tcms-cli: the per-case results of a run are combined into one run record (TCMS-47.3), `tcms vv metrics` reads the per-product `vv-config.yaml` and a stability window (TCMS-47.2), and `tcms-vv-report` renders the metrics as Markdown or AsciiDoc without computing anything (TCMS-47.1). Acceptance criteria of the backlog tasks are the source of truth; context and open items are in `.planning/phases/09-tcms-47-vv-metrics-refinement/09-CONTEXT.md`.
**Requirements**: TCMS-47.3, TCMS-47.2, TCMS-47.1, TCMS-47
**Depends on:** Phase 8
**Plans:** 3 plans

Plans:
- [ ] 09-01-PLAN.md — TCMS-47.3 producer: `tcms vv combine` writes one test-run.yaml and verification-results.yaml from the per-case files, duplicate and verdict rules, validation before write, 63-file fixture (wave 1)
- [ ] 09-02-PLAN.md — TCMS-47.2 consumer: vv-config.yaml reader, `--stability-window`, parameter sources in output, error cases, schema check via checkpoint (wave 2, depends on 09-01)
- [ ] 09-03-PLAN.md — TCMS-47.1 report: `tcms-vv-report` crate, Markdown and AsciiDoc, golden files for sut-1 and derived fixtures, exit codes, docs and DATA_FLOW 4.5 (wave 3, depends on 09-02)
**Notes**: Open items (snake_case vs camelCase field, missing vv-config and verification-results schemas, combinator command name) are listed in 09-CONTEXT.md and resolved at the plan checkpoints.


### Phase 10: Acceptance and make failure propagation

**Goal**: `make acceptance-test`, `make test` and `make coverage-e2e` report honestly: the acceptance suite finds every test case file (both `.yml` and `.yaml`) and fails on zero inputs, `make acceptance-test` exits non-zero on any unlisted stage failure with stages 8 and 9 passing or listed as known failures with a backlog id, and a failing E2E script or verifier step makes `make test` and `make coverage-e2e` exit non-zero while green runs stay green.
**Requirements**: TCMS-45, TCMS-46
**Depends on:** Phase 9
**Spec**: `.planning/phases/10-acceptance-and-make-failure-propagation/10-CONTEXT.md` (decisions D-45-1..3, D-46-1..3, open items O-1, O-2)
**Plans:** 3 plans

Plans:
- [ ] 10-01-PLAN.md — TCMS-45 AC 1: shared discovery for `.yml` and `.yaml` used by Stage 1 and Stage 2, zero inputs fail, count-guard test first (wave 1)
- [ ] 10-02-PLAN.md — TCMS-46 AC 1-3: E2E runner with failing and missing script handling, Makefile `;` chain fixed, exit-code audit, verification warning fixed at cause, gates recorded (wave 1, TDD)
- [ ] 10-03-PLAN.md — TCMS-45 AC 2-3: suite exit contract, known-failure list with backlog ids, stages 8 and 9 fixed or listed, `make acceptance-test` recorded (wave 2, depends on 10-01 and 10-02, has a decision checkpoint)
**Notes**: On `dev` the Stage 1 find already matches `*.yml` only (commit 428fb36); the TCMS-45 description refers to `main`. The `;`-chained verifier steps in the `test-e2e` affected branch are the likely reason TCMS-46 failures were swallowed; 10-02 confirms this from a log before fixing. `coverage-e2e` SKIPPED paths stay green (open item O-1).
