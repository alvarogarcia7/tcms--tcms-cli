# Verification glossary entities in the CLI (TCMS-40.11)

This page maps the terms of the verification glossary to the schema that holds them, the code
that reads or writes them, and the command that does so. The schemas themselves are the vendored
[tcms-schemas](TCMS_SCHEMAS.md) copy in `schemas/tcms-schemas/`. The mapping of glossary terms to
schemas is the upstream `docs/entity-mapping.md` of tcms-schemas.

Where a term has no command in this CLI, the table says **schema only**: the record can be
validated with `validate-yaml` but the CLI does not produce or check it.

## Glossary term to file and command

| Glossary term | Schema (`tcms/`) | Code | Command or input | Feature doc |
|---------------|------------------|------|------------------|-------------|
| Test case | `test-case-main` | `crates/testcase-models` | test case YAML (`requirements`, `test_type`, `author`, `independence_required`) | this page |
| Normal range test, robustness test | `test-case-main` (`test_type`) | `crates/testcase-models` (`TestType`) | `test_type: normal_range` or `robustness` on a test case | [TRACEABILITY.md](TRACEABILITY.md) |
| Security test | `test-case-main` (`test_type`) | `crates/testcase-models` | `test_type: security`; does not count for requirements-based coverage | [TRACEABILITY.md](TRACEABILITY.md) |
| Test procedure | `test-case-main` (`test_sequences`, `initial_conditions`) | `crates/testcase-models` | kept merged with the test case: the test case YAML is the procedure | this page |
| Test run | `test-run` | `crates/testcase-verification/src/test_run.rs` | `verifier ...` (written by default; `--test-run run.yaml` to choose the path, `--no-test-run` to skip) | [RUN_METADATA.md](RUN_METADATA.md) |
| Test result | `verification-result`, `test-run.results` | `crates/testcase-verification/src/verification.rs` | `verifier ... --output results.yaml` (stamped with run metadata) | [RUN_METADATA.md](RUN_METADATA.md) |
| Run, build, environment, tester (run metadata) | `execution-log`, `verification-result`, `test-run` | `crates/test-executor` (`generate --json-log --run-id`), `crates/verifier` | `--run-id`, `--build-id`, `--environment`, `--tester`; env `TCMS_RUN_ID`, `TCMS_BUILD_ID`, `TCMS_TESTER` | [RUN_METADATA.md](RUN_METADATA.md) |
| Problem report | `problem-report` | `crates/verifier` | `verifier ... --problem-report TC_ID=PR_ID` (on the test run) | [RUN_METADATA.md](RUN_METADATA.md) |
| Requirements-based coverage, trace link, traceability matrix | `trace-matrix`, `coverage-report.requirements_based` | `crates/testcase-verification/src/trace_matrix.rs` | `test-executor trace-matrix TEST_CASE_DIR --requirements FILE` | [TRACEABILITY.md](TRACEABILITY.md) |
| Coverage check (every HLR and LLR needs a normal-range and a robustness test) | `trace-matrix` | `trace_matrix.rs` | exit status 1 of `trace-matrix`; `--no-coverage-check` to skip | [TRACEABILITY.md](TRACEABILITY.md) |
| Structural coverage (statement, decision, MC/DC) | `coverage-report.structural` | `crates/testcase-verification/src/coverage.rs` | `test-executor coverage-import --lcov ... --mcdc ...` | [COVERAGE_IMPORT.md](COVERAGE_IMPORT.md) |
| Coverage gap, disposition (dead code, deactivated code, missing test, missing requirement, justified, tool limitation) | `coverage-report.gaps[]` | `coverage.rs` | annotation file for `coverage-import --annotations`; unannotated gaps fail the command | [COVERAGE_IMPORT.md](COVERAGE_IMPORT.md) |
| Data and control coupling | `analysis-report`, `coverage-report.coupling` | none | schema only | [COVERAGE_IMPORT.md](COVERAGE_IMPORT.md) (not imported) |
| Independence | `independence-assignment`, `test-case-main.independence_required` + `author`, `test-run.reviewer` | `crates/testcase-verification/src/independence.rs` | `test-executor independence-assignment create` and `check`; `verifier --reviewer` | [INDEPENDENCE.md](INDEPENDENCE.md) |
| Baseline, change impact analysis | `baseline` (`change_impact`) | `crates/testcase-verification/src/regression.rs` | `test-executor regression-select --baseline FILE` | [REGRESSION.md](REGRESSION.md) |
| Regression testing, regression plan | `test-plan.regression`, `baseline` | `regression.rs` | `test-executor regression-select --baseline FILE --plan PLAN` | [REGRESSION.md](REGRESSION.md) |
| Requirement (HLR, LLR) | `requirement` (StrictDoc is the source) | `trace_matrix.rs` (IDs only) | requirement ID list (`ID:HLR`, `ID:LLR`) or requirement YAML given to `trace-matrix` | [TRACEABILITY.md](TRACEABILITY.md) |
| Verification, validation, review, review checklist, action item, analysis | `test-plan.activity`, `review`, `action-item`, `analysis-report` | none | schema only | this page |
| Software level (A to E) | `test-plan.software_level` | none | schema only | this page |
| Test plan | `test-plan` | none beyond `regression` | `--plan` of `regression-select` | [REGRESSION.md](REGRESSION.md) |

