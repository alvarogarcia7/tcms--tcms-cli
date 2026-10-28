---
id: TCMS-42.7
title: 'Milestones: test plan as Jira Fix Version'
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A test plan is the milestone. The sync creates or updates a Fix Version named after the plan, writes plan state, executed/total cases, pass rate, baseline and TCMS link into its description, assigns it to the features in scope when feature_fields is allowed, and marks it released when the plan completes and release_on_plan_complete is set.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A plan without external_refs gets a Fix Version created and its id stored
- [ ] #2 Progress in the description matches the plan's runs after every sync
- [ ] #3 Completing the plan releases the Fix Version with the release date; reopening never un-releases automatically but reports it
- [ ] #4 milestone.target epic and issue are accepted by the config and documented as not yet implemented, failing clearly
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
