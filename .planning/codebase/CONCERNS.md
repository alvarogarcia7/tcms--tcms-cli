# Codebase Concerns

**Analysis Date:** 2026-10-30

## Tech Debt

**Duplicated code between `testcase-manager` and the split-out layer-3 crates:**
- Issue: The monolith `crates/testcase-manager/src/` (about 13.5k lines) still holds copies of modules that were extracted into `testcase-ui`, `testcase-storage`, `testcase-validation`, `testcase-verification` and `testcase-execution`. Examples: `fuzzy.rs` exists in both `crates/testcase-manager/src/` and `crates/testcase-ui/src/`; `creator.rs`, `prompts.rs` and `validation.rs` likewise. `database.rs` exists in `crates/testcase-manager/src/` and `crates/testcase-storage/src/`, and `storage.rs` overlaps `crates/testcase-storage/src/lib.rs`. The copies have already diverged slightly.
- Files: `crates/testcase-manager/src/{fuzzy,creator,prompts,validation,database,storage,verification}.rs`, `crates/testcase-ui/src/`, `crates/testcase-storage/src/`
- Impact: Bug fixes land in only one copy. The layer rules in `AGENTS.md` are undermined.
- Fix approach: Make `testcase-manager` re-export from the layer-3 crates, delete the copies, and run `make test-from`.

**Unresolved TODOs and placeholders:**
- `crates/testcase-storage/src/lib.rs:540` and `crates/testcase-manager/src/storage.rs:524`: delete failures are not returned as errors.
- `crates/testcase-ui/src/fuzzy.rs:149` and `crates/testcase-manager/src/fuzzy.rs:149`: the requirement column is a hard-coded `"TODO AGB"`.
- `crates/testcase-storage/src/database.rs:13`: rename to `TestCaseDatabase`.
- `crates/testcase-ui/src/creator.rs:1306`: generated hook scripts contain a TODO stub.

**Repository clutter in the root:**
- Issue: There are roughly 110 tracked root-level markdown files (`IMPLEMENTATION_*.md`, `MANUAL_STEPS_*.md`, `*_SUMMARY.md`, `CODE_REVIEW_TCMS-8.md`). There are also stray artifacts: `errors.txt`, `cleanup.txt` (a terminal typescript), `convert_to_binary.py`, `create_files.py`, `create_control_chars.py`, `yaml_editor.py`, `rw.sh`, `proj`, and a root `src/lib.rs`.
- Impact: Docs are stale and contradict each other, which is hard for agents and humans to navigate. It also bloats context loading.
- Fix approach: Move the docs into `docs/` or `backlog/`, keep only current ones, and delete the throwaway scripts and logs.

**Oversized files:**
- `crates/testcase-execution/src/executor.rs` (5.1k lines), `crates/testcase-models/src/lib.rs` (3.1k), `crates/testcase-verification/src/verification.rs` (2.8k), `crates/testcase-ui/src/validation.rs` (1.9k), `crates/testcase-manager/src/prompts.rs` (1.8k) and `crates/testcase-execution/src/bdd_parser.rs` (1.8k).
- Fix approach: Split by responsibility (script generation, hooks, variable capture). Move the model types in `testcase-models/src/lib.rs` into submodules.

## Known Bugs

No confirmed bugs were found in static analysis. `crates/testcase-storage/src/lib.rs:540` swallows delete errors (see above).

## Security Considerations

**Shell command injection in generated scripts:**
- Risk: `crates/testcase-execution/src/executor.rs` (around lines 491-567 and 896) interpolates YAML-provided commands, hook paths and `resolved_command` directly into bash source via `format!`. Hook paths are embedded in double quotes without escaping, and a `"` or `$(...)` in a path or description would break out.
- Current mitigation: Test cases are trusted, author-written input, and schema validation runs first.
- Recommendations: Escape or single-quote interpolated values, and treat YAML from untrusted sources as code. Document this trust boundary.

**Environment mutation in tests:**
- Risk: `env::set_var` is used in about 28 places across tests. It is unsafe in multi-threaded tests and leaks state between them.
- Recommendations: Use serial test guards or pass configuration explicitly.

## Performance Bottlenecks

**Build and CI time:**
- The workspace has 30+ crates and 123k lines of Rust, with very large test files. sccache was measured slower in CI and deliberately not used (`docs/GITHUB_CI_CACHING.md`, comment in `.github/workflows`). `.cargo/config.toml` still points `SCCACHE_DIR` at a global directory, and `CLAUDE.md` still documents sccache as recommended, which is inconsistent.
- Improvement path: Reconcile the docs, and use `make test-from` for local iteration.

## Fragile Areas

**Bash script generation and JSON escaping:**
- Files: `crates/testcase-execution/src/executor.rs`, `crates/json-escape/`, `crates/testcase-manager/tests/json_escape*_test.rs`, `jq_json_escaping_test.rs`
- Why fragile: There are three escaping paths (jq, json-escape, shell sed fallback) with different edge-case behavior (null bytes, invalid UTF-8). The scripts must stay compatible with bash 3.2 and with BSD/GNU tools. Tests shell out to `bash` and `sh`, so results depend on the host tools.
- Safe modification: Run the E2E suite (`make test-e2e-all`) and `make verify-scripts` after any change, and test all three escaping methods.

**Crate layering enforcement:**
- Layer rules (`AGENTS.md`) are by convention only, with no tooling check. New crates (`audit-traceability`, `audit-verifier`, `id-resolver`, `jira-client`, `requirement-import`, `tcms`, `test-backends`) are not in the layer diagram or the E2E mapping.
- Safe modification: Check `cargo tree -p <crate> --invert` before adding dependencies. Update `mk/incremental.mk` for new crates.

## Scaling Limits

Not applicable. This is a local CLI toolchain, and no scaling limits were identified.

## Dependencies at Risk

**External `test-plan-documentation-generator`:**
- Risk: It is installed via `cargo install` or an env var and lives outside the repo. The acceptance suite and the docs stages fail without it.
- Impact: `make acceptance-test` (required before commit) cannot run on machines without it.
- Migration plan: Pin the version in CI and in `Dockerfile`.

**Python 3.14 and `uv`:**
- Risk: Python 3.14 is required for utility scripts, and CI and Docker must stay in sync (`pyproject.toml`, `uv.lock`).

## Missing Critical Features

Not detected beyond the TODO items above.

## Test Coverage Gaps

**Interactive UI code:**
- Files: `crates/testcase-ui/src/fuzzy.rs`, `prompts.rs`, and the editor binary entry point are excluded from coverage.
- Risk: Interactive regressions go unnoticed. Priority: Low.

**Panics in non-test code:**
- About 129 `.unwrap()` calls and 59 `expect(`/`panic!` calls appear in `crates/*/src` (not all in test modules). Replace them with `anyhow` context on user-input paths.
- Priority: Medium.

**Test suite weight:**
- Giant integration test files (`crates/testcase-manager/tests/executor_test.rs` at 7.6k lines, `verification_test.rs` at 3.6k, `json_escape_integration_test.rs` at 3.4k). A few `#[ignore]` tests exist (4).
- Priority: Medium. Split by feature and share fixtures.

---

*Concerns audit: 2026-10-30*
