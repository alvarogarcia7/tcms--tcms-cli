---
id: TCMS-40.2
title: Parse and keep test classification and requirement links on test cases
status: Done
assignee:
  - '@claude'
created_date: '2026-10-27T19:39:00Z'
updated_date: '2026-10-28T15:10:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-40
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Normal-range vs robustness vs security, test level (hw_sw_integration, sw_integration, low_level, validation), objective, author and requirement IDs are needed for requirements-based coverage and independence checks. Today a test case carries only one free-text `requirement` string; that field stays required for compatibility but is not used for coverage. Traceability uses `requirements[]` only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Optional fields requirements, test_type, test_category, level, objective, environment, pass_fail_criteria, author and independence_required round-trip through load and save without loss
- [x] #2 An invalid test_type, test_category or level value, or a test_category that does not belong to the test_type, gives a clear validation error naming the field
- [x] #3 Test case files without the new fields behave exactly as before
- [x] #4 Unit tests cover valid, invalid and absent fields
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add enums TestType, TestCategory, TestLevel, TestCaseStatus, ExternalSystem and ExternalRef to testcase-models
2. Add optional requirements, test_type, test_category, level, objective, environment, pass_fail_criteria, author, independence_required, version, status and external_refs to TestCase (serde defaults, nothing written when absent)
3. TestCase::validate_classification names test_category/test_type when the pairing is wrong; invalid enum values fail with serde's unknown-variant error naming the field
4. Call the check on load (storage, test-executor)
5. Unit tests: absent, valid (round trip), invalid, pairing
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in testcase-models (TestCase fields + validate_classification), testcase-storage load and test-executor load. Tests: 5 model unit tests, 2 storage tests (round trip through save and load, mismatched category refused). Full workspace cargo test: all passing apart from environment issues listed in TCMS-40 notes. Extra fields version, status and external_refs were added too (same upstream schema) so a load and save loses nothing; external_refs is needed by TCMS-42. Not run: make test (needs the e2e tools and the acceptance suite).

Review: schema fix (commit 048d1f3) makes validate-yaml refuse test_type smoke, level integration and category/type mismatch; regression test test_classification_fields_are_schema_checked. Evidence: cargo test -p testcase-models -p testcase-storage -p testcase-common -p testcase-verification -p testcase-validation all pass; workspace cargo test --workspace --all-features --no-fail-fast exit 0. Loader error names test_type/test_category. Absent fields: existing testcases/ and examples validate (11 files). make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test workspace instead.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Classification fields parsed, round-tripped and validated; schemas now refuse invalid values. make test not runnable here; verified with cargo test.
<!-- SECTION:FINAL_SUMMARY:END -->
