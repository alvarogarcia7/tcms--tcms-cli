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
  3. When several runs contain the same test case, the output names the run with the latest `started_at` and lists the excluded runs
  4. Two run records sharing a `run_id` make the command fail
**Plans**: TBD
**Notes**: CLI-01 is satisfied by the command and its report-kind selection existing from here; later phases add the kinds. Research flag: check in `trace_matrix.rs` whether requirements with zero test cases are emitted; if not, "no tests" needs a requirements input (affects Phase 3 GAP-01). Loaders are extracted from `test-executor/main.rs` behaviour-preserving; use `load_and_validate_yaml`. Record which quality gates could run (uv, shellcheck, StrictDoc, TPDG missing locally).

### Phase 2: Traceability Report (Markdown)
**Goal**: A user can generate a Markdown traceability report showing, for every requirement, its test cases, their verdicts, and an honest verification status.
**Mode:** mvp
**Depends on**: Phase 1
**Requirements**: ROLL-01, ROLL-02, ROLL-03, ROLL-04, TRC-01, TRC-02, OUT-03, OUT-04
**Success Criteria** (what must be TRUE):
  1. Report lists each requirement with its linked test cases, each case's verdict, and the run it came from; HLR and LLR requirements appear distinctly
  2. A requirement shows VERIFIED only if every linked test case has verdict `pass`; otherwise it shows FAILED, ERRORED, NOT RUN, PARTIAL or NO TESTS per the documented rule
  3. Test coverage (normal-range, robustness present) is displayed separately from verification status, and a linked test case with no result shows NOT RUN
  4. Obsolete test cases are excluded from the roll-up and counted in the header
  5. Running twice with the same inputs and `--generated-at` gives byte-identical output; the header shows matrix, baseline, run, build, environment, tester and generation time, with "not recorded" for missing values
**Plans**: TBD
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
**Plans**: TBD
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
**Plans**: TBD
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
**Plans**: TBD
**Notes**: Add Make target and E2E mapping; record which of `make build`, `lint`, `test`, coverage (>= 70% with E2E) and acceptance could run in this environment.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Strict Input Loading and Reconciliation | 0/0 | Not started | - |
| 2. Traceability Report (Markdown) | 0/0 | Not started | - |
| 3. Summary, Gaps and Problem Reports | 0/0 | Not started | - |
| 4. AsciiDoc and Machine-Readable Output | 0/0 | Not started | - |
| 5. Pipeline Use: Exit Codes and E2E | 0/0 | Not started | - |
