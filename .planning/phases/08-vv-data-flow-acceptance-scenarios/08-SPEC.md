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

## 3. Determinism meta test
- Run one scenario twice with an injected fixed date (--generated-at). The two outputs must be byte-identical: `diff file1 file2` returns zero lines.

## 4. CI
- Run the acceptance set in .gitlab-ci.yml (GitLab) and .github/workflows/workspace.yml (GitHub).

## Assumptions (default, since the 3-minute window passed)
- Use the existing tcms CLI and test-executor report/trace commands; no new report renderer.
- Markdown and AsciiDoc only; no PDF conversion.
