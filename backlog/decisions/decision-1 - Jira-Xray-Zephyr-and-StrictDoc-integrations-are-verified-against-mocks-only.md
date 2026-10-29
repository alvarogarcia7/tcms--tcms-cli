---
id: decision-1
title: Jira, Xray, Zephyr and StrictDoc integrations are verified against mocks only
date: '2026-10-28T17:07:00Z'
status: accepted
---
## Context

TCMS-42 integrates with Jira (feature issues, test entities, statuses, Fix Versions, defects) through plain Jira, Xray, Zephyr Scale and Zephyr Squad, and TCMS-40.6 and TCMS-42.3 read requirements from StrictDoc exports. None of these systems was available while the integration was built: there is no Jira, Xray or Zephyr instance and no credentials, and StrictDoc produced no export sample for the layout the importers read. The work was done by agents without access to the owner's systems.

## Decision

Every part that talks to one of these systems is verified against in-repository mock servers and crafted fixtures, and the shapes it relies on are ASSUMPTIONS, labelled as such:

- Jira Cloud REST v3 and Data Center REST v2 request and response shapes, from documentation knowledge
- the Xray, Zephyr Scale and Zephyr Squad endpoints, payloads and status codes, from documentation knowledge
- the layout of the StrictDoc JSON export (crates/requirement-import/src/strictdoc.rs and the fixtures under crates/*/tests/fixtures/)
- GitLab CI behaviour of the `tcms:jira-sync` job (its structure is tested statically; it has not run on GitLab)

The mocks (crates/jira-client/src/mock.rs and the `mock-jira` binary) are built from the same assumptions, so a passing test shows that the code is consistent with them, not that the real systems behave that way. Tasks verified this way are marked Done with that qualification in their notes; Done does not claim real-system behaviour. The assumptions are kept in few places (the backends, the StrictDoc parser) so that correcting one is a local change.

## Consequences

- The first run against a real Jira, Xray, Zephyr or StrictDoc instance is itself a verification step. Run `tcms jira test-connection`, keep the default dry run, and leave `write_policy` closed for features until the previews look right.
- A shape that differs in reality shows up as a request error naming the issue key, or as an empty import; the write guard still prevents writes outside the policy, and no secret reaches a record or a log.
- Recommended follow-up, not yet a task: verify the integration once against a real instance of each backend and replace the assumptions in the documentation (docs/JIRA_INTEGRATION.md, docs/TRACEABILITY.md) by what was observed. The StrictDoc layout needs one real export.
- The same limit applies to the GitLab job: confirm it once in a real pipeline.
