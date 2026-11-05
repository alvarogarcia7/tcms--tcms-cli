# V&V metrics: `tcms vv metrics` (TCMS-47)

`tcms vv metrics` computes the V&V metrics and the quality-gate verdict of recorded test runs, so a CI job,
a document pipeline and the Verdict web application report the same figures. It reads a data root and
prints one JSON document on stdout.

```bash
tcms vv metrics --root <DATA_ROOT> [--run <RUN_ID>] [--gate <PERCENT>] [--flaky-threshold <N>] [--stability-window <N>]
tcms vv history --root <DATA_ROOT> --case <CASE_ID> [--plan <PLAN_ID>]
tcms vv trends  --root <DATA_ROOT> [--runs <N>] [--plan <PLAN_ID>] [--gate ...] [--flaky-threshold ...] [--stability-window ...]
```

| Option | Default | Meaning |
|--------|---------|---------|
| `--root` | (required) | Directory with `runs/<RUN_ID>/test-run.yaml`, the test case files and `requirements.json` |
| `--run` | last recorded run | Run to report. The run **folder name** under `runs/` is the only id accepted (TCMS-50.6); the `run_id` written inside `test-run.yaml` is not used for lookup. An unknown id fails and the message lists the valid folder names. Runs are ordered by `started_at`, then folder name |
| `--plan` | all runs | Plan to report on: only runs whose `plan_id` equals it take part (selection, previous run, stability and `runsRecorded`). No such run is an error; `--run` must be a run of that plan |
| `--gate` | `90`, or the file, see below | Quality-gate threshold in percent, `50` to `100` (D-89). Other values are refused |
| `--flaky-threshold` | `4`, or the file, see below | Flips at or above which a case is `FLAKY` (D-97). Must be at least 1 |
| `--stability-window` | `12`, or the file, see below | Runs of history the stability rule looks at: the last N runs up to and including the selected run. Must be at least 1. Overrides `stability_window_runs` of `vv-config.yaml` (TCMS-47.2, D-47-4) |
| `--schemas-root` (global) | `$TCMS_SCHEMAS_ROOT` or `schemas/` | Used to validate the test case files |

The formulas follow the decisions D-95 to D-97 and the tcms-web reference snapshot in
`docs/reference/tcms-web/`. The reference (`report/PassRate.java`, `report/StabilityClassifier.java`,
`trace/CoverageClassifier.java`, `vv/TestRunDataService.java`) is the specification; the rules below are
what this command implements.

## Per-product settings (`vv-config.yaml`, TCMS-47.2)

Verdict keeps the per-product V&V settings in `<DATA_ROOT>/vv-config.yaml`. When that file exists, the command
reads it and uses these keys:

```yaml
type: vv-config
schema: tcms/vv-config.schema.v1.json
gate_threshold_percent: 95   # integer 50 to 100
flaky_flip_threshold: 3      # integer, at least 1
stability_window_runs: 12    # integer, at least 1
```

- **Precedence:** a flag over the file over the default. `--gate` overrides `gate_threshold_percent`,
  `--flaky-threshold` overrides `flaky_flip_threshold`, `--stability-window` overrides `stability_window_runs`.
  The default is 12 for the window.
- **Missing file:** the command behaves as without the file (defaults and flags only).
- **Errors:** an unknown key, a value of the wrong type (for example `gate_threshold_percent: "90"`) or an
  out-of-range value fails, and the message names the file and the key. `type` and `schema` are accepted with any
  value and not checked; checking them comes with the schema (TCMS-47.2 AC 7). A bad flag fails whether or not the
  file exists.
- **Source labels:** the output's `parameters` object says where each value came from:
  `"gateThresholdPercent"`, `"flakyThreshold"` and `"stability_window_runs"` each have `value` and `source`, where
  `source` is `file`, `flag` or `default`.
- **Schema validation: BLOCKED.** TCMS-47.2 AC 7 (validation against `tcms/vv-config.schema.json`) cannot be met yet:
  the schema is not in this repository. The test that will check it is in `crates/tcms/tests/vv_config_cli.rs`,
  marked `ignore`, and runs once the schema is vendored. Until then the checks above are the only checks.

