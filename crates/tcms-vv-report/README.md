# tcms-vv-report

Renders the V&V metrics of `tcms vv metrics` as a Markdown or AsciiDoc report (TCMS-47.1).

The crate reads the metrics JSON and writes the report. It computes no figure of its own and does not depend on
the `tcms` crate, so a CI job, a document pipeline and the Verdict web application report the same numbers as the
command. Every figure in the report is the value of the matching field of the JSON (see `docs/VV_METRICS.md`,
"Output").

## Pipeline

```bash
tcms vv metrics --root <DATA_ROOT> [--run ID | --plan ID] [--gate N] [--flaky-threshold N] --out metrics.json
tcms-vv-report --metrics metrics.json [--format markdown|asciidoc] [--out FILE] [--fail-on-gate]
```

| Option | Default | Meaning |
|--------|---------|---------|
| `--metrics <FILE>` | (required) | The metrics JSON. A file that lacks a documented field fails with exit 1 and names the field |
| `--format` | `markdown` | `markdown` or `asciidoc`. JSON is not a report format |
| `--out <FILE>` | stdout | Writes the report; parent directories are created |
| `--fail-on-gate` | off | Exit 2 when the verdict is FAILED, after the report is written. NO_DATA alone does not exit 2 |

The report is deterministic: the same input gives byte-identical output. `generatedAt` comes from the input.

## Exit codes

| Code | When |
|------|------|
| `0` | Report written. Also for a FAILED verdict and for NO_DATA, unless `--fail-on-gate` is given |
| `1` | Any error: unreadable file, invalid JSON, or a documented field missing (the message names it) |
| `2` | `--fail-on-gate` and the verdict is FAILED. The report has been written first |

## Sections (fixed order)

1. **Run**: run ID, started and finished times (`in progress` while the run has no `finished_at`), provisional flag,
   `generatedAt`, gate threshold with its source.
2. **Gate verdict**: verdict, gate pass rate (display and exact), denominator, counts (passed / failed / blocked /
   skipped / untested), delta from the previous run, previous run ID.
3. **V&V run figures**: V&V test-case pass rate (a separate figure from the gate pass rate), passed and total test
   cases, step coverage, passed and total steps.
4. **Requirement coverage**: summary counts, then one row per requirement (coverage, executed of total, problem
   reports, open problem reports).
5. **Stability**: flaky threshold with its source, flaky count, then the cases whose stability is
   FLAKY or FAILING.
6. **Load summary**: distinct cases, runs recorded, schema error count, and the first schema error only (file and
   error).

## Null rendering

| Field | When null | Shown as | Reason |
|-------|-----------|----------|--------|
| `gate.passRate` | The gate denominator is 0 (no passed, failed or blocked result) | `NO_DATA`, with no percentage | Nothing was counted, so there is no rate |
| `gate.deltaFromPrevious` | No previous run, or either run's denominator is 0 | `n/a` | There is no comparable previous rate |
| `gate.previousRunId` | No previous run | `none` | No run precedes the selected one |
| `vvRun.vvPassRate` | No test case results in the run | `n/a` | The total is 0; "0 percent" would be wrong |
| `vvRun.stepCoverage` | No steps in the run | `n/a` | The total is 0 |
| `run.finishedAt` | The run is in progress | `in progress` | The run has no finish time yet |
| `requirements.items[].cases[].status` | The case is not in the run | Not shown; the report lists requirements, not their cases | The requirement row already carries the coverage |

## Escaping

In Markdown and AsciiDoc tables, a pipe in a cell is written `\|`, and a line break becomes a space, so case IDs and
schema error messages cannot split a table row.

## Relation to other reports

Traceability (requirement to test case to verdict) stays with `test-executor report` (TCMS-43). This crate does not
change it.

## Tests

`cargo test -p tcms-vv-report`. The golden reports in `tests/golden/` were generated from the metrics JSON of
`tcms vv metrics` for the SUT-1 fixture and each derived fixture in `crates/tcms/tests/fixtures/vv-metrics/`. A golden
test whose file is missing fails and names the path.
