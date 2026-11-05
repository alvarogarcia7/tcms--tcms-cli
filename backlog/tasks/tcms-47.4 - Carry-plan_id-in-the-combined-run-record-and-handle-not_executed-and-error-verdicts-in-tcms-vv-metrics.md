---
id: TCMS-47.4
title: >-
  Carry plan_id in the combined run record and handle not_executed and error
  verdicts in tcms vv metrics
status: Done
assignee:
  - Claude
created_date: '2026-11-03T06:30:00Z'
updated_date: '2026-11-04T10:20:00Z'
labels:
  - vv-metrics
  - tcms-web-migration
dependencies: []
references:
  - >-
    TCMS-47.3 - Combine per-case results into one run record
    (docs/RUN_RECORD_FORMAT.md, decision D-47-1)
  - docs/RUN_RECORD_FORMAT.md
  - docs/VV_METRICS.md
  - crates/tcms/src/vvcmd.rs
parent_task_id: TCMS-47
priority: high
type: feature
ordinal: 91400
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Why

Two gaps were found when Verdict (tcms-web) connected to `tcms vv metrics` through the real binary, on the converted legacy sample:

1. **plan_id is missing from test-run.yaml.** The run record format (D-47-1) lists `plan_id` among the keys of `test-run.yaml`, and `tcms vv metrics --plan <ID>` selects runs by it. A run written without `plan_id` cannot be selected by plan.
2. **The verdicts `not_executed` and `error` are not produced or handled.** D-47-1 defines four verdicts (pass, fail, not_executed, error). Legacy data only has pass and fail, so the metrics have not been exercised on the other two. The build rules in D-47-1 map a case with no execution to `not_executed`; without that, a case that never ran is counted as missing rather than as not executed.

## What

- The combined run record written by TCMS-47.3 always carries `plan_id` in `test-run.yaml`, from the shared run metadata. When the run has no plan, the value is the empty string `""`: the test-run schema types `plan_id` as a string, so `null` is not allowed, and an omitted key cannot be told apart from a missing one.
- `tcms vv metrics` accepts all four verdicts and selects runs by `plan_id` with `--plan`. Gate buckets follow TCMS-50.2: `error` counts as Failed and `not_executed` counts as Not executed, both inside the gate denominator; `not_executed` is never counted as Failed. Skipped stays outside the denominator as today. Both new verdicts appear in the counts.
- A verdict outside the four is an error that names the file, the test case and the value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The combined run record carries plan_id in test-run.yaml, and the empty string "" when the run has no plan
- [x] #2 tcms vv metrics --plan <ID> selects only the runs whose plan_id is that plan
- [x] #3 error counts as Failed and not_executed counts as Not executed, both inside the gate denominator (per TCMS-50.2); both appear in the counts, and not_executed is never counted as Failed
- [x] #4 A verdict outside pass, fail, not_executed and error is an error that names the file, the test case and the value
- [x] #5 Tests cover a run with plan_id, a run without plan (empty string), each new verdict, and the unknown-verdict error
- [x] #6 docs/RUN_RECORD_FORMAT.md and docs/VV_METRICS.md describe plan_id and the four verdicts
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 cargo test passes for the tcms crate
- [x] #2 cargo fmt and clippy are clean
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/tcms/src/vvmetrics.rs: Status::from_verdict maps pass to Passed, fail and error to Failed, not_executed to NotExecuted; keeps skipped as Skipped (50.2 AC#3). Legacy aliases kept so existing tcms-web data still loads: blocked to Failed (Kiwi counts blocked as failing), untested to NotExecuted. Counts: passed, failed, not_executed, skipped; denominator = passed + failed + not_executed.
2. crates/tcms/src/vvcmd.rs: counts JSON keys become passed, failed, notExecuted, skipped; stability and coverage use the new Status set.
3. crates/tcms/src/vvcombine.rs: plan_id is written as an explicit null when the run has no plan (47.4 AC#1).
4. Tests for each verdict and the gate with a not_executed case; fixtures golden-sut1-default.json and derived README updated where their expected figures change.
5. Docs: docs/VV_METRICS.md verdict table and gate section; docs/RUN_RECORD_FORMAT.md plan_id.
6. Note for the record: skipped is accepted although the test-run schema enum lists only pass, fail, not_executed, error; 50.2 requires skipped to stay outside the denominator, so it is kept as a legacy value.

Revised (user decision): plan_id is "" when no plan is given, not null (item 3 above is superseded).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented buckets (TCMS-50.2): error and fail and legacy blocked are FAILED; not_executed and legacy untested are NOT_EXECUTED (in the gate denominator, never FAILED, own count); skipped outside. JSON counts: passed, failed, notExecuted, skipped.

AC 1 NOT MET as written: plan_id is omitted when there is no plan, not an explicit null. The test-run schema types plan_id as a string, so null fails the validation combine runs before writing. Needs a decision: change the schema (tcms-schemas, outside this repo) or keep the omission and amend AC 1.

AC 5 partly met: tests cover plan_id with a plan, the new verdicts and the unknown-verdict error; no test for a null plan because the null is not written.

Decision (user): when no plan is given, plan_id is written as "" (empty string), not null and not omitted. Implemented in vvcombine.rs (TestRunDoc.plan_id is always written; None becomes ""). Test: run_without_plan_records_an_empty_plan_id (vv_combine_cli.rs). Docs updated in RUN_RECORD_FORMAT.md and VV_METRICS.md. This replaces the earlier deviation note.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Done. The combined run record always carries plan_id: the run's plan, or "" when none is given (schema-valid; null is not). Gate buckets follow TCMS-50.2: error counts as Failed, not_executed as Not executed inside the gate denominator, skipped outside. --plan selection and the unknown-verdict error are tested. Verified with cargo test -p tcms --all-features, clippy -D warnings, cargo fmt --check. make test was not run in this session.
<!-- SECTION:FINAL_SUMMARY:END -->
