---
id: TCMS-50
title: Verdict integration - replace the temporary Python shims with tcms vv outputs
status: In Progress
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:21:00Z'
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Order: 47.3 (winner rule, TCMS-50.1), 47.4 (buckets, TCMS-50.2, plan_id null), 50.3 (delta on floored rates), 50.6 (folder-name --run contract), 50.4 (history and failingCases), 50.5 (trends and heatmap). Each child is implemented in the CLI with tests and docs in docs/VV_METRICS.md; tcms-web changes are out of this repository and are listed as follow-ups, not made here. Known gaps: TCMS-47.2 AC#7 stays blocked (no vv-config schema in the repo); the Markdown report of TCMS-47.1 is not implemented, so 50.3 AC#3 is verified for the JSON output only; the tcms-web legacy duplicates fixture is not in this repository, so 50.1 AC#2 is verified against a fixture added here.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Constraint: keep the tcms-web change set as small as possible. Per child, tcms-web changes are limited to deleting the matching shim, switching Verdict to the CLI output, and updating expected values in parity fixtures. No rewrite of Java logic that the CLI replaces unless the switch cannot work without it.

Status of the children in this repository: 47.3, 47.4, 50.1 to 50.6 implemented and tested (In Review). Open: 47.4 AC 1 (plan_id null vs schema), 50.1 AC 2 and 50.2 AC 5 (tcms-web parity and recorded-run/Verdict follow-up), 50.3 AC 3 Markdown part is met through tcms-vv-report, 47.2 AC 7 (no vv-config schema).

tcms-web changes (shim removal and switch to the CLI output) are out of this repository and are not made here; AC 1 to 3 of this task depend on them.

Update: 47.4 is Done. plan_id is "" when no plan is given (user decision), so the schema conflict is resolved in this repo. Still open: 50.1 AC 2 and 50.2 AC 5 (tcms-web work), 47.2 AC 7 (no vv-config schema), and the tcms-web follow-up issue, which could not be created from this environment (see the session report).
<!-- SECTION:NOTES:END -->
