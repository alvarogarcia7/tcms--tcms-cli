---
id: TCMS-47.4
title: >-
  Carry plan_id in the combined run record and handle not_executed and error
  verdicts in tcms vv metrics
status: To Do
assignee: []
created_date: '2026-11-03T06:30:00Z'
updated_date: '2026-11-04T09:57:00Z'
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

- The combined run record written by TCMS-47.3 carries `plan_id` in `test-run.yaml` (from the shared run metadata). When the run has no plan, the key is written with an explicit null, so a reader can tell a missing plan from a missing key.
- `tcms vv metrics` accepts all four verdicts and selects runs by `plan_id` with `--plan`. Gate buckets follow TCMS-50.2: `error` counts as Failed and `not_executed` counts as Not executed, both inside the gate denominator; `not_executed` is never counted as Failed. Skipped stays outside the denominator as today. Both new verdicts appear in the counts.
- A verdict outside the four is an error that names the file, the test case and the value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The combined run record carries plan_id in test-run.yaml, and an explicit null when the run has no plan
- [ ] #2 tcms vv metrics --plan <ID> selects only the runs whose plan_id is that plan
- [ ] #3 error counts as Failed and not_executed counts as Not executed, both inside the gate denominator (per TCMS-50.2); both appear in the counts, and not_executed is never counted as Failed
- [ ] #4 A verdict outside pass, fail, not_executed and error is an error that names the file, the test case and the value
- [ ] #5 Tests cover a run with plan_id, a run without plan (null), each new verdict, and the unknown-verdict error
- [ ] #6 docs/RUN_RECORD_FORMAT.md and docs/VV_METRICS.md describe plan_id and the four verdicts
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo test passes for the tcms crate
- [ ] #2 cargo fmt and clippy are clean
<!-- DOD:END -->
