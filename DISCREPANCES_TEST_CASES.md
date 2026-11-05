# Discrepancies found by the tcms vv test cases

Branch: `backlog/verdict-integration-tcms-50` (from origin, which is GitLab). `origin/dev` was already up to date.
**No code was changed.** The test folders are in `crates/tcms/tests/fixtures/vv-cases/` (untracked; see its
`README.md`). Each folder has its own inputs, a `README.md` describing what it covers, and a `case.json` with the
arguments and the expected values.

## How this was checked

- **296 case folders**: `metrics` 81, `combine` 52, `history` 50, `trends` 51, `report` 62.
- **Expected values** come from an independent reference model written from `docs/VV_METRICS.md` (and the report
  README for the report). The binary never computes its own expectation.
- **Results:** 294 cases pass and 2 fail. The 2 failures are D-03 (`trends/301`) and D-08 (`metrics/081`). The
  expectations of D-01 and D-02 were changed to the Opus decisions, so those cases now pass. D-04 to D-07 are
  documentation gaps or contradictions, and their cases pass because the binary is right and the docs are wrong.
- **Randomized checks** (scratch scripts, not in the repo): about 1,200 random metrics datasets, 300 random trends
  datasets. Metrics: 509 matched, 679 where both sides refused, and 12 mismatches, all of them D-08. Trends: 260
  matched, 40 both refused, 0 mismatches.
- **Build:** the system has no OpenSSL headers and no sudo, so the binary was built in a scratch copy with
  `openssl-sys` `vendored`, using a writable `CARGO_HOME`. The source is the branch's. All existing VV test suites
  pass in that build (102 unit tests and every `vv_*` integration suite).

## Summary

| ID | Area | Kind | Case folder | Severity | Status |
|----|------|------|-------------|----------|--------|
| D-01 | metrics, vv-config | docs wrong or self-contradictory | `metrics/078-config-type-key-not-checked` | low | **resolved** (docs; test expectation exit 0) |
| D-02 | tcms-vv-report | docs too broad | `report/052-missing-failingCases` | low | **resolved** (docs; test expectation exit 0) |
| D-03 | trends `--runs 0` | binary: silent clamp | `trends/301-runs-zero-assumed-refused` | medium | **resolved** (code) |
| D-04 | tcms-vv-report README | docs wrong | `metrics/080-not-executed-only-is-failed-not-no-data` | medium | **resolved** (docs) |
| D-05 | history, trends | docs gap (unknown `--plan`) | `history/015-two-plans-plan-9`, `trends/021-unknown-plan` | low | **resolved** (docs) |
| D-06 | trends heatmap | docs gap (shape) | all `trends/` cases | low | **resolved** (docs) |
| D-07 | combine | docs gap (no timestamps) | `combine/112-single-not-executed-with-no-log` | low | **resolved** (docs) |
| D-08 | metrics `distinctCases` | binary differs from docs | `metrics/081-plan-filter-distinct-cases-all-runs` | low-medium | **resolved** (code) |

---

## D-01: a vv-config.yaml with a wrong `type` is accepted

- **Docs:** `docs/VV_METRICS.md` around line 45 says "a file with a wrong type ... fails"; the next line (around 46) says
  "`type` and `schema` are accepted and not used". The two conflict.
- **Binary:** `crates/tcms/src/vvconfig.rs` (around lines 23 and 112) accepts any value for `type` and `schema`.
  A file with `type: test_case` exits 0. Behaviour is consistent with the second sentence.
- **Opus decision:** do not validate `type` yet. Reason: the TCMS-47.2 acceptance criterion #4 is about naming the
  file and key for a bad setting value; no vv-config schema is in the repo (AC 7 is blocked); and the expected
  `type` value is not defined anywhere. Change the docs so that the "wrong type" sentence refers to a setting's
  value type, and say that `type` and `schema` are accepted with any value until the schema arrives.
  Confidence about 70%.
- **Proposed docs text** (`VV_METRICS.md` around lines 45-46):
  "- **Errors:** an unknown key, a value of the wrong type (for example `gate_threshold_percent: "90"`) or an
  out-of-range value fails, and the message names the file and the key. `type` and `schema` are accepted with any
  value and not checked; checking them comes with the schema (AC 7)."
- **Test change:** `metrics/078-config-type-key-not-checked` now expects exit 0 and passes. The wrong-value-type case
  is `metrics/` "invalid-config-type" (`gate_threshold_percent: "ninety"`), which still expects exit 1.

## D-02: the report accepts a metrics JSON with `failingCases` removed

- **Docs:** `crates/tcms-vv-report/README.md` (around lines 19 and 30) says a file that "lacks a documented field" fails
  with exit 1. `docs/VV_METRICS.md` lists `failingCases` in its output.
- **Binary:** `crates/tcms-vv-report/src/input.rs` (around lines 13-22) does not read `failingCases`, and the
  renderer never shows it. The report exits 0.
