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

- **ID**: D-02-G1
- **Phase / stage**: 2 / plan (gap closure)
- **Question**: Which existing-plans option closes the 02-VERIFICATION gap (blank baseline_id or matrix id renders an empty cell instead of "not recorded"; SC5, OUT-04)?
- **Options considered**: add more plans; view existing; replan from scratch
- **Decision**: Add more plans: one small gap-closure plan (02-05) covering model.rs, render_markdown.rs, lib.rs and a CLI test. **Why**: 02-01..02-04 are executed and committed and must not be rewritten; the gap is small and localized; a new plan lets the verifier re-check the same gap items. TRC-03 stays deferred to Phase 7.
- **Supersedes / affects**: 02-VERIFICATION.md gap; adds 02-05

- **ID**: D-03-G1
- **Phase / stage**: 3 / execute (blocker)
- **Question**: How do `make test` and `make coverage-e2e` find `validate-yaml` (fail with `Failed to execute validate-yaml: NotFound`)? The shell sets CARGO_TARGET_DIR=$HOME/.cache/cargo-target (~/.zshrc:21) but tests look for `target/debug/validate-yaml` in the repo; about 1 GB to 2.9 GB free, so a full rebuild into ./target may not fit.
- **Options considered**: A. move the cargo-target dir to ./target (same filesystem, rename, reuses build) and run gates with `env -u CARGO_TARGET_DIR`; B. change tests to resolve the binary via CARGO_TARGET_DIR (touches test files of another phase); C. leave it and report
- **Decision**: A. Moved with `mv` only, nothing deleted. Gates run as `env -u CARGO_TARGET_DIR make test` and `env -u CARGO_TARGET_DIR make coverage-e2e`. **Why**: no code change, no rebuild, no extra disk. The acceptance-suite failures remain open and are not addressed by this decision.
- **Supersedes / affects**: Phase 3 gate runs; 03-05 verification

- **ID**: D-03-G2
- **Phase / stage**: 3 / execute (blocker)
- **Question**: What to do about `make test` failing in crates/testcase-manager/tests/integration/test_json_escape_e2e.sh (7 of 19 cases fail with `error: unexpected argument '--config' found`; the script runs `test-executor --config ... generate ...` at lines ~243, 343, 414, 568)?
- **Options considered**: A. add the `--config` flag to test-executor now; B. record as known pre-existing failure, change no code, continue Phase 3 gates, create a backlog task; C. stop the whole run
- **Decision**: B. Backlog task TCMS-44 created. **Why**: `--config` is absent from crates/test-executor/src/main.rs on this branch and on `main`, and `git log -S config` on that file finds no commit adding it, so the failure predates Phase 3 (not from 14a68f4..HEAD); Phase 3 does not use `--config`; adding it would change another crate's CLI outside phase scope. Note: the `make test` target masks this failure, it exits 0 even though the script reports failures, so a green exit code cannot be trusted for this script.
- **Supersedes / affects**: Phase 3 gate runs; 03-05 verification; TCMS-44

- **ID**: D-03-G3
- **Phase / stage**: 3 / execute (test fix)
- **Question**: `views_are_deterministic` in crates/test-executor/tests/report_cli.rs expects byte-equal output for summary, gaps and problems across two different `--generated-at` values, but the summary fails. How should the test change?
- **Options considered**: (a) assert the two outputs differ only on the "Generated" row; (b) drop the timestamp from the header (renderer change, contradicts OUT-04); (c) drop the cross-timestamp comparison and keep only same-args equality
- **Decision**: (a). **Why**: summary.rs, gaps.rs and problems.rs all render a `| Generated | <ts> |` header row (summary.rs:368, gaps.rs:530, problems.rs:397), so all three kinds differ on it. OUT-04 requires the generation time in the header and OUT-03 requires determinism only for the same inputs and the same `--generated-at`. (b) breaks OUT-04. (c) loses the check that nothing else depends on the timestamp. Rule: same args give byte-equal output; different `--generated-at` gives output that is identical after removing the line starting `| Generated |`, and the full outputs must differ (that row holds each timestamp).
- **Supersedes / affects**: report_cli.rs `views_are_deterministic`; 03-05 verification
