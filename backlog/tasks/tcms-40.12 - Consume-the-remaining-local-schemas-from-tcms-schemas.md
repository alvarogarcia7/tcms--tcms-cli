---
id: TCMS-40.12
title: Consume the remaining local schemas from tcms-schemas
status: To Do
assignee: []
created_date: '2026-10-29T19:52:00Z'
labels:
  - schemas
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: medium
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TCMS-40.1 moved test-case, execution-log and verification-result to the vendored tcms-schemas (the CLI keeps no copy of them). The CLI still owns schemas that tcms-schemas does not have: test-result, test-results-container and container-config (schemas/tcms/*.schema.v1.json), verification-output, container_config and the legacy schemas/*.schema.json that remain, the verification_methods, and the envelope meta-schema tcms-envelope.schema.json. Decide for each: move it upstream (MR to tcms-schemas, with samples and invalid samples, as SCH-006 did), or keep it as a CLI-only schema and say so in docs/TCMS_SCHEMAS.md. Also: tcms-schemas has a test-plan schema the CLI does not read yet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every schema file under schemas/ is either vendored from tcms-schemas or listed in docs/TCMS_SCHEMAS.md as CLI-only, with the reason
- [ ] #2 Schemas moved upstream have samples and invalid samples in tcms-schemas, and the same verdicts as before on the CLI's fixtures
- [ ] #3 The pin points at a commit on the main branch of tcms-schemas that contains SCH-006 (merge request !11 merged)
- [ ] #4 make test passes
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
