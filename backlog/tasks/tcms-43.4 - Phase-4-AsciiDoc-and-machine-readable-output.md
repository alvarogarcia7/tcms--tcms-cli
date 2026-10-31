---
id: TCMS-43.4
title: 'Phase 4: AsciiDoc and machine-readable output'
status: To Do
assignee: []
created_date: '2026-10-30T16:20:00Z'
labels: []
milestone: m-0
dependencies:
  - TCMS-43.3
parent_task_id: TCMS-43
priority: medium
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requirements: OUT-01, OUT-02. Goal: every report available as AsciiDoc and as JSON or YAML validated against a local report schema. Native in-process renderers; HTML/TPDG is v2 (FOUT-01). Do not modify shared schemas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All report kinds render as Markdown and AsciiDoc, with hostile strings escaped in cells
- [ ] #2 JSON and YAML output validated against a local schema before writing; a failure writes no file
- [ ] #3 Determinism holds for every format
- [ ] #5 The traceability report (both shapes) is validated against the local report schema before it is written; a schema failure aborts without writing a file.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
