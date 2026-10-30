# Technology Stack

**Project:** TCMS traceability reports (milestone: report generation)
**Researched:** 2026-10-30
**Overall confidence:** MEDIUM-HIGH (workspace facts verified from the repo; crates.io versions checked via the crates.io API; TPDG internals NOT verified, see Gaps)

## Headline Recommendation

Add **zero new third-party dependencies**. Build a small, pure, library-only reporting module that turns `TraceMatrix` + `MatrixFindings` + `TestRunRecord` (+ problem reports) into a typed `Report` model, then renders it to Markdown and AsciiDoc with plain `std::fmt::Write` string builders. Emit a JSON/YAML form of the same model, validated by the existing `jsonschema` path. Treat TPDG as an optional downstream renderer (HTML conversion), not the primary renderer for the matrix.

Why: everything the reports need already exists in the workspace (`trace_matrix.rs` has `TraceLink`, `MatrixFindings`, `gap_report()`; `test-run.schema.json`, `problem-report.schema.json`, `trace-matrix.schema.json`, `coverage-report.schema.json` are vendored in `schemas/tcms-schemas/tcms/`). The output is tables and lists, which do not justify a template engine.

## Recommended Stack

### Core (all already in the workspace; use `{ workspace = true }`)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Rust, edition 2021 | existing | Implementation language | Matches all 31 crates | HIGH |
| serde / serde_json / serde_yaml | 1.0 / 1.0 / 0.9 | Read matrix, run, problem-report artifacts; write report data file | Already the (de)serialization layer. `serde_yaml` 0.9.34 is marked `+deprecated` on crates.io but is the workspace choice; migrating is out of scope and would touch every crate | HIGH |
| `testcase_common::load_and_validate_yaml` | existing | Load every input artifact | `AGENTS.md` makes this mandatory; hand-parsing is a listed anti-pattern | HIGH |
| jsonschema | 0.17 (keep) | Validate inputs and, where a schema exists, outputs | Already pinned. crates.io latest is 0.60.2, but the API changed substantially across those versions; a bump is a workspace-wide change unrelated to this milestone. Do not bump here | HIGH (current version), MEDIUM (that bump is risky) |
| clap | 4.4 (derive) | New `report` subcommand(s) | Same as every binary | HIGH |
| anyhow | =1.0.99 | Errors with `.context()` | Workspace convention | HIGH |
| indexmap / `BTreeMap` | 2.1 / std | Deterministic ordering of requirements and verdict counts | Reports must be reproducible byte-for-byte; `BTreeMap` for counts, `indexmap` where input order matters | HIGH |
| chrono | 0.4 | "Generated at" stamp | Already used; take the timestamp as a parameter so tests can fix it | HIGH |

### Rendering

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| `std::fmt::Write` string builders | std | Markdown tables and AsciiDoc tables | Output is 4 table/list shapes. No dependency, deterministic, trivially unit-testable with string snapshots, no bash/BSD concerns | HIGH |
| test-plan-documentation-generator (TPDG) | external binary, found via `TEST_PLAN_DOC_GEN` (`scripts/download-tpdg.sh`) | Optional HTML/AsciiDoc rendering of run results | Not on crates.io (API search returned no crates), so it cannot be a Cargo dependency. Existing contract is container YAML in, docs out. Use it only for the part of the report that already fits a container (per-run results) | MEDIUM |
| `asciidoctor` (external, optional) | system | AsciiDoc to HTML | Already the documented HTML path ("HTML generated from AsciiDoc, requires asciidoctor"). Invoke it from a script, skip with a clear message when missing | MEDIUM |

### Where the code lives (layering)

| Concern | Location | Rationale |
|---------|----------|-----------|
| Report model + builders (pure functions, no I/O) | `crates/testcase-verification` (new `report/` module) | Layer 3. It already owns `trace_matrix.rs`, `coverage.rs`, `independence.rs` and `TestRunRecord`, so there is no new edge. A new crate is only justified if the module grows past about 1.5k lines |
| Markdown / AsciiDoc renderers | same module, `render_markdown`, `render_asciidoc` | Pure `&Report -> String`; keeps lower layers free of I/O |
| CLI wiring | `crates/test-executor` (next to `trace-matrix`) | Entry point that already hosts coverage, trace-matrix, regression, independence |
| Shell glue for HTML | `scripts/` using `scripts/lib/logger.sh` | Must stay bash 3.2 and BSD/GNU portable |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Rendering | Hand-written `fmt::Write` | `minijinja` (3.0.5 current) or `tera` | New dependency plus a template-language layer for four fixed tables; templates are harder to snapshot-test than `String` output and add a second place for logic. Revisit only if users need customizable report templates |
| Rendering | Hand-written | `askama` (compile-time templates) | Same objection; also couples template files to the crate build |
| Rendering | Hand-written Markdown | `pulldown-cmark` / `comrak` HTML | We emit Markdown, we do not parse it. HTML goes via AsciiDoc and asciidoctor, the existing path |
| Tables | Own helper (about 40 lines) | `comfy-table`, `tabled` | They target terminal box-drawing, not Markdown/AsciiDoc cell escaping; the real work is escaping `|` and newlines, which you must write anyway |
| Primary renderer | In-process renderer | Route everything through TPDG | TPDG's documented input is the test-results container; there is no evidence it accepts a trace matrix. It is also an external binary that is missing in the dev environment (`make test` was not runnable), so core report logic and its tests must not depend on it |
| Output validation | Existing `jsonschema` 0.17 | Bump to 0.60 | Cross-workspace API churn, unrelated to the feature |
| Data format | JSON/YAML report data file next to the rendered docs | Only rendered text | Rendered-only output cannot be diffed, re-rendered, or consumed by TPDG or other tools |
| Snapshot tests | Plain `assert_eq!` against fixture files in `tests/fixtures/` | `insta` | New dev dependency; fixtures are enough for 4 reports. Optional later |
| PDF | None | `printpdf`, `genpdf` | The repo just removed Python PDF generation in favour of TPDG; do not reintroduce a PDF path |

