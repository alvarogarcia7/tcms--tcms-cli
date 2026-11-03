# V&V metrics: `tcms vv metrics` (TCMS-47)

`tcms vv metrics` computes the V&V metrics and the quality-gate verdict of recorded test runs, so a CI job,
a document pipeline and the Verdict web application report the same figures. It reads a data root and
prints one JSON document on stdout.

```bash
tcms vv metrics --root <DATA_ROOT> [--run <RUN_ID>] [--gate <PERCENT>] [--flaky-threshold <N>]
```

| Option | Default | Meaning |
|--------|---------|---------|
| `--root` | (required) | Directory with `runs/<RUN_ID>/test-run.yaml`, the test case files and `requirements.json` |
| `--run` | last recorded run | Run to report. Runs are ordered by `started_at`, then run id |
| `--plan` | all runs | Plan to report on: only runs whose `plan_id` equals it take part (selection, previous run, stability and `runsRecorded`). No such run is an error; `--run` must be a run of that plan |
| `--gate` | `90` | Quality-gate threshold in percent, `50` to `100` (D-89). Other values are refused |
| `--flaky-threshold` | `4` | Flips at or above which a case is `FLAKY` (D-97). Must be at least 1 |
| `--schemas-root` (global) | `$TCMS_SCHEMAS_ROOT` or `schemas/` | Used to validate the test case files |

The formulas follow the decisions D-95 to D-97 and the tcms-web reference snapshot in
`docs/reference/tcms-web/`. The reference (`report/PassRate.java`, `report/StabilityClassifier.java`,
`trace/CoverageClassifier.java`, `vv/TestRunDataService.java`) is the specification; the rules below are
what this command implements.

## Data root

- `runs/<RUN_ID>/test-run.yaml`: `run_id`, `plan_id` (optional, used by `--plan`), `started_at`, `finished_at` and `results` (one per test case with a
  `verdict` and an optional `problem_report_id`). A run with no `finished_at` is `provisional`.
- `runs/<RUN_ID>/verification-results.yaml` (optional): `test_results` with `total_steps` and `passed_steps` per
  test case. A case without an entry has 0 steps.
- Test case files: any YAML file with `type: test_case` under the root, except in `runs/`. Each is validated
  against its schema. A file that fails is listed under `loadSummary.schemaErrors`, not counted as a case.
- `requirements.json` (optional): `{"requirements": [{"id": ...}]}`. Requirements cited by a case file (`requirement:`)
  are included as well.

### Verdict vocabulary

| `verdict` in test-run | Status |
|-----------------------|--------|
| `pass` | PASSED |
| `fail` | FAILED |
| `error`, `blocked` | BLOCKED |
| `skipped` | SKIPPED |
| `not_executed`, `untested` | UNTESTED |

Any other value is an error that names the test case. `error` is mapped to BLOCKED because the test could not
reach a verdict; this mapping is a decision of this command, not of the test-run schema.

## Gate and pass rate (D-95)

Counts are taken from the selected run. A selected run with zero test case results is an error (the command
exits non-zero and names the run); it is not reported as `NO_DATA`.

- `denominator` = Passed + Failed + Blocked. Skipped and Untested are outside it.
- `passRate.exact` = Passed / denominator × 100, not rounded.
- `passRate.display` = the exact value rounded **down** (integer). A badge never reads "90% FAILED".
- `verdict` = `PASSED` when `passed × 100 ≥ gateThresholdPercent × denominator` (exact integer arithmetic, no
  rounding), otherwise `FAILED`. A denominator of 0 gives `NO_DATA`.
- `provisional` is `true` while the run has no `finished_at`.

## Delta from the previous run (D-96)

- `deltaFromPrevious` = the exact pass rate of this run minus the previous run's, in percentage points, rounded to
  an integer with halves rounded up (`-12.5` gives `-12`).
- The previous run is the one before the selected run in the ordering above. The same threshold applies.
- `null` when there is no previous run, or when either run's denominator is 0 (`NO_DATA`).

## V&V run figures (TestRunDataService)

These are a second, separately named figure. The gate figure and this one are never emitted under the same name.

- `vvRun.vvPassRate` = passed test cases / total test cases, where a test case is one result of the run. It
  uses the rounding of the V&V view (half up), so `1 of 8` shows `13`. Skipped and untested cases stay in the
  total. This is why a run with one skipped case can show `100` for the gate and `50` here.
- `vvRun.stepCoverage` = passed steps / total steps, over the steps of the verification results (half up).

Both are `null` when the total is 0. The reference returns 0 in that case; this command does not, so that "no
steps" is not shown as "0 percent".

## Requirement coverage (D-116.3)

Each requirement is classified from the statuses of its linked case files in the selected run:

| Class | Rule |
|-------|------|
| `NO_CASES` | No linked case file |
| `FAILING` | Any linked case is FAILED |
| `COVERED` | Every linked case is PASSED |
| `NOT_RUN` | Anything else: a case is untested, blocked, skipped, or not in the run |

`executed` counts the linked cases that have a result other than UNTESTED; `total` is the number of linked cases.

`summary` counts: `covered`, `failing`, `notRun`, `noCases`, `withDefect`, `openDefects`.

- `withDefect` = requirements with at least one `problem_report_id` on a linked result of the selected run.
- `openDefects` = distinct problem report ids on a linked result that did not pass.

**Limitation:** tcms-cli has no defect tracker state. A problem report counts as open while its result did not
pass; the tracker's closed state is not read. The reference counts defects from the failing streak
(D-98), which is not implemented here.

## Stability and flaky cases (D-97)

