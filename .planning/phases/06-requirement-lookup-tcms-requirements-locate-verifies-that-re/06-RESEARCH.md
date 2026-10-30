# Phase 6: Requirement lookup - Research

**Researched:** 2026-10-30
**Domain:** Rust CLI subcommand (`tcms requirements locate`) over existing Jira client, ID resolver and StrictDoc export reader
**Confidence:** HIGH for code facts (read in this session), MEDIUM for real-Jira behaviour (only mock-verified)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** The lookup is a new subcommand of the existing `tcms` binary (`crates/tcms`), next to `requirements import`. It is not a new binary.
- **D-02:** Each ID is resolved with `id_resolver::Resolver::resolve_id`. The system it resolves to decides the source. An ID whose format matches no system fails the command, naming the ID.
- **D-03:** Jira: a search by key, narrowed to the configured `project_keys` and `feature_issue_types`. A Bug, or an issue in another project, is reported as not found.
- **D-04:** StrictDoc: the ID is looked up in the export given with `--input`, read with the same rules as `requirements import` (`strictdoc::parse_export`). An item whose level is not a requirement type (for example `SYS`) is reported as "not a requirement type", not as found. Found-at-another-level is detected with `strictdoc::export_levels`.
- **D-05:** Output: one line per ID (found with title and status and link, not a requirement, or not found). Exit 0 when every ID is a requirement, exit 1 when any ID is missing or not a requirement, and exit 1 with a clear error for config or input problems.
- **D-06:** The Jira client is built once and reused for all IDs of that run. Credentials come only from the environment variable named by `auth.secret_ref`, as in `requirements import`.
- **D-07 (assumption):** The StrictDoc export layout is the one assumed by `requirement-import` (see docs/JIRA_INTEGRATION.md). Not verified against a real StrictDoc export.

### Claude's Discretion
- Exact wording of the output lines, provided D-05 holds.
- Whether the JSON output format is added in this phase (not required by LOOK-01..04).

### Deferred Ideas (OUT OF SCOPE)
- JSON output format for the lookup result.
- Checking a StrictDoc HTML or server URL directly (only the export file is read in this phase).
- Verifying the StrictDoc export layout against a real StrictDoc export (open question, D-07).
- Requirement lookup for Gitlab systems (system type exists, not a requirement source).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LOOK-01 | `locate <ID>...` checks each ID in the source it belongs to, via the integration config | `Resolver::new(&cfg)` + `resolve_id` give `Resolved{system, system_type, id, url}`; dispatch on `system_type.is_jira()/is_strictdoc()` |
| LOOK-02 | Jira ID found only as a feature issue of a configured project and type | JQL with `key`, `project in`, `issuetype in`; plus client-side key equality check (mock ignores `key =`) |
| LOOK-03 | StrictDoc ID checked against `--input`; level not a requirement type reported as such | `parse_export` records, plus a locate-specific level rule (see Blocking Question B1) |
| LOOK-04 | exit 0 / exit 1; unknown format fails naming the ID | `main()` already exits 1 on any `Err` (main.rs:54-57); resolver error text names the ID |
</phase_requirements>

## Summary

All pieces already exist; no new crate, no new dependency (`crates/tcms/Cargo.toml` already depends on `id-resolver`, `jira-client`, `requirement-import`; dev-dep has `jira-client` with `mock`, and `tempfile`). The change is one new `Locate` variant and a `locate` function in `crates/tcms/src/reqcmd.rs`, one small shared JQL helper in `crates/requirement-import/src/jira.rs`, a new test file, and doc lines. Layering is respected: `tcms` is layer 5 and already depends on the layer 3 crates it needs.

There is one real conflict in the decisions that the planner must resolve first (B1): in `requirement-import`, `"system"` IS a requirement type (`REQUIREMENT_TYPES`, lib.rs:28-38), and the fixture's `SYS-4` has level `system`. So with the literal D-04 rule ("level not in the requirement types") SYS-4 would be reported as *found*, contradicting LOOK-03 and the CONTEXT example. Also `parse_export` and `export_levels` both refuse the whole export if any item has a level outside `REQUIREMENT_TYPES` (strictdoc.rs:101-131), so they can never return a "level that is not a requirement type" other than the in-list ones such as `system`. The locate command therefore needs its own rule for which levels count.