## Stability window (TCMS-47.2)

`--stability-window N` limits the stability history (D-97) to the last `N` runs up to and including the selected
run. The default is 12. The value can also be set in `vv-config.yaml` as `stability_window_runs` (integer, at
least 1); the flag overrides the file (D-47-4).

**Behaviour change:** before this option, the history was every run up to the selected one. For a data set
with more than 12 runs, a case's flip count and its `FLAKY`/`STABLE` status can differ from the earlier output.
Data sets of 12 runs or fewer are unchanged. To reproduce the earlier figures for a longer history, pass
`--stability-window` with the number of runs in the data set.

## Data root

- `runs/<RUN_ID>/test-run.yaml`: `run_id`, `plan_id` (optional, used by `--plan`), `started_at`, `finished_at` and `results` (one per test case with a
  `verdict` and an optional `problem_report_id`). A run with no `finished_at` is `provisional`.
- `runs/<RUN_ID>/verification-results.yaml` (optional): `test_results` with `total_steps` and `passed_steps` per
  test case. A case without an entry has 0 steps.
- Test case files: any YAML file with `type: test_case` under the root, except in `runs/`. Each is validated
  against its schema. A file that fails is listed under `loadSummary.schemaErrors`, not counted as a case.
- `requirements.json` (optional): `{"requirements": [{"id": ...}]}`. Requirements cited by a case file (`requirement:`)
  are included as well.

### Verdict vocabulary (TCMS-50.2)

| `verdict` in test-run | Status | In the gate denominator |
|-----------------------|--------|-------------------------|
| `pass` | PASSED | yes, passed |
| `fail` | FAILED | yes, failed |
| `error` | FAILED | yes, failed |
| `not_executed` | NOT_EXECUTED | yes, not passed (its own count, never FAILED) |
| `skipped` | SKIPPED | no |
| `blocked` (legacy) | FAILED | yes, failed |
| `untested` (legacy) | NOT_EXECUTED | yes, not passed |

Any other value is an error that names the test case. The bucket of `error` and `not_executed` follows Kiwi TCMS,
where failing statuses count against the pass ratio and the not-run state stays in the denominator as not passed.
`skipped` is kept as a value outside the denominator, as before. The test-run schema enum lists only `pass`,
`fail`, `not_executed` and `error`; `skipped`, `blocked` and `untested` are accepted so existing tcms-web data
still loads.

## Gate and pass rate (D-95)

Counts are taken from the selected run. A selected run with zero test case results is an error (the command
exits non-zero and names the run); it is not reported as `NO_DATA`.

- `denominator` = Passed + Failed + NotExecuted. Skipped is outside it (TCMS-50.2).
- `passRate.exact` = Passed / denominator × 100, not rounded.
- `passRate.display` = the exact value rounded **down** (integer). A badge never reads "90% FAILED".
- `verdict` = `PASSED` when `passed × 100 ≥ gateThresholdPercent × denominator` (exact integer arithmetic, no
  rounding), otherwise `FAILED`. A denominator of 0 gives `NO_DATA`.
- `provisional` is `true` while the run has no `finished_at`.

## Delta from the previous run (D-96)

- `deltaFromPrevious` = the printed gate pass rate of this run (floored) minus the printed rate of the previous run
  (floored), an integer (TCMS-50.3, option a). A move from 0 to 12.5 percent prints 0 and 12 and gives `+12`; the
  reverse gives `-12`. Every printed delta equals the difference of the printed rates.
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
| `NOT_RUN` | Anything else: a case is not executed, skipped, or not in the run |

`executed` counts the linked cases that have a result other than NOT_EXECUTED; `total` is the number of linked cases.

`summary` counts: `covered`, `failing`, `notRun`, `noCases`, `withDefect`, `openDefects`.

- `withDefect` = requirements with at least one `problem_report_id` on a linked result of the selected run.
- `openDefects` = distinct problem report ids on a linked result that did not pass.

**Limitation:** tcms-cli has no defect tracker state. A problem report counts as open while its result did not
pass; the tracker's closed state is not read. The reference counts defects from the failing streak
(D-98), which is not implemented here.

