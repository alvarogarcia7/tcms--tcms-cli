---
id: TCMS-47
title: Compute V&V metrics and quality-gate verdict in tcms-cli
status: In Progress
assignee: []
created_date: '2026-11-02T06:31:00Z'
updated_date: '2026-11-03T06:00:00Z'
labels:
  - vv-metrics
  - tcms-web-migration
dependencies: []
references:
  - >-
    Reference snapshot: docs/reference/tcms-web/ (tcms-web branch dev, commit
    739083b7a1f9dc2a7c4601ad6059d99e400f3755)
  - >-
    Related CLI tasks: TCMS-40.5 (trace matrix), TCMS-42.6 (per-feature
    verification and coverage roll-up)
  - >-
    Related schema issue: 'Define schemas for V&V metrics, quality-gate verdict
    and coverage' in tcms-schemas (copy in
    docs/reference/tcms-web/ISSUE-tcms-schemas-vv-metrics.md)
documentation:
  - docs/reference/tcms-web/README.md
  - docs/reference/tcms-web/DECISIONS-D95-D100.md
  - docs/reference/tcms-web/ISSUE-tcms-schemas-vv-metrics.md
  - docs/reference/tcms-web/src/main/java/ae/tii/java/base/report/PassRate.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/vv/TestRunDataService.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/service/TraceabilityService.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/trace/CoverageClassifier.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/vv/VvLoadSummarizer.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/service/ReportService.java
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/service/PdfJobService.java
  - >-
    docs/reference/tcms-web/src/test/java/ae/tii/java/base/report/ReportClassesTest.java
  - >-
    docs/reference/tcms-web/src/test/java/ae/tii/java/base/vv/TestRunDataServiceTest.java
  - crates/tcms/tests/fixtures/vv-metrics/README.md
  - docs/reference/tcms-web/src/pages/runs/model.ts
  - docs/reference/tcms-web/src/pages/dashboard/StatusBar.tsx
  - docs/reference/tcms-web/src/pages/ReportsScreen.tsx
  - docs/reference/tcms-web/src/pages/TraceabilityScreen.tsx
  - docs/reference/tcms-web/src/pages/reports/PlanRunReports.tsx
  - >-
    docs/reference/tcms-web/src/main/java/ae/tii/java/base/report/StabilityClassifier.java
  - docs/reference/tcms-web/src/main/java/ae/tii/java/base/report/CsvWriter.java
priority: high
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict (the tcms-web application) computes the V&V metrics itself: pass rate, quality-gate verdict, step coverage, requirement coverage and the V&V load summary. Those calculations belong to the test-reporting process and to the tooling that defines the data, not to the web application. Computing them in tcms-cli makes the figures reproducible from the command line, so a CI job, a document pipeline and Verdict report the same numbers.

Verdict keeps its current implementation until tcms-cli provides this output (and tcms-schemas defines its shape, see the dependency). After that, Verdict will call or consume tcms-cli output instead of recomputing it. Until then, tcms-web is the reference: the snapshot in docs/reference/tcms-web/ is the specification to reproduce.

Known inconsistency to resolve here: the gate uses Passed + Failed + Blocked as its denominator, while the V&V view uses total test cases. The same run can therefore show two pass rates. This task resolves it by naming, not by hiding one of them (see acceptance criteria).

