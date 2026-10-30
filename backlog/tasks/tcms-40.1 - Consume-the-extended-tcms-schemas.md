---
id: TCMS-40.1
title: Consume the extended tcms-schemas
status: In Progress
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-29T20:06:00Z'
labels: []
dependencies:
  - TCMS-25
parent_task_id: TCMS-40
priority: high
type: chore
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CLI keeps its own copies of the test-case, execution-log and verification-result schemas (legacy schemas/*.schema.json plus the envelope versions schemas/tcms/test-case, test-execution and test-verification .schema.v1.json); it has no test-plan schema. These copies now drift from the extended versions in tcms-schemas (new optional fields plus new schemas test-run, trace-matrix, baseline, review, action-item, independence-assignment, analysis-report, coverage-report). Continues the schema unification of TCMS-25.

Two things must be decided here, not assumed: (1) how the CLI obtains tcms-schemas (git submodule, vendored copy at a pinned tag, or build-time download) and (2) how the envelope meta-schema (tcms-envelope.schema.json) fits: its `type` enum is closed and its `schema` URI pattern requires `tcms/<name>.schema.v<n>.json`, while tcms-schemas files are unversioned (`tcms/<name>.schema.json`) and declare no envelope fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The CLI uses the schema set that contains the new fields and new schemas
- [x] #2 No duplicate local copy remains of test-case, test-plan, execution-log or verification-result
- [ ] #3 Existing sample test case files still validate
- [ ] #4 The mechanism by which the CLI obtains tcms-schemas (and the version pinned) is documented
- [ ] #5 The envelope `type` enum and `schema` URI pattern accept the new types and the tcms-schemas file names, or the envelope is made optional for them
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Vendor the tcms-schemas schemas/tcms/*.schema.json at a pinned commit into schemas/tcms/ (unversioned names) plus VERSION file; scripts/sync-tcms-schemas.sh refreshes it
2. Widen the envelope meta-schema: type enum lists every tcms-schemas schema, schema URI pattern accepts tcms/<name>.schema.json and tcms/<name>.schema.v<n>.json
3. Document the mechanism and the pinned version
4. Tests: every vendored schema is a valid Draft-07 schema and is reachable through the envelope; sample files from tcms-schemas validate
5. Not done here (blocked): removing the legacy test-case/execution-log/verification-result copies, because the upstream test-case schema rejects the CLI's include: dependency syntax
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision D-40.1: vendor a pinned copy (offline, reproducible, works in Docker/CI). Rejected: git submodule (CI/Docker friction), build-time download (needs network at build). Upstream files go into schemas/tcms/ under their unversioned names; they do not clash with the existing *.schema.v1.json files, so the existing resolver (join of schemas root and the schema URI) finds tcms/<name>.schema.json unchanged.

Implemented: vendored tcms-schemas @ 6c29a5cd (schemas/tcms-schemas, scripts/sync-tcms-schemas.sh), envelope type enum + schema URI pattern widened, resolver fallback to the vendored dir, docs/TCMS_SCHEMAS.md, tests (crates/testcase-common/tests/tcms_schemas.rs + envelope unit tests). NOT done: criterion #2 (no duplicate local copy). The upstream test-case-main / execution-log / verification-result are less strict than the CLI copies (hooks, hydration_vars, prerequisites, include:, hook_type and the verification flags are missing upstream), so deleting the local copies would silently drop validation. Needs a tcms-schemas change first; the exact list is in docs/TCMS_SCHEMAS.md. Finding: requirement, design-element, plan-document and qa-record have a domain field 'type', so their documents can carry only 'schema:'.

Review (TCMS-40.1, reviewer, branch tcms-40-review): NOT Done. Verified: pinned vendored copy and docs (AC #4); envelope type enum and schema URI resolution for tcms-schemas types (AC #5); the 11 repo test case samples in testcases/ validate (AC #3). Open: AC #1 is only partly true (new record types come from tcms-schemas, but test-case, execution-log and verification-result still use the CLI's own schemas); AC #2 open (duplicate local copies of test-case, execution-log, verification-result remain, documented in docs/TCMS_SCHEMAS.md as blocked on upstream strictness gaps - needs an upstream change or a product decision).

Blocked on the upstream epic SCH-006 'Absorb the tcms-cli schema extensions so the CLI can drop its local schema copies' (tcms-schemas, branch sch-006-tcms-cli-parity, 8 subtasks: hooks, hydration_vars/prerequisites, include dependencies, boolean verification fields, execution-log hook/verification fields, verification-result extras, the envelope decision, parity samples + fixture check). When it is merged: bump the pin with scripts/sync-tcms-schemas.sh, delete the local test-case/execution-log/verification-result schemas, point the tcms/*.schema.v1.json URIs at the vendored ones, run the whole suite; that closes AC #2. Owner decision: leave this task open until then.

2026-10-29: criteria #1 and #2 met. tcms-schemas MR !11 (SCH-006, branch sch-006-tcms-cli-parity, pipeline green) added hooks, hydration_vars, prerequisites, include dependencies and reference objects, sequence/step extras, boolean verification and the execution-log/verification-result fields, by copying the sub-schemas of the CLI copies. Verified: the CLI's 198 test case files get the same verdict from the CLI copy and the updated upstream schema; 19 invalid variants are rejected by both; tcms-schemas now also runs 9 invalid samples that must be rejected. The pin (schemas/tcms-schemas/TCMS_SCHEMAS_VERSION) is the head of the MR branch (7991383); re-pin to the merge commit on main after the MR is merged (TCMS-40.12). Deleted: schemas/test-case.schema.json, execution-log.schema.json, verification-result.schema.json and schemas/tcms/test-case|test-execution|test-verification.schema.v1.json. The versioned URIs in documents stay; resolve_schema_uri maps them to the vendored files (SCHEMA_URI_ALIASES), a file at the URI under a given --schemas-root still wins. The CLI now checks the envelope type of these three itself (envelope_violations), because the upstream schemas are envelope-free. Remaining schemas owned by the CLI: TCMS-40.12.
<!-- SECTION:NOTES:END -->
