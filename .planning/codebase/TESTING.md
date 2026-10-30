# Testing Patterns

**Analysis Date:** 2026-10-30

## Test Framework

**Runner:**
- Built-in Rust `cargo test` (libtest). No external runner, no `rstest`, `proptest`, `mockito`, `wiremock`, `assert_cmd` or `serial_test` in workspace manifests.
- Config: none beyond `Cargo.toml`; features enabled with `--all-features`.

**Assertion Library:** std `assert!`, `assert_eq!`, `assert_ne!` with a trailing message (`"{layout:?}"`, `"{s}"`) to show context on failure.

**Supporting crates:** `tempfile` (workspace dep) for temp dirs; `serde_json::json!` for payloads; shell E2E tests use `jq`, `shellcheck`, `expect`.

**Run Commands:**
```bash
cargo test --all-features                  # Run all tests (required before completing any task)
cargo test -p <crate> --all-features       # One crate
cargo test -p testcase-validation test_schema_validation   # One test
make test                                  # unit + E2E + verify-testcases + strictdoc-validate
make test-from BASE_REF=main               # Only affected crates + mapped E2E scripts
make test-all                              # Clean full build + all tests + test-e2e-all
make coverage                              # Unit coverage, 50% threshold (cargo-llvm-cov)
make coverage-e2e                          # Unit + E2E, 70% threshold
make coverage-html                         # HTML report
make acceptance-test                       # 7-stage acceptance suite
make test-e2e-acceptance                   # Tests for the acceptance orchestrator itself
```

## Test File Organization

**Location:**
- Unit tests: co-located `#[cfg(test)] mod tests { use super::*; ... }` at the bottom of source files (about 61 files, e.g. `crates/testcase-common/src/yaml_loader.rs`, `envelope.rs`, `yaml_output.rs`).
- Integration tests: per-crate `crates/<crate>/tests/*.rs`; each file is its own test binary.
- Shared helpers: `crates/<crate>/tests/common/mod.rs` imported with `mod common; use common::*;` (`crates/jira-client/tests/common/mod.rs`).
- Fixtures: `crates/<crate>/tests/fixtures/`, `tests/sample/`, `tests/integration/` (shell E2E scripts in `crates/testcase-manager/tests/integration/`), repo-level `testcases/`, `examples/integration-config/`, `schemas/`.
- Acceptance suite: `test-acceptance/` (`run_acceptance_suite.sh`, `00_test_cases`, `20_verification`, `tests/`).

**Naming:**
- Integration files: `<topic>_test.rs` (`crates/testcase-manager/tests/hooks_test.rs`) or `<topic>_cli.rs` / `<topic>.rs` (`crates/tcms/tests/sync_cli.rs`, `crates/id-resolver/tests/resolve.rs`).
- Test functions: descriptive snake_case sentences (`wrong_token_is_an_error_without_the_token_cloud`, `unknown_id_fails_with_clear_message`); older code uses a `test_` prefix (`test_resolve_schema_from_yaml_payload`).

## Test Structure

**Suite Organization (variant fan-out, `crates/jira-client/tests/mock_jira.rs`):**
```rust
// AC1: connection test reports site, user and the projects in project_keys
#[test]
fn connection_reports_site_user_and_projects_cloud() { connection(Layout::Cloud); }

#[test]
fn connection_reports_site_user_and_projects_datacenter() { connection(Layout::DataCenter); }

fn connection(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    ...
    assert_eq!(report.version.as_deref(), Some("1000.0.0"), "{layout:?}");
}
```

**Patterns:**
- Arrange with helper constructors, act, then assert. Tests for acceptance criteria carry `// ACn:` comments.
- Table-driven tests use a `const CASES: &[(&str, &str, &str, &str)]` iterated in one test (`crates/id-resolver/tests/resolve.rs`).
- Unit tests create `TempDir::new().unwrap()` and drop it on scope exit (no manual teardown).
- Use `.unwrap()`/`.expect("message")` in tests; `unwrap_or_else(|e| panic!("{name}: {e:#}"))` to show the full error chain.
- Inline YAML/JSON fixtures as `const` raw strings (`TEST_CASE`, `LOG` in `crates/verifier/tests/default_test_run_test.rs`).

## Mocking

**Framework:** hand-written in-process mocks, no mocking crate.

