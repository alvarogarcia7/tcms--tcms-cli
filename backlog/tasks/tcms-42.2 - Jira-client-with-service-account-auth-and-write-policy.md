---
id: TCMS-42.2
title: Jira client with service-account auth and write policy
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.1
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One Jira client for Cloud (REST v3) and Data Center (REST v2) with token auth from auth.secret_ref (environment variable or secret store), rate-limit handling, and a write guard that refuses any write not allowed by write_policy and any field not listed in fields. Every write produces an audit-trail-entry with action sync, the acting TCMS user and the external_refs written.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Connection test command reports site, user and the projects in project_keys
- [ ] #2 A write to a field not in fields, or disallowed by write_policy, is refused before any request is sent
- [ ] #3 Each sync run writes audit-trail entries naming the acting user; the Jira comment body names the same user
- [ ] #4 Rate limiting (429) is retried with back-off; other errors fail the sync with the issue key in the message
- [ ] #5 Integration tests run against a mocked Jira for both Cloud and Data Center URL layouts
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
