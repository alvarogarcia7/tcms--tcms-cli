---
id: TCMS-50.6
title: Address a run by its run folder name in every tcms vv command
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
labels:
  - vv-metrics
  - decision
dependencies:
  - TCMS-47.3
parent_task_id: TCMS-50
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Verdict found that `tcms vv metrics --run <ID>` fails when the ID is the one Verdict stores for the load (for example run-2) rather than the run folder name the CLI knows (for example RUN-E2E): "run run-2 is not among the recorded runs". Verdict works around it by passing no run id. The CLI should accept the run folder name in every command that takes --run, and say clearly which ids it accepts. Decide whether Verdict's load id or the folder name is the contract, record it, and make the CLI error message list the valid ids.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision entry names the id contract for --run, in docs/VV_METRICS.md
- [ ] #2 tcms vv metrics --run <folder name> returns the metrics of that run (a test with the derived fixture)
- [ ] #3 An unknown --run value fails and lists the valid run ids in the message
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
