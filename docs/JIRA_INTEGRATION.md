# Jira and StrictDoc integration

Mapping, roll-up rules and the configuration schema are defined upstream in tcms-schemas
(`docs/jira-integration.md`, `schemas/tcms-schemas/tcms/integration-config.schema.json`). This
page describes what the CLI implements. Parts are verified only against mocks, see **Limits**.

## Crates

| Crate | Layer | Role |
|-------|-------|------|
| `id-resolver` | 3 | Config types and loader, id to URL and URL to id, `external_refs` fill |
| `jira-client` | 3 | Jira Cloud and Data Center client, write guard, audit entries, mock (TCMS-42.2) |
| `requirement-import` | 3 | Requirement adapters: Jira features, StrictDoc export (TCMS-42.3) |
| `tcms` | 5 | Binary `tcms`: `ref`, `jira test-connection`, `requirements import` |

## Configuration

One `integration-config` YAML per product. It is validated against the vendored schema when
loaded; a missing `schema:` line is added, so the upstream samples load unchanged. Examples that
validate (and are covered by tests in both directions) are in `examples/integration-config/`:

| File | Deployment |
|------|-----------|
| `jira_cloud.yaml` | Jira Cloud (REST v3) |
| `jira_datacenter.yaml` | Jira Data Center (REST v2), personal access token |
| `strictdoc_html.yaml` | StrictDoc static HTML export |
| `strictdoc_server.yaml` | StrictDoc server (ASSUMPTION: same page layout as the export; adjust `url_template`) |

`tcms` finds the config with `--config`, then `$TCMS_INTEGRATION_CONFIG`, then
`./integration-config.yaml`; the schemas with `--schemas-root`, then `$TCMS_SCHEMAS_ROOT`, then
`./schemas/`.

Rules checked at load, beyond JSON Schema:

* every `id_pattern` and `url_pattern` compiles; `url_pattern` has the named group `id`;
* an `id_pattern` must match the whole id (it is anchored for you);
* `id_pattern`s must not overlap. Detection tries witness strings generated from each pattern
  (alternation branches, class end points, repetition counts) against the other patterns. A
  reported overlap is always real; an overlap that exists only on strings outside the witness set
  is not found (limit);
* `{doc}` in `url_template` needs a `document_map`; the entry is looked up by the id prefix
  (`HLR` of `HLR-12`);
* `requirement_source` is exactly one of `jira` or `strictdoc`; a value naming both is rejected
  with a dedicated error, and the selected source needs a system of its family.

## `tcms ref`

```
tcms ref resolve PAY-2231                  # system, id, url
tcms ref resolve https://example.atlassian.net/browse/PAY-2231
tcms ref open HLR-12                       # xdg-open (Linux) or open (macOS)
tcms ref open HLR-12 --print-only          # or TCMS_OPEN_PRINT=1: print the URL, start nothing
tcms ref fill record.yaml [--write]        # fill empty external_refs[].url
```

An id matching no system, or several, is an error naming the systems. `external_refs[].url` is
meant to stay empty in records: `fill_external_refs` (for `testcase_models::ExternalRef`) and
`fill_yaml_value` (any YAML record) derive it on read. `tcms ref fill` prints the filled record and
leaves the file alone unless `--write` is given.

## Jira client (TCMS-42.2)

Crate `jira-client` (layer 3): one client for Jira Cloud (REST v3, `/rest/api/3`) and Data Center
(REST v2, `/rest/api/2`), picked from `systems[].type`.

* **Token.** Read only from the environment variable named by `auth.secret_ref`. Cloud uses Basic
  auth when `TCMS_JIRA_USER` (the service account email) is set, otherwise Bearer. Data Center
  uses a personal access token (Bearer), or Basic when `TCMS_JIRA_USER` is set. The token is never
  written to a record, a log line or an error message; the test `secret_hygiene` checks the audit
  files, the logs captured at debug level, the error texts and `Debug` output.
* **Rate limits.** HTTP 429 is retried up to 5 times with exponential back-off from 500 ms, capped
  at 30 s; a `Retry-After` in seconds is honoured. Any other HTTP error fails at once with the issue
  key in the message (`issue PAY-2231: Jira PUT ... failed with HTTP 500: ...`).