- **Opus decision:** the report requires only the fields it renders. It computes no figure of its own, so a field it
  never shows adds nothing to a reproducible report, and making it required would break metrics output from before
  TCMS-50.4. A CI check for a missing field needs schema validation of the metrics output, which is deferred.
  Confidence about 75%.
- **Proposed README text** (`crates/tcms-vv-report/README.md` line 19): "A file that lacks a field the report shows
  fails with exit 1 and names the field. Fields the report does not show (for example `failingCases`) are not read."
  Line 30 should say "a field the report shows" too.
- **Test change:** `report/052-missing-failingCases` now expects exit 0 and passes. The other removed-field cases
  (`report/050`, `051`, `053`, `054`, `055`) still expect exit 1 because those fields are rendered.

## D-03: `tcms vv trends --runs 0` prints one row

- **Binary:** `crates/tcms/src/vvcmd.rs` around line 968 computes `scoped.len().saturating_sub(runs_n.max(1))`. The
  `.max(1)` turns 0 into 1, so the output is the newest run only, with exit 0. Nothing documents this.
- **Docs:** `docs/VV_METRICS.md` around line 252 says "the last `N` runs (default 12)" with no range. Every other numeric
  flag in the same document states a minimum of 1 and is refused below it.
- **Proposed fix (not applied):** in the `Trends` arm of `vvcmd.rs`, refuse `--runs 0` with "`--runs must be at least
  1`", and delete the `.max(1)` at line 968. Add "at least 1" to the flag text and to `VV_METRICS.md` around line 252.
- **Confidence:** high on the cause. The choice of refusal over an empty result follows the other flags.
- **Case:** `trends/301-runs-zero-assumed-refused` expects exit 1; the README marks it as an assumption by analogy.

## D-04: the report README gives the wrong denominator and the wrong count names

- **Docs (wrong):** `crates/tcms-vv-report/README.md` says the gate pass rate is null when there is "no passed, failed or
  blocked result" (around line 53), and lists the counts as "passed / failed / blocked / skipped / untested" (around
  lines 38-39).
- **Binary and `VV_METRICS.md`:** the denominator is passed + failed + not executed, and skipped sits outside it
  (VV_METRICS.md around line 99). The metrics JSON has `passed`, `failed`, `notExecuted` and `skipped`; there is no
  blocked or untested count. `blocked` and `untested` are legacy verdicts that map to FAILED and NOT_EXECUTED.
- **Check:** a run whose only result is `not_executed` gives `denominator: 1`, `passRate.display: 0`, verdict FAILED.
  This is what the case expects. The README's NO_DATA claim is wrong.
- **Proposed docs fix:** change the null-rendering row to "the gate denominator (passed + failed + not executed) is 0",
  and the counts to "passed / failed / not executed / skipped". No code change. Confidence high.

## D-05: `history` and `trends` with an unknown `--plan` (docs gap)

- **Binary:** both exit 1 with `no recorded run for plan PLAN-404 (no run has plan_id PLAN-404)`, as `metrics` does.
- **Docs:** the unknown-plan rule is stated only for `metrics`.
- **Proposed sentence** for "Case history" and "Trends and heatmap": "`--plan` keeps only the runs whose `plan_id`
  equals it; a plan that no recorded run has is an error (exit 1), as for `metrics`." Confidence high.

## D-06: the trends heatmap shape is undocumented (docs gap)

- **Binary:** `heatmap` is `{"flakyThreshold", "flakyCount", "cases": [{"caseId", "stability", "flips"}]}`, built from the
  newest run of the plan (if given), one entry per case, sorted by case id.
- **Docs:** `VV_METRICS.md` says only "the stability label of each case of the newest run".
- **Proposed sentence:** "`heatmap` is about the newest run (of the plan, if `--plan` is given): `cases` has one entry
  per case, `{caseId, flips, stability}`, sorted by case id, with `flips` and `stability` computed over the last
  `--stability-window` runs up to and including that run; `flakyCount` is the number of flaky cases and
  `flakyThreshold` the value used." Confidence high.

## D-07: `combine` refuses a run with no execution timestamps (docs gap)

- **Binary:** with no case log carrying a timestamp, `vv combine` fails with "no execution timestamps in the execution
  logs, so started_at cannot be set" (exit 1). `test-run.yaml` is not written.
- **Docs:** `VV_METRICS.md` ("Run times") and `RUN_RECORD_FORMAT.md` say `started_at` comes from the case timestamps,
  but not what happens without any. The test-run schema requires `started_at`, so the refusal is right.
- **Proposed sentence:** "`started_at` is required by the test-run schema, so `vv combine` fails (exit 1) when no case
  has an execution log with a timestamp; no run record is written." Confidence high.

## D-08: `distinctCases` is narrowed by `--plan`; the docs say "in all runs"

- **Docs:** `VV_METRICS.md` (Output) says `loadSummary.distinctCases` counts "the ids of the valid case files and the
  test cases of the verification results in all runs, each id once".
- **Binary:** `build_report` in `crates/tcms/src/vvcmd.rs` (around lines 630-646) builds a plan-scoped `VvData` when
  `--plan` is given. The `distinctCases` computation (lines 723-725) reads `data.runs`, so runs of other plans are left
  out. Without `--plan` the count matches the docs.
- **Reproduction:** `metrics/081-plan-filter-distinct-cases-all-runs`. RUN-1a (plan P2) has TC_01; RUN-2b (no plan) has
  TC_02. The docs give `distinctCases: 2`; the binary gives 1.
- **Randomized evidence:** all 12 metrics mismatches are this case, and each is explained exactly by counting only the
  plan's runs.
- **Recommendation (Sonnet check, confirmed from the code and history):** make the count cover all runs, as the docs say.
  In `build_report` (`crates/tcms/src/vvcmd.rs`, lines 723-725), take the ids from `all.runs` instead of `data.runs`:
  `for r in &all.runs { distinct.extend(r.verified_case_ids.iter().map(String::as_str)); }`. `runsRecorded` stays
  plan-scoped. The `--plan` commit (6b37abad) lists the things plan scoping covers, and `distinctCases` is not among
  them; the TCMS-47 backlog AC says nothing about plan scope. No document says the count is plan-scoped.
  Confidence high. No Opus check needed.
  would settle the intent if you want it.

---

## Checked and matching (no discrepancy)

- Rounding: the gate display floors (1 of 8 gives 12) and the V&V pass rate rounds half up (1 of 8 gives 13); step
  coverage 1 of 8 gives 13.
- Gate boundaries are inclusive and use exact integers (9 of 10 at 90 passes; 2 of 3 at 66 passes, at 67 fails); 100
  percent; zero-blocked runs.
- Deltas use the printed (floored) rates and are null when either run has no denominator.
- Stability ignores not_executed and skipped between kept results; error and blocked count as FAILED.
- Duplicate attempts (TCMS-50.1): the newest pass or fail decides; not_executed never decides.
- `--run` takes the folder name only (TCMS-50.6); `--run` with a path (`../x`) is refused, and nothing is written outside.
- Missing required combine flags are refused, with nothing written.
- Report pipe escaping (`\|`) in Markdown and AsciiDoc.
- Report exit codes 0, 1 and 2, including `--fail-on-gate` and NO_DATA.

---

## Found by the coverage and property-based fuzzing round

Each defect below has an ignored test that fails on the current binary; remove `#[ignore]` when fixing.

