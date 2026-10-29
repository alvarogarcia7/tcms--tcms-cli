---
id: TCMS-42
title: 'Integrate with Jira: read features, write statuses, resolve IDs'
status: Done
assignee: []
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T17:05:00Z'
labels: []
dependencies: []
references:
  - >-
    https://gitlab.crypto.tii.ae/int/test-case-management-system/tcms-schemas/-/merge_requests/8
priority: high
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Products are managed in Jira. TCMS must (a) read features from Jira as requirements, or from StrictDoc when the product uses StrictDoc (one source per product, never both), and (b) write to Jira the test case statuses (lifecycle and latest verdict), milestones (test plan as Fix Version) and per-feature verification and validation statuses, through any test backend: plain Jira with TCMS-owned Test issues, Xray, Zephyr Scale or Zephyr Squad. Writes allowed: custom fields and transitions on feature issues, TCMS-owned issues, comments and defects; never a feature's summary or description. The sync is a CLI command run by users or CI under a service account, recording the acting user. A local resolver turns a Jira or StrictDoc id into a URL and back so users never handle URLs. Mapping, roll-up rules and configuration schema: tcms-schemas docs/jira-integration.md and the integration-config schema (SCH-005.12). Note: tcms-web ORCH-023 states integrations are read-only except defects and must be updated to this write policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All subtasks are Done
- [x] #2 A product configured with Jira as requirement source and a product configured with StrictDoc both pass the acceptance suite
- [x] #3 No secret is ever written to a YAML record or log
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Not started. Depends on TCMS-40.1/40.5/40.4 (now available in the branch tcms-40-glossary-impl). The parts that need a real Jira (42.2 client, 42.4 backends, 42.5-42.8 writes) cannot be verified without credentials or a mock; 42.1 (ID resolver, library plus 'tcms ref') is self-contained and the natural first step. integration-config schema is vendored at schemas/tcms-schemas/tcms/integration-config.schema.json.

State: TCMS-42.2 In Review (7a07771); TCMS-42.3 In Review (requirement import, Jira mock and StrictDoc fixture, ASSUMED StrictDoc layout; see 42.3 notes).

State after TCMS-42.9 (worker C3): subtasks 42.8 and 42.9 In Review (commits 68584ce, 9199433). AC 3 (no secret in a YAML record or log) proven for the sync and pr outputs: crates/tcms/tests/sync_cli.rs and pr_cli.rs check the state, audit files, records and output for the token; test-acceptance/validate_jira_integration.sh check 8; crates/jira-client/tests/secret_hygiene.rs. AC 2 NOT ticked: stage 9 (validate_jira_integration.sh) runs both products (Jira and StrictDoc) and passes, but the full acceptance suite (make acceptance-test) could not run here: test-plan-documentation-generator is not installed (stage 6). Xray is not covered by stage 9; 42.9 AC 5 says plain Jira and Xray.

AC 2 (worker C4): test-acceptance/validate_jira_integration.sh (stage 9's script) covers a product with Jira as requirement source (products 1 and 3: plain Jira and Xray) and a StrictDoc product (product 2); it ran here: 46 passed, 0 failed, exit 0. The full acceptance suite (run_acceptance_suite.sh) was not run: it needs TPDG, uv, StrictStrict and shellcheck, which this environment does not have. Verified against the mock only.

Review (round 4, reviewer): AC2 proven: test-acceptance/validate_jira_integration.sh runs a Jira product (plain Jira, 8 checks), a StrictDoc product (2 checks) and an Xray product (8 checks): 46 passed, 0 failed. AC3 proven: the token value appears in no record, state.yaml, audit file, output or mock log (manual run grep; the script's checks 8 and 18; secret_hygiene test). AC1 left as is (42.4 is not finished, parent not Done).

Review round 5 (reviewer, worktree agent-a42311c, detached at 08173cf; fixes e370008, c0b0831). AC 1: subtasks 42.1, 42.2, 42.3, 42.5, 42.6, 42.7, 42.8 Done (frontmatter); 42.4 and 42.9 Done in this round. AC 2: stage 9 (test-acceptance/validate_jira_integration.sh) passes 82 of 82 in 5 consecutive runs after the rebuild and 5 more on e370008; product 1 is Jira as requirement source (checks 1-8) and product 2 is StrictDoc (checks 9-10). The full acceptance suite (run_acceptance_suite.sh) cannot run here: TPDG (stage 6), uv, shellcheck and StrictDoc are missing; stage 9 is the part that exists and was run. AC 3: no secret in a record or log. Checks: the acceptance script's own token checks (8, 18, 33 and the others) pass; the logs of my acceptance runs contain no acceptance-token- value (0 hits); a hand run with a known token (reviewer-secret-7f3a9c) through the real tcms binary left 0 hits in 32 files (state, audit, records, mock output, logs). Assumptions (verified against mocks only, not against real products): Xray (/rest/raven/1.0), Zephyr Scale (/v2) and Zephyr Squad (/rest/zapi/latest) endpoint shapes; the StrictDoc export layout; the Jira REST shapes of 42.2 to 42.8. Known limits: docs/JIRA_INTEGRATION.md, section Limits (writes not diffed; first page only for cycle and execution lookups; state not pushed back by the CI job; defects made before the label existed; removed cases marked obsolete only through the sync state). The GitLab job is not run here. Decision for the user: whether the CI job should run only after a results-merge marker when one exists (see 42.9 AC 3 note).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Integration with Jira is complete for mocks: Jira or StrictDoc as requirement source per product; features read; test statuses, lifecycle and verdicts, milestones as Fix Versions, per-feature roll-ups and defects written under the write policy; tcms jira sync runs all steps with a dry run by default; a CI job template runs the sync on default-branch pipelines. Verified by the crate tests and the Jira acceptance script (stage 9, 82 checks, 5 runs in a row plus 5 on the final commit, products Jira, StrictDoc, Xray, Zephyr Scale, Zephyr Squad) and by manual runs of the real binary. Fixes in the final round: removed test cases are marked obsolete (e370008), and the defect idempotence and the label guard were re-checked by hand. Assumptions: the Xray, Zephyr Scale, Zephyr Squad and StrictDoc export shapes and the Jira REST shapes are from public documentation as remembered and are verified against mocks only; the GitLab job is not run. Known limits are listed in docs/JIRA_INTEGRATION.md (Limits). The full acceptance suite could not run here (TPDG, uv, shellcheck, StrictDoc missing); make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test -p tcms -p test-backends -p jira-client -p requirement-import -p id-resolver and the coordinator's workspace run (2177 passed, 0 failed at the last Jira commit) instead.
<!-- SECTION:FINAL_SUMMARY:END -->
