---
id: TCMS-42.8
title: Defects: create or link Jira issues from problem reports
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: medium
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A failed result raises a problem-report; the sync creates a Jira issue of defect_issue_type from it or links an existing issue, stores the key in external_refs, and keeps the problem report status and the Jira status visible to each other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tcms pr push creates the Jira issue with summary, description, severity to priority mapping, and links to the feature and test entity
- [ ] #2 tcms pr link <PR> <KEY> links an existing issue
- [ ] #3 The feature's verification roll-up treats an open linked issue as blocking passed
- [ ] #4 Closing in Jira is reflected in the problem report at the next sync as a reported difference, not an automatic status change
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