## Key Design Constraints Driving the Stack

1. **Inputs are the existing artifacts.** Matrix (`trace-matrix.schema.json`), runs (`test-run.schema.json`: `run_id`, `results`, `reviewer`, ...), problem reports (`problem-report.schema.json`: `id`, `run_id`, `test_case_id`, `severity`, `status`). Joining is by `run_id` / `test_case_id`; no re-execution.
2. **Schemas are shared and must not change.** A report data file is a new artifact, so it needs its own report-local schema in `schemas/` (with the required `type` and `schema` envelope fields per `AGENTS.md`). `coverage-report.schema.json` is for code coverage tools (`per_code_unit`, `structural`), not requirement gaps; do not overload it.
3. **Determinism.** Sort requirements by id, test cases by id, and verdicts in a fixed enum order (`pass`, `fail`, `error`, `not_executed`). Inject the clock.
4. **Gaps are explicit.** A requirement with no test cases, or missing normal-range or robustness coverage (already `MatrixFindings.gaps` and `untraced`), must render as a visible row ("NO TEST CASES"), never be omitted.
5. **Output escaping.** Markdown cells escape `|`, backticks and newlines; AsciiDoc cells escape `|`. Problem-report text is user-controlled, so test with hostile strings.

## Installation

```bash
# No new crates. In the consuming crate's Cargo.toml use workspace deps only:
#   serde = { workspace = true }, serde_json = { workspace = true },
#   serde_yaml = { workspace = true }, anyhow = { workspace = true },
#   chrono = { workspace = true }, indexmap = { workspace = true }
# dev: tempfile = { workspace = true }

# Optional external tools (already documented in the repo)
./scripts/download-tpdg.sh && export TEST_PLAN_DOC_GEN="<dir>/tpdg"
# asciidoctor for AsciiDoc -> HTML (system package, optional)
```

## What NOT to Use

- **New template engines (tera/minijinja/askama/handlebars):** see table above.
- **Python for report logic:** the repo is reducing Python to utility scripts; report logic must be testable under `cargo test`.
- **`jq`/bash for aggregation:** `jq` is a runtime tool for generated scripts; aggregation across matrix and runs belongs in typed Rust.
- **`serde_yaml::from_str` directly on inputs:** skips schema validation (explicit anti-pattern).
- **A second renderer pipeline for run results:** where the per-run content fits the TPDG container, feed TPDG rather than duplicating its output (PROJECT.md decision).

## Gaps / Needs Validation

- **TPDG input capabilities (MEDIUM-LOW):** not verified whether the installed TPDG can render a trace matrix or a gap table from a container. Source repo and binary were unavailable here (not on crates.io; `scripts/download-tpdg.sh` fetches releases). Before the roadmap commits to "TPDG renders the traceability report", a spike should feed TPDG a container containing requirement-linked results. Fallback is already the recommended primary design, so the risk is limited to the "reuse" goal.
- **Exact `TestRunRecord` and problem-report Rust types:** read `trace_matrix.rs` only; confirm field names in `testcase-verification` before API design.
- **`jsonschema` 0.17 `$ref` resolution** for the vendored `tcms-schemas` bundle was not re-tested; existing validation of trace-matrix output already works (TCMS-40.5), so reuse that code path.
- **Environment:** `make test` is not fully runnable here (uv, shellcheck, StrictDoc, TPDG missing); report unit tests must be pure Rust so they run anyway.

## Sources

- Repo: `Cargo.toml` `[workspace.dependencies]`, `crates/testcase-verification/src/trace_matrix.rs`, `schemas/tcms-schemas/tcms/*.schema.json`, `scripts/download-tpdg.sh`, `.planning/codebase/STACK.md`, `.planning/codebase/ARCHITECTURE.md`, `AGENTS.md` (HIGH, primary)
- crates.io API, queried 2026-10-30: jsonschema 0.60.2, serde_yaml 0.9.34+deprecated, minijinja 3.0.5; `test-plan-documentation-generator` not published (HIGH for versions, used only to justify "do not bump / do not add")
