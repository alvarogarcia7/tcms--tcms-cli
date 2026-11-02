# Phase 8 spec: V&V data flow diagram and parameterised acceptance scenarios

Status: refined from the user request; implementation requested via the autonomous loop (default if no answer).

## 1. Diagram
- Update docs/data_flow/2026-01-20_diagram_steps.mermaid (and DATA_FLOW.md) to show the V&V path: test plan config -> test-executor run -> verifier (verification binary) -> test-run record -> trace matrix -> report (tcms report, traceability .md and .adoc).
- Keep one mermaid source; render-check it with mermaid syntax validation.

## 2. Acceptance scenarios (parameterised)
One bash driver, one folder per scenario, containing the original inputs:
- plans/ (test plan and test case YAML), runs/ (recorded run inputs), requirements/ (trace matrix input), tcms-config/ (CLI configuration used).
- Each scenario runs: plan -> generate run -> execute -> verify -> validate -> trace matrix -> traceability document (.md and .adoc only, no PDF).
Scenarios (parameter SCENARIO):
- full-two: every requirement covered by at least two test cases.
- full-one: every requirement covered by exactly one test case.
- all-but-one: every requirement covered except one.
- half: 50% of requirements covered.
- none: 0% of requirements covered.
Assertions per scenario: the verdict roll-up matches the coverage (VERIFIED, NOT RUN, NO TESTS as the rule says), and the document lists every requirement.

### 2.1 Equivalence of inputs

User rules (verbatim):
* Every valid input (validated by JSON schemas python before executing) should return 0 (OK) / error code, except `INVALID_INPUT_FILES` error code (TBD which one)
* Every invalid input (throws error when validated by JSON schemas python before executing) `INVALID_INPUT_FILES` error code (TBD which one)

Decision (D-08-1): `INVALID_INPUT_FILES` = exit code 1, the code tcms and test-executor already use for input, parse and schema-validation errors (ROADMAP Phase 5 SC1; D-05-1).

Mapping:
- Invalid input (fails JSON schema validation): always exit 1 (INVALID_INPUT_FILES), no other code.
- Valid input: exit 0 on success. A valid input may exit with another documented non-zero code (for example 2 for --fail-on-gaps or the --shape conflict), never 1 for a validation reason.
- Exit 1 therefore means "invalid input files"; a valid input must not produce 1 because of its content.

Test to implement: an equivalence test that runs each fixture of a valid set and an invalid set through the real binary (no mocks) and checks exit codes: valid set must exit 0 or a documented non-1 code; invalid set must exit exactly 1 with a validation message on stderr. Each fixture is also checked with the Python jsonschema validator so the set membership (valid or invalid) is derived from the schema, not asserted by hand.

Implementation status: fixtures in `test-acceptance/equivalence/` (valid/test-cases, valid/runs, invalid/test-cases, invalid/runs); test in `test-acceptance/scripts_test/test_equivalence_inputs.sh`. Test cases run through `test-executor generate`, run records through `validate-yaml`. Observed: schema failure exits 1 for both binaries; `validate-yaml` writes its violation message to stdout, not stderr, so the test checks the combined output. The Python jsonschema cross-check is not implemented; set membership is fixed by the fixture directory and confirmed by the binary's own schema check (exit 1 plus a "Schema" message).

## 3. Determinism meta test
- Run one scenario twice with an injected fixed date (--generated-at). The two outputs must be byte-identical: `diff file1 file2` returns zero lines.

## 4. CI
- Run the acceptance set in .gitlab-ci.yml (GitLab) and .github/workflows/workspace.yml (GitHub).

## Assumptions (default, since the 3-minute window passed)
- Use the existing tcms CLI and test-executor report/trace commands; no new report renderer.
- Markdown and AsciiDoc only; no PDF conversion.

## 1a. Diagram detail (added by user)
- For every arrow in the diagram, show an example invocation with fake data and fake files (for example: `test-executor report --matrix <scenario>/requirements/trace_matrix.yaml --runs <scenario>/runs --kind traceability --format markdown --generated-at <fixed-date>`).
- For every step, list the input paths and output paths it reads and writes (plans/, runs/, requirements/, tcms-config/, reports/ ...), so the diagram doubles as a file map.
- Use fake, example values only; no real customer or Jira data.


## 5. Follow-up (after 08 is implemented and validated)
1. Verify the final conversion step (.yaml/.yml/.json -> .adoc/.md) is repeatable (same input, same bytes) and correct (content matches the model; markup escapes hold).
2. Find up to 50 more valid and invalid test cases for the pipeline, not overlapping existing ones (check by content hash and by test_case_id), and add them as fixtures to the equivalence set from 2.1.
3. Push to GitHub (origin) and GitLab (gitlab), then babysit both pipelines until green or until each failure is recorded with its cause.
