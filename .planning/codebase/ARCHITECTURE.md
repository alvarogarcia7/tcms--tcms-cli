<!-- refreshed: 2026-10-30 -->
# Architecture

**Analysis Date:** 2026-10-30

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ Binary crates (CLI microservices, clap-based)                            │
│ test-executor  verifier  test-verify  test-orchestrator  trm  editor     │
│ validate-yaml  validate-json  script-cleanup  json-escape  json-to-yaml  │
│ tpdg-compat  audit-verifier(+3 bins)  tcms  mock-jira                    │
│ `crates/<name>/src/main.rs`                                              │
└──────────────┬───────────────────────────────┬──────────────────────────┘
               ▼                               ▼
┌──────────────────────────────┐  ┌───────────────────────────────────────┐
│ Orchestration                │  │ Integration (TCMS-42 Jira/StrictDoc)  │
│ `crates/testcase-orchestration`│ │ `crates/test-backends`                │
│ `crates/testcase-manager`    │  │ `crates/requirement-import`           │
│ (facade, re-exports)         │  │ `crates/jira-client` `crates/id-resolver`│
└──────────────┬───────────────┘  └───────────────────┬───────────────────┘
               ▼                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Core: testcase-validation, -execution, -verification, -storage, -ui,    │
│ -git, -cli; audit-verifier, audit-traceability                           │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Foundation `crates/testcase-common` (YAML load+schema validate, config, │
│ envelope)                                                                │
└──────────────────────────────┬──────────────────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ Base `crates/testcase-models`, `crates/bash-eval`                        │
└─────────────────────────────────────────────────────────────────────────┘
 Data on disk: `testcases/*.yml` -> generated `.sh` -> execution log `.json`
 -> verification/container `.yaml` -> TPDG docs. Schemas: `schemas/`.
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Data models | `TestCase`, sequences, steps, hooks, `ExternalRef` | `crates/testcase-models/src/lib.rs` |
| Bash eval | Bash expression/variable evaluation | `crates/bash-eval/src/lib.rs` |
| YAML loader | Read, resolve `schema` URI, JSON-schema validate, deserialize | `crates/testcase-common/src/yaml_loader.rs` |
| Envelope | Schema resolution from `type`/`schema` fields | `crates/testcase-common/src/envelope.rs` |
| Config | TOML/env config (json_escaping etc.) | `crates/testcase-common/src/config.rs` |
| Executor | Bash script generation, hydration, BDD parsing | `crates/testcase-execution/src/executor.rs`, `hydration.rs`, `bdd_parser.rs` |
| Verification | Compare logs vs expected; coverage, trace matrix, regression, independence | `crates/testcase-verification/src/verification.rs`, `coverage.rs`, `trace_matrix.rs`, `regression.rs`, `independence.rs` |
| Storage | YAML/DB persistence, test runs, recovery | `crates/testcase-storage/src/` (`parser.rs`, `database.rs`, `test_run_storage.rs`) |
| Validation | Dependencies, JUnit XML XSD | `crates/testcase-validation/src/` |
| UI | Interactive creation/editing (dialoguer, fuzzy, oracle) | `crates/testcase-ui/src/` |
| Orchestration | Coordinates execute/validate/verify flows | `crates/testcase-orchestration/src/lib.rs` |
| Manager facade | Re-exports ui/git/cli; holds dependency resolver, sample extensions | `crates/testcase-manager/src/lib.rs` |
| Audit | Signed audit logs (P-521), traceability | `crates/audit-verifier/src/`, `crates/audit-traceability/src/lib.rs` |
| ID resolver | Config-driven id<->URL mapping (`integration-config.systems[]`) | `crates/id-resolver/src/lib.rs` |
| Jira client | Jira Cloud/DC REST, write guard, audit entries, mock | `crates/jira-client/src/` |
| Test backends | `TestBackend` trait: plain Jira, Xray, Zephyr Scale/Squad | `crates/test-backends/src/` |
| Requirement import | Jira and StrictDoc requirement import | `crates/requirement-import/src/` |
| tcms CLI | Subcommands for refs, Jira sync, PRs | `crates/tcms/src/main.rs` + `*cmd.rs` |

## Pattern Overview

**Overall:** Cargo workspace of layered library crates plus single-purpose CLI binaries (Unix-style pipeline over files).

**Key Characteristics:**
- Bottom-up layering; lower crates never depend on higher ones (documented in `AGENTS.md`, `crates/README.md`).
- Declarative YAML test cases validated by JSON Schema (`schemas/`) before deserialization.
- Stages communicate through files (YAML, bash scripts, JSON logs, container YAML), not in-process calls.
- External-system code is behind a trait (`TestBackend` in `crates/test-backends/src/lib.rs`) with mock/fake implementations behind a `mock` cargo feature.

## Layers

**Base:** `crates/testcase-models`, `crates/bash-eval`. Types and evaluation; no I/O orchestration.
**Foundation:** `crates/testcase-common`. Depends on models. Used by nearly everything.
**Core:** validation, execution, verification, storage, ui, git, cli. Depend on models/common (storage -> validation; ui -> storage, verification, git).
**Integration:** `id-resolver` (models, common) -> `jira-client` -> `requirement-import`, `test-backends`.
**Orchestration:** `testcase-orchestration` (execution, storage, verification); `testcase-manager` (aggregates all core + orchestration).
**Binaries:** thin `main.rs` with clap, delegating to libraries.

