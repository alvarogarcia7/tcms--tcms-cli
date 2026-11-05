---
id: TCMS-50.6
title: Address a run by its run folder name in every tcms vv command
status: In Review
assignee:
  - Claude
created_date: '2026-11-04T00:00:00Z'
updated_date: '2026-11-04T10:15:00Z'
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
- [x] #1 docs/VV_METRICS.md names the run folder name as the only --run contract, with the reason
- [x] #2 tcms vv metrics --run <folder name> returns the metrics of that run; a test uses the derived fixture
- [x] #3 Every command that takes --run uses the same resolver; a test covers each command that takes --run as it exists when this task is done
- [x] #4 An unknown --run value fails and the message lists the valid run ids (folder names)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. vvcmd.rs load_runs: the run id used for --run and in output is the run folder name (confirmed contract).
2. build_report: unknown --run lists the valid folder names in the error.
3. Tests with a derived fixture: --run with a folder name differing from the run_id inside test-run.yaml; unknown id lists the valid ids.
4. docs/VV_METRICS.md states the contract.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
load_runs uses the run folder name as the run id (the run_id inside test-run.yaml is ignored for lookup). Unknown --run errors list the valid folder names. Test: run_is_addressed_by_its_folder_name_not_by_the_run_id_inside (vv_trends_history_cli.rs).

AC 3: the commands that take --run are metrics only (history takes --case, trends takes --runs N, combine writes a folder), so one resolver covers all of them.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
--run now takes the run folder name only, and the output reports that folder name as runId. An unknown --run fails and lists the valid folder names. Documented in docs/VV_METRICS.md. Tested with a fixture whose run_id inside the record differs from its folder.
<!-- SECTION:FINAL_SUMMARY:END -->
