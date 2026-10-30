# Decisions Log

Decisions taken during autonomous GSD execution. Each entry is taken by a Sonnet decision agent (or recorded from a user answer) when a grey area, a blocker workaround, or a scope call needs a choice. Entries are append-only; a later decision supersedes an earlier one by referencing it.

Entry format:

- **ID**: D-<phase>-<n>
- **Phase / stage**: discuss, plan, execute, review, verify
- **Question**
- **Options considered**
- **Decision** and **Why**
- **Supersedes / affects**

---


- **ID**: D-07-1
- **Phase / stage**: 7 / discuss
- **Question**: How is StrictDoc installed and pinned for fixture regeneration?
- **Options considered**: global pip install; second pin in a script; `uv run` with the repo lock
- **Decision**: `uv run strictdoc` with the version in `uv.lock` (0.20.0). **Why**: already used by `mk/strictdoc.mk`; one pin, no global install.
- **Supersedes / affects**: 07-02

- **ID**: D-07-2
- **Phase / stage**: 7 / discuss
- **Question**: Does fixture regeneration run in CI?
- **Options considered**: CI drift check; on demand only
- **Decision**: On demand only (`make strictdoc-regen-test-fixtures`, `strictdoc-check-test-fixtures`); not in `make test`, `cargo test` or CI. **Why**: needs uv and network on a cold cache; exports are checked in.
- **Supersedes / affects**: 07-02

- **ID**: D-07-3
- **Phase / stage**: 7 / discuss
- **Question**: Exact scope of the reader fix?
- **Options considered**: read both layouts; real layout only
- **Decision**: Real layout only; the old lowercase layout is dropped and refused with a message naming `DOCUMENTS`; all old-layout tests and fixtures move in 07-01. **Why**: the old layout was an assumption that StrictDoc never writes; no legacy users.
- **Supersedes / affects**: Phase 6 D-09; 07-01

- **ID**: D-07-4
- **Phase / stage**: 7 / discuss
- **Question**: Skip or fail the e2e tests when strictdoc is missing?
- **Options considered**: skip with message; fail; not depend on strictdoc
- **Decision**: Tests never call StrictDoc (they read checked-in real exports), so they always run; no `#[ignore]`, no skip. A missing fixture fails. Only `regenerate.sh` needs uv and exits 2 with an error without it. **Why**: a skipped test hides regressions.
- **Supersedes / affects**: 07-03, 07-04

- **ID**: D-07-5
- **Phase / stage**: 7 / discuss
- **Question**: What if StrictDoc cannot be installed (offline) during execution?
- **Options considered**: hand-write JSON; stop and report
- **Decision**: Stop and report a blocker; never write or edit exports by hand. **Why**: the phase goal is real-tool output.
- **Supersedes / affects**: 07-02

- **ID**: D-07-6
- **Phase / stage**: 7 / discuss
- **Question**: What closes the phase?
- **Options considered**: docs only; docs plus recorded gates
- **Decision**: 07-05 updates docs from "assumed" to "verified against StrictDoc 0.20.0 output" and records build, lint, test, coverage and acceptance results. **Why**: CLAUDE.md pre-commit workflow.
- **Supersedes / affects**: 07-05
