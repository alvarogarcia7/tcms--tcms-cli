---
id: TCMS-50
title: Verdict integration - replace the temporary Python shims with tcms vv outputs
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:54:00Z'
labels:
  - vv-metrics
  - tcms-web-migration
dependencies:
  - TCMS-47.2
  - TCMS-47.3
  - TCMS-47.4
references:
  - >-
    Verdict (tcms-web) decisions D-M8-25 (V&V business logic belongs in Rust or
    Python) and D-M8-26 (Parallel Change migration), DECISIONS.md in tcms-web
  - >-
    Verdict shims that stand in for this work until it lands:
    scripts/vv/shim/vv_config.py, vv_dedupe.py, vv_metrics_file.py,
    vv_history.py, vv_views_file.py, vv_report.py, jira_figures.py
  - docs/VV_CASE_HISTORY_GAP.md and VV_BUSINESS_LOGIC_REPLACEMENT.md in tcms-web
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict (tcms-web) moves all V&V business logic out of Java into tcms-cli (or Python where the CLI cannot yet do it). Until the CLI provides the outputs below, Verdict runs small Python shims under scripts/vv/shim. Each child task here removes one shim by giving the CLI the missing output. Once a child is Done, the matching shim is deleted in tcms-web and Verdict calls the binary directly. Java only maps and presents the output.

Why: two implementations of one rule (Java and Python) drifted apart in the audit, for example the rounding rule and the dedupe rule. One implementation in the CLI removes the drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each child task (TCMS-50.1 to TCMS-50.6) is Done
- [ ] #2 For each child, the matching tcms-web shim is removed in the same change set as the Verdict switch, and tcms-web's parity tests pass against the CLI binary
- [ ] #3 The tcms-web parity tests (TcmsCliLegacyParityTest, TcmsCliContainersParityTest, VvCliViewsParityTest) run against the released binary without skips
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Constraint: keep the tcms-web change set as small as possible. Per child, tcms-web changes are limited to deleting the matching shim, switching Verdict to the CLI output, and updating expected values in parity fixtures. No rewrite of Java logic that the CLI replaces unless the switch cannot work without it.
<!-- SECTION:NOTES:END -->
