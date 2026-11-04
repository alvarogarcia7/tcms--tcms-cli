---
id: TCMS-50.6
title: Address a run by its run folder name in every tcms vv command
status: To Do
assignee: []
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T09:51:00Z'
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
Verdict found that `tcms vv metrics --run <ID>` fails when the ID is the one Verdict stores for the load (for example run-2) rather than the run folder name the CLI knows (for example RUN-E2E): "run run-2 is not among the recorded runs". Verdict works around it by passing no run id.

Decision (recommended; confirm before implementing): the run folder name is the only contract for --run. Reason: the CLI writes and reads run folders, so only it knows their names. Verdict's load id is a name only Verdict knows, so the CLI cannot resolve it. Verdict passes the folder name it stored at load time.

Scope: every command that takes --run accepts the folder name. Today that is `tcms vv metrics`. TCMS-50.4 adds `tcms vv history`; it uses the same resolver if it takes a run id. `tcms vv combine` writes a folder and `tcms vv trends` takes --runs N, so neither is affected.

Error behaviour: an unknown --run fails and the message lists the valid run ids (folder names) found under the root.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/VV_METRICS.md names the run folder name as the only --run contract, with the reason
- [ ] #2 tcms vv metrics --run <folder name> returns the metrics of that run; a test uses the derived fixture
- [ ] #3 Every command that takes --run uses the same resolver; a test covers each command that takes --run as it exists when this task is done
- [ ] #4 An unknown --run value fails and the message lists the valid run ids (folder names)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
