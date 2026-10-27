---
id: TCMS-42.6
title: Write per-feature verification and validation status and coverage
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.3
  - TCMS-42.5
  - TCMS-40.5
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Each feature issue gets verification_status, validation_status and coverage, rolled up from the requirements that trace to it (the feature itself when Jira is the source, StrictDoc requirements whose external_refs name it otherwise), the trace matrix, the coverage report, latest verdicts and open problem reports, following the roll-up rules in the mapping doc. Optional workflow transitions per verification status.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Roll-up produces not_started, in_progress, passed, failed, blocked exactly as the mapping doc defines; unit tests cover every rule
- [ ] #2 Validation status follows requirement.validation_status; a validation review changes it
- [ ] #3 Transitions configured in transitions are applied only when feature_transitions is allowed; a missing workflow transition is logged and skipped
- [ ] #4 Features with no traced requirement are reported, not written
- [ ] #5 A dry-run flag prints every intended write without sending it
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
