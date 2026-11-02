---
id: TCMS-48.4
title: 'Phase 2: separate Jira integration from core traceability'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Jira (jira-client, requirement-import, test-backends Jira paths) is an integration and should not be a dependency of core. Traceability is part of the core and stays in core crates. Decide the crate boundary, then move code so that core crates do not depend on jira-client.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
