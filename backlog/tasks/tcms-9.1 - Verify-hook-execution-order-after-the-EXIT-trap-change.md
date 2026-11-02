---
id: TCMS-9.1
title: 'Verify hook execution order after the EXIT-trap change (re-enable test_hook_execution_order_in_script)'
status: To Do
assignee: []
created_date: '2026-11-03T06:00:00Z'
labels:
  - hooks
dependencies: []
parent_task_id: TCMS-9
ordinal: 90100
---

## Description

Commit f666f6d (TCMS-9) runs teardown_test and script_end from the EXIT trap. The test
`test_hook_execution_order_in_script` in crates/testcase-manager/tests/hooks_test.rs asserted the
old order (after_sequence before teardown_test) and failed. It is commented out until the new order
is verified.

## Acceptance Criteria

- [ ] #1 The generated script order is written down: script_start, setup_test, before_sequence, before_step, after_step, after_sequence, then teardown_test and script_end from the EXIT trap, including on failure.
- [ ] #2 test_hook_execution_order_in_script is rewritten for that order (or split into a normal-path test and an EXIT-trap test) and re-enabled.
- [ ] #3 make test passes with the test re-enabled.
