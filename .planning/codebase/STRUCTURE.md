# Codebase Structure

**Analysis Date:** 2026-10-30

## Directory Layout

```
beefy-buffalo/
├── Cargo.toml, Cargo.lock      # Workspace root (also root package `testcase-generator`, `src/lib.rs`)
├── crates/                     # All Rust crates (31 workspace members)
│   ├── README.md               # Layer/dependency documentation
│   ├── testcase-models/        # Base: data types
│   ├── bash-eval/              # Base: bash expression evaluation
│   ├── testcase-common/        # Foundation: yaml_loader, envelope, config
│   ├── testcase-{validation,execution,verification,storage,ui,git,cli}/  # Core
│   ├── testcase-orchestration/ testcase-manager/   # Orchestration
│   ├── audit-verifier/ audit-traceability/         # Audit signing + traceability
│   ├── id-resolver/ jira-client/ requirement-import/ test-backends/  # Jira/StrictDoc integration
│   ├── tcms/                   # `tcms` CLI (refcmd, reqcmd, jiracmd, synccmd, prcmd, extrefs)
│   └── editor/ test-executor/ test-orchestrator/ test-verify/ test-run-manager/ verifier/
│       validate-yaml/ validate-json/ script-cleanup/ json-escape/ json-to-yaml/ tpdg-compat/  # Binaries
├── schemas/                    # JSON schemas (`tcms/`, `tcms-schemas/`, `templates/`, top-level *.schema.json)
├── testcases/                  # Test case YAML (examples, bdd_examples, verifier_scenarios, generated_samples)
├── test-acceptance/            # 7-stage acceptance suite (`run_acceptance_suite.sh`, validate_stage*.sh, tests/)
├── scripts/                    # Shell/Python tooling; `scripts/lib/logger.sh` shared logger
├── mk/                         # Makefile includes (incremental, python, backlog, projector, strictdoc)
├── Makefile                    # Top-level targets
├── bin/                        # check_build, check_lint, check_tests, check_release_build
├── docs/                       # Feature docs; `docs/data_flow/*.mermaid`, `docs/strictdoc/`
├── backlog/                    # Backlog.md tasks, decisions, config.yml
├── data/ examples/ reports/    # Sample data, examples, generated reports
├── .github/workflows/workspace.yml, .gitlab-ci.yml   # CI
├── Dockerfile, docker-compose*.yml
├── pyproject.toml, uv.lock     # Python 3.14 tooling (uv)
└── *.md (root)                 # Many historical implementation summaries; AGENTS.md/CLAUDE.md are authoritative
```

## Directory Purposes

**`crates/<name>/`:**
- Contains `Cargo.toml`, `src/` (`lib.rs` or `main.rs`), optional `src/bin/`, `tests/`, `examples/`.
- Key files: `crates/testcase-models/src/lib.rs`, `crates/testcase-common/src/yaml_loader.rs`, `crates/testcase-execution/src/executor.rs`, `crates/testcase-verification/src/verification.rs`.

**`schemas/`:** JSON Schemas; URI `tcms/test-case.schema.v1.json` resolves to a file here. `schemas/tcms-schemas/TCMS_SCHEMAS_VERSION` pins the schema set.

**`testcases/`:** Inputs for validation, execution, and verifier scenarios.

**`test-acceptance/`:** Stage scripts `validate_stage1_yaml.sh` .. `validate_stage7_consolidated_docs.sh`, cases in `00_test_cases/`, results in `results/`.

## Key File Locations

**Entry Points:** `crates/*/src/main.rs`, `crates/audit-verifier/src/bin/*.rs`, `crates/jira-client/src/bin/mock_jira.rs`.

**Configuration:** `Cargo.toml` (workspace deps), `.cargo/config.toml`, `container_config.yml` and `*.example` variants, `verifier-config.example.yaml`, `pyproject.toml`.

**Core Logic:** see Component table in `.planning/codebase/ARCHITECTURE.md`.

**Testing:** per-crate `crates/<name>/tests/*.rs` (e.g. `crates/testcase-manager/tests/`, `crates/test-executor/tests/`); shell E2E in `crates/testcase-manager/tests/integration/*.sh`; acceptance in `test-acceptance/`.

## Naming Conventions

**Files:**
- Rust modules: `snake_case.rs` (`dependency_resolver.rs`); CLI subcommand modules in `tcms` end with `cmd.rs`.
- Integration tests: `*_test.rs` or `*_cli.rs` in `tests/`.
- Shell: `snake_case.sh`; test scripts `test_*_e2e.sh`.
- Docs at root: `UPPER_SNAKE.md`; in `docs/` `UPPER_SNAKE.md`.
- Test case YAML: `TC_*`, `TEST_*` IDs, `.yml` or `.yaml`.

**Directories:**
- Crates: `kebab-case`; library crates prefixed `testcase-`; binary crates named for the tool (bin names may differ, e.g. `test-run-manager` -> `trm`).

## Where to Add New Code

**New domain feature:** the matching core crate (`crates/testcase-<area>/src/`), with tests in `crates/<crate>/tests/`.
**New data type:** `crates/testcase-models/src/lib.rs` plus schema update in `schemas/`.
**New CLI tool:** new `crates/<tool>/` with `[[bin]]`, add to `[workspace] members` in `Cargo.toml`, add Make target (`build-<tool>`), and E2E mapping in `mk/incremental.mk`.
**New Jira/backend feature:** `crates/test-backends/src/` (implement `TestBackend`, route writes through `Ctx::write`); new `tcms` subcommand as `crates/tcms/src/<name>cmd.rs` registered in `main.rs`.
**New YAML-loading code:** use `testcase_common::load_and_validate_yaml`.
**Utilities:** shared Rust helpers in `crates/testcase-common/src/`; shell helpers in `scripts/lib/`.
**Dependencies:** add to root `[workspace.dependencies]`, reference with `workspace = true`.
**Docs:** `docs/<TOPIC>.md`; backlog tasks via Backlog.md MCP in `backlog/tasks/`.

## Special Directories

**`target/`:** build output; generated, not committed.
**`.venv/`:** uv Python environment; generated.
**`reports/`, `test-acceptance/results/`:** generated outputs.
**`.planning/codebase/`:** GSD codebase maps; committed.
**`schemas/tcms-schemas/`:** versioned schema set (see `TCMS_SCHEMAS_VERSION`); committed.

---

*Structure analysis: 2026-10-30*
