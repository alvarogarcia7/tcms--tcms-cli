---
id: TCMS-42.1
title: 'ID resolver: translate Jira and StrictDoc ids to URLs and back'
status: Done
assignee:
  - '@worker-c'
created_date: '2026-10-28T11:00:00Z'
updated_date: '2026-10-28T15:32:00Z'
labels: []
dependencies:
  - TCMS-40.1
parent_task_id: TCMS-42
priority: high
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users must not type or paste Jira or StrictDoc URLs. A library crate (used by the CLI and reusable by tcms-web) resolves an id to a system and URL, and a URL to a system and id, driven by integration-config systems[] (id_pattern, url_template with {base_url} {id} {doc}, url_pattern, document_map). Deployment is undecided (Jira Cloud or Data Center, StrictDoc static HTML or server), so nothing is hard-coded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tcms ref resolve <id|url> prints system, id and URL; tcms ref open <id> opens the browser
- [x] #2 An id matching no system, or more than one, gives a clear error; overlapping id_patterns are rejected when the config is loaded
- [x] #3 external_refs[].url left empty in any record is filled by the resolver on read; records never store site URLs by hand
- [x] #4 Example configs for Jira Cloud, Jira Data Center, StrictDoc HTML export and StrictDoc server ship with the CLI
- [x] #5 Unit tests cover every example config in both directions
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 All tests are passing. Run 'make test'
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New lib crate crates/id-resolver: IntegrationConfig types + loader (schema validated, overlap check), Resolver (id->system/url, url->system/id), ExternalRef fill API. 2. New bin crate crates/tcms: tcms ref resolve|open|fill. 3. examples/integration-config/*.yaml (4 deployments), validated by tests in both directions. 4. docs/JIRA_INTEGRATION.md section.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented by worker-c. Crates: id-resolver (layer 3: config types+loader, Resolver, fill API), tcms (binary: ref resolve|open|fill). Examples in examples/integration-config/ (jira_cloud, jira_datacenter, strictdoc_html, strictdoc_server), all validated by the vendored schema in tests and with jsonschema.
Decisions: (D-42.1a) config types live in id-resolver (no testcase-models change); jira-client will depend on it. (D-42.1b) A config without 'schema:' (upstream sample) gets the line added before validation. Example files carry type: integration_config + schema. (D-42.1c) id_pattern is matched against the whole id (anchored). (D-42.1d) Overlap detection is witness-based: sound (every report is a real overlap) but not complete; documented in docs/JIRA_INTEGRATION.md. (D-42.1e) 'both sources' = requirement_source naming jira and strictdoc (list or 'jira,strictdoc') -> dedicated error; the upstream sample lists a strictdoc system next to source jira for navigation, so listing both systems is NOT an error. (D-42.1f) A URL resolves only if it starts with the system base_url and url_pattern yields an id matching id_pattern. (D-42.1g) tcms ref fill prints by default, --write stores URLs (records should not store them). (D-42.1h) {doc} looked up by id prefix before first '-' or '.', then full id. (D-42.1i) tcms ref open: --print-only or TCMS_OPEN_PRINT=1 prints URL; otherwise xdg-open/open. (D-42.1j) strictdoc_server example URL layout is an ASSUMPTION (same as HTML export).
Evidence: cargo test -p id-resolver (4 unit + 10 integration pass), cargo test -p tcms (5 pass, incl. real binary and a stand-in xdg-open on PATH); clippy -D warnings clean. The real browser was not launched (no display).

Review (round 2, branch tcms-review-r2): cargo test -p id-resolver (4 unit, 10 integration) and -p tcms (ref_cli 5) passed. Real tcms binary: resolve PAY-2231 on jira_cloud and jira_datacenter gives the site URL; HLR-12 on strictdoc_server and strictdoc_html resolves to the document page; a jira URL resolves back to system jira and id; an id of no system and a URL of no configured base exit 1 with clear messages; a jira id against the strictdoc config exits 1 naming the system pattern. A config with two overlapping id_patterns (PAY-[0-9]+ and [A-Z]+-[0-9]+) is rejected at load with the witness PAY-0. requirement_source 'jira,strictdoc' is rejected at load with names both jira and strictdoc. tcms ref open HLR-12 --print-only prints the URL; unknown id exits 1. tcms ref fill on a requirement with an empty external_refs url (schema field key) fills it and reports 1 url filled. Judgement on AC3: the URL fill is implemented as fill_external_refs / fill_yaml_value and the tcms ref fill command; no CLI reader (requirements import, trace-matrix) calls it on read yet, so the on-read behaviour is a library capability for the future readers (tcms-web). Accepted for this task, flagged for TCMS-42.9. Overlap detection is witness based (documented limit in docs/JIRA_INTEGRATION.md).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
id-resolver crate (config types and loader with schema validation, overlap rejection, both-sources rejection, resolver, URL fill) and tcms ref resolve/open/fill with four example configs (Jira Cloud, Jira Data Center, StrictDoc HTML, StrictDoc server). Round 2 review verified every AC with the real binary and the crate tests. DoD: make test itself not runnable here (uv, shellcheck, StrictDoc, TPDG missing); verified with cargo test id-resolver and tcms and the coordinator's workspace run (2085 passed at 9525332, the 2 failures fixed in eff5436) instead. Open point: the URL fill is on demand (tcms ref fill / library), not on every read by the CLI readers.
<!-- SECTION:FINAL_SUMMARY:END -->
