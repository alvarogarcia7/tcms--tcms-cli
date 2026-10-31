---
id: TCMS-44
title: Fix test-executor --config flag for json-escape E2E
status: Done
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

## Implementation Notes

- Fixed in fix/tcms-44 (commits 9f9f36e, 824edf8) and merged into traceability-matrix-report (bc4799f).
- Decision D-44-1: global --config on test-executor, loaded only in generate and execute; json-escape E2E script uses -o instead of --output-dir.
- Also fixed: testcase-execution replace("\\r") deleted literal backslash-r from user commands; json-escape put on PATH for rust_binary runs (subshell scope).
- Result: test_json_escape_e2e.sh 37/37 passed, exit 0. cargo test -p test-executor passes; clippy -D warnings passes.
- Remaining CI-only issue, separate: acceptance job cannot download TPDG (needs TPDG_TOKEN or job-token allowlist; GitLab setting, outside the repo).
