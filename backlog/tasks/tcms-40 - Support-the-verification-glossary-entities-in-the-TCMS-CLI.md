---
id: TCMS-40
title: Support the verification glossary entities in the TCMS CLI
status: To Do
assignee: []
created_date: '2026-10-27T19:38:00Z'
labels: []
dependencies: []
references:
  - >-
    https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas/-/merge_requests/8
priority: high
type: feature
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tcms-schemas project is adding test classification, requirement traceability, runs, baselines, reviews, coverage and analysis records (verification glossary: verification, validation, review, analysis, test case/procedure/result, requirements-based and structural coverage, independence, regression). The CLI must read, produce and validate them or the new data is unused. The schema changes are additive, so existing test case files and CLI behaviour must not regress. Requirements (HLR/LLR) are owned by StrictDoc; the CLI only handles requirement IDs. The glossary-to-schema mapping is in tcms-schemas docs/entity-mapping.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All subtasks are Done
- [ ] #2 Existing test case files run unchanged
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