For each case in the selected run, over the runs up to and including the selected run, oldest first:

- Only PASSED and FAILED results count. BLOCKED, SKIPPED, UNTESTED and absent results are ignored.
- A flip is a change between two neighbouring kept results.
- `FLAKY` when flips ≥ `--flaky-threshold` (default 4; a parameter, not a constant). `FLAKY` outranks `FAILING`.
- Otherwise `FAILING` when the last kept result is FAILED, otherwise `STABLE` (including no kept results).

`stability.flakyCount` counts the `FLAKY` cases in the selected run.

## Output

All field names are camelCase. Nullable fields are `null` as described.

```json
{
  "run":          { "runId", "startedAt", "finishedAt" (null while in progress), "provisional" },
  "gate":         { "gateThresholdPercent", "denominator", "counts": {passed, failed, blocked, skipped, untested},
                    "passRate": {"display", "exact"} | null, "verdict": PASSED|FAILED|NO_DATA,
                    "provisional", "deltaFromPrevious": int | null, "previousRunId": string | null },
  "vvRun":        { "totalTestCases", "passedTestCases", "vvPassRate": {"display","exact"} | null,
                    "totalSteps", "passedSteps", "stepCoverage": {"display","exact"} | null },
  "requirements": { "summary": {total, covered, failing, notRun, noCases, withDefect, openDefects},
                    "items": [ {id, coverage, executed, total, cases: [{caseId, status|null}],
                                problemReports, openProblemReports} ] },
  "stability":    { "flakyThreshold", "flakyCount", "cases": [ {caseId, stability, flips} ] },
  "loadSummary":  { "distinctCases", "schemaErrorCount", "schemaErrors": [{file, error}] (at most 20),
                    "runsRecorded" }
}
```

- `passRate` and `deltaFromPrevious` are `null` for `NO_DATA`. `display` and `exact` are both `null` only as a pair
  (the whole object is `null`).
- `status` in `cases` is `null` when the case is not in the run.
- `loadSummary.distinctCases` counts the ids of the valid case files and the test cases of the verification
  results in all runs, each id once.

## Combining per-case results (tcms vv combine)

`tcms vv combine` turns the per-case results of one run into the run record that `tcms vv metrics` reads
(TCMS-47.3). It is the producer; `tcms vv metrics` is still read-only.

```bash
tcms vv combine --root <DATA_ROOT> --run RUN-42 --plan PLAN-7 \
  --build-id fw-1.4.2 --environment "Staging bench" --tester ci-bot
```

Input, under `runs/<RUN_ID>/`:

- `per-case/<KEY>_result.yaml`: one `type: test_result` file per case, as written by
  `scripts/convert_verification_to_result_yaml.py` (`test_case_id`, `total_steps`, `passed_steps`,
  `failed_steps`, `overall_pass`).
- `execution_logs/<KEY>_execution_log.json`: the execution log of the case. Its `timestamp` entries give the
  case's start and finish. `KEY` is the file name without `_result.yaml`; it equals the test case id for
  ordinary files.

Output: `test-run.yaml` and `verification-results.yaml` in the same run folder. Both are validated before
either is written. A failing validation names the problem and writes nothing.

Rules (see `docs/RUN_RECORD_FORMAT.md`):

- **Verdict:** a case with steps and none executed is `not_executed`. Otherwise `overall_pass: true` gives
  `pass` and `false` gives `fail`.
- **Duplicates:** when a test case id has several attempts, the executed attempt with the latest start wins.
  A `not_executed` attempt never replaces an executed one; it counts only when no attempt was executed.
- **Run times:** `started_at` is the earliest and `finished_at` the latest case timestamp of the run.
- **Metadata:** `plan_id`, `build_id`, `environment` and `tester` come from the flags. The per-case files do
  not carry them. `build_id`, `environment` and `tester` are required by the test-run schema, so a missing one
  makes the command fail.
- **Layout of `verification-results.yaml`:** validated against `schemas/tcms/verification-results.schema.json`
  before writing. The list it carries is `test_results`, the key `tcms vv metrics` reads.

## Schema validation (deferred)

The result objects are to be published in tcms-schemas (ISSUE-tcms-schemas-vv-metrics.md). Until then, this
document defines the fields and the CLI output is not validated against a published schema in CI. TCMS-47
does not wait for it.

## Out of scope

Verdict (tcms-web) still computes these figures; switching it to read this output is a follow-up. The report
endpoints, PDF rendering, the failing-since rule (D-98) and the browser-side calculations are not part of this
command.

## Fixtures and tests

- `crates/tcms/tests/fixtures/vv-metrics/sut-1/`: recorded SUT-1 runs, not edited.
- `crates/tcms/tests/fixtures/vv-metrics/derived/`: failing, no-data, blocked-only and skipped cases, each with
  a `DERIVED_FROM` file and the expected figures in `derived/README.md`.
- `crates/tcms/tests/fixtures/vv-combine/`: one run of 62 cases in the layout of the conversion script, with a
  duplicate case and three not-executed cases. Layout and expected figures: its `README.md`.
- `crates/tcms/tests/vv_combine_cli.rs`: `tcms vv combine` on that fixture, the duplicate rule, the
  not_executed rule, the read-back by `tcms vv metrics`, and the no-write-on-failure rule.
- `crates/tcms/tests/vv_metrics_cli.rs`: the CLI on the fixtures; `crates/tcms/src/vvmetrics.rs` has the unit
  tests for the boundaries (gate exactly reached, one below, denominator zero, all skipped, all untested,
  only blocked, delta signs, flip boundaries).
