---
id: TCMS-42
title: 'Integrate with Jira: read features, write statuses, resolve IDs'
status: To Do
assignee: []
created_date: '2026-10-28T11:00:00Z'
labels: []
dependencies: []
references:
  - https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas/-/merge_requests/8
priority: high
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Products are managed in Jira. TCMS must (a) read features from Jira as requirements, or from StrictDoc when the product uses StrictDoc (one source per product, never both), and (b) write to Jira the test case statuses (lifecycle and latest verdict), milestones (test plan as Fix Version) and per-feature verification and validation statuses, through any test backend: plain Jira with TCMS-owned Test issues, Xray, Zephyr Scale or Zephyr Squad. Writes allowed: custom fields and transitions on feature issues, TCMS-owned issues, comments and defects; never a feature's summary or description. The sync is a CLI command run by users or CI under a service account, recording the acting user. A local resolver turns a Jira or StrictDoc id into a URL and back so users never handle URLs. Mapping, roll-up rules and configuration schema: tcms-schemas docs/jira-integration.md and the integration-config schema (SCH-005.12). Note: tcms-web ORCH-023 states integrations are read-only except defects and must be updated to this write policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All subtasks are Done
- [ ] #2 A product configured with Jira as requirement source and a product configured with StrictDoc both pass the acceptance suite
- [ ] #3 No secret is ever written to a YAML record or log
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->
