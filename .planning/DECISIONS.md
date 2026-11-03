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

## D-05-1: Exit codes of `report`: clap parse errors exit 1, the --shape conflict stays exit 2
- **Phase / stage**: 5 / plan (05-02)
- **Question**: 05-02 says every usage error of `report` exits 1, but D-02-6-2 locks the `--shape a|b` with non-traceability kind conflict at exit 2, and ten earlier tests assert 2.
- **Options considered**: (1) all usage errors exit 1 (rewrite the --shape tests and amend D-02-6-2); (2) all usage errors stay 2 (violates ROADMAP SC1 and D-03 for bad input); (3) split: clap parse errors (missing --runs/--matrix, unknown option, invalid --kind/--format value) exit 1 per D-03 and SC1 (input errors); the semantic --shape conflict, raised inside `fn report`, keeps exit 2 per D-02-6-2.
- **Decision**: (3). ROADMAP SC1 names only input or validation errors (1) and --fail-on-gaps (2); it does not cover a --shape conflict, so D-02-6-2 stays true. The 05-02 must-have wording "usage" is narrowed to clap parse errors.
- **Code change**: in `fn report`, revert the `anyhow::bail!` to `clap::Error::raw(ErrorKind::ArgumentConflict, "...").exit()` (exit 2). `parse_cli` (try_parse, exit 1 for `report`) only wraps parse errors, so both stay distinct. Keep `Result<bool>` and the rest.
- **Test change**: tests asserting 2 for shape conflicts stay; tests asserting 2 for clap errors (missing --runs, unknown --kind, other clap cases) change to 1. `assert_usage_error` helpers: split by case.
- **Supersedes / affects**: 05-02 must-have 5 (narrowed); D-02-6-2 unchanged

## D-05-2: `report --kind summary --format json` is valid (exit 0), not a usage error
- **Phase / stage**: 5 / plan (05-03)
- **Question**: 05-03 listed `--kind summary --format json` among the "errors win" cases (exit 1), but the code exits 0 and 05-02 recorded it as valid.
- **Options considered**: (a) fix the plan criterion: summary JSON is a valid format since Phase 4 (D-04-2, plan 04-04), so it is not an error case; (b) make the code reject it, contradicting D-04-2.
- **Decision**: (a). ROADMAP SC1 reserves exit 1 for input or validation errors; a valid format is neither. Case removed from 05-03 section D; the E2E already asserts exit 0 (section B, test_report_exit_codes_e2e.sh lines 284-291). No production code change.
- **Supersedes / affects**: 05-03 plan task criterion D; D-04-5 and D-05-1 unchanged

- **ID**: D-07-7
- **Phase / stage**: 7 / execute (07-01)
- **Question**: Who edits the "assumed layout" help text in `crates/tcms/src/export_ids.rs:20` and `reqcmd.rs:22`, outside 07-01 `files_modified`?
- **Options considered**: (a) widen 07-01 to reword the two help strings now; (b) defer the wording and the grep criterion to 07-05.
- **Decision**: (b). 07-05 owns the "assumed" to "verified" move (D-07-6) and its own grep gate covers `crates` `*.rs`; its `files_modified` and action now list both files. 07-01 records the criterion as deferred. No source change. **Why**: wording only, no behaviour change, and one plan owns all such wording.
- **Supersedes / affects**: 07-01 summary criteria; 07-05 plan

- **ID**: D-07-8
- **Phase / stage**: 7 / execute (07-01 review)
- **Question**: `collect_items` in `crates/requirement-import/src/strictdoc.rs` hardcodes `safety_related` and `security_related` to false; should the reader take them from the export?
- **Options considered**: (a) read the flags from StrictDoc node fields; (b) keep false, document that StrictDoc exports are not read for these flags, and name the follow-up.
- **Decision**: (b). The checked-in real export (`crates/requirement-import/tests/fixtures/strictdoc_export.json`) and the 07-01 and 07-05 plans show no safety or security field, so (a) would need an invented field name. Keep `Some(false)`; the None-versus-Some(false) question stays out of scope (it changes YAML output). Fixer: add a module doc paragraph in strictdoc.rs and a comment at the two `false` lines saying the flags are not read from StrictDoc exports and are always false; add the same sentence to 07-01-SUMMARY.md; tests at lines 283-288 stay. Follow-up: read the flags only if a real export with them is added to the fixtures (custom StrictDoc fields), through a new plan.
- **Supersedes / affects**: 07-01 summary; 07-05 plan line 67 (consistent)

