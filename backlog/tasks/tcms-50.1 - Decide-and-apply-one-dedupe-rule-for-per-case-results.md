---
id: TCMS-50.1
title: Decide and apply one dedupe rule for per-case results
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
labels:
  - vv-metrics
  - decision
dependencies:
  - TCMS-47.3
parent_task_id: TCMS-50
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two rules choose the one result per test case when a run has duplicates:
- TCMS-47.3 AC #3: the latest started_at wins; not_executed never overrides an executed result.
- Verdict (tcms-web, D-M8-25 shim scripts/vv/shim/vv_dedupe.py): the result with the detail flag wins, then the newest date, then the path.

They disagree on which result wins when the detail flags differ. Pick one, record it as a decision, and apply it in the CLI. Verdict then drops its Python dedupe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision entry states the one rule, with the reason, in docs/VV_METRICS.md
- [ ] #2 tcms vv metrics applies that rule to the legacy duplicates fixture and returns the same result per case that the tcms-web Java parser returned on the same fixture
- [ ] #3 not_executed never overrides an executed result (a test covers it)
- [ ] #4 A test covers the tie cases: same date, and same date with a different path
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
