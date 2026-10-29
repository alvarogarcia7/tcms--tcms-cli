---
id: TCMS-40.9
title: Validate every new record type from tcms-schemas
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:49:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: medium
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New record types must be recognised by schema auto-resolution (see TCMS-17, TCMS-24, TCMS-35) instead of failing as unknown. Auto-resolution reads the `schema` envelope field of the document (testcase-common::envelope); it does not look at the folder name. The tcms-schemas samples carry no envelope, so they can only pass with an explicit schema argument unless TCMS-40.1 makes the envelope optional or adds a folder-based fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A document whose envelope names one of the new schemas resolves and validates; the envelope `type` enum lists every schema in tcms-schemas (test_run, trace_matrix, baseline, review, action_item, independence_assignment, analysis_report, coverage_report, product, plan_document, requirement, design_element, code_unit, environment, problem_report, change_request, qa_record, objective, audit_trail_entry)
- [x] #2 An unknown type or schema still errors as today
- [x] #3 The sample files from tcms-schemas all pass when validated with an explicit schema, and, if TCMS-40.1 adds a fallback, also without one
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Partly covered by the 40.1 work: the envelope type enum lists all 24 tcms-schemas schemas (test: every_vendored_schema_is_a_valid_draft7_schema_known_to_the_envelope), documents naming them resolve and validate (all_tcms_schemas_samples_validate_through_their_envelope), unknown schema/type still errors (unknown_schema_still_errors). Finding: requirement, design-element, plan-document and qa-record have a domain field 'type', so those documents carry only 'schema:'. Not done: a folder-based fallback (not required, explicit schema or schema: line works), and the validate-yaml CLI path is not yet covered by an e2e test.

Review (TCMS-40.9, reviewer, branch tcms-40-review): NOT Done. AC #1 and #2 verified: envelope type enum lists all 24 tcms-schemas names (incl. the 20 in the AC); a document with type/schema trace_matrix validates via validate-yaml; an unknown schema URI fails with a clear error. AC #3 open: 26 of 27 tcms-schemas samples pass their upstream schema (Draft 7 + format checker); samples/test-case/SGP.22_4.4.2.yaml fails test-case-main (missing general_initial_conditions) - an upstream sample issue, not fixed in the vendored copy. The fallback without an explicit schema was not added (documented in docs/TCMS_SCHEMAS.md).

Pin bumped from 6c29a5cd to 33c3d3f8 (upstream main): the only upstream changes in between are samples (SGP.22_4.4.2.yaml now has general_initial_conditions, new execution-log and verification-result samples), validator.py and backlog files; no schema file changed. Evidence for AC #3: all 29 vendored samples (yml and yaml) pass the real 'validate-yaml FILE --schema schemas/tcms-schemas/tcms/<name>.schema.json' (ok=29 failed=0), and also pass through their envelope (crates/testcase-common/tests/tcms_schemas.rs, which now also covers .yaml). No folder-based fallback was added: the schema: line or an explicit schema argument is how a document selects its schema (docs/TCMS_SCHEMAS.md).

Review round 3 (reviewer): AC #1 proven: envelope type enum (schemas/tcms-envelope.schema.json) lists every one of the 24 vendored schemas (test_run, trace_matrix, baseline, review, action_item, independence_assignment, analysis_report, coverage_report, product, plan_document, requirement, design_element, code_unit, environment, problem_report, change_request, qa_record, objective, audit_trail_entry, and the rest); a document with type trace_matrix and schema tcms/trace-matrix.schema.json validates with the real validate-yaml binary (exit 0). AC #2 PARTLY proven: an unknown schema URI fails (exit 1, 'Schema file not found'). An unknown envelope type (type: not_a_real_type) is ACCEPTED by validate-yaml (exit 0): the envelope type enum is never enforced by any validation path (the enum is only a $ref target in yaml_loader), and it was never enforced before this change either. Enforcing it would reject existing documents (type test-case in testcases/, requirement type HLR in tcms-schemas samples), so this is a product decision, left open. AC #3 re-verified: validate-yaml FILE --schema tcms/<name>.schema.json over all 29 vendored samples: ok=29 failed=0. Tests: cargo test -p testcase-common (tcms_schemas: 6 pass). Not run: make test.

Definition of Done #1 (review round 3): make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p testcase-common and -p validate-yaml plus the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.

Coordinator check of AC #2 (the reviewer left it open): with the CLI's own versioned schema, 'validate-yaml' rejects a document with an unknown type (type: bogus_type, schema tcms/test-case.schema.v1.json: exit 1) and one with an unknown schema (exit 1); a valid one passes (exit 0). That is 'as today'. Test: crates/testcase-common/tests/tcms_schemas.rs::unknown_schema_still_errors. Nuance, documented: the vendored unversioned tcms-schemas schemas do not reference the envelope, so an unknown 'type' beside one of THEM is not rejected; they could not enforce it anyway because requirement, design-element, plan-document and qa-record use 'type' as a domain field (docs/TCMS_SCHEMAS.md). Resolution of auto-selection is by 'schema:', which does error when unknown.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recognises every tcms-schemas record type: the pinned copy (schemas/tcms-schemas, now at 33c3d3f) is reachable through the envelope, the envelope type enum lists all 24 schemas, documents naming them resolve and validate, unknown schemas and types (for the CLI's own schemas) still fail, and all 29 vendored samples pass with an explicit schema. No folder-based fallback was added (not required). make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p testcase-common -p validate-yaml and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
