---
id: TCMS-40.6
title: Fail when a requirement lacks normal-range or robustness tests
status: To Do
assignee: []
created_date: '2026-10-27T19:39:00Z'
labels: []
dependencies:
  - TCMS-40.5
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Glossary rule: every high-level and low-level requirement needs both normal-range and robustness tests; today nothing enforces it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The check exits non-zero and names each requirement and which kind of test is missing
- [ ] #2 The check can be disabled with a flag
- [ ] #3 The requirement set is read from a StrictDoc export or an ID list
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
