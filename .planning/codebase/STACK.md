# Technology Stack

**Analysis Date:** 2026-10-30

## Languages

**Primary:**
- Rust (edition 2021) - all 31 workspace crates under `crates/` plus root package `testcase-generator` in `Cargo.toml`. Docker builds on `rust:1.95-bookworm` (`Dockerfile`); GitHub CI uses `stable` toolchain (`.github/workflows/workspace.yml`). No `rust-toolchain` file.

**Secondary:**
- Bash 3.2+ (BSD and GNU portable) - generated test scripts and `scripts/*.sh`, `mk/*.mk` helpers; shared logger at `scripts/lib/logger.sh`
- Python >=3.14 (`.python-version` = 3.14, `pyproject.toml`) - utility scripts (`yaml_editor.py`, `convert_to_binary.py`, `scripts/convert_verification_to_result_yaml.py`), output schema validation
- Makefile - primary task runner (`Makefile`, `mk/incremental.mk`)

## Runtime

**Environment:**
- Native Rust binaries; Linux/macOS (Windows via Docker, see `docker-compose.ps1`, `docker-compose.bat`)
- Runtime tools used by generated scripts: `jq` (preferred JSON escaping), `json-escape` binary, `shellcheck` (optional)

**Package Manager:**
- Cargo (workspace, resolver 2) - Lockfile: `Cargo.lock` present
- uv (Python) - Lockfile: `uv.lock` present
- npm - `package.json` only depends on `backlog.md` ^1.55.3 (`package-lock.json` present)

## Frameworks

**Core:**
- clap 4.4 (derive) - CLI parsing in every binary crate
- serde 1.0 / serde_json 1.0 / serde_yaml 0.9 - (de)serialization of test cases, logs, containers
- jsonschema 0.17 - schema validation (`crates/testcase-validation`, `crates/testcase-common`)
- skim 0.10, dialoguer 0.12 - interactive fuzzy finder and prompts (`crates/testcase-ui`)
- edit 0.1 - launches `$EDITOR`/`$VISUAL`
- reqwest 0.11 (blocking) - HTTP client used by `crates/jira-client`, `crates/test-backends`

**Testing:**
- Built-in `cargo test` (`--all-features`), `tempfile` for fixtures
- cargo-llvm-cov - coverage (50% unit, 70% with e2e thresholds); aliases in `.cargo/config.toml`
- pytest >=9.0.2 (Python), bash E2E scripts under `crates/testcase-manager/tests/integration/` and `test-acceptance/`

**Build/Dev:**
- sccache (optional, `RUSTC_WRAPPER`), configured in `.cargo/config.toml`, `Dockerfile`, `.gitlab-ci.yml`; not used on GitHub (`docs/GITHUB_CI_CACHING.md`)
- clippy, rustfmt (`make lint`), ruff >=0.15.6, mypy >=1.19.1 (Python)
- strictdoc >=0.20.0 (`strictdoc.toml`) - requirements documentation
- Backlog.md MCP (`.mcp.json`, server `backlog` via `backlog mcp start`) - task management

## Key Dependencies

**Critical (workspace.dependencies in `Cargo.toml`):**
- anyhow `=1.0.99` (pinned) - error handling
- git2 0.20.3 - Git integration (`crates/testcase-git`)
- regex 1.10 / regex-syntax 0.8 - variable capture, id patterns (`crates/id-resolver`)
- chrono 0.4 - timestamps
- indexmap 2.1 - ordered maps
- toml 0.9.11 - config files (json_escaping etc.)
- dotenvy 0.15 - `.env` loading
- roxmltree 0.20 - JUnit XML / XSD validation
- sha2 0.10 - hashing
- notify 8.2 - watch mode (`validate-yaml --watch`)
- walkdir 2.4, tempfile 3.8, strip-ansi-escapes 0.2, log 0.4, env_logger 0.11, serde_path_to_error `*` (unpinned)

**Crate-local (`crates/audit-verifier/Cargo.toml`):**
- p256 / p521 0.13 (ecdsa, pem), signature 2.2, hex 0.4, rand_core 0.6, base64ct 1.6, hostname 0.3 - audit log signing and verification

## Configuration

**Environment:**
- `.env.example`, `.env.docker`, `.env.sh` present (contents not read). Documented vars: `EDITOR`, `VISUAL`, `TESTCASE_EDITOR`
- Vars read in code: `AUDIT_LOG_ENABLED`, `AUDIT_LOG_FILE`, `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `TCMS_USER`, `TCMS_REVIEWER`, `TCMS_SCHEMAS_ROOT`, `TCMS_INTEGRATION_CONFIG`, `TCMS_JIRA_USER`, `TCMS_JIRA_TOKEN`, `TCMS_ZEPHYR_API_URL`
- JSON escaping method in TOML config (`[json_escaping] method = auto|jq|json-escape|shell`)
- Verifier metadata: `verifier-config.example.yaml`, `container_config*.yml.example`
- Integration config YAML per product (examples in `examples/integration-config/`)

**Build:**
- `Cargo.toml` (workspace root), `.cargo/config.toml` (aliases, sccache dir), `Makefile`, `mk/incremental.mk`, `Dockerfile`, `docker-compose.yml`, `.dockerignore`
- JSON Schemas in `schemas/` (test-case, verification-result, container, execution-log, etc.)

## Platform Requirements

**Development:**
- Rust toolchain with `clippy`, `rustfmt`, `llvm-tools-preview`; `jq`, `expect`, `shellcheck`; uv + Python 3.14; Node/npm for Backlog.md; optional sccache
- `test-plan-documentation-generator` (tpdg) installed via `cargo install` or `TEST_PLAN_DOC_GEN` env var, required for `make acceptance-test`

**Production:**
- Docker image (`Dockerfile`, Debian bookworm, adds git, inotify-tools, expect, shellcheck, make) and distributable binaries; CI on GitHub Actions (`.github/workflows/workspace.yml`) and GitLab CI (`.gitlab-ci.yml`)

---

*Stack analysis: 2026-10-30*
