# Coding Conventions

**Analysis Date:** 2026-10-30

## Naming Patterns

**Files:**
- Rust modules and files: `snake_case.rs` (`crates/testcase-common/src/yaml_loader.rs`, `crates/testcase-manager/src/dependency_resolver.rs`).
- Crates: `kebab-case` directories and package names under `crates/` (`crates/test-backends`, `crates/id-resolver`); library names become `snake_case` in code (`id_resolver`, `jira_client`).
- Binary entry points: `src/main.rs` (`crates/verifier/src/main.rs`, `crates/tcms/`); extra binaries under `src/bin/` (`crates/jira-client/src/bin`).
- Test files: `*_test.rs` (older crates, `crates/testcase-manager/tests/verification_test.rs`) or topic/`*_cli.rs` names (newer crates, `crates/tcms/tests/ref_cli.rs`).
- Shell scripts: `snake_case.sh` or `kebab-case.sh` in `scripts/`; shared logger in `scripts/lib/logger.sh`.
- Test-case YAML IDs: `UPPER_SNAKE` (`TC_HOOKS_001`, `TEST_SUCCESS_001`).

**Functions:** `snake_case`. Test helpers are short nouns/verbs (`tcms()`, `out()`, `target()`, `config()`, `client()`).

**Variables/constants:** `snake_case`; constants `SCREAMING_SNAKE_CASE` (`PROTECTED_FEATURE_FIELDS`, `CREATE_SYSTEM_FIELDS` in `crates/jira-client/src/guard.rs`).

**Types:** `PascalCase` structs/enums (`WriteKind`, `WriteRequest`, `TestCase`). Enum variants PascalCase; serialized with `#[serde(rename_all = "snake_case")]` or `"lowercase"`.

## Code Style

**Formatting:**
- `rustfmt` with default settings (no `rustfmt.toml`). `cargo fmt` via `make fmt`; CI runs `cargo fmt --all -- --check` (`.github/workflows/workspace.yml`).
- Edition 2021 (root `Cargo.toml`).

**Linting:**
- `cargo clippy -p <crate> --all-features --tests -- -D warnings` (`Makefile` `clippy` target, CI). Warnings are errors; no workspace `[lints]` table and no `clippy.toml`.
- Test-only helper modules use `#![allow(dead_code)]` (`crates/jira-client/tests/common/mod.rs`).
- Shell: must be bash 3.2+ and BSD/GNU portable (no `grep -P`, `sed -r`, `declare -A`); verify with `make verify-scripts`. Use `scripts/lib/logger.sh` (`log_info`, `log_warning`, `log_error`, `pass`, `fail`, `section`).
- Python: `ruff` and `mypy` (`pyproject.toml`), Python 3.14 via `uv`.

## Import Organization

**Order (rustfmt-sorted, three blocks):**
1. `std::...`
2. External crates (`anyhow`, `serde_json`, `clap`)
3. Workspace crates / `crate::` / `super::` (`testcase_models`, `id_resolver`)

**Path Aliases:** none. Workspace crates are path dependencies (`testcase-models = { path = "crates/testcase-models" }`); shared third-party versions come from `[workspace.dependencies]` and are consumed as `dep = { workspace = true }`. Never pin a version in a crate manifest.

**Layering:** respect the five-layer rule in `CLAUDE.md`; lower layers never depend on higher ones. Document the layer in crate docs (`crates/id-resolver/src/lib.rs`: "Layer 3: depends on `testcase-models` and `testcase-common`").

## Error Handling

**Patterns:**
- Application and library code returns `anyhow::Result<T>`; `anyhow = "=1.0.99"` is pinned in the workspace. `thiserror` is not used.
- Add context with `.context(...)` / `.with_context(|| ...)` (667 uses across `crates/`). Use `bail!` for early validation failures (`crates/jira-client/src/guard.rs`).
- Domain-specific error enums exist only where callers branch on them (`ResolutionError` in `crates/testcase-manager/src/dependency_resolver.rs`, `DependencyErrorType` in `crates/testcase-validation/src/dependency_validator.rs`).
- Binaries use `fn main() -> Result<()>` (`crates/verifier/src/main.rs`); tests print the full chain with `format!("{:#}", err)`.
- Secrets must never appear in errors, logs or records; wrap with `jira_client::Secret` and assert in tests (`crates/jira-client/tests/secret_hygiene.rs`).
- Writes to external systems pass a guard before any request is built (`crates/jira-client/src/guard.rs`).

## Logging

**Framework:** `log` + `env_logger` (workspace deps). Binaries init it in `main`:
```rust
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();
```
- `--verbose` maps to `debug`; `--log-level` accepted by CLIs (`crates/verifier/src/main.rs`, `crates/validate-yaml`).
- User-facing output uses `println!`/`eprintln!` in binaries and interactive crates (`crates/editor/src/main.rs`, `crates/testcase-ui/src/prompts.rs`); diagnostics use `log::` macros.

## Comments

- Every crate and test file starts with a `//!` module doc stating purpose, ticket id and layer (`crates/test-backends/src/lib.rs`: "Test backend adapters (TCMS-42.4)").
- `///` on public items and constants. Reference ticket ids (`TCMS-40.4`) and acceptance criteria (`// AC1: ...`) in comments and test names.
- Inline comments explain intent, not mechanics.

## Function Design

- Small helpers plus `fn name(layout: Layout)` parameterised bodies, called from thin `#[test]` wrappers per variant (see `crates/jira-client/tests/mock_jira.rs`).
- Builders/constructors: `Type::new(...)`, `with_token(...)`. Return owned values or `Result`.
- Prefer `&str`/`&[&str]` params in helpers; `impl AsRef<Path>` for paths (`load_and_validate_yaml`).

## Module Design

- `lib.rs` declares modules and re-exports public API; single-crate-per-concern.
- Serde models in `crates/testcase-models/src/lib.rs`: `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`, optional fields use `#[serde(skip_serializing_if = "Option::is_none")]` (often with `default`).
- Feature `mock` gates mock implementations (`jira_client::mock`), enabled by dev-dependencies of consumers.

## YAML/Schema Conventions

- Every YAML file has root `type` and `schema` fields (e.g. `schema: tcms/test-case.schema.v1.json`); schemas live in `schemas/`.
- Load YAML only through `testcase_common::load_and_validate_yaml` or `parse_and_validate_yaml_string` (mandatory per `CLAUDE.md`).
- Generated bash scripts must be bash 3.2+ compatible; JSON escaping order is jq, then `json-escape`, then shell fallback.

## Process Conventions

- Pre-commit order: `make build`, `make lint`, `make test`, `make coverage-e2e`, plus `make acceptance-test`.
- Task tracking uses Backlog.md (`backlog/`); commit messages reference ticket ids (`TCMS-40, TCMS-41, TCMS-42: ...`).

---

*Convention analysis: 2026-10-30*
