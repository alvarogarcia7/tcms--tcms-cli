---
id: TCMS-48.2
title: 'Phase 1: path-based test tiers in make test-from'
status: To Do
assignee: []
created_date: '2026-11-02T09:11:00Z'
labels:
  - dx
dependencies: []
parent_task_id: TCMS-48
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Four tiers: T0 unit (cargo test for changed crates and reverse deps), T1 contract (schema validation of fixtures and writer outputs, triggered by schemas/ or fixture changes), T2 e2e (mapped per binary), T3 full (test-all, coverage-e2e, acceptance-test; before merge and nightly). Extend the existing mk/incremental.mk mapping rather than adding a new mechanism.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