* **Write guard.** Every write is checked against the config before a request is built
  (`jira_client::check_write`). Refused, with no request sent: an issue outside `project_keys`
  (creates too, by their project); a field not listed in `fields`; `summary` or `description` of a
  feature; a write kind whose `write_policy` switch is off (`feature_fields`,
  `feature_transitions`, `dedicated_issues`, `comments`, `defects`); a transition name not listed
  in `transitions`.
* **Audit.** Each successful write yields an `audit-trail-entry` (`action: sync`, `who` = the
  acting TCMS user, `entity` = the TCMS record, `after.external_refs` and `after.written_fields`).
  `write_audit_entries` writes them as `<id>.yaml`. Comments end with the sync run id and the
  acting user, and the audit entry names the same user.
* **Commands.** `tcms jira test-connection [--system NAME] [--as USER]` prints the site,
  deployment, version, authenticated user and each project of `project_keys` (accessible or not);
  it exits non-zero when a project is not accessible. The acting user defaults to `$TCMS_USER`,
  then `$USER`.

## Requirement import (TCMS-42.3)

Crate `requirement-import` (layer 3) and `tcms requirements import`.

```
tcms requirements import --out requirements/                           # Jira source
tcms requirements import --out requirements/ --safety-label safety \
                         --security-label security --as alice          # Jira, label flags
tcms requirements import --out requirements/ --input srs.json --commit 9f1c2ab   # StrictDoc
```

The source is `requirement_source` of the config. A config naming both sources is rejected at load,
with `names both jira and strictdoc`. A config whose source has no system of its family is
rejected too. `--input` and `--commit` apply to StrictDoc only; the label flags to Jira only.

**Jira** (`jira` module): JQL `project in (...) AND issuetype in (...)` over `project_keys` and
`feature_issue_types`. Mapping: Epic becomes `type: system`, Story and Feature become `type: HLR`,
any other issue type is refused. `id` and `source` = issue key; `title` = summary; `text` =
description as plain text; `version` = `updated` timestamp; `status` = Jira status name, kept in
`external_refs[].status` (a requirement `status` is a different enumeration, so it is not set);
`parent_ids` = parent issue; `safety_related` / `security_related` set only when a label flag is
given (true when the issue carries that label).

**StrictDoc** (`strictdoc` module): `id` = UID; `type` = level; `text` = statement; `version` =
the commit given with `--commit` (required); `source` = `<document path>#<UID>`;
`external_refs` names the UID in system `strictdoc`. The export layout is an ASSUMPTION, see below.