**Primary recommendation:** In `reqcmd.rs` add `Locate { ids, input, actor }`; resolve all IDs first (fail fast, no output, naming the ID), build the Jira client lazily once, read/parse the export once, print one line per ID, return `Err` ("N of M ... ") when any ID is missing or not a requirement so `main` exits 1. Count only levels `HLR` and `LLR` as requirements for StrictDoc (same rule as the trace matrix, `crates/test-executor/src/main.rs:520-524`), pending confirmation of B1.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| CLI parsing, line output, exit code | Binary (`tcms`, layer 5) | - | `reqcmd.rs` owns subcommands; `main()` maps `Err` to exit 1 |
| ID to system routing | `id-resolver` (layer 3) | - | `Resolver::resolve_id` already errors on unknown/ambiguous formats |
| Jira key search | `jira-client` (layer 3) | `requirement-import::jira` (JQL builder) | `search_issues` handles Cloud and Data Center paging |
| StrictDoc export reading | `requirement-import::strictdoc` (layer 3) | - | single place that knows the assumed layout |
| Which levels are requirements | `tcms` locate (local const) | - | `REQUIREMENT_TYPES` is the import schema list, not the lookup rule (B1) |

## Standard Stack

No new packages. Everything is in the workspace; no external package is installed in this phase, so no Package Legitimacy Audit is needed.

| Crate | Use | Verified at |
|-------|-----|-------------|
| `id-resolver` | `Resolver::new`, `resolve_id`, `Resolved` | crates/id-resolver/src/lib.rs:36-88 |
| `jira-client` | `JiraClient::from_config`, `search_issues` | crates/jira-client/src/lib.rs:127, 263 |
| `requirement-import` | `strictdoc::parse_export`, `jira::FIELDS`/`jql` | strictdoc.rs:55, jira.rs:19-60 |
| `clap` (workspace) | `Subcommand` variant with `Vec<String>` positional | reqcmd.rs:13 |

Environment (verified this session): OpenSSL is the system package 3.5.7 (`pkg-config --modversion openssl`); `cargo 1.99.1`; with `CARGO_HOME=$HOME/.cargo-user`, `cargo test -p tcms --test requirements_cli` builds and passes 7/7 with no workaround variables. No other external tool is needed (Jira is the in-process mock).

## Code Facts (with file:line)

### reqcmd.rs
- `RequirementsCommand` has one variant, `Import { out, input: Option<PathBuf>, commit, safety_label, security_label, actor }` (reqcmd.rs:13-36). `actor` is `#[arg(long = "as")]` (line 33).
- `run(ctx: &Context, cmd)` matches on the enum (line 38-95). Import: `ctx.load_config()` (49), `cfg.source_system()` (50), then dispatches on `cfg.requirement_source`. Jira: `JiraClient::from_config(system, &cfg.sync, &acting_user(actor))?` (60) then `jira::fetch`. StrictDoc: `input.context(...)` (67), `std::fs::read_to_string(&input).with_context(|| format!("read {}", input.display()))` (72-73), `strictdoc::parse_export(&text, &commit)?` (74).
- Locate must NOT use `cfg.source_system()`: D-02 says the resolved system decides the source, so find the system by `resolved.system` in `cfg.systems`.
- `main.rs:52-58`: any `Err` prints `error: {e:#}` to stderr and `exit(1)`. `main.rs:65` already dispatches `Requirements` to `reqcmd::run`; no change in main.rs except optionally the doc comment at line 45.
- `Context` (refcmd.rs:10-39): `load_config()`, `schemas_root()`. `acting_user` (jiracmd.rs:76).
- Existing single-ID checks: `refcmd.rs` has `Resolve` (prints system, id, URL; pure format/URL mapping, no existence check), `Open` (xdg-open), `Fill`. None checks that an ID exists in Jira or an export. Nothing to duplicate; `locate` is the first existence check. Do not call into refcmd except `Context`.

