# Phase 6: Requirement lookup - Context

**Gathered:** 2026-10-30
**Status:** Ready for planning
**Source:** Decisions taken in discussion on 2026-10-30 (not an interactive discuss-phase run)

<domain>
## Phase Boundary

`tcms requirements locate <ID>...` verifies that each requirement ID exists in its source: a Jira
feature issue in the configured backlog, or an item of a StrictDoc export. It reports what it
found and exits non-zero when any ID is missing. It does not import, write or change anything.

</domain>

<decisions>
## Implementation Decisions

- **D-01:** The lookup is a new subcommand of the existing `tcms` binary (`crates/tcms`), next to
  `requirements import`. It is not a new binary.
- **D-02:** Each ID is resolved with `id_resolver::Resolver::resolve_id`; the system it resolves to
  decides the source. An ID that matches no system fails the command with exit 2, before any
  lookup is sent, naming the ID.
- **D-03 (Jira, batch):** All Jira IDs of one run are looked up in one search:
  `key in ("PAY-1", "PAY-2", ...)`. Keys are escaped (`\` and `"`). The search requests the
  `summary` and `status` fields. Each returned issue is matched to its ID by its `key`, because
  the mock does not filter on key.
- **D-04 (Jira, scope):** Any issue type counts ("include everything"): Bugs, Epics, Stories,
  Features and others are found when their key matches. The configured project keys are NOT used
  to narrow the search. (Interpretation of "include everything"; confirm.)
- **D-05 (StrictDoc):** Each StrictDoc ID is looked up in the export given with `--input`, a local
  file. No URL is read. Every item of the export counts as found, whatever its level; the line
  shows the level (`HLR`, `LLR`, `system`, ...). (Interpretation of "depends on the type"; confirm.)
  An export with a level that the importer refuses fails the command with exit 2, as `import` does.
- **D-06 (exit codes):** 0 when every ID is found; 1 when any ID is not found; 2 on an error (D-02
  unknown ID format, a StrictDoc ID without `--input`, a missing or empty credential, a
  configuration problem, an unreadable or invalid export).
- **D-07 (output):** Text by default. `--format=json` prints one JSON array of result objects.
  `--format=plaintext` prints the text lines without decoration (no colour or markers).
- **D-08:** The Jira client is built once per run and reused for the batch. The token comes only
  from the environment variable named by `auth.secret_ref`.
- **D-09 (assumption):** The StrictDoc export layout is the one assumed by `requirement-import`
  (docs/JIRA_INTEGRATION.md). Not verified against a real StrictDoc export.
- **D-10:** Tests as planned in 06-RESEARCH.md: mock-based Jira (Cloud and Data Center) with batch
  and key-match checks, and StrictDoc fixture tests.
- **D-11 (export-ids input):** `tcms requirements export-ids` reads `requirements.json` in the
  current directory by default, or the file named by `--requirements <FILE>`. Shape:
  `{ "product_id": "<text>", "requirements": [ { "id": "<text>" }, ... ] }`. A file that does not
  parse, or whose `product_id` differs from the configured `product_id`, fails with exit 2.
- **D-12 (export-ids validity):** "Valid" means the source finds the ID, using the same batch
  lookup as `locate` (D-03 to D-05). An ID repeated in the file is checked and reported once.
  (Confirmed by the owner: valid means the source finds the ID.)
- **D-13 (export-ids output):** The valid IDs are written as JSON in the same shape:
  `{ "product_id": "<text>", "requirements": [ { "id": "<text>" }, ... ] }`, to stdout or to
  `--output <FILE>` (JSON only; confirmed, no `--format`). Invalid IDs are listed on stderr. Exit 0 when every ID is valid, 1 when any
  is invalid (the valid ones are still written), 2 on an error. (Exit codes follow D-06.)
</decisions>

<canonical_refs>
## Canonical References

### Planning
- `.planning/REQUIREMENTS.md` - LOOK-01..LOOK-04
- `.planning/ROADMAP.md` - Phase 6 goal
- `.planning/PROJECT.md` - Key Decisions

### Code
- `crates/tcms/src/reqcmd.rs` - `RequirementsCommand` (add the subcommand here); `Import` shows the source dispatch
- `crates/tcms/src/main.rs` - the `Requirements` dispatch
- `crates/tcms/tests/requirements_cli.rs` - binary test pattern with the Jira mock
- `crates/id-resolver/src/lib.rs` - `Resolver::new`, `resolve_id`, `Resolved`
- `crates/id-resolver/src/config.rs` - `SystemConfig` (project_keys, feature_issue_types), `RequirementSource`
- `crates/jira-client/src/client.rs` - `JiraClient::from_config`, `search_issues`
- `crates/jira-client/src/mock.rs` - `MockJira` (JQL is matched on the mock)
- `crates/requirement-import/src/strictdoc.rs` - `parse_export`, `export_levels`
- `crates/requirement-import/tests/fixtures/strictdoc_export.json` - includes a `system` item (SYS-4) and HLR items

### Docs
- `docs/JIRA_INTEGRATION.md` - Jira and StrictDoc configuration
- `examples/integration-config/` - sample integration configs

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Resolver` maps any ID to its system; `resolve_id` already fails on an unknown format.
- `requirements import` already parses the StrictDoc export and checks levels.
- `MockJira` serves Cloud and Data Center search, so the Jira path is testable without network.

### Established Patterns
- Subcommands take `--config`, `--schemas-root`, and `--as` for the acting user.
- Tests run the real `tcms` binary (`CARGO_BIN_EXE_tcms`) with the mock and an env token.

### Integration Points
- `RequirementsCommand` gets a `Locate` variant; `reqcmd::run` dispatches it.

</code_context>

<specifics>
## Specific Ideas

- Example output line: `PAY-2: found in jira (Jira) "Pay by card" [In progress] https://example.atlassian.net/browse/PAY-2`
- Example StrictDoc line: `SYS-4: found in strictdoc at level system`

</specifics>

<deferred>
## Deferred Ideas

- Checking a StrictDoc HTML or server URL directly (only the export file is read in this phase).
- Verifying the StrictDoc export layout against a real StrictDoc export (open question, D-07).
- Requirement lookup for Gitlab systems (system type exists, not a requirement source).

</deferred>
