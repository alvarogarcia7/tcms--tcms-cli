---
id: TCMS-42.1
title: ID resolver: translate Jira and StrictDoc ids to URLs and back
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users must not type or paste Jira or StrictDoc URLs. A library crate (used by the CLI and reusable by tcms-web) resolves an id to a system and URL, and a URL to a system and id, driven by integration-config systems[] (id_pattern, url_template with {base_url} {id} {doc}, url_pattern, document_map). Deployment is undecided (Jira Cloud or Data Center, StrictDoc static HTML or server), so nothing is hard-coded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms ref resolve <id|url> prints system, id and URL; tcms ref open <id> opens the browser
- [ ] #2 An id matching no system, or more than one, gives a clear error; overlapping id_patterns are rejected when the config is loaded
- [ ] #3 external_refs[].url left empty in any record is filled by the resolver on read; records never store site URLs by hand
- [ ] #4 Example configs for Jira Cloud, Jira Data Center, StrictDoc HTML export and StrictDoc server ship with the CLI
- [ ] #5 Unit tests cover every example config in both directions
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