## Data Flow

### Primary Test Path

1. User/editor writes a test case YAML (`type: test-case`, `schema: tcms/test-case.schema.v1.json`) in `testcases/`.
2. `validate-yaml` / `load_and_validate_yaml` validate against `schemas/` (`crates/testcase-common/src/yaml_loader.rs`).
3. `test-executor generate` builds a bash script via `crates/testcase-execution/src/executor.rs`; `execute` runs it and emits a JSON execution log.
4. `verifier` / `test-verify` compare log vs expected (`crates/testcase-verification/src/verification.rs`), writing verification or container YAML.
5. `tpdg-compat` / external test-plan-documentation-generator render AsciiDoc/Markdown/HTML.
6. `test-acceptance/run_acceptance_suite.sh` drives all 7 stages end to end.

### Jira Sync Path

1. `tcms` subcommand (`crates/tcms/src/synccmd.rs`) loads integration config, resolves ids (`id-resolver`).
2. A `TestBackend` from `build_backend` (`crates/test-backends/src/backends.rs`) is used via `&mut dyn TestBackend`.
3. Writes pass `jira-client` write guard (`guard.rs`) and produce audit entries (`audit.rs`).

**State Management:** Stateless CLIs; persistent state in YAML files, sqlite/`database.rs` in storage, and run history via `trm`.

## Key Abstractions

**TestCase model:** `crates/testcase-models/src/lib.rs`; serde-derived structs.
**Envelope (type + schema):** `crates/testcase-common/src/envelope.rs`; every YAML carries `type` and `schema`.
**Oracle:** interactive-answer trait with `HardcodedOracle`, `MenuCliOracle`, `TtyCliOracle` in `crates/testcase-ui/src/oracle.rs`, enabling non-interactive tests.
**TestBackend trait:** `crates/test-backends/src/lib.rs`.

## Entry Points

- `crates/test-executor/src/main.rs` (bin `test-executor`): generate/execute, coverage, trace matrix, regression, independence subcommands.
- `crates/verifier/src/main.rs` (`verifier`), `crates/test-verify/src/main.rs`, `crates/test-orchestrator/src/main.rs`, `crates/test-run-manager/src/main.rs` (bin `trm`), `crates/editor/src/main.rs`.
- `crates/validate-yaml/src/`, `crates/validate-json/src/main.rs`, `crates/script-cleanup`, `crates/json-escape`, `crates/json-to-yaml`.
- `crates/tcms/src/main.rs` (`tcms`), `crates/jira-client/src/bin/mock_jira.rs` (`mock-jira`).
- `crates/audit-verifier/src/bin/` (`sign-audit-log`, `verify-audit-log`, `verify-audit-signature`).
- `crates/tpdg-compat` (`test-plan-documentation-generator-compat`).

## Architectural Constraints

- **Layering:** never add a dependency from a lower layer to a higher one. Known deviations: `validate-yaml` depends on `testcase-manager`; `testcase-execution` depends on `audit-verifier`; `test-executor` depends on `requirement-import`.
- **Global state:** none significant; config passed explicitly or via env (`TCMS_INTEGRATION_CONFIG`, `TCMS_SCHEMAS_ROOT`).
- **Shell compat:** generated and repo scripts must work on bash 3.2+ with BSD and GNU tools; use `scripts/lib/logger.sh`.
- **Dependencies:** declared once in root `Cargo.toml` `[workspace.dependencies]`, used with `{ workspace = true }`.
- **Duplication:** `testcase-manager/src` still holds copies of modules also in `testcase-storage` and `testcase-ui` (e.g. `parser.rs`, `database.rs`, `editor.rs`); `lib.rs` re-exports the ui ones.
- **Integration code unverified against real services:** Jira/Xray/Zephyr only tested against mocks.

## Anti-Patterns

### Hand-parsing YAML
**What happens:** deserializing YAML with `serde_yaml::from_str` directly.
**Why it's wrong:** skips schema validation.
**Do this instead:** `testcase_common::load_and_validate_yaml` (`crates/testcase-common/src/yaml_loader.rs`).

### Upward dependency
**What happens:** a lower crate imports a higher one.
**Do this instead:** extract shared code to a lower crate or use a trait.

## Error Handling

**Strategy:** `anyhow::Result` with `.context(...)` in libraries and binaries; schema violations reported with JSON path and constraint.
**Patterns:**
- Hook `on_error: fail|continue` semantics in generated scripts.
- Exit code 0/1 from binaries; verifier treats non-log files as unverifiable.

## Cross-Cutting Concerns

**Logging:** `log`/`tracing` in Rust (`--log-level`, `--verbose`); `scripts/lib/logger.sh` in shell.
**Validation:** JSON Schema in `schemas/`, XSD for JUnit.
**Authentication:** Jira token read from env var named by `auth.secret_ref`; never logged. Audit logs signed with P-521 keys.

---

*Architecture analysis: 2026-10-30*