- **ID**: D-07-9
- **Phase / stage**: 7 / execute (07-05 gate)
- **Question**: The narrowed 07-05 grep gate still hits `crates/tcms/tests/requirements_locate_strictdoc_e2e.rs:442`; residue or correct?
- **Options considered**: (a) remove or reword the fixture; (b) amend the gate's exclusion list to name the fixture.
- **Decision**: (b). Line 442 is the intentional old lowercase `{"documents":[{"path"...` fixture that tests refusal of the old layout (D-07-3); the hit is correct behaviour, not residue. The criterion now excludes `requirements_locate_strictdoc_e2e.rs:442:` (line-scoped, so new hits in that file still fail). Test and fixture unchanged. Command: `grep -rn 'assumed layout\|"documents":\[{"path"' crates --include="*.rs" --include="*.json" | grep -v '.../strictdoc.rs' | grep -v '.../requirements_locate_cli.rs' | grep -v 'requirements_locate_strictdoc_e2e.rs:442:' | wc -l` = 0. **Why**: the gate should check residue only; weakening the test would lose D-07-3 coverage.
- **Supersedes / affects**: 07-05 plan grep criterion (line 98); D-07-7 unchanged

- **ID**: D-08-1
- **Phase / stage**: 8 / spec (requirement 2.1)
- **Question**: What concrete exit code is `INVALID_INPUT_FILES`, and how is "0 (OK) / error code, except INVALID_INPUT_FILES" read?
- **Options considered**: (a) a new dedicated code (for example 3); (b) reuse 1, the existing input and validation error code.
- **Decision**: (b). `INVALID_INPUT_FILES` = 1, consistent with D-05-1 and ROADMAP Phase 5 SC1. Valid inputs exit 0, or another documented non-zero code other than 1 (for example 2); invalid inputs (fail JSON schema validation) always exit 1. Verified by an equivalence test over a valid set and an invalid set through the real binary.
- **Supersedes / affects**: 08-SPEC.md section 2.1 (TBD resolved); D-05-1 unchanged

## D-47-1: combined run record format (quick decision, 2026-11-03)
- **Phase / stage**: TCMS-47.3 / discuss
- **Question**: what format does the combined per-case run take?
- **Options considered**: (a) one run folder with test-run.yaml + verification-results.yaml; (b) keep 63 separate run folders; (c) a new container format
- **Decision**: (a), as in docs/RUN_RECORD_FORMAT.md. Schema-validated, one folder per run.
- **Why**: `tcms vv metrics` already reads that layout; the legacy Java path reads a folder as one run; (b) makes each case its own run; (c) adds a format nothing reads.
- **Supersedes / affects**: TCMS-47.3, TCMS-47.2 (consumer)

## D-47-2: verification-results.yaml schema (AC 2, plan 09-01)
- **Phase / stage**: 9 / execute (09-01 gate)
- **Question**: AC 2 needs a schema for verification-results.yaml; none exists in schemas/ or schemas/tcms-schemas/tcms/.
- **Options considered**: (a) add a minimal local schema and validate against it; (b) drop the schema requirement and validate in code.
- **Decision**: (a). Add `schemas/tcms/verification-results.schema.json` (root `type`, `schema`, `run_id`, and `results` keyed by test_case_id, each requiring integer `total_steps` and `passed_steps`; step results optional). Document it in docs/RUN_RECORD_FORMAT.md (Files section) and make the combinator validate against it before writing, replacing the field-only `check_steps`. Set the file's `schema:` header to this schema's URI.
- **Why**: keeps schema-validated output and the format doc as the single source of truth (D-47-1); code-only checks would hide the format.
- **Supersedes / affects**: 09-01 AC 2, open item Q3; 09-01-SUMMARY.md

## D-47-3: "missing run" verdict mapping (AC 4, plan 09-01)
- **Phase / stage**: 9 / execute (09-01 gate)
- **Question**: Must a case with no per-case file be emitted as not_executed?
- **Options considered**: (a) restate the mapping in docs/RUN_RECORD_FORMAT.md; (b) amend the criterion only.
- **Decision**: (a), which also amends the criterion. The combinator does not read the plan's case list, so a case with no per-case file is omitted. `not_executed` means a per-case file reporting no step executed. Change the Build rules line in RUN_RECORD_FORMAT.md to say exactly this, and reword AC 4 and the matching truth in 09-01-PLAN.md to "no execution recorded = not_executed; no per-case file = absent".
- **Why**: the script emits no missing-run marker; the doc stays the single source of truth.
- **Supersedes / affects**: 09-01 AC 4 and truth; open item on "missing run"
