---
id: TCMS-47.1
title: >-
  Add the tcms-vv-report crate: render the V&V metrics as a Markdown or AsciiDoc
  report
status: To Do
assignee: []
created_date: '2026-11-02T23:25:00Z'
updated_date: '2026-11-02T23:26:00Z'
labels:
  - vv-metrics
dependencies: []
references:
  - TCMS-47 - Compute-VV-metrics-and-quality-gate-verdict-in-tcms-cli.md
  - docs/VV_METRICS.md
  - crates/tcms/src/vvcmd.rs
  - crates/tcms/tests/fixtures/vv-metrics/README.md
  - docs/data_flow/DATA_FLOW.md
  - Cargo.toml
parent_task_id: TCMS-47
priority: high
type: feature
ordinal: 91100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Why

Document pipelines need the V&V gate verdict, pass rate and coverage as a readable report, not only as JSON. Today the only report command is test-executor report, which renders the traceability matrix, summary, gaps and problems, but not the gate verdict or the V&V metrics. The DATA_FLOW document (section 4.5) states that the tcms crate has no report command, so the metric report has no source in the CLI.

TCMS-47 says a CI job, a document pipeline and Verdict must report the same numbers. The report must therefore not compute figures of its own.

## Decided design

A new workspace crate, crates/tcms-vv-report, with a binary tcms-vv-report and a library that renders the report. It does not depend on the tcms crate and computes nothing.

Input is the JSON document written by `tcms vv metrics` (its output contract is in docs/VV_METRICS.md). The pipeline is:

    tcms vv metrics --root <DIR> [--run ID | --plan ID] [--gate N] [--flaky-threshold N] --out metrics.json
    tcms-vv-report --metrics metrics.json [--format markdown|asciidoc] [--out FILE] [--fail-on-gate]

CLI of tcms-vv-report:
- --metrics <FILE>: required. The file is read and checked against the documented fields; a file that lacks a field fails with exit 1 and names the field.
- --format markdown (default) or asciidoc. JSON is not a report format: the input already is JSON.
- --out <FILE>: writes the report (parent directories created); without it the report goes to stdout.
- --fail-on-gate: exit 2 when the verdict is FAILED, after the report is written. NO_DATA alone does not exit 2.
- Exit codes: 0 on success, including FAILED and NO_DATA without the flag; 1 on any error; 2 only as above.

Because the report reads the metrics JSON, the figures are the ones `tcms vv metrics` computed, and the output is deterministic for a given input (generatedAt comes from the input).

## Report sections (fixed order)

1. Header: run ID, started and finished times, provisional flag, generatedAt, gate threshold.
2. Gate verdict: the D-95 gate pass rate (display and exact value, with its denominator and counts), verdict, delta from the previous run, previous run ID.
3. V&V run figures: the V&V test-case pass rate (labelled as vvPassRate, not as the gate rate), passed and total test cases, step coverage with passed and total steps.
4. Requirement coverage: the summary counts, then one row per requirement (coverage, executed of total, problem reports).
5. Stability: flaky threshold, flaky count, and the cases that are Flaky or Failing.
6. Load summary: distinct cases, runs recorded, schema error count, and the listed schema errors (file and first error only).

Null rendering: a null passRate shows NO_DATA with no percentage. A null delta shows n/a. Each null field has its reason written in the crate README.

Markdown and AsciiDoc tables escape pipes and line breaks in IDs and error messages.

## Relation to other reports

traceability (requirement to test case to verdict) stays with test-executor report (TCMS-43). This task does not change it.

## Out of scope

JSON output; schema validation of the report (deferred with the metrics schema); PDF rendering; switching Verdict (tcms-web) to read the output; the failing-since rule (D-98); any change to `tcms vv metrics`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/tcms-vv-report is a workspace member with a binary tcms-vv-report; cargo test -p tcms-vv-report passes; the crate does not depend on the tcms crate
- [ ] #2 tcms-vv-report --help documents --metrics, --format, --out and --fail-on-gate with their exit codes, and the null rendering
- [ ] #3 Every figure in the markdown and asciidoc output equals the matching field of the metrics JSON for sut-1 and each derived fixture; a test compares them
- [ ] #4 The gate pass rate and the V&V test-case pass rate are shown under different labels; a test on one fixture shows both values with their own labels
- [ ] #5 NO_DATA: the gate pass rate shows NO_DATA with no percentage and the delta shows n/a; tested on a derived no-data fixture
- [ ] #6 Exit codes: a FAILED verdict exits 0 by default; with --fail-on-gate it writes the report and exits 2; NO_DATA with --fail-on-gate exits 0; tested for each case
- [ ] #7 Input missing a documented field exits 1 and names the field; tested with a metrics file that lacks vvRun
- [ ] #8 Output is byte-for-byte identical for the same input, checked by running the renderer twice in a test
- [ ] #9 Pipes and line breaks in case IDs and schema error messages are escaped in Markdown and AsciiDoc tables; a fixture with a pipe in a schema error message covers it
- [ ] #10 Golden report files for sut-1 and each derived/ fixture live under crates/tcms-vv-report/tests/golden/, generated from the metrics JSON of tcms vv metrics; the fixtures in crates/tcms/tests/fixtures/vv-metrics/sut-1/ are not edited
- [ ] #11 docs/VV_METRICS.md states that the report is produced by tcms-vv-report from the metrics JSON, and the crate README documents sections, exit codes and null rendering; traceability stays with test-executor report
- [ ] #12 docs/data_flow/DATA_FLOW.md section 4.5 names tcms-vv-report as the source of the metric report, replacing the statement that the tcms crate has no report command
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
