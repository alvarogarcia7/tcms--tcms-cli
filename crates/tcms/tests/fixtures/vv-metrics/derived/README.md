# Derived V&V fixtures

SUT-1 contains only passing results, so the cases it lacks are derived here (TCMS-47). Each folder is
self-contained: the unchanged SUT-1 run `runs/RUN-32921` plus one derived run, with edits listed in its
`DERIVED_FROM` file. Run them with:

    tcms vv metrics --root crates/tcms/tests/fixtures/vv-metrics/derived/<folder> --run <derived run id>

| Folder | Derived run | What it covers | Gate | Expected figures |
|--------|-------------|----------------|------|------------------|
| `failing-run` | RUN-DERIVED-FAIL-01 | One failed case after a pass | 1 of 2 = 50, FAILED | delta -50; requirement SUT1-2 FAILING; SUT1_PASS_001 stability FAILING with 1 flip; steps 5 of 6 = 83 |
| `no-data-run` | RUN-DERIVED-NODATA-01 | Skipped only, zero steps | NO_DATA | passRate, delta and stepCoverage null; both requirements NOT_RUN |
| `not-executed-run` | RUN-DERIVED-NOTEXEC-01 | One `not_executed` case (in the denominator as not passed, TCMS-50.2) and one skipped case | 0 of 1 = 0, FAILED | display 0; delta -100; counts notExecuted 1, skipped 1; both requirements NOT_RUN |
| `blocked-only-run` | RUN-DERIVED-BLOCKED-01 | Every result blocked (`error`) | 0 of 2 = 0, FAILED | display 0; delta -100; both requirements NOT_RUN |
| `skipped-outside-denominator` | RUN-DERIVED-SKIP-01 | A skipped case outside the gate denominator | 1 of 1 = 100, PASSED | V&V figure 1 of 2 = 50; SUT1-1 NOT_RUN, SUT1-2 COVERED; delta 0 |
| `uncovered-requirement` | RUN-DERIVED-UNCOVERED-01 | Requirement SUT1-3 listed in requirements.json, no case cites it; results unchanged | 2 of 2 = 100, PASSED | SUT1-3 NO_CASES; requirements summary noCases 1; delta 0 |
| `zero-test-cases` | RUN-DERIVED-EMPTY-01 | A run with `results: []` (no test cases at all) | refused: error, exit 1, nothing on stdout | error names the run; no figures are produced |
| `missing-case-result` | RUN-DERIVED-MISSING-01 | SUT1_PASS_001 has a not_executed result; SUT1_CALC_001 passes | 1 of 2 = 50, FAILED (not_executed in the denominator, TCMS-50.2) | V&V figure 1 of 2 = 50; steps 3 of 6 = 50; SUT1-2 NOT_RUN, SUT1-1 COVERED |

## Hand derivation and the JDK check

The expected figures were derived by hand from D-95 and D-96 (`docs/reference/tcms-web/DECISIONS-D95-D100.md`).
They were then compared with the tcms-web reference classes (`PassRate`, `StabilityClassifier`,
`CoverageClassifier`) compiled and run with JDK 21 (`javac`/`java`), on the same counts:

- 1 pass, 1 fail: `FAILED`, display 50, delta -50
- no results counted (skipped only): `NO_DATA`, delta null
- 1 not_executed, 1 skipped: `FAILED`, display 0 (TCMS-50.2: not_executed is in the denominator as not passed)
- 2 blocked: `FAILED`, display 0, delta -100 against the 1 of 1 run
- 1 pass, 1 skipped: `PASSED`, display 100, delta 0
- 2 passed of 3 against 1 of 1: delta -33 (rounded from -33.3)
- stability `[PASSED, FAILED]`: FAILING, 1 flip; `[PASSED, PASSED]`: STABLE
- coverage `[FAILED]`: FAILING; `[NOT_EXECUTED]`: NOT_RUN with 0 executed; `[SKIPPED]`: NOT_RUN with 1 executed

The JDK check used the tcms-web buckets. Where TCMS-50.2 changes a bucket (`error` as FAILED, `not_executed` in the
denominator as not passed), the figures above follow TCMS-50.2 and differ from the reference snapshot; the
reference is not changed.

The reference's unit tests (`ReportClassesTest`, `TestRunDataServiceTest`) were not run; the Gradle build is not
available offline here.
