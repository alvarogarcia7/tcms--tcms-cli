---
id: TCMS-42.3
title: 'Requirement source adapters: Jira features and StrictDoc'
status: Done
assignee:
  - '@worker-c'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T15:32:00Z'
labels: []
dependencies:
  - TCMS-42.2
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A product reads requirements from exactly one source. Jira adapter: issues of feature_issue_types in project_keys become requirement records (Epic = system, Story/Feature = HLR; version = updated timestamp; source = issue key). StrictDoc adapter: items from the StrictDoc export at the baseline's commit become requirement records (version = commit). Both produce the same requirement YAML so coverage, trace matrix and reviews work unchanged (TCMS-40.5, TCMS-40.6 read from this).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tcms requirements import writes requirement YAML that validates, from either source, chosen by integration-config.requirement_source
- [x] #2 A product configured with both sources is rejected with a clear error
- [x] #3 Jira import keeps issue key, type, status, summary as title, description as text, labels for safety/security flags when configured
- [x] #4 StrictDoc import records the document path, UID and commit
- [x] #5 Re-import is idempotent: unchanged requirements produce no diff
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New library crate crates/requirement-import (layer 3: id-resolver, jira-client, testcase-common, testcase-models). RequirementRecord serialises to requirement YAML (schema: tcms/requirement.schema.json), validated with testcase_common before any write.
2. Jira adapter: JQL over project_keys x feature_issue_types through JiraClient::search_issues; Epic=system, Story/Feature=HLR, other types refused; version=updated; source=key; title=summary; text=ADF text; Jira status kept in external_refs[].status; labels for safety/security only when a label flag is given.
3. StrictDoc adapter: JSON export (documents[].requirements[]) with an ASSUMED format, isolated in one module; version=commit (required flag); source=path#UID.
4. Writer: one file per id (<id>.yaml), stable sort, write only when content differs (no diff on re-import), refuse when the directory holds records of the other source, report stale files without deleting them.
5. tcms requirements import --out DIR [--input EXPORT --commit SHA] [--safety-label L] [--security-label L]; source chosen by integration-config.requirement_source; both sources in requirement_source rejected at config load (existing check).
6. Tests: unit (mapping), mock both layouts (import, re-import no diff, change detection), StrictDoc fixture, CLI tests in crates/tcms. Validate with the vendored schema.
7. docs/JIRA_INTEGRATION.md; notes with decisions D-42.3a.. and ASSUMPTION for the StrictDoc format.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence (mock and fixture, never a real Jira or StrictDoc): cargo test -p requirement-import: unit 10 passed, import 6 passed (Jira Cloud and DC mock: import, re-import with no file touched, one changed issue updates one file; StrictDoc fixture; stale kept; other-source directory refused before any write; duplicate ids refused). cargo test -p tcms --test requirements_cli: 7 passed (real binary: Jira both layouts, StrictDoc, both sources rejected, requirement_source without its system rejected, jira-only flags refused for StrictDoc). Python schema check (Draft7 + format checker, yaml.safe_load) of 3 Jira and 3 StrictDoc records: 0 errors. clippy -D warnings clean.
D-42.3a: requirement YAML has schema: tcms/requirement.schema.json and NO envelope type key: the schema's own type is the requirement type (HLR, system), so the envelope rule of CLAUDE.md cannot apply to this record.
D-42.3b: Jira type mapping Epic = system, Story and Feature = HLR; any other type is refused with the issue key, not guessed.
D-42.3c: The Jira status name is kept in external_refs[].status; requirement status (draft, in_review, approved, obsolete) is a different enumeration and is left unset.
D-42.3d: The safety and security flags have no key in the vendored integration-config schema; they are command line flags (--safety-label, --security-label) and set safety_related / security_related only when given. Flag to upstream schema owners.
D-42.3e: Jira parent (epic link or parent issue) becomes parent_ids.
D-42.3f: Re-import: records are validated before any write, written in id order, and a file whose YAML is unchanged is not touched. Files of the same source that the source no longer has are reported as stale and kept, never deleted. A directory holding records of the other source is refused before any write.
D-42.3g: A product naming both sources in requirement_source is rejected at config load (existing check, with the message names both jira and strictdoc). A config with a jira requirement_source plus a strictdoc system is still accepted, because the upstream sample jira_001.yml uses that for id resolution only; the import reads only the requirement source system.
D-42.3h: Import output default is requirements/, one <id>.yaml per requirement; ids that cannot be file names are refused.
ASSUMPTION (not verified; StrictDoc is not installed and no export sample exists): the StrictDoc JSON export is {documents: [{path, requirements: [{uid, level, title, statement, status, parent_uids, safety_related, security_related}]}]}. The layout is read only in crates/requirement-import/src/strictdoc.rs; the fixture crates/requirement-import/tests/fixtures/strictdoc_export.json follows it. version = --commit (required), source = <path>#<UID>.
Not implemented: reading StrictDoc from a server or HTML site (the import reads an export file only); the sync that writes back is TCMS-42.9.

Review (round 2, branch tcms-review-r2): cargo test -p requirement-import (unit 10, import 6) and -p tcms --test requirements_cli (7) passed; clippy was not re-run (no code change). Real binary, StrictDoc path: fixture export with --commit abc1234 creates HLR-3, HLR-12, SYS-4; each record validates against tcms/requirement.schema.json (Draft 7 with format checker over yaml.safe_load); version = commit, source = docs/srs.sdoc#HLR-3, external_refs strictdoc HLR-3; second run reports 0 created, 0 updated, 3 unchanged. config requirement_source 'jira,strictdoc' is rejected with names both jira and strictdoc and nothing written. AC2 reading: the vendored schema allows one requirement_source value, and the upstream jira-integration.md says a product uses one requirement source and a Jira feature key may be listed for navigation; the upstream sample jira_001.yml lists a strictdoc system beside requirement_source jira. So a strictdoc system beside source jira is not two sources; naming both sources in requirement_source is. Reading confirmed. AC3 type: the record keeps the mapped requirement type (system or HLR), not the raw Jira issue type name; the status name is kept in external_refs[].status. StrictDoc layout is an ASSUMPTION, stated in notes and in docs/JIRA_INTEGRATION.md. Requirement YAML has no envelope type key (schema type is the requirement type), a conflict with the CLAUDE.md envelope rule that the schema forces; flagged to the coordinator.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
requirement-import crate (Jira features as Epic=system, Story/Feature=HLR; StrictDoc export at a commit with an assumed layout) and tcms requirements import, source chosen by requirement_source, records validated against the schema before writing, idempotent re-import, stale records kept, other-source directories refused. Round 2 review re-ran the crate tests and the real StrictDoc import, validated the output against the vendored requirement schema, and confirmed the both-sources reading against the upstream doc and sample. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test requirement-import and tcms requirements_cli and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead. Risks: StrictDoc export layout unverified (no StrictDoc here); Jira side verified against the mock only.
<!-- SECTION:FINAL_SUMMARY:END -->
