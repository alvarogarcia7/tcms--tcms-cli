# Phase 7: E2E tests for locating requirements by ID in StrictDoc - Context

**Gathered:** 2026-10-30
**Status:** Ready for planning
**Mode:** Decided by Sonnet decision agent
**Source:** Grey areas settled by the decision agent from ROADMAP.md, the five Phase 7 plans and the Phase 6 context (no interactive discuss-phase run). Entries D-07-1 to D-07-6 in `.planning/DECISIONS.md`.

<domain>
## Phase Boundary

The real `tcms` binary is tested end to end on JSON exports written by the real StrictDoc tool:
`tcms requirements locate` finds HLR, LLR and `system` items by ID, reports missing IDs and exits
0/1/2; `tcms requirements export-ids` writes the valid IDs. The StrictDoc reader in
`requirement-import` first learns the layout StrictDoc really writes. No new command, no new
option, no Jira change.

</domain>

<decisions>
## Implementation Decisions

- **D-01 (install and pin):** StrictDoc is run only through `uv run strictdoc`, which uses the
  version locked in `uv.lock` (0.20.0; `pyproject.toml` says `>=0.20.0`). No `pip install`, no
  global install, no second pin. `strictdoc` is not on PATH and need not be. The note that 0.30.4
  gives byte-identical output stays in the fixture README only.
- **D-02 (when regeneration runs):** On demand only. `make strictdoc-regen-test-fixtures` writes the
  exports; `make strictdoc-check-test-fixtures` (`regenerate.sh --check`) detects drift. Neither is
  part of `make test`, `cargo test` or `.gitlab-ci.yml` in this phase (needs uv and network on a
  cold cache). The exports are checked in.
- **D-03 (reader scope):** The reader accepts only the real layout: `DOCUMENTS` / `NODES`,
  REQUIREMENT nodes, uppercase fields, path = document `TITLE`, `Parent` relations, a missing
  `LEVEL` refused. The old lowercase layout (`documents`/`path`/`requirements`/`uid`/`level`) is
  dropped with no compatibility reader; it is refused with a message naming `DOCUMENTS`. Public
  functions of `strictdoc.rs` keep their signatures. Every test and fixture of the old layout
  (requirement-import, test-executor, tcms) moves to the real layout in 07-01.
- **D-04 (gating when StrictDoc is missing):** The e2e tests read the checked-in real exports and
  never call StrictDoc, Python or uv, so there is nothing to skip: they always run and fail on a
  real failure. No `#[ignore]`, no skip-on-missing-tool. A missing or unreadable fixture is a hard
  test failure. Only `regenerate.sh` needs uv: without uv it logs an error and exits 2.
- **D-05 (fixture hand-editing):** Exports are never written or edited by hand. If uv cannot
  install StrictDoc (offline), execution stops and reports a blocker rather than fabricating JSON.
- **D-06 (docs and gates):** 07-05 changes the docs from "assumed" to "verified against StrictDoc
  0.20.0 output" and records the quality gates (build, lint, test, coverage, acceptance) in its
  summary. Phase 6 D-09 is superseded by this phase's reader fix.

</decisions>

<canonical_refs>
## Canonical References

### Planning
- `.planning/ROADMAP.md` - Phase 7 goal
- `.planning/DECISIONS.md` - D-07-1..D-07-6
- `.planning/phases/06-requirement-lookup-tcms-requirements-locate-verifies-that-re/06-CONTEXT.md` - D-05, D-06, D-09 (superseded)
- `.planning/phases/07-e2e-tests-for-locating-requirements-by-id-in-strictdoc/07-01..07-05-PLAN.md`

### Code
- `crates/requirement-import/src/strictdoc.rs` - reader to fix
- `crates/tcms/tests/requirements_cli.rs`, `crates/tcms/src/locate.rs` - callers and tests on the old layout
- `crates/test-executor/tests/trace_matrix_cli.rs`, `crates/test-executor/src/main.rs` - also read StrictDoc exports
- `mk/strictdoc.mk`, `pyproject.toml`, `uv.lock` - StrictDoc through uv

### Docs
- `docs/JIRA_INTEGRATION.md` - export layout section to update in 07-05

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `mk/strictdoc.mk` already runs `uv run strictdoc` (`make strictdoc-validate` in CI).
- `scripts/lib/logger.sh` for the regenerate script.
- Phase 6 e2e helpers (config writer, binary runner) are copied per test file.

### Established Patterns
- Tests run the real binary via `CARGO_BIN_EXE_tcms`; each test file carries its own helpers.
- Fixtures live under `crates/<crate>/tests/fixtures/`.

### Integration Points
- New fixtures in `crates/tcms/tests/fixtures/strictdoc/{locate,refused-level,no-level}`.
- New test files `requirements_locate_strictdoc_e2e.rs` and `requirements_export_ids_strictdoc_e2e.rs`.

</code_context>

<specifics>
## Specific Ideas

- Expected line: `HLR-1: found in strictdoc (strictdoc) at level HLR "Update profile metadata" [approved] https://pages.example.com/srs/software-requirements.html#HLR-1`
- `locate` export holds SYS-1 (system), HLR-1, LLR-1 (Parent HLR-1); `refused-level` holds HLR-8 (level `chapter`, exit 2); `no-level` holds HLR-9 (no LEVEL, exit 2).

</specifics>

<deferred>
## Deferred Ideas

- Running the fixture drift check in CI (needs uv and network; revisit if exports drift in practice).
- Backward-compatible reading of the old lowercase layout.
- Testing against several StrictDoc versions.
- Reading a StrictDoc server or HTML URL directly.

</deferred>
