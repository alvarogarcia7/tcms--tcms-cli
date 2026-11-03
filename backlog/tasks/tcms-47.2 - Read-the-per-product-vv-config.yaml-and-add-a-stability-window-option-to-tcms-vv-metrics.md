---
id: TCMS-47.2
title: >-
  Read the per-product vv-config.yaml and add a stability-window option to tcms vv metrics
status: To Do
assignee: []
created_date: '2026-11-03T00:00:00Z'
updated_date: '2026-11-03T00:00:00Z'
labels:
  - vv-metrics
  - tcms-web-migration
dependencies:
  - TCMS-47.3
references:
  - TCMS-47 - Compute-VV-metrics-and-quality-gate-verdict-in-tcms-cli.md
  - docs/VV_METRICS.md
  - crates/tcms/src/vvcmd.rs
  - tcms-schemas schemas/tcms/vv-run-metrics.schema.json (parameters block)
  - tcms-schemas schemas/tcms/vv-config.schema.json (merge request 15 in tcms-schemas, branch feat/vv-config-schema)
  - tcms-web backlog ORCH-087.09 (Verdict side, depends on this task)
parent_task_id: TCMS-47
priority: high
type: feature
ordinal: 91200
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
## Why

Verdict (tcms-web) keeps the per-product V&V settings in `<DATA_ROOT>/vv-config.yaml`: the gate threshold, the flaky threshold and the stability window. Today `tcms vv metrics` reads only `runs/`, the test-case files and `requirements.json`. It cannot see those settings, so a CI job or a document pipeline that runs the CLI directly uses the defaults (gate 90, flaky 4) and can report a different verdict from Verdict. The CLI also has no flag for `stability_window_runs` (decision D-M8-06 in Verdict), so the window Verdict configures never reaches the CLI output.

## What

- Read `<DATA_ROOT>/vv-config.yaml` when it exists, and use its `gate_threshold_percent` and `flaky_flip_threshold` unless a command-line flag overrides them.
- Add `--stability-window <N>` (default 12, at least 1), used for the stability rule.
- Report where each parameter came from in the output, so a reader can see whether a value came from the file, a flag or the default.
- Validate the file. Its shape is the one in tcms-schemas `schemas/tcms/vv-config.schema.json`; until that merge request lands, use the copy in the CLI's vendored schemas.

Out of scope: the Verdict side (ORCH-087.09 in tcms-web) and the report rendering (TCMS-47.1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `tcms vv metrics` reads `<DATA_ROOT>/vv-config.yaml` when the file exists and uses its gate_threshold_percent and flaky_flip_threshold, unless a command-line flag overrides them
- [ ] #2 A new `--stability-window <N>` flag sets the stability window (default 12, at least 1); the value appears in `parameters.stability_window_runs` of the output
- [ ] #3 The output reports the source of each parameter (file, flag or default)
- [ ] #4 A missing file keeps today's behaviour. A file with a wrong type, an unknown key or an out-of-range value is an error that names the file and the key
- [ ] #5 Tests cover precedence (flag over file over default), the stability window in the output, and each error case
- [ ] #6 docs/VV_METRICS.md documents the file, the flag and the precedence
- [ ] #7 Validation against tcms/vv-config.schema.json passes for a valid file and fails for an invalid one
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 cargo test passes for the tcms crate
- [ ] #2 cargo fmt and clippy are clean
<!-- DOD:END -->
