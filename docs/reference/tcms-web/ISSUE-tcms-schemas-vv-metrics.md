# tcms-schemas issue: Define schemas for V&V metrics, quality-gate verdict and coverage

Copied from the tcms-schemas issue tracker (issue 1), for reference. Links to the tcms-web repository were replaced by its name.

## Why

The Verdict web application (tcms-web) computes V&V metrics and the quality-gate verdict. Those figures are reported by the CLI, in printed and shared reports, and in documents, so their shape should be defined here, once. Without a published schema, each consumer invents its own fields and the same run can show different numbers.

Verdict keeps its current code until tcms-cli and this repository provide the feature. After that, Verdict should read these values from the schema-defined output instead of computing them.

## What to define

1. **Pass-rate and gate result** (per run):
   - `denominator`, `passed`, `failed`, `blocked`, `skipped`, `untested`
   - `passRate`: display value (rounded down, integer) and exact value
   - `gateThresholdPercent`
   - `verdict`: `PASSED` | `FAILED` | `NO_DATA`
   - `provisional`: true while the run is still in progress
   - `deltaFromPrevious`: integer percentage points, nullable
2. **V&V run metrics**: `totalTestCases`, `passedTestCases`, `totalSteps`, `passedSteps`, step coverage as passed steps / total steps.
3. **Requirement coverage**: per requirement, `coverage` as one of `COVERED` | `FAILING` | `NOT_RUN` | `NO_CASES`, with the case list and latest results; summary counts (covered, failing, not run, no cases, with defect, open defects).
4. **V&V load summary**: distinct case count, list of case files with schema errors (file, first error), recorded run count.
5. **Report gate block**: the fields in 1 as embedded in report and document outputs.

Each field needs a type, a description, a unit (percent or count), and whether it can be null, with the null meaning spelled out (for example, `passRate` is null when the denominator is zero).

## Decisions to record here

- **Denominator for pass rate.** The gate uses Passed + Failed + Blocked; the V&V view currently uses total test cases. Pick one rule, or define two named fields so the difference is explicit. Do not leave it implicit.
- **Rounding.** Display is rounded down; exact value is kept alongside.
- **Flaky-case rule** (4 or more pass/fail flips across recent runs): a parameter in the report, not a constant in the schema.

## Acceptance criteria

- [ ] JSON Schema files for the result objects above, versioned and published with the other tcms schemas.
- [ ] Every field has a description, type, unit, and nullability; NO_DATA and null cases are shown with an example.
- [ ] Examples are included for: a passing run, a failing run, a run with no data, a run with skipped and untested cases, and a requirement with NOT_RUN and FAILING coverage.
- [ ] The denominator decision above is written into the schema documentation and linked from the glossary.
- [ ] Schema validation runs in CI on the examples.
- [ ] tcms-cli is able to produce output that validates against the schema (see the matching tcms-cli issue).
- [ ] Verdict's current behaviour is recorded as the baseline, so the change is checkable: a fixture run from tcms-web gives the same figures as the new schema examples, or the difference is listed.

## References

- Verdict implementation: tcms-web repository, `src/main/java/ae/tii/java/base/report/PassRate.java`, `vv/TestRunDataService.java`, `service/TraceabilityService.java`, `trace/CoverageClassifier`, `vv/VvLoadSummarizer.java`, `service/ReportService.java`.
- Related: tcms-cli issue "Move V&V metric calculations from tcms-web to tcms-cli".
