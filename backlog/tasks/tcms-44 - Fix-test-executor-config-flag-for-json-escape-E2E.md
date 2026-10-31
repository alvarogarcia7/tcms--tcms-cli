---
id: TCMS-44
title: Fix test-executor --config flag for json-escape E2E
status: To Do
assignee: []
created_date: '2026-10-31T00:00:00Z'
labels: []
milestone: m-0
dependencies: []
priority: high
type: bug
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/testcase-manager/tests/integration/test_json_escape_e2e.sh fails 7 of 19 cases with `error: unexpected argument '--config' found`. The script runs `test-executor --config ... generate ...` (lines ~243, 343, 414, 568), but the `--config` flag does not exist in crates/test-executor/src/main.rs on branch traceability-matrix-report or on `main`. `git log -S config` on that file finds no commit that adds it, so the failure predates Phase 3 (not from commits 14a68f4..HEAD).

Additionally, `make test` exits 0 even though this script reports failures, so the Makefile target hides the failure. Fix both: add `--config` to test-executor (CLAUDE.md documents `test-executor generate ... --config config.toml` for json_escaping), or correct the script; and make the Makefile propagate the script's exit status. See D-03-G2 in .planning/DECISIONS.md.
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run make test
<!-- DOD:END -->