### id-resolver
- `Resolver::new(&IntegrationConfig)` runs `config.check()` and compiles id patterns anchored with `overlap::full_match` (lib.rs:38-57).
- `resolve_id` (lib.rs:64-88): no match -> `bail!("id '{id}' matches no system id_pattern (systems: {name} {pattern}; ...)")`; several -> `"id '{id}' is ambiguous: it matches systems ..."` (cannot occur after `check()` rejects overlaps). Both name the ID. Success -> `Resolved { system (name), system_type, id, url }`.
- `build()` (lib.rs:156-182) fails if `url_template` contains `{doc}` and `document_map` lacks the ID's prefix: error `system '<n>': document_map has no entry for '<prefix>' (needed for {doc} of id '<id>')`. In the test config (STRICTDOC_YAML, requirements_cli.rs:52-63) `document_map` has `SYS` and `HLR` but not `LLR`, so an `LLR-*` ID would fail resolution there. Tests must avoid LLR IDs or add `LLR` to the map. This error is acceptable behaviour (config problem, exit 1, names ID).
- `SystemConfig` fields used (config.rs:145-180): `name`, `system_type`, `base_url`, `auth.secret_ref`, `project_keys`, `feature_issue_types`. `SystemType::is_jira()/is_strictdoc()` (config.rs:45-53); `Gitlab` is neither.

### jira-client
- `JiraClient::from_config(system, sync, actor)` (lib.rs:127-154): token from env var `auth.secret_ref`; errors `system '<n>': auth.secret_ref is not configured`, `... environment variable <VAR> (auth.secret_ref) is not set`, `... is empty`. Optional `TCMS_JIRA_USER` switches to Basic auth.
- `search_issues(&self, jql: &str, fields: &[&str]) -> Result<Vec<Value>>` (lib.rs:263-317): returns issues as `serde_json::Value`, each with `["key"]` (string) and `["fields"][...]`. Cloud pages by `/search/jql` token, Data Center by `/search` startAt. HTTP errors are `Err` (cannot be told apart from "no match" by the caller).

### jira-client mock
- `matching_issues` (mock.rs:836-857) filters ONLY on the `project`, `issuetype` and `labels` clauses (`jql_values`, mock.rs:815-833, matches `field = "v"` or `field in (a, "b")`). **A `key = "X"` clause is ignored**: with project and type clauses present the mock returns every matching issue. Therefore the locate code MUST filter the returned issues by `issue["key"].as_str() == Some(id)`; this is also correct against real Jira and must be a unit-tested behaviour. The mock still honours project and issuetype, which is what proves LOOK-02 (Bug and other-project cases).
- `mock.add_issue(key, issue_type, summary, description, status, updated, labels)` (mock.rs:151-181); project key is derived from the key prefix (`PAY-2` -> `PAY`). `MockJira::start(layout, "Bearer <token>")`, `base_url()`, `requests()`, `request_count()`, `clear_requests()` (mock.rs:89-219).
- Token env var in tests: `const ENV: &str = "TCMS_TEST_JIRA_TOKEN"`, config `auth.secret_ref: {ENV}`, child process gets `.env(ENV, token)` and `.env_remove(ENV)` otherwise (requirements_cli.rs:9-10, 65-75).

### strictdoc.rs and fixture
- `parse_export(json, commit)` requires a non-empty whitespace-free `commit` (strictdoc.rs:56-58), returns `Vec<RequirementRecord>` sorted by id with `id`, `req_type` (=level), `title: Option`, `status: Option`, `source` (`path#UID`). `export_levels(json)` returns `Vec<(uid, level)>` (89-97). Both go through `read_export` -> `check_item`, which bails if the level is not in `REQUIREMENT_TYPES` or the status is not in `REQUIREMENT_STATUSES` (101-131). `REQUIREMENT_TYPES` = system, HLR, LLR, derived, SFR, TSFI, threat, objective, assumption (requirement-import/src/lib.rs:28-38).
- Fixture `crates/requirement-import/tests/fixtures/strictdoc_export.json`: three items. `SYS-4` level `system` (docs/system.sdoc, "Remote profile management", approved); `HLR-12` level `HLR` ("Update profile metadata over SMS", approved); `HLR-3` level `HLR` ("Reject unsigned profiles", in_review). Records: HLR-12 and HLR-3; "levels only / not a requirement" in the matrix sense: SYS-4. (As an import record SYS-4 IS valid, hence B1.)
- No duplicate-UID check in `parse_export`; `test-executor` rejects duplicates itself (main.rs:521-527).

