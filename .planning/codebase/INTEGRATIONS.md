# External Integrations

**Analysis Date:** 2026-10-30

## APIs & External Services

**Issue tracking / test management:**
- Jira Cloud (REST v3) and Jira Data Center (REST v2) - issue sync, connection test, external refs
  - SDK/Client: `crates/jira-client` (reqwest blocking, 429 retry with back-off, `Secret` wrapper that redacts tokens)
  - Auth: token read from the env var named by `auth.secret_ref` in the integration config (e.g. `TCMS_JIRA_TOKEN`), user via `TCMS_JIRA_USER`
  - Write guard: `crates/jira-client/src/guard.rs`; audit entries: `crates/jira-client/src/audit.rs`
  - Verified only against the in-process mock (`crates/jira-client/src/mock.rs`, binary `mock-jira`, feature `mock`), never a real Jira
- Test backends behind one trait: plain Jira, Xray, Zephyr Scale, Zephyr Squad
  - Client: `crates/test-backends` (feature `mock` for fakes)
  - Auth/config: `TCMS_ZEPHYR_API_URL` plus integration config
- StrictDoc (static HTML export or server) - requirement import
  - Client: `crates/requirement-import`; docs in `docs/JIRA_INTEGRATION.md`
  - Config examples: `examples/integration-config/*.yaml`; schema vendored in `schemas/`
- CLI entry: `crates/tcms` (`tcms ref`, `tcms jira test-connection`, `tcms requirements import`)

**Documentation generation:**
- test-plan-documentation-generator (tpdg) - external binary rendering AsciiDoc/Markdown/HTML from container YAML
  - Interface: subprocess; location via `TEST_PLAN_DOC_GEN` env var or global cargo install
  - Compat checker: `crates/tpdg-compat`

**Task management:**
- Backlog.md MCP server - `.mcp.json` (stdio, `backlog mcp start --cwd .`), docs `docs/BACKLOG_MCP_SETUP.md`

## Data Storage

**Databases:**
- None. State is YAML/JSON files (test cases in `testcases/`, logs, container YAMLs, `data/`), persisted by `crates/testcase-storage`

**File Storage:**
- Local filesystem only; Git repository storage via `git2` (`crates/testcase-git`)

**Caching:**
- None at runtime (build caching only: sccache in GitLab/Docker, RustCrypto cargo-cache on GitHub)

## Authentication & Identity

**Auth Provider:**
- None for the application itself. Jira token auth only (see above)
- Audit log signing/verification with ECDSA P-256/P-521 keys: `crates/audit-verifier` (binaries `audit-verifier`, `sign-audit-log`, `verify-audit-log`); audit logging via `crates/audit-traceability`, controlled by `AUDIT_LOG_ENABLED`, `AUDIT_LOG_FILE`
- Acting user identity from `TCMS_USER`, `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `USER`

## Monitoring & Observability

**Error Tracking:**
- None

**Logs:**
- `log` + `env_logger` in Rust; `scripts/lib/logger.sh` in shell; JSON execution logs from generated scripts; JUnit XML export (`docs/JUNIT_EXPORT.md`)

## CI/CD & Deployment

**Hosting:**
- Docker image built from `Dockerfile` (`rust:1.95-bookworm`), run via `docker-compose.yml`; GitLab registry image tagged by commit SHA

**CI Pipeline:**
- GitHub Actions: `.github/workflows/workspace.yml` (build, test, lint, llvm-cov coverage; uses `dtolnay/rust-toolchain`, `taiki-e/install-action`, `actions/cache`, uv)
- GitLab CI: `.gitlab-ci.yml` (stages docker, rust, e2e, release; DinD host, sccache)

## Environment Configuration

**Required env vars:**
- None strictly required for core tools. For Jira: token var named in config (`TCMS_JIRA_TOKEN`), `TCMS_JIRA_USER`; optional `TCMS_INTEGRATION_CONFIG`, `TCMS_SCHEMAS_ROOT`, `TEST_PLAN_DOC_GEN`, `EDITOR`/`VISUAL`/`TESTCASE_EDITOR`, `AUDIT_LOG_*`

**Secrets location:**
- Environment variables only; `.env.example`, `.env.docker`, `.env.sh` exist at repo root (not inspected). Secrets are never written to records or logs (`Secret` type)

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- Jira REST calls only (no webhooks). Example hook scripts in `testcases/examples/hooks/` may POST to a user-defined `MONITORING_ENDPOINT` via curl

---

*Integration audit: 2026-10-30*
