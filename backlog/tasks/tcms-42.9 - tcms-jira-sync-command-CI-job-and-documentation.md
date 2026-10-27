---
id: TCMS-42.9
title: tcms jira sync command, CI job and documentation
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.3
  - TCMS-42.5
  - TCMS-42.6
  - TCMS-42.7
  - TCMS-42.8
parent_task_id: TCMS-42
priority: medium
type: docs
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One command runs import of requirements and all write-backs for a product, usable by people and by the CI job that follows a committed run. Documentation explains the mapping, the configuration, the roll-up rules and the resolver.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms jira sync --product <id> runs requirement import, test status, feature status, milestone and defect sync in that order and summarises writes
- [ ] #2 A --dry-run prints the plan of writes; --only <step> limits the scope
- [ ] #3 A GitLab CI job template runs the sync after a results merge
- [ ] #4 Docs cover integration-config with the four example configs, the roll-up rules and tcms ref
- [ ] #5 Acceptance suite covers a full cycle: import features, run, sync, verify Jira mock state for plain Jira and Xray
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
