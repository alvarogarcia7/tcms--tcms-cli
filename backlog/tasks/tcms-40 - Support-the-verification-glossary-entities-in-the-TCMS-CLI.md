---
id: TCMS-40
title: Support the verification glossary entities in the TCMS CLI
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-27T19:38:00Z'
updated_date: '2026-10-28T14:07:00Z'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Environment notes for running the whole workspace test suite: (1) crates/testcase-manager/tests/dependencies_test.rs hardcodes <repo>/target/debug/validate-yaml, so it fails when CARGO_TARGET_DIR is elsewhere; (2) validate-yaml, testcase-git and anything using git2 need libssl-dev and pkg-config installed; (3) cargo clippy -D warnings currently fails on crates/testcase-verification/src/log_cleaner.rs:64 (useless borrow in format!), not caused by this work.

Status after this work (branch tcms-40-glossary-impl, 8 commits, not pushed). Done and in review: 40.2, 40.3, 40.5, 40.6 (AC 1-2), 40.8, 40.4 (AC 2-4). Partly done: 40.1 (vendored pinned tcms-schemas, envelope, resolver, docs; the local copies stay, see below), 40.9 (envelope/types/samples; no folder fallback). Not started: 40.7 regression selection, 40.10 coverage import, 40.11 docs and acceptance suite. Verification of the whole workspace at the last commit: cargo fmt --check clean, cargo build --workspace --all-features, cargo test --workspace --all-features: 1931 passed, 0 failed, 19 ignored; doc tests 10 passed; cargo clippy --workspace --all-targets --all-features -D warnings clean apart from the pre-existing log_cleaner.rs lint. Not run, because the tools are not installed here (uv, shellcheck, StrictDoc, the documentation generator): make test as a whole, make acceptance-test, scripts/validate-output-schemas.sh. So no Definition of Done item is checked and no task is Done. Blockers needing a decision outside this repo: tcms-schemas test-case-main/execution-log/verification-result must absorb hooks, hydration_vars, prerequisites, the include: dependency syntax, boolean verification.output and the hook_type/verification flags before the local copies can be removed (40.1 AC 2); exact list in docs/TCMS_SCHEMAS.md.
<!-- SECTION:NOTES:END -->