**Patterns:**
- `jira_client::mock::MockJira` (feature `mock`) is a local HTTP server with `Layout::Cloud` / `Layout::DataCenter`; expose `base_url()`, `requests()`, `write_requests()`. Tests point the real client at it (`crates/jira-client/tests/common/mod.rs`, `crates/test-backends/tests/*.rs`, `crates/tcms/tests/*`).
- Enable the feature in consumers: `jira-client = { path = "../jira-client", features = ["mock"] }` under `[dev-dependencies]`.
- Assert the mock saw no writes when the operation must only read: `assert!(mock.write_requests().is_empty())`.
- CLI tests spawn the real binary with `Command::new(env!("CARGO_BIN_EXE_tcms"))`, build args from `env!("CARGO_MANIFEST_DIR")/../..`, and assert on `status`, stdout, stderr (`crates/tcms/tests/ref_cli.rs`, `crates/test-executor/tests/*_cli.rs`). Remove ambient env with `.env_remove(...)`.

**What to Mock:** external services (Jira Cloud/Data Center); no real Jira is ever contacted.

**What NOT to Mock:** schema validation, YAML parsing, the filesystem (use `tempfile`), bash script generation.

## Fixtures and Factories

**Test Data:**
```rust
pub const TOKEN: &str = "tcms-test-token-4b1d9e";
pub fn schemas_root() -> String { format!("{}/../../schemas/", env!("CARGO_MANIFEST_DIR")) }
pub struct Policy { pub feature_fields: bool, ... }   // impl Default = all enabled
pub fn config_yaml(layout, base_url, project_keys, p: Policy) -> String
```
- Factory helpers return config YAML strings built with `format!`; `Policy::default()` plus struct-update overrides vary behaviour.

**Location:** `crates/<crate>/tests/common/mod.rs`, `tests/fixtures/`, `examples/integration-config/*.yaml`, `testcases/**` (including `testcases/verifier-scenarios/` and `testcases/examples/`).

## Coverage

**Requirements:** 50% line (unit, `make coverage`), 70% line (unit + e2e, `make coverage-e2e`). Excluded: `fuzzy.rs`, `prompts.rs`, `main_editor.rs`. The CI coverage step is currently commented out in `.github/workflows/workspace.yml` (`--fail-under-lines 63`). Coverage must not decrease.

**View Coverage:**
```bash
make coverage-html        # interactive HTML
make coverage-report      # terminal summary
make generate-docs-coverage   # tarpaulin coverage of documentation generation
```

## Test Types

**Unit Tests:** in-file `mod tests` for pure logic (YAML loading, envelope/schema resolution, guards).

**Integration Tests:** `crates/*/tests/*.rs` using real schemas from `schemas/`, mock Jira and spawned binaries (about 80 files; `crates/testcase-manager/tests/` is the largest).

**E2E Tests:** bash scripts under `crates/testcase-manager/tests/integration/` (executor, json-escape, pipeline, BDD), mapped to crates in `mk/incremental.mk`; `make test-e2e-pipeline` and `make test-e2e-pipeline-batch` run the 5-stage pipeline; `make validate-output-schemas` checks sample outputs; `crates/audit-verifier/tests/run_all_tests.sh`. Shell tests source `scripts/lib/logger.sh` and must run non-interactively on bash 3.2+.

**Acceptance:** `test-acceptance/run_acceptance_suite.sh` runs validation, generation, execution, verification, container validation, per-test and consolidated docs (needs `test-plan-documentation-generator` or `TEST_PLAN_DOC_GEN`). Part of the `pre-commit` checks.

## Common Patterns

**Async Testing:** not used; tests are synchronous (`reqwest` with the `blocking` feature).

**Error Testing:**
```rust
let err = format!("{:#}", c.test_connection().unwrap_err());
assert!(err.contains("HTTP 401"), "{err}");
assert!(!err.contains(TOKEN), "{err}");   // secrets never leak
```
For CLIs: `assert!(!o.status.success())` then check stderr text.

**Ignored tests:** `#[ignore]` is rare (16 occurrences); keep tests runnable in CI.

## Adding Tests

- New library logic: add `#[cfg(test)] mod tests` in the same file; use `tempfile::TempDir`.
- New CLI behaviour: add `crates/<crate>/tests/<topic>_cli.rs` using `CARGO_BIN_EXE_<bin>`.
- New external-system behaviour: extend `MockJira` and test both `Layout::Cloud` and `Layout::DataCenter`.
- New binary crate: add its E2E mapping in `mk/incremental.mk`.
- New test-case YAML: include `type` and `schema`; check with `make validate-testcases-report`.

---

*Testing analysis: 2026-10-30*