Development constraints:
- The implementer has no access to the tcms-web or tcms-schemas repositories. Everything needed is in docs/reference/tcms-web/ (see its README.md, which lists the backend, frontend and decision files).
- A JDK 21 is installed on the development machine at /home/orca/.local/jdk21 (javac and java). The reference Java can be compiled and run with it; the Gradle build and JUnit jars are not available offline.
- Fixtures: the recorded V&V data of SUT-1 (main branch) is in crates/tcms/tests/fixtures/vv-metrics/sut-1/, with its README. SUT-1 contains only passing results, so the failing, no-data, skipped/untested and NOT_RUN/FAILING cases must be derived as labelled fixture variants (see acceptance criteria).
- Branch facts: tcms-schemas main is the schema repository's default branch and has no V&V metric schema yet; the vendored copy is pinned to an older commit. tcms-web's reference stays on its dev branch, because tcms-web main is 526 commits behind dev and has no report package.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The CLI exposes each metric as a command, or as a field of an existing command's output: pass rate and gate verdict per run, delta from the previous run, V&V run metrics (pass rate by test cases, step coverage), requirement coverage with summary counts, and the V&V load summary. The formulas are written out in the CLI documentation.
- [x] #2 Pass rate and verdict follow D-95 in docs/reference/tcms-web/DECISIONS-D95-D100.md exactly: pass rate = Passed / (Passed + Failed + Blocked); Skipped and Untested are outside it; gate passed when passed * 100 >= gate * denominator, in exact integer arithmetic with no rounding; the display percentage is rounded down; the exact value is kept alongside it; denominator 0 gives NO_DATA. The gate threshold is an input parameter (default 90, range 50..100 as in D-89).
- [x] #3 Unit tests cover the verdict boundaries: gate reached exactly, one below the gate, denominator zero (NO_DATA), all skipped, all untested, and a run with only Blocked results.
- [x] #4 The pass-rate denominator is decided and documented once: the gate verdict uses the D-95 rule; the V&V test-case figure (passed test cases / total test cases) is a separately named field (for example vvPassRate). The two are never emitted under the same name. Both rules are tested on the same fixture run.
- [x] #5 Run-over-run delta follows D-96: exact pass rate of this run minus the previous run's, in percentage points, rounded to an integer; null when there is no previous run or either side is NO_DATA. Tests cover first run, NO_DATA on either side, and a positive, negative and zero delta.
- [x] #6 Requirement coverage classifies each requirement as COVERED, FAILING, NOT_RUN or NO_CASES, following the classification in docs/reference/tcms-web/src/main/java/ae/tii/java/base/trace/CoverageClassifier.java and TraceabilityService.java. Tests cover each class and mixed statuses. Summary counts are produced: covered, failing, not run, no cases, with defect, open defects.
- [x] #7 Step coverage is passed steps / total steps, and V&V run pass rate is passed test cases / total test cases, as in TestRunDataService.java. Tests cover a run with zero steps.
- [x] #8 Flaky-case detection follows D-97: per case, only Passed and Failed results count (oldest to newest); flips are changes between neighbouring kept results; a case is Flaky when flips >= threshold, otherwise Failing when its last kept result is Failed, otherwise Stable. The threshold is a parameter with default 4, not a constant. Tests cover flips just below, at and above the threshold, Blocked/Skipped/Untested being ignored, and Flaky outranking Failing.
- [x] #9 V&V load summary reports distinct case count, case files with schema errors (file and first error only, capped as in VvLoadSummarizer.java) and the number of recorded runs.
- [x] #10 Machine-readable output is JSON with camelCase field names, matching the result objects in docs/reference/tcms-web/ISSUE-tcms-schemas-vv-metrics.md (passRate display and exact, verdict, provisional, deltaFromPrevious, denominator, counts, and so on). Null semantics are documented per field.
- [ ] #11 When the result objects are published in tcms-schemas, the CLI output validates against those schemas in CI. Until then, the fields are defined by the schema issue copy in docs/reference/tcms-web/ISSUE-tcms-schemas-vv-metrics.md and this task does not wait for the published schema.
- [x] #12 The CLI documentation and the requirements the repository tracks describe the metrics, formulas, null semantics, the denominator decision and the flaky threshold parameter. The reference snapshot in docs/reference/tcms-web/ is listed as the source of the rules.
- [x] #13 Out of scope and not done here: switching Verdict (tcms-web) to read the CLI output; the browser-side calculations; the report endpoints and PDF rendering in tcms-web; the failing-since rule (D-98) unless a CLI report command already needs it.
- [x] #14 A shared fixture set lives in crates/tcms/tests/fixtures/vv-metrics/. The sut-1/ folder holds the recorded SUT-1 runs and must not be edited.
- [x] #15 The cases SUT-1 lacks (failing, NO_DATA, skipped and untested, and NOT_RUN/FAILING coverage) are added as separate folders under derived/. Each has a DERIVED_FROM file naming the SUT-1 run and the exact edits. Expected figures are derived by hand from D-95 and D-96, checked against the reference code run with the JDK, and noted in the test or fixture.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run make test (passes; test_hook_execution_order_in_script disabled until TCMS-9.1)
<!-- DOD:END -->