- **D-09** (`crates/tcms/tests/vv_edge_cli.rs`, `a_case_file_that_does_not_parse_is_listed_in_schema_errors`):
  a case file that is not valid YAML is skipped silently (`vvcmd.rs`, `let Ok(doc) = load_yaml(&path) else { continue }`).
  `VV_METRICS.md` says a failing file is listed under `loadSummary.schemaErrors`. Fix: record a schema error instead.
- **D-10** (`crates/tcms-vv-report/tests/report_properties.rs`, `backslash_before_pipe_keeps_the_table_row_intact`):
  a backslash in a requirement id is not escaped, so `a\|b` renders as `a\\|b`, which Markdown reads as an escaped
  backslash followed by a column separator. Fix: escape the backslash before escaping the pipe.
- **D-11** (`crates/tcms/tests/vv_args_edge_cli.rs`, `an_empty_plan_argument_selects_no_runs`): `--plan ""` selects the
  runs whose `plan_id` is empty, and exits 0. `VV_METRICS.md` says `--plan ""` selects nothing (TCMS-47.4 AC 1).

The second fuzzing round (invalid and schema-violating inputs, by a subagent) also reported these, with reproductions
in its scratch directory: test-run files that are empty, `null`, a list, or missing `run_id`/`verdict` still load in
`metrics` (or fail without naming the field); `verification-results.yaml` values outside the schema (`passed > total`,
negative counts) are accepted; `requirements.json` with numeric ids is accepted; a symlink loop under the root makes
`metrics` run for over a minute; 500k results take more than seven minutes (quadratic scaling near 20k results); the
report accepts a verdict in lowercase or unknown, and rates above 100 are rendered as percentages. These are not yet
ignored tests; they are candidates for the next round.


## Resolution status (after the fix round)

All logged discrepancies are resolved:

- Code: D-03 (`--runs 0` refused, clamp removed), D-08 (`distinctCases` over all runs), D-09 (an unparseable case
  file is a schema error), D-10 (Markdown cells double the backslash before escaping the pipe), D-11 (an empty
  `--plan` selects nothing). The ignored tests for D-09, D-10 and D-11 are enabled.
- Docs: D-01, D-02, D-04, D-05, D-06 and D-07 (`VV_METRICS.md` and the report README), as decided above.
- The 296 captured case folders are recaptured: all match the reference (`capture_expected.py --check`).
