---
id: TCMS-42.3
title: 'Requirement source adapters: Jira features and StrictDoc'
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A product reads requirements from exactly one source. Jira adapter: issues of feature_issue_types in project_keys become requirement records (Epic = system, Story/Feature = HLR; version = updated timestamp; source = issue key). StrictDoc adapter: items from the StrictDoc export at the baseline's commit become requirement records (version = commit). Both produce the same requirement YAML so coverage, trace matrix and reviews work unchanged (TCMS-40.5, TCMS-40.6 read from this).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms requirements import writes requirement YAML that validates, from either source, chosen by integration-config.requirement_source
- [ ] #2 A product configured with both sources is rejected with a clear error
- [ ] #3 Jira import keeps issue key, type, status, summary as title, description as text, labels for safety/security flags when configured
- [ ] #4 StrictDoc import records the document path, UID and commit
- [ ] #5 Re-import is idempotent: unchanged requirements produce no diff
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
