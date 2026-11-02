---
id: TCMS-48.9
title: 'Phase 4: dependency debug settings and profiles'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure the effect of debug = false for dependencies ([profile.dev.package."*"]) and of the line-tables-only setting already in Cargo.toml. Keep CARGO_BUILD_JOBS in the Makefile with ?=. No sccache (see CI history).
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
