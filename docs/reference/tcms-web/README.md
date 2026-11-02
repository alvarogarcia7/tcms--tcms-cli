# tcms-web reference snapshot (V&V metrics)

Read-only copy of the code that computes the V&V metrics today in the tcms-web application
(branch `dev`, commit `739083b7a1f9dc2a7c4601ad6059d99e400f3755`, 2026-11-02). It exists so that
tcms-cli can be developed without access to the tcms-web repository.

- Nothing here is compiled or tested by this repository. Do not edit these files.
- Paths mirror the tcms-web layout under `src/main/java/ae/tii/java/base/` and `src/test/java/ae/tii/java/base/`.
- `DECISIONS-D95-D100.md`: the product decisions the code implements (pass rate, verdict, delta,
  stability, failing-since, report endpoints, CSV export). Treat them as the specification.
- `ISSUE-tcms-schemas-vv-metrics.md`: the schema issue that defines the output shape.

## Files

| Snapshot path | Purpose |
|---------------|---------|
| `report/PassRate.java` | Pass rate, gate verdict (PASSED / FAILED / NO_DATA), delta |
| `vv/TestRunDataService.java` | V&V run metrics: pass rate by test cases, step coverage |
| `service/TraceabilityService.java` | Requirement coverage and summary counts |
| `trace/CoverageClassifier.java` | COVERED / FAILING / NOT_RUN / NO_CASES classification |
| `vv/VvLoadSummarizer.java` | V&V load summary (distinct cases, schema-error files, run count) |
| `service/ReportService.java` | Gate and verdict in reports (run summary, history, CSV) |
| `service/PdfJobService.java` | Gate and verdict in printed/shared PDF jobs |
| `report/StabilityClassifier.java` | Stable / Failing / Flaky classification (D-97) |
| `report/FailingSince.java` | Failing-since streak (D-98) |
| `report/SuiteBreakdown.java` | Per-suite rows for reports (D-99) |
| `report/CsvWriter.java` | CSV export (D-100) |
| `report/ReportClassesTest.java` | Unit tests for the report classes (pass rate, delta, stability) |
| `vv/TestRunDataServiceTest.java` | Unit tests for V&V run metrics |

### Frontend (browser-side calculations to be replaced)

| Snapshot path | Calculation |
|---------------|-------------|
| `pages/runs/model.ts` | Plan-run tallies per status, progress, executed |
| `pages/runs/RunTally.tsx`, `pages/runs/RunResultsStatus.tsx` | Tally display of a run |
| `pages/dashboard/StatusBar.tsx`, `pages/dashboard/PlanDashboard.tsx` | Dashboard KPIs and run progress |
| `pages/DashboardScreen.tsx` | Dashboard V&V KPIs |
| `pages/ReportsScreen.tsx`, `pages/reports/PlanRunReports.tsx` | Report pass rate, coverage, suite sorting, flaky detection |
| `pages/TraceabilityScreen.tsx`, `pages/traceability/PlanTraceability.tsx` | Traceability summary counts and the CSV export of the V&V matrix |

## Verified with the JDK

`PassRate.java` was compiled with JDK 21 (`/home/orca/.local/jdk21`) and run on the boundary cases
named in TCMS-47. Results: 9/10 at gate 90 gives PASSED with display 90; 89/100 gives FAILED with
display 89; 2/3 gives display 66 (rounded down, exact 66.67); a zero denominator gives NO_DATA with
null display and null exact; 0/0/3 (blocked only) gives FAILED with display 0; the delta of 2/3
against 1/1 is -33, and the delta against a NO_DATA run is null. The unit tests in `report/` and
`vv/` were not run, since the Gradle build and JUnit jars are not available offline here.

## Branch

The snapshot is taken from `dev`, the branch where the V&V code lives. The `main` branch of
tcms-web is 526 commits behind `dev` and has no `report/` package, so it cannot be the reference.