**Output.** One `<id>.yaml` per requirement, each with `schema: tcms/requirement.schema.json` and
no envelope `type` (the schema's `type` is the requirement type, `HLR` and the like). Every record
is validated against the vendored schema before anything is written. Records are written in id
order; a file whose YAML is unchanged is not touched, so a re-import with nothing new produces no
diff. Files of this source that the import no longer contains are reported (`not in source, kept`)
and kept. A directory holding records of the other source is refused before any write.

### ASSUMPTION: StrictDoc export layout

StrictDoc is not installed here and no export sample exists, so the JSON read by `strictdoc` is an
assumed shape, not StrictDoc's documented output:

```json
{ "documents": [ { "path": "docs/srs.sdoc", "requirements": [
    { "uid": "HLR-12", "level": "HLR", "title": "...", "statement": "...", "status": "approved",
      "parent_uids": ["SYS-4"], "safety_related": false, "security_related": true } ] } ] }
```

The fixture `crates/requirement-import/tests/fixtures/strictdoc_export.json` follows it. If a real
export differs, only `crates/requirement-import/src/strictdoc.rs` changes. Nothing in this
repository has run StrictDoc.

## Limits

* Everything Jira-side is verified against `jira_client::mock` (feature `mock`), a model of the
  Cloud and Data Center REST layouts written from their public documentation. No real Jira site
  was used. Field names such as `customfield_*` are the example ids of the configs.
* The safety and security flags have no key in the vendored `integration-config` schema, so they
  are given as command line flags, not config.
* Only the Jira source is read from a live system; the StrictDoc import reads a file.
* `tcms jira sync`, status write-back, test backends, milestones and defects are TCMS-42.4 to
  TCMS-42.9 and are not implemented here.

### Roll-up rules (TCMS-42.6)

The feature roll-up uses the mapping rules of `tcms-schemas` `docs/jira-integration.md`, section 2,
as implemented in `crates/test-backends/src/rollup.rs` (unit-tested rule by rule):

* **verification**: `not_started` when no requirement traces to the feature or no traced test
  case has a result; `failed` when a latest verdict failed or errored; `blocked` when no failure
  and a case is not executed on a finished run; `passed` when every requirement is covered
  (normal range and robustness), every latest verdict passed, and no problem report blocks it
  (TCMS-42.8); `in_progress` otherwise.
* **validation**: `failed` when a requirement is rejected; `passed` when all are validated;
  `not_started` when none has a value; `in_progress` otherwise.
* **coverage**: covered requirements over all requirements, rounded half up.
* Latest verdict of a case: the verdict of its last run record (by `started_at`); `not_executed`
  is `blocked` on a finished run, `in_progress` otherwise.

Each value is written to the configured `fields` and `status_values`; a feature without a traced
requirement is reported and not written.

### Feeding the roll-up

The `features` step builds the trace matrix from `--requirements-dir`, `--tests-dir` and
`--runs-dir` (`testcase_verification::build_matrix`), the latest verdicts from the run records,
and the open problem reports from `--problem-reports` (a report blocks the features it names in
`affected_items` while it is open or its linked issue is not done).

## Defects: `tcms pr push` and `tcms pr link` (TCMS-42.8)

A problem report (`problem-reports/<ID>.yaml`) becomes a Jira issue of `defect_issue_type`, or
is linked to one that exists.

* `tcms pr push <ID> [--test-entity KEY] [--dry-run] [--as USER]` creates the defect: summary
  `[<ID>] <title>`, the description (ADF on Cloud) with the report id, `raised_by` and the
  pushing user, `priority` from `severity_to_priority` (a severity without a mapping leaves the
  priority unset and the output says so), labels `tcms` and `tcms-pr-<ID>`, and a link of
  `tcms_issue_types.link_type` to each feature the report affects (its `affected_items` of type
  `requirement`) and to the given test entity. Without `link_type` the defect is created
  unlinked, with a note.
* Finding an existing defect (idempotence, TCMS-42.9): before it creates anything, push searches
  the first project of `project_keys` for issues with the label `tcms-pr-<ID>` (JQL
  `project = "<P>" AND labels = "tcms-pr-<ID>"`; an exact label match, a read that is not
  guarded). One issue found: nothing is created or linked, and the key is recorded in the report.
  Two or more: the push is refused and names the keys, so one is kept by hand. The search runs in
  dry runs too, so the plan says `existing <KEY>` or `would create`. A report id that cannot be a
  label (anything but letters, digits, `-` and `_`) is refused before any request. The label
  needs no configuration: `labels` is already an allowed field of a created issue.
  A report that already holds a key in `external_refs` is never searched.
* `tcms pr link <ID> <KEY>` links an existing issue. It only reads the issue (it must be in
  `project_keys`) and records its key. A report already linked to another key is not re-linked.
* The Jira key is written to the report's own `external_refs` (system `jira`). A report that has
  a key is never pushed again.
* Writes go through the write guard with the `defects` policy; with `defects: false` the push is
  refused before any request.

Reconciliation (AC #4): sync compares each linked report's status with its issue's status
category. A disagreement is printed as a difference. Nothing is changed in either system.

Blocking (AC #3): a feature is not `passed` while a problem report on it is open (any status
outside `fixed`, `verified`, `closed`, `rejected`; an unknown status counts as open) or its
linked Jira issue is not done.

## `tcms jira sync` (TCMS-42.9)

One command runs the write-backs of a product, for people and for the CI job below:

```
tcms --config integration-config.yaml jira sync --product PROD-001 [--apply] [--only import,tests,...] [--as USER]
```

Steps, in this order (`--only` runs a subset, in the same order):

| Step | What it does | Needs |
|------|--------------|-------|
| `import` | Requirements from the configured source into `--requirements-dir` (Jira features, or a StrictDoc export with `--strictdoc-input` and `--commit`) | the source |
| `tests` | Test entity per test case (found by `tcms_id`, created when missing), lifecycle status, latest verdict per case from `--runs-dir`, hand edits overwritten and logged | a Jira system |
| `features` | Roll-up per feature from the trace matrix (built from `--requirements-dir`, `--tests-dir`, `--runs-dir`), the latest verdicts and the open problem reports; field values, transitions (when configured) and a comment (when `write_policy.comments` is on) | a Jira system |
| `milestones` | Each record of `--test-plans` as a Fix Version (created once, its id kept in the state), progress in its description, release on completion when `release_on_plan_complete` is set | `milestone.target: fix_version` |
| `defects` | `tcms pr push` for each open report without a key; reconciliation for the others | a Jira system |

A step whose input is missing is skipped with a line in the output (for example, `milestones`
when the config has no `milestone` block). A product whose config has no Jira system runs
`import` only.

### Dry run and `--apply`

**Dry run is the default.** Without `--apply` the sync prints the plan (`would write: ...`) and
the differences, sends no write and writes no file (not the requirement records, not the state,
not the audit). It still reads from Jira, so the plan reflects the current state. The summary
ends with `dry run: nothing written; rerun with --apply`. `--dry-run` says the same explicitly
and cannot be combined with `--apply`.

### Acting user

The acting user is `--as`, else `$TCMS_USER`, else `$USER`. It is written into every audit entry
of the run (`who`) and into the body of every Jira comment the sync adds (the jira-client
appends "TCMS sync run <id> by <user>, service account <identity>"). The defect description
names the pushing user too.

### State

`<state-dir>/state.yaml` (default `.tcms/jira-sync/state.yaml`), written only with `--apply`:

* `test_cases.<id>`: the Jira key of the test entity (its `external_refs` entry) and the values
  TCMS last wrote to its configured fields. The next run compares these with what Jira holds;
  a difference is a hand edit, it is overwritten and logged;
* `plans.<id>.version_id`: the Fix Version of a test plan.

The state holds keys and status values only. It is product-scoped: a state file of another
product is refused. Its `schema` line names a local type (`jira_sync_state`); there is no
vendored schema for it, so it is not validated (limit).

Problem-report keys are not in the state: they are in the report records (`tcms pr`). The sync
also stores the key in the `external_refs` of the record it belongs to, with a targeted edit that
changes only that block (`crates/tcms/src/extrefs.rs`, decision D-42.4j): a test case gets its
entity of the backend's system (`jira`, `xray`, ...), a test plan its Fix Version (`jira`). The
edited record must validate against its schema before it is written. A record that holds the
key is not touched, so a re-sync leaves the files byte-identical. A sync without state finds the
entities through the records. Requirement records are not updated by the sync.

### Audit

With `--apply` and at least one change, each changed record gets one `audit-trail-entry`
(`action: sync`) in `<state-dir>/audit/<run-id>/<run-id>-NNN.yaml`: `who`, `when`, the entity
(`test-case`, `requirement`, `test-plan`, `problem-report`), `after` (the external ref and the
fields written) and `reason`. The entry for an overwritten hand edit has `before` and `after`.
These files are validated against the vendored `audit-trail-entry` schema (checked by hand).

### Write policy

Every write goes through the guard in `jira-client` (`check_write`), from the config alone:

* feature summary and description are never written (refused, tested);
* feature fields only with `write_policy.feature_fields` and fields listed in `fields`;
* transitions only with `feature_transitions` and a configured transition name;
* test entities and defects only with `dedicated_issues` and `defects`;
* comments only with `comments`; Fix Version writes only with `milestone.target: fix_version`;
* an issue outside `project_keys` is refused before any request.

A test case deleted in TCMS marks its entity `obsolete`; nothing is deleted in Jira.

### CI job

`.gitlab-ci.yml` has the hidden template `.tcms-jira-sync` and the job `tcms:jira-sync`. It
runs on every pipeline of the default branch (`$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH`), builds
`tcms` and runs `jira sync` as the user `TCMS_USER` or the GitLab login. The rule is the default
branch, not "a results merge" (D-42.9d): neither the repository nor GitLab marks a merge of test
results (no predefined variable, no results branch naming, and the other jobs key only on tags
and merge request ids), so no precise condition exists. A default-branch pipeline without result
changes runs the sync again: the same roll-ups are sent again (writes are not diffed), and
nothing is created twice. Only the default branch applies; every other branch is a dry run. The mode is chosen in the script (D-42.4n): `--apply` on the default branch, `--dry-run`
on any other branch, including a manual run there. The CI/CD variables are `TCMS_INTEGRATION_CONFIG`
(the config path in the repo), `TCMS_PRODUCT` (the product id; the job needs it), the token named
by `auth.secret_ref` in the config (`TCMS_JIRA_TOKEN` in the examples) and `TCMS_JIRA_USER`. Set
them as masked variables; the job file holds no secret. The audit directory is kept as an
artifact for 90 days. The job does not commit the records it updates (the keys written into the
test case and plan records stay in the job's working copy); a later run finds the same entities
through the `tcms_id` field and the state, so it creates no duplicate test entities. Defects are
found by their label `tcms-pr-<ID>` before a create, so a fresh checkout does not duplicate them
either (see Defects; the one exception is a defect made before the label existed: see Limits).

The job is checked statically by `crates/tcms/tests/ci_template.rs` (parses `.gitlab-ci.yml`,
asserts the rules, the mode choice, the variables and the referenced files). GitLab CI itself
was not run: the authoring environment cannot run it.

### Test backends (TCMS-42.4)

`test_backend` of the Jira system selects where the test entities and runs live. One sync step
(`tests`) runs for all four; the code behind it is the `TestBackend` trait of
`crates/test-backends`.

| `test_backend` | Test entity | Run (`record_run`) | Where the API is |
|----------------|-------------|--------------------|------------------|
| `none` (or absent) | TCMS-owned Test issue (`tcms_issue_types.test`), found by `tcms_id` | verdicts on the Test issues | Jira REST |
| `xray` | Test issue | one Test Execution per run (summary = run id), status per test | Jira site, `/rest/raven/1.0` |
| `zephyr_scale` | Zephyr test case, found by label `tcms:<id>` | one cycle per run (name = run id), one execution per case | `TCMS_ZEPHYR_API_URL` (default `https://api.zephyrscale.smartbear.com/v2`), bearer token from `auth.secret_ref` |
| `zephyr_squad` | Test issue | one cycle per run (name = run id), one execution per case | Jira site, `/rest/zapi/latest` |

* The key of each test case entity is stored in its record's `external_refs` with the system of
  the backend (`jira`, `xray`, `zephyr_scale`, `zephyr_squad`), by a targeted edit.
* A second sync of the same run reuses its cycle or execution and updates the executions in it
  (D-42.4m for Xray, D-42.4o for Zephyr Scale and Squad); a second sync creates no entity.
* Zephyr Scale writes no Jira links; features are not linked for that backend (D-42.4e).
* A Zephyr Scale cycle or Squad cycle is found by its name among the first page of the project's
  cycles. As for the Jira search, large projects need paging (not implemented).

### ASSUMPTION: Zephyr Scale and Zephyr Squad endpoints

The endpoint shapes below are from public documentation as remembered. They are NOT verified
against the real products; the jira-client mock models them (routes `/v2/...` and
`/rest/zapi/latest/...`) and the acceptance script runs against that mock.

* Zephyr Scale: `GET /testcases?projectKey=&query=labels = "tcms:<id>"`, `POST /testcases`,
  `GET|PUT /testcases/{key}` (`statusName`, `customFields`), `GET|POST /testcycles`
  (`query=name = "<run>"`), `GET /testexecutions?projectKey=&testCycleKey=`, `POST
  /testexecutions` (`testCaseKey`, `testCycleKey`, `statusName` Pass, Fail, Blocked, In Progress,
  Not Executed), `PUT /testexecutions/{id}`.
* Zephyr Squad: `GET /project/{key}` (the project id), `GET|POST /rest/zapi/latest/cycle`
  (`projectId`; the cycle listing is an object keyed by cycle id), `GET /rest/zapi/latest/execution?cycleId=`
  (`executions` list with `issueId`), `POST /rest/zapi/latest/execution` (`issueId`, `cycleId`,
  `status`: 1 pass, 2 fail, 3 work in progress, 4 blocked, -1 unexecuted), `PUT
  /rest/zapi/latest/execution/{id}/execute`.

The Zephyr Scale base URL is an environment setting (`TCMS_ZEPHYR_API_URL`), not a config field:
the integration config schema has none for it.

### Secrets

The token is read only from the environment variable named by `auth.secret_ref`. It is never
written to a YAML record, the state, the audit files or the output. The secret hygiene tests
(`crates/tcms/tests/sync_cli.rs`, `crates/jira-client/tests/secret_hygiene.rs`) check the state,
the audit files and the output of a sync run against the mock.

### Verified and not verified

* Verified against the jira-client mock (`MockJira`, Cloud layout): the dry run sends nothing;
  apply creates one test entity per case, writes lifecycle and verdict, finds the same entities
  on a second run, creates one defect per report, comments name the user; the state and audit
  files have no secret. A defect is found by its label on a fresh checkout (no state, no key in
  the report) and two applies from fresh checkouts create one defect (`tests/sync_cli.rs`).
* Verified against the mock through the `tcms` binary: `tests/sync_cli.rs` (plain Jira, Zephyr
  Scale and Zephyr Squad: the key lands in the record, a second apply creates nothing),
  `tests/pr_cli.rs`, and the acceptance script `test-acceptance/validate_jira_integration.sh`
  (products 1 to 5: plain Jira, StrictDoc, Xray, Zephyr Scale, Zephyr Squad; see below).
* NOT verified against a real Jira, Xray, Zephyr Scale or Zephyr Squad instance. Every endpoint
  shape (Xray `/rest/raven/1.0`, Zephyr Scale `/v2`, Zephyr Squad `/rest/zapi/latest`), the
  Jira link and issue payloads, and the StrictDoc export layout are ASSUMPTIONS from public
  documentation as remembered.

### Limits

* `validation_status` is read from a requirement record that has the field (requirement.schema);
  the Jira import does not set it, so a feature without it is `not_started` until a validation
  review writes the field.
* Writes are not diffed against Jira: an unchanged roll-up is sent again on every apply. Only
  test entities are found and not duplicated; `last_written` drives hand-edit detection only.
* The trace matrix is built from the test cases and run records in the directories given; the
  baseline is the `--baseline` value (a run is not matched to a baseline by its record).
* Requirement records are not rewritten by the sync. Test case and plan records get their key in
  `external_refs` by a targeted edit (`extrefs.rs`); a flow list in `external_refs` (not the block
  form) is refused with an error and left for hand editing. `url` is never stored in them.
* A test case removed from the product is marked obsolete only when the sync state holds its
  entity (the state lists the entities; a removed case is found through it). With no state, its
  entity is left as it was. The run records of a removed case still write their verdict on each
  apply (an existing rule, not changed by this).
* The state is a file in the repository or in the job's working copy. The CI job does not push
  it back: keep the state under version control (commit it after a sync) or accept that each
  run starts from an empty state (then the sync finds test entities by `tcms_id` and creates no
  duplicates, but hand-edit detection starts afresh).
* Jira searches follow every page (`search_issues`), so a large project costs more requests per
  run; Zephyr Scale and Squad listings are read as one page (no paging).
* Defects are found by label, not by a key kept on the machine. A defect created before the
  label `tcms-pr-<ID>` was set on creation has no label, so the lookup does not find it: before
  `write_policy.defects` is turned on for CI, add the label `tcms-pr-<ID>` to each such defect in
  Jira, or record its key with `tcms pr link <ID> <KEY>` and keep the report record. The lookup
  follows every page of results. Someone who removes the label, or who moves the issue out of the
  first project, gets a second defect on the next run.
* `milestone.target: epic` and `issue` are accepted by the config and refused at sync with a
  message: not implemented.