## Architecture Patterns

### Flow
```
argv IDs --> load_config --> Resolver::new --> resolve_id (ALL ids first; first failure = Err naming the ID, no output)
   |
   +-- system_type.is_jira()      --> lazy JiraClient::from_config (once per system) --> search_issues(key JQL) --> filter key == id
   +-- system_type.is_strictdoc() --> read --input once, parse_export once --> lookup uid, apply level rule
   +-- other (gitlab)             --> Err "system '<n>' (gitlab) is not a requirement source"
   |
 one stdout line per ID (input order) --> any not-found/not-requirement --> Err("N of M ...") --> main: exit 1
```

### Exact changes
1. `crates/tcms/src/reqcmd.rs`
   - Add variant:
     ```rust
     /// Check that each ID is a requirement in its source (Jira feature issue or StrictDoc item)
     Locate {
         /// Requirement IDs, for example PAY-2231 HLR-12
         #[arg(required = true, num_args = 1..)]
         ids: Vec<String>,
         /// StrictDoc only: JSON export of the document set (assumed layout)
         #[arg(long)]
         input: Option<PathBuf>,
         /// Acting TCMS user for the Jira read (default: $TCMS_USER, then $USER)
         #[arg(long = "as")]
         actor: Option<String>,
     }
     ```
   - In `run`, arm `Locate`: delegate to `fn locate(ctx, ids, input, actor) -> Result<()>`.
   - `locate`: `let cfg = ctx.load_config()?; let resolver = Resolver::new(&cfg)?;` resolve every ID first (`resolver.resolve_id(id)?`; error text is the resolver's). Then:
     - `Option<JiraClient>` built on first Jira ID via `JiraClient::from_config(system, &cfg.sync, &acting_user(actor.clone()))` where `system = cfg.systems.iter().find(|s| s.name == resolved.system)`; reused for all later Jira IDs (D-06). If a config had two Jira systems, key the cache by system name (`BTreeMap<String, JiraClient>`).
     - `Option<Vec<RequirementRecord>>` for StrictDoc, read with `fs::read_to_string(&input).with_context(|| format!("read {}", input.display()))` and `strictdoc::parse_export(&text, "locate")` (placeholder commit; the version is not printed). Missing `--input` when a StrictDoc ID is present: `bail!("{id}: belongs to StrictDoc system '{name}'; pass --input <export.json>")`. `--input` given but no StrictDoc ID: accept silently (or bail; discretion, recommend silent).
   - Outcome enum (private): `Found { family, system, title, status, url }`, `NotRequirement { level }`, `NotFound`. A `fn line(id, &Outcome) -> String` so formatting is unit-testable.
2. `crates/requirement-import/src/jira.rs`: add `pub fn jql_for_key(system: &SystemConfig, key: &str) -> Result<String>` reusing the same empty-`project_keys` / empty-`feature_issue_types` bails as `jql()` (jira.rs:37-60). Factor the shared pieces so the project and type lists are built once.
3. `crates/tcms/tests/requirements_locate_cli.rs` (new) - see Test Plan.
4. Docs: `docs/JIRA_INTEGRATION.md` (line 14 table row lists commands; section at line ~91 shows `requirements import`): add `locate` with the output/exit contract and the "export layout is assumed" caveat. Update `main.rs:45` doc comment and `Cargo.toml` description only if wanted.

### Jira JQL (exact)
```
key = "PAY-2" AND project in (PAY, OTH) AND issuetype in ("Epic", "Story", "Feature")
```
Why: `key = "<id>"` selects the issue; `project in (...)` and `issuetype in (...)` are copied from the import JQL builder (jira.rs:50-59: `project in ({keys joined ", "})`, types quoted) so "feature issue of a configured project and type" has one definition (D-03/LOOK-02) and the Bug / other-project cases return no issue. No `ORDER BY` (single issue). The mock understands the `project in (` and `issuetype in (` forms (mock.rs:815-833) and ignores `key =`, so the key equality filter is mandatory. Escape `\` and `"` in the key value before interpolating (the id passed `id_pattern`, but a permissive configured pattern could allow them): `key.replace('\\', "\\\\").replace('"', "\\\"")`.
Fields requested: `["summary", "status", "issuetype"]` (subset of `jira::FIELDS`, jira.rs:19-27). One `search_issues` call per ID (not one batched `key in (...)`) so a failure or odd answer affects one ID and D-06 reuse is meaningful.
Found line data: title `fields.summary`, status `fields.status.name`, link `Resolved.url` (from `url_template`, e.g. `https://.../browse/PAY-2`).

### Output and exit contract (D-05)
Stdout, one line per ID in command-line order (duplicates: print once, keep first position; discretion). Wording (D-05 holds; this follows the CONTEXT examples):

| Outcome | Line | Counts as |
|---------|------|-----------|
| Found (Jira) | `PAY-2: found in jira (jira) "Pay by card" [In progress] https://.../browse/PAY-2` | ok |
| Found (StrictDoc) | `HLR-12: found in strictdoc (strictdoc) "Update profile metadata over SMS" [approved] <url>` | ok |
| Not a requirement | `SYS-4: found in strictdoc at level system, which is not a requirement type` | failure |
| Not found | `PAY-99: not found in jira (jira): no feature issue of projects PAY with type Epic, Story, Feature` / `HLR-99: not found in strictdoc export <path>` | failure |

Format `found in {requirement-family} ({system name})`; the CONTEXT example `jira (Jira)` is family then system name, so the parenthesis holds `Resolved.system`. A missing title prints `""`; a missing status omits the `[...]` group.
Exit codes: 0 = every ID found as a requirement. 1 = any ID not found or not a requirement: print all lines to stdout first, then `return Err(anyhow!("{n} of {m} requirement IDs are missing or not a requirement"))` so `main` prints `error: ...` on stderr and exits 1 (no `process::exit` in library code). 1 = config/input problem (config load, unknown ID format, env var unset, `--input` unreadable or wrong layout, HTTP failure), message on stderr, **no lookup lines on stdout** for the pre-flight failures (unknown format, missing `--input`, missing token), because those are checked before any lookup. A mid-run HTTP error after some lines were printed may leave partial stdout (acceptable; stderr has the error). Exact error text that matters:
- Unknown format: from the resolver, `error: id 'foo' matches no system id_pattern (systems: ...)` (id_resolver lib.rs:72-79). Tests assert the ID and the phrase `matches no system id_pattern`.
- Token unset: `environment variable TCMS_TEST_JIRA_TOKEN (auth.secret_ref) is not set` (jira-client lib.rs:138-143).
- Never print the token (existing tests assert this, requirements_cli.rs:138).

The command is read-only: it writes no files; tests assert a temp dir is unchanged.

## Don't Hand-Roll

| Problem | Don't build | Use instead |
|---------|-------------|-------------|
| ID format to system | own regex table | `Resolver::resolve_id` |
| Jira paging, auth, retry | own HTTP | `JiraClient::search_issues` |
| Feature issue project/type scoping | a second JQL builder | `requirement-import::jira` shared builder (`jql_for_key`) |
| StrictDoc layout | own JSON structs | `strictdoc::parse_export` |
| Link | own URL formatting | `Resolved.url` |

## Common Pitfalls

1. **Mock ignores `key =`.** Without the client-side `issue["key"] == id` check, Jira tests pass for the wrong reason (any PAY Story would match any PAY ID). Add a test with two issues where only one key is asked.
2. **SYS-4 has a valid import type.** See B1; do not implement the rule as `!REQUIREMENT_TYPES.contains(level)`.
3. **`parse_export` needs a commit.** Pass a fixed placeholder ("locate"); it is not shown.
4. **`parse_export` refuses the whole export** if any item has an out-of-list level or status: that is exit 1 "input problem" for every ID, not per-ID "not a requirement". This matches D-04 "same rules as import". Document it.
5. **`document_map` gaps** fail resolution (`LLR-*` in the test config); keep LLR out of tests or extend the map.
6. **Real Jira behaviour for a key that does not exist** (see Assumptions A1): the call may return HTTP 400 rather than an empty list. The current client cannot tell; the command would then exit 1 with an error instead of a "not found" line.
7. Clippy `-D warnings` and `cargo fmt` are required (CLAUDE.md pre-commit workflow).

## Test Plan

Test framework: Rust `cargo test`; integration pattern copied from `crates/tcms/tests/requirements_cli.rs` (real binary via `env!("CARGO_BIN_EXE_tcms")`, `MockJira`, `tempfile`). New file `crates/tcms/tests/requirements_locate_cli.rs` (copy helpers `root`, `fixture`, `write`, `jira_yaml`, `STRICTDOC_YAML`, `tcms`, `text`; use token env `TCMS_TEST_JIRA_TOKEN`). Unit tests for `line()`/level rule inside `reqcmd.rs` (`#[cfg(test)]`) and for `jql_for_key` in `jira.rs`.

| Req | Behaviour | Test (name) | Command |
|-----|-----------|-------------|---------|
| LOOK-01/02 | Cloud: Story PAY-2 found with title, status, link; exit 0 | `locate_jira_found_cloud` | `cargo test -p tcms --test requirements_locate_cli locate_jira` |
| LOOK-01/02 | Data Center same | `locate_jira_found_datacenter` | same |
| LOOK-02 | Bug PAY-3 reported not found (add as Bug); `OTH-1` Story in project OTH (matches id_pattern, not in `project_keys: [PAY]`) not found; `PAY-99` absent not found; exit 1; both layouts | `locate_jira_bug_and_other_project_are_not_found` | same |
| LOOK-02 | Key filter: PAY-2 and PAY-4 both Stories; asking PAY-2 never reports PAY-4 | `locate_jira_matches_the_exact_key` | same |
| D-03 | Request JQL contains `key = "PAY-2"`, `project in (PAY)` and the quoted types (read from `mock.requests()`, URL-decoded path) | `locate_jira_sends_key_project_and_type_clauses` | same |
| D-06 | Two Jira IDs: exactly 2 requests whose path contains `/search`; token never in stdout/stderr | `locate_jira_reuses_one_client` | same |
| D-06 | Token env unset: exit 1, stderr names `TCMS_TEST_JIRA_TOKEN`, `mock.request_count() == 0` | `locate_jira_needs_the_token` | same |
| LOOK-03 | StrictDoc fixture: `HLR-12` found (title, status approved), `SYS-4` -> `found in strictdoc at level system, which is not a requirement type`, `HLR-99` not found; mixed run exit 1; `HLR-12 HLR-3` alone exit 0 | `locate_strictdoc_*` | `cargo test -p tcms --test requirements_locate_cli locate_strictdoc` |
| LOOK-03 | StrictDoc ID without `--input`: exit 1, stderr contains `--input` | `locate_strictdoc_needs_input` | same |
| LOOK-03 | `--input` with a wrong layout (`[1,2]`): exit 1, stderr contains `assumed layout` | `locate_strictdoc_wrong_layout` | same |
| LOOK-04 | Unknown format `abc` (no pattern): exit 1, stderr has `abc` and `matches no system id_pattern`, stdout empty even when a valid ID is listed first, `mock.request_count() == 0` | `locate_unknown_id_format_fails_naming_the_id` | `cargo test -p tcms --test requirements_locate_cli locate_unknown` |
| LOOK-04 | All found -> status success; any missing -> code 1 (`o.status.code() == Some(1)`) | covered by the above | - |
| Read-only | temp dir listing unchanged after a run | assert in the Jira and StrictDoc tests | - |

Sampling: per task `cargo test -p tcms --test requirements_locate_cli` and `cargo test -p requirement-import`; per wave `cargo test -p tcms -p requirement-import`; phase gate (CLAUDE.md): `make build`, `make lint`, `make test`, `make coverage-e2e` and `make acceptance-test` before commit. Wave 0 gaps: only the new test file; framework and mock exist.

## Security Domain

| ASVS | Applies | Control |
|------|---------|---------|
| V5 Input validation | yes | IDs pass `id_pattern` via the resolver before use; escape `\` and `"` in the JQL string; do not build JQL from unresolved input |
| V2/V6 secrets | yes | token only from env var in `auth.secret_ref`; never echo it (assert in tests) |
| V4 Access control | no | read-only command; Jira side is the server's permission model (acting user via `--as`) |
| V12 files | minor | `--input` is read only, never written |

Threat: JQL injection through a permissive `id_pattern` (Tampering) - escape + resolver gate. Token disclosure in output (Information disclosure) - tests.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Real Jira answers a `key = "X"` JQL for a non-existent key with HTTP 400 ("issue does not exist") instead of an empty result [ASSUMED, training knowledge; only the mock was run] | Pitfalls 6 | A missing real key would exit 1 with an error line rather than a clean "not found" line; the exit code is still 1 |
| A2 | Real Jira returns a moved issue under its new key for `key = "<old>"`; the exact-key filter then reports not found [ASSUMED] | Jira JQL | A moved requirement shows as missing |
| A3 | The trace-matrix rule (only HLR and LLR are requirements, test-executor main.rs:520-524) is the intended definition for locate [inferred from code, not from the user] | Summary / B1 | SYS-4 or other levels classified differently from what the user expects |
| A4 | Jira real-site field shapes (`fields.summary`, `fields.status.name`) equal the mock's; import code already relies on them (jira.rs:94,101) but only mock-verified | Jira JQL | Missing title/status in real output |

## Blocking Open Questions

**B1. Which StrictDoc levels count as "a requirement"?** The code and CONTEXT disagree: `system` is in `REQUIREMENT_TYPES` (lib.rs:28-29) and the fixture's SYS-4 has level `system`; LOOK-03 and the CONTEXT example say SYS-4 must be "not a requirement type". `export_levels`/`parse_export` cannot report out-of-list levels (they refuse the export). Recommendation (used in this research): requirement levels for locate = `HLR` and `LLR` (the trace-matrix rule), via a local `const LOCATE_LEVELS: &[&str] = &["HLR", "LLR"]` in reqcmd.rs; every other level that survives `parse_export` (system, derived, SFR, ...) prints the "not a requirement type" line. The user must confirm, because it also decides whether `derived`/`SFR` items are findable. Note the Jira side is inconsistent with this (an Epic is found if `Epic` is in `feature_issue_types`, although the import maps Epic to `system`); D-03 is explicit, so keep it.

## Non-blocking Open Questions (planner may decide)

- Duplicate UIDs in an export: recommend bail like test-executor (exit 1, names the UID).
- Repeated IDs on the command line: recommend de-duplicate, first position.
- Should `--input` with only Jira IDs be an error? Recommend ignore.
- D-07 stays open: layout unverified against real StrictDoc; say so in the docs.

## Project Constraints (from CLAUDE.md)

- Layering: tcms is layer 5; add no dependency from lower layers to higher ones. The helper added to `requirement-import` (layer 3) must not depend on `tcms`.
- All dependencies come from the workspace (`workspace = true`); none are added here.
- New YAML loading must use `testcase_common::load_and_validate_yaml`; this phase loads only the integration config through the existing `IntegrationConfig::load`.
- Shell scripts, if any are touched, bash 3.2 and BSD/GNU portable (not expected here).
- Before commit: `make build`, `make lint` (fmt + clippy `-D warnings`), `make test`, `make coverage-e2e` (70%), and `make acceptance-test`. `cargo test --all-features` must pass.
- CLAUDE.md asks to use Backlog.md MCP for task tracking; the `backlog` MCP server failed to connect in this session (executable not found), so no task was read or created.
- Do not commit during research (instruction for this task).

## Sources

### Primary (HIGH, read in this session)
- crates/tcms/src/reqcmd.rs, main.rs, refcmd.rs, jiracmd.rs
- crates/id-resolver/src/lib.rs, config.rs
- crates/jira-client/src/lib.rs, mock.rs (note: there is no `client.rs`; `JiraClient` lives in `lib.rs`)
- crates/requirement-import/src/lib.rs, jira.rs, strictdoc.rs; tests/fixtures/strictdoc_export.json
- crates/tcms/tests/requirements_cli.rs (7 tests run green with `CARGO_HOME=$HOME/.cargo-user`)
- crates/test-executor/src/main.rs:495-530 (matrix level rule)

### Tertiary (LOW)
- Real-Jira behaviour of `key =` on missing/moved issues: training knowledge only (A1, A2).

## Metadata

**Confidence:** code facts HIGH; test design HIGH; real Jira behaviour LOW (mock only); level rule B1 awaiting user.
**Research date:** 2026-10-30. **Valid until:** until `requirement-import` or the mock change.
