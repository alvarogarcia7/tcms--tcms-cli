---
id: TCMS-48
title: 'DX: Optimizing compilation'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Epic. Split the build and test graph so that a change only rebuilds and retests what it touches. Source: plan agreed in the session of 2026-11-02 (phases 0-4). Phase 0 measurement script: scripts/dx/measure-baseline.sh.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