## Stability and flaky cases (D-97)

For each case in the selected run, over the last `--stability-window` runs (default 12) up to and including the
selected run, oldest first:

- Only PASSED and FAILED results count; `error` and `blocked` count as FAILED. NOT_EXECUTED, SKIPPED and absent
  results are ignored.
- A flip is a change between two neighbouring kept results.
- `FLAKY` when flips ≥ `--flaky-threshold` (default 4; a parameter, not a constant). `FLAKY` outranks `FAILING`.
- Otherwise `FAILING` when the last kept result is FAILED, otherwise `STABLE` (including no kept results).

`stability.flakyCount` counts the `FLAKY` cases in the selected run.

## Output

All field names are camelCase. Nullable fields are `null` as described.

```json
{
  "generatedAt": string,
  "parameters":   { "gateThresholdPercent": {"value", "source"}, "flakyThreshold": {"value", "source"},
                    "stability_window_runs": {"value", "source"} },
  "run":          { "runId", "startedAt", "finishedAt" (null while in progress), "provisional" },
  "gate":         { "gateThresholdPercent", "denominator", "counts": {passed, failed, notExecuted, skipped},
                    "passRate": {"display", "exact"} | null, "verdict": PASSED|FAILED|NO_DATA,
                    "provisional", "deltaFromPrevious": int | null, "previousRunId": string | null },
  "vvRun":        { "totalTestCases", "passedTestCases", "vvPassRate": {"display","exact"} | null,
                    "totalSteps", "passedSteps", "stepCoverage": {"display","exact"} | null },
  "requirements": { "summary": {total, covered, failing, notRun, noCases, withDefect, openDefects},
                    "items": [ {id, coverage, executed, total, cases: [{caseId, status|null}],
                                problemReports, openProblemReports} ] },
  "stability":    { "flakyThreshold", "flakyCount", "cases": [ {caseId, stability, flips} ] },
  "failingCases": [ {caseId, failureStartRunId} ],   // sorted by caseId (TCMS-50.4)
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
- **Duplicates (TCMS-50.1):** when a test case id has several attempts, they are ordered by `started_at` (oldest
  first), and equal `started_at` is broken by the attempt file key (the greater key is later). Walking from the
  newest back, the first `pass`, `fail` or `error` decides the case. `not_executed` and `skipped` never decide. When
  nothing decides, the case is `not_executed`. For the verdicts `combine` reads (`pass`, `fail`, `not_executed`) this
  gives the same result as the earlier rule of TCMS-47.3 AC 3; the rule is now stated once, here, and the `error`
  and `skipped` cases are defined by it.
- **Run times:** `started_at` is the earliest and `finished_at` the latest case timestamp of the run. `started_at` is
  required by the test-run schema, so `vv combine` fails (exit 1) when no case has an execution log with a timestamp,
  and no run record is written.
- **Metadata:** `plan_id`, `build_id`, `environment` and `tester` come from the flags. The per-case files do
  not carry them. `build_id`, `environment` and `tester` are required by the test-run schema, so a missing one
  makes the command fail.
- **Layout of `verification-results.yaml`:** validated against `schemas/tcms/verification-results.schema.json`
  before writing. The list it carries is `test_results`, the key `tcms vv metrics` reads.

## Case history and failing cases (`tcms vv history`, TCMS-50.4)

`--plan` keeps only the runs whose `plan_id` equals it. A plan that no recorded run has is an error (exit 1: `no
recorded run for plan <id>`), as for `tcms vv metrics`.

```bash
tcms vv history --root <DATA_ROOT> --case <CASE_ID> [--plan <PLAN_ID>]
```

Prints a JSON array with one row per recorded run, oldest first (runs ordered as above, scoped by `--plan`):
`[{"runId": "RUN-32386", "status": "PASSED"}, {"runId": "...", "status": null}]`. `status` is `PASSED`, `FAILED`,
`NOT_EXECUTED` or `SKIPPED` from the run's result for the case, and `null` when the run has no result for it.

`failingCases` in `tcms vv metrics` lists every case that is `FAILED` in the selected run, sorted by case id. Each
entry has `failureStartRunId`: the oldest run of the consecutive `FAILED` streak that ends at the selected run
(D-98). A `PASSED` run ends the streak; `SKIPPED`, `NOT_EXECUTED` and absent runs are passed over and neither end nor
extend it. `error` and `blocked` count as `FAILED`.

## Trends and heatmap (`tcms vv trends`, TCMS-50.5)

`--runs` must be at least 1; `--runs 0` is refused (exit 1). `--plan` and an unknown plan behave as for `tcms vv history`.

```bash
tcms vv trends --root <DATA_ROOT> [--runs <N>] [--plan <PLAN_ID>] [--gate ...] [--flaky-threshold ...] [--stability-window ...]
```

- `runs`: the gate figure of the last `N` runs (default 12), oldest first. Each row has `runId`, `startedAt`, `passRate`
  (`{display, exact}`, or `null` for a run with no data) and `verdict`. The figure is the one `tcms vv metrics`
  prints for that run.
- `heatmap`: an object about the newest run (of the plan, if `--plan` is given). `cases` has one entry per case,
  `{caseId, flips, stability}`, sorted by case id, with `flips` and `stability` computed over the last
  `--stability-window` runs up to and including that run (D-97); `flakyCount` is the number of flaky cases and
  `flakyThreshold` the value used. The stability label is `FLAKY`,
  `FAILING` or `STABLE`. The window and the flaky threshold come from `vv-config.yaml` and the flags, with the
  precedence of TCMS-47.2.

## Decisions recorded for TCMS-50

- **Gate buckets (TCMS-50.2):** `error` counts as FAILED and `not_executed` as NOT_EXECUTED, both in the gate
  denominator. `not_executed` is never counted as FAILED. Reason: Kiwi TCMS (checked against its source) counts
  failing statuses against the pass ratio and keeps the idle, not-run state in the denominator as not passed.
  `skipped` stays outside the denominator.
- **Duplicates (TCMS-50.1):** see "Combining per-case results" above.
- **Delta (TCMS-50.3):** the difference of the two printed (floored) rates.
- **`--run` contract (TCMS-50.6):** the run folder name, and nothing else.
- **`plan_id` (TCMS-47.4 AC 1):** when no plan is given, `combine` writes `plan_id: ""` (an empty string). The
  test-run schema types `plan_id` as a string, so `null` fails the validation that `combine` runs before writing, and
  an omitted key cannot be told apart from a missing one. `--plan ""` selects nothing.

## Report (tcms-vv-report)

The metric report is produced by `tcms-vv-report` from the metrics JSON that `tcms vv metrics --out` writes (TCMS-47.1).
The report computes no figure of its own: every number in it is a field of that JSON, and the gate pass rate and the
V&V test-case pass rate are shown under different labels.

```bash
tcms vv metrics --root <DATA_ROOT> --out metrics.json
tcms-vv-report --metrics metrics.json --format markdown --out report.md [--fail-on-gate]
```

Exit codes: `0` (also for FAILED and NO_DATA), `1` on error or a missing documented field, and `2` only with
`--fail-on-gate` and a FAILED verdict, after the report is written. The sections, the null rendering and the table
escaping are in [crates/tcms-vv-report/README.md](../crates/tcms-vv-report/README.md). Traceability stays with
`test-executor report` (TCMS-43).

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
- `crates/tcms/tests/fixtures/vv-metrics/config/`: `vv-config.yaml` files (valid, wrong type, unknown key,
  out of range) and `golden-sut1-default.json`, the output before TCMS-47.2. Described in its `README.md`.
- `crates/tcms/tests/vv_config_cli.rs`: the config file, precedence (flag over file over default), the stability window
  and each error case, on a copy of SUT-1.
- `crates/tcms/tests/vv_metrics_cli.rs`: the CLI on the fixtures; `crates/tcms/src/vvmetrics.rs` has the unit
  tests for the boundaries (gate exactly reached, one below, denominator zero, all skipped, all untested,
  only blocked, delta signs, flip boundaries).
