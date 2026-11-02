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

- **ID**: D-02-G4
- **Phase / stage**: 2 / plan (new requirement: tcms-43.2 AC#7-8, tcms-43.4 AC#5)
- **Question**: Where does the new traceability report shapes work go (shape A: REQUIREMENT | test cases (ID, status) | final verdict; shape B: REQUIREMENT | final verdict plus TESTCASE ID | status rows; both validated against the local report schema before writing)?
- **Options considered**: A. new plan inside Phase 3 (before verification); B. follow-up plan on Phase 2 (renderers) with schema validation carried by Phase 4; C. new phase after Phase 5
- **Decision**: B. Add plan 02-06 to Phase 2: shapes A and B as new Markdown renderers over the existing ReportModel (verdicts from the existing roll-up, no model change). Phase 4 (04-02 schema, 04-03 render, 04-04 CLI) carries validation: the local report schema must describe both shapes and the traceability report is validated before write. **Why**: the traceability report is owned by Phase 2 (TRC-*, ROLL-*); Phase 3 covers summary, gaps and problems and is in gate verification, so adding there would reopen it; Phase 4 already owns validate-before-write (OUT-02) and the local schema; a new phase after Phase 5 would ship the shapes after exit-code/E2E work and split one report across phases. Same pattern as D-02-G1 (gap plan added to an executed phase, earlier plans untouched).
- **Supersedes / affects**: adds 02-06; adds shape coverage to 04-02..04-04; tcms-43.2, tcms-43.4; Phase 3 verification unaffected

- **ID**: D-02-6-1
- **Phase / stage**: 2 / plan (02-06)
- **Question**: How do D-04 (coverage columns) and D-08 (fixed type columns) apply to shapes A and B?
- **Options considered**: A. grid-only, shapes define own columns; B. add coverage and type columns to shapes; C. drop document headings in shapes
- **Decision**: A. D-04 and D-08 apply to the default grid only. Shapes A and B keep D-07 document headings, D-14 Inputs section and the "No traceable content" rule. **Why**: tcms-43.2 AC#7/#8 name the exact columns; extra columns would contradict them; headings and the empty rule are structure, not columns, and keep output consistent with the grid.
- **Supersedes / affects**: narrows D-04 and D-08 scope; 02-06

- **ID**: D-02-6-2
- **Phase / stage**: 2 / plan (02-06)
- **Question**: CLI surface for selecting the shape?
- **Options considered**: `--shape grid|a|b` on `report`; separate kinds; separate subcommand
- **Decision**: `--shape grid|a|b`, default `grid`; `a` or `b` rejected (usage error, exit 2) unless `--kind traceability` is the only kind. **Why**: shapes are views of one kind over the same ReportModel; default output stays unchanged.
- **Supersedes / affects**: 02-06; Phase 4 CLI (04-04) extends the same option

- **ID**: D-02-6-3
- **Phase / stage**: 2 / plan (02-06)
- **Question**: What do heading and per-test-case rows show?
- **Options considered**: keep AC#8 wording literally; add roll-up to case rows
- **Decision**: Keep AC#7/#8 wording unchanged. Shape B requirement row = REQUIREMENT | final verdict (existing roll-up); each test-case row = TESTCASE ID | that case's own status (pass/fail/error/NOT RUN), not the requirement verdict. Shape A: REQUIREMENT | test cases as "ID (status)" | final verdict. **Why**: matches the user's words; the final verdict is on the heading row, per-case status on case rows.
- **Supersedes / affects**: 02-06; tcms-43.2 AC#7, #8

- **ID**: D-02-6-4
- **Phase / stage**: 2 / plan (02-06)
- **Question**: Reject `--shape a|b` combined with grid-only options?
- **Options considered**: reject; silently ignore
- **Decision**: Reject: any option that only affects the grid (coverage or type-column flags, if present) with `--shape a|b` is a usage error naming both options. If none exist today, add the guard test so future flags inherit it. **Why**: silent ignoring hides a user's intent.
- **Supersedes / affects**: 02-06 CLI tests

## D-44-1: tcms-44 test-executor `--config` for json-escape E2E
- **Phase / stage**: backlog tcms-44 / triage
- **Question**: Is the script or the binary wrong about `--config`?
- **Options considered**: A. change script only; B. add `--config` to test-executor; C. stop and report
- **Decision**: B. Add `--config <TOML>` to test-executor as a `global = true` arg on `Cli` (crates/test-executor/src/main.rs), so both `test-executor --config f generate ...` and `generate ... --config f` (the AGENTS.md form) work. When given, load `testcase_common::Config` from that file (TOML, `[script_generation.json_escaping]`, methods `Auto|Jq|RustBinary|ShellFallback`, optional `binary_path`) and build the executor with `TestExecutor::with_config(config)` (keep `.with_run_id`) in Generate and Execute. Without it, behaviour is unchanged. **Why**: AGENTS.md documents the flag and docs/JSON_ESCAPING_CONFIG.md documents the section; the library already has `with_config` and the escaping methods (executor.rs ~309-343); only the CLI wiring is missing. Not A: the script's intent is valid.
- **Script fixes also required**: the script passes `--output-dir "$TEMP_DIR"`, which also does not exist on generate (it has `-o FILE`). Replace with `generate "$TEST_YAML" -o "$TEMP_DIR/<TC_ID>_test.sh"` (lines ~244, 344, 415, 569) or add an equivalent flag; the script expects `$TEMP_DIR/TEST_RUST_BINARY_test.sh`. The docs list lowercase method names ("rust_binary") while the script and code use `RustBinary`; check serde rename in config.rs and align docs. Also make `make test` propagate the script exit status (second half of tcms-44).
- **container_config.yml error**: unrelated to this flag (verifier `--config` for run_verifier_and_generate_reports.sh; default container_config.yml missing at repo root). Leave to a separate task; do not fold in.
- **Supersedes / affects**: D-03-G2; tcms-44

## D-04-1: Branch for Phase 4 (AsciiDoc and machine-readable output)
- **Phase / stage**: 4 / pre-execution
- **Question**: Which branch carries Phase 4 work?
- **Options considered**: A. `dev` (tracks gitlab/dev, lacks Phase 3, diverged from the feature branch at a282978); B. `traceability-matrix-report` (tracks origin on GitHub, contains Phases 1-3 and their verification records); C. new branch off either (not needed)
- **Decision**: B. Develop Phase 4 on `traceability-matrix-report`. **Why**: it holds the verified Phase 3 code and is the branch already pushed to GitHub, so Phase 4 commits fast-forward origin with no history rewrite. `dev` would need a merge or rebase to get Phase 3.
- **Supersedes / affects**: .planning/phases/04-*

## D-04-2: Phase 4 branch is dev (supersedes D-04-1)
- **Phase / stage**: 4 / pre-execution
- **Question**: Which branch carries Phase 4 work now?
- **Decision**: Phase 4 is developed on `dev`, not `traceability-matrix-report`. `dev` now contains the same code plus later CI commits. **Why**: the work is already on `dev`; correcting the record avoids a wrong branch being used.
- **Supersedes / affects**: D-04-1; .planning/phases/04-*

## D-04-3: AsciiDoc Inputs section follows the Markdown renderers
- **Phase / stage**: 4 / plan (04-01)
- **Question**: The plan gives AsciiDoc an Inputs section on every document, but the Markdown summary, gaps and problems views print none. Which side moves?
- **Options considered**: (a) AsciiDoc matches Markdown per renderer; (b) AsciiDoc always has Inputs, Markdown views stay as they are
- **Decision**: (a). AsciiDoc grid and shapes A/B keep the Inputs section (Markdown grid and shapes print it via `inputs_block`). AsciiDoc summary, gaps and problems have no Inputs section, like their Markdown views. Markdown output is not touched, so it stays byte-identical. **Why**: OUT-01 and the plan's own rule are "same sections, columns and fixed words as Markdown"; (b) would make the two formats differ and the Markdown determinism tests stay valid either way.
- **Plan changes required (04-01-PLAN.md, not edited)**: (1) must_haves line 18: drop "and with the Inputs section"; (2) line 21: change "every AsciiDoc document" to "the AsciiDoc grid and shapes A/B documents"; (3) line 60: replace the "Mandatory Inputs" paragraph with "AsciiDoc views have no Inputs section, matching Markdown"; (4) Task 2: remove the `inputs: &InputsSection` parameter from `render_asciidoc_summary`, `_gaps`, `_problems`, drop Inputs from their frames, goldens and tests, and from the SUMMARY note; (5) other plans (04-02..04) that call these three functions must stop passing inputs.
- **Supersedes / affects**: 04-01 Task 2; D-14 reading

## D-04-4: One-pipe cell check exempts the table header row
- **Phase / stage**: 4 / plan (04-01 Task 3)
- **Question**: Task 3 requires the literal header line `| TESTCASE ID | Status |` and also exactly one pipe per cell line. Both cannot hold for the header row.
- **Options considered**: (a) exempt the header row (first cell line after the `options="header"` attribute line) from the one-pipe check; (b) drop the literal header requirement
- **Decision**: (a). Body cell lines keep the exactly-one-pipe check, so value escaping stays tested; the header row is asserted by its literal. **Why**: header text is fixed words, not input values, so pipes there carry no injection risk, and the literal keeps parity with Markdown.
- **Supersedes / affects**: 04-01 Task 3 acceptance criteria (lines 227, 263)

## D-04-5: Input summary goes to stderr for non-Markdown formats
- **Phase / stage**: 4 / plan (04-04)
- **Question**: With `--format asciidoc` (also json/yaml), does the reconciliation (input summary) text go to stdout with the document?
- **Options considered**: (a) stdout carries the document only; summary goes to stderr; (b) stdout carries summary and document for every format
- **Decision**: (a). Current code in `report` (crates/test-executor/src/main.rs, `format != Markdown` branch) already does `eprint!(input_summary)` and prints only the rendered text to stdout. Markdown path is untouched (its summary stays inside stdout/file as today, byte-identical). The AsciiDoc Inputs section (grid and shapes, per D-04-3) is part of the document and does not contain the "Reconciliation" heading. **Why**: (b) would break JSON/YAML parseability on stdout and change Markdown behaviour; (a) needs no code change.
- **Plan/test change required**: none. The existing `asciidoc_all_kinds` assertions (no "Reconciliation" on stdout, present on stderr) already match (a); do not add a stdout summary assertion.
- **Supersedes / affects**: 04-04 report_formats_cli.rs asciidoc_all_kinds

## D-04-6: Traceability shapes A and B are validated before write (fix now)
- **Phase / stage**: 4 / verification gap (tcms-43.4 AC #5)
- **Question**: Shapes A/B are Markdown only (main.rs:769-773), so no shape reaches validate_document. Fix now or defer?
- **Decision**: Fix now (option 1); the user required it. **Why**: deferring leaves a stated requirement unmet.
- **Change required**: (1) schemas/tcms/report.schema.v1.json: add shape A and shape B content variants for kind traceability (closed objects, if/then like the other kinds); (2) crates/testcase-reporting/src/render_data.rs: build a shape-aware document from the same model and run validate_document in render_json/render_yaml; (3) crates/test-executor/src/main.rs: drop the "--shape a|b requires --format markdown" guard for json/yaml and, for Markdown shapes, also build and validate the document before any fs::write. Add E2E tests for both shapes (valid, and failure writes no file). Needs a new plan (04-05) or gap-closure plan.
- **Supersedes / affects**: tcms-43.4 AC #5; 04-VERIFICATION gap 2

## D-04-7: Keep the envelope enum change ("report")
- **Phase / stage**: 4 / verification gap (tcms-43.4.2 AC #1)
- **Question**: Commit 57da2e7 added "report" to the type enum of the shared schemas/tcms-envelope.schema.json. Keep or revert?
- **Decision**: Keep, and amend tcms-43.4.2 AC #1 to "shared schemas unchanged except the additive `report` type value". **Why**: report.schema.v1.json $refs the envelope, which requires type to be in its enum, so a local report type cannot validate otherwise. The change only widens the enum (one added value): every document valid before stays valid, and the other users (testcase-common tcms_schemas tests, yaml_loader, scripts/validate_envelope_schemas.sh) are unaffected. Reverting would need a duplicated local envelope. Record the value in schemas/CHANGELOG.md.
- **Supersedes / affects**: tcms-43.4.2 AC #1; ROADMAP Phase 4 note

## D-04-8: mk/incremental.mk mapping is not applicable
- **Phase / stage**: 4 / verification gap (tcms-43.4.3 AC #2)
- **Decision**: Accept as not applicable. report_formats_e2e.rs is a Rust integration test run by `cargo test -p test-executor`, which test-from already runs for the affected crate; mk/incremental.mk maps shell scripts only, and the bash 3.2/BSD/GNU part has no shell code to check. Record this in the tcms-43.4.3 AC #2 note and mark it done.
- **Supersedes / affects**: tcms-43.4.3 AC #2

## D-04-9: Record the E2E results in a phase SUMMARY
- **Phase / stage**: 4 / verification gap (tcms-43.4.3 AC #3)
- **Decision**: Write 04-E2E-SUMMARY.md (12 tests, commit ca70196, all passed). Left uncommitted for the user's commit.
- **Supersedes / affects**: tcms-43.4.3 AC #3