Any record in the table can be validated against its schema with the envelope, for example:

```bash
validate-yaml -s schemas/tcms-schemas/tcms/test-run.schema.json run.yaml
```

## Commands

Commands are listed with the glossary terms they serve. Options are in `--help` of each command.

| Command | Purpose | Terms |
|---------|---------|-------|
| `test-executor generate --json-log --run-id ID` | Script whose execution log entries carry the run ID | Run metadata |
| `test-executor execute` / `generate` | Run a test case | Test case, test result |
| `verifier -f LOG_DIR --run-id --build-id --tester --environment [--reviewer] [--test-run FILE]` | Verification results and test run record | Test result, test run, run metadata, independence |
| `test-executor trace-matrix TEST_CASE_DIR --requirements FILE [--runs DIR] [-o FILE]` | Traceability matrix and coverage check | Traceability matrix, coverage check |
| `test-executor coverage-import --baseline-id ID [--lcov FILE] [--mcdc FILE] [--trace-matrix FILE] [--annotations FILE] [-o FILE]` | Coverage report with gap dispositions | Structural coverage, coverage gap |
| `test-executor regression-select TEST_CASE_DIR --baseline FILE [--plan FILE] [--format ids\|json\|yaml]` | Regression scope from change impact | Regression, baseline, change impact |
| `test-executor independence-assignment create --id ID --objective TEXT --artifact ID --author NAME --verifier NAME [-o FILE]` | Independence record | Independence |
| `test-executor independence-assignment check FILE...` | Schema and independence check of records | Independence |

## Acceptance scenarios

`test-acceptance/validate_glossary_entities.sh` runs these commands against the fixtures in
`test-acceptance/glossary_entities/fixtures/` and checks their output. It is stage 8 of
`test-acceptance/run_acceptance_suite.sh`; `--skip-glossary` leaves it out.

| Scenario | Ticket | What is checked |
|----------|--------|-----------------|
| Runs with metadata | TCMS-40.3, 40.4 | run IDs on execution logs; run metadata and results in the test run record; the record validates against tcms-schemas `test-run`; a reviewer who is the author of an independent test case, or a missing tester, is refused and no record is written |
| Traceability matrix | TCMS-40.5, 40.6 | a complete matrix passes the coverage check and validates against `trace-matrix`; an incomplete one fails and names the requirement; `--no-coverage-check` writes it anyway |
| Regression selection | TCMS-40.7 | the selected IDs and their reasons; a plan that names another previous baseline is refused |
| Independence | TCMS-40.8 | a valid record passes `check`; a record that makes the author the independent verifier is rejected; `create` writes a record that passes `check` |

Run it on its own with `./test-acceptance/validate_glossary_entities.sh` (`-v` prints every
command's output). It needs `test-executor`, `verifier` and `validate-yaml` built with
`make build`, or in `$CARGO_TARGET_DIR`.

## Not in this CLI

- Review gating (a status change needs an approving review) is not enforced here. Records are validated by schema only.
- Coupling analysis is not imported; coverage-import reports structural and requirements-based figures only.
- Test plan activities, software levels and objectives are carried by the schemas and not yet read by any command.
