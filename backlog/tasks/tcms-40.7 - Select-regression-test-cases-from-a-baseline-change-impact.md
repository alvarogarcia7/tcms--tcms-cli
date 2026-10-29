---
id: TCMS-40.7
title: Select regression test cases from a baseline change impact
status: Done
assignee:
  - '@worker-a'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:31:00Z'
labels: []
dependencies:
  - TCMS-40.2
  - TCMS-40.5
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Regression scope must follow change impact analysis rather than re-running everything.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Given a baseline YAML with change_impact, only the listed test cases and those tracing to impacted requirements run
- [x] #2 A plan with a regression block is honoured: the scope is the change_impact of the baseline named by the plan's baseline_id, measured from regression.previous_baseline_id
- [x] #3 A missing or invalid baseline gives a clear error
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. regression.rs in testcase-verification: read/validate baseline and plan, select = baseline.change_impact.regression_test_cases + cases tracing to impacted_requirements. 2. test-executor regression-select subcommand (ID list / JSON output). 3. unit + CLI tests. 4. docs/REGRESSION.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented: crates/testcase-verification/src/regression.rs, test-executor 'regression-select' subcommand, docs/REGRESSION.md. Evidence: cargo test -p testcase-verification regression (8 passed); cargo test -p test-executor --test regression_select_cli (4 passed, runs the real binary); clippy clean.
(D-40.7a) Scope = change_impact.regression_test_cases (match on test case id) union cases whose requirements list names an impacted requirement; obsolete cases never selected.
(D-40.7b) Baseline validated by hand in Rust (required id/build_id/created/items, change_impact needs previous_baseline_id, string lists) because testcase-verification has no jsonschema dependency; vendored sample bl_001.yml is a unit test input.
(D-40.7c) Listed test case IDs that do not exist are an error (exit 1, selection still written) unless --allow-unknown-test-cases: silently dropping a regression test is worse than failing. Impacted requirements without any test only warn.
(D-40.7d) --plan picks the baseline by id among several --baseline files/dirs and requires baseline.previous_baseline_id == regression.previous_baseline_id; without --plan exactly one baseline is required.
(D-40.7e) Output formats ids (default, one per line, to feed scripts), json, yaml; added serde_json to crates/test-executor/Cargo.toml (Cargo.lock changed, left for coordinator).

Review (round 2, branch tcms-review-r2 at eff5436): cargo test -p testcase-verification -p test-executor passed (regression unit tests 8, regression_select_cli 4). Real test-executor binary: BL-2 with TC_REG_001..003 selects TC_REG_001 (reason HLR-1) and TC_REG_003 (reason listed), json and ids output agree; unknown listed id exits 1 and still writes the selection; missing baseline exits 1 with 'Baseline not found'; baseline without change_impact exits 1; plan whose regression.previous_baseline_id is BL-0 against BL-1 is refused with exit 1. AC1-3 verified by these runs. Note: serde_json added to test-executor (Cargo.lock change is in the coordinator's hands, not committed by review).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Regression scope from baseline change_impact (testcase-verification regression.rs, test-executor regression-select, docs/REGRESSION.md). Reviewed in round 2: all three ACs verified by the real binary and by the crate tests. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test testcase-verification and test-executor and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
