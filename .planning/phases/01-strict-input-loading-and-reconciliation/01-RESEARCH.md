# Phase 1: Strict Input Loading and Reconciliation - Research

**Researched:** 2026-10-30
**Domain:** Rust workspace refactor plus a new fail-closed loader and selection policy (no new third-party packages)
**Confidence:** HIGH for code facts (read in this session), MEDIUM for the open design points marked below

## User Constraints (from CONTEXT.md)

### Locked Decisions
- D-01: For each test case, the latest run (by `started_at`, RFC 3339) that has an executed result for it decides.
- D-02: A later `pass` overrides an earlier `fail` or `error`. A later `fail` or `error` overrides an earlier `pass`.
- D-03: A run that omits the test case, or records `not_executed` for it, does not change the result.
- D-04: Runs are not restricted to one build. The report lists every build it used.
- D-05: The report lists the runs it used and the runs it excluded.
- D-06: Every matrix and run input is read through `testcase_common::load_and_validate_yaml`. A schema failure fails the command and names the file.
- D-07: A reconciliation block lists inputs discovered, loaded, and skipped with a reason. No input is dropped silently.
- D-08: Two run records with the same `run_id` fail the command, naming both files.
- D-09: New Layer 3 crate `testcase-reporting`. Only `test-executor` depends on it.
- D-10: The `report` subcommand sits next to `trace-matrix` in `test-executor`.
- D-11: Loaders move out of `crates/test-executor/src/main.rs` into `testcase-reporting` behaviour-preserving. The existing `trace_matrix_cli` tests must still pass.
- D-12: Closed. `build_matrix` emits a row for every requirement (not re-researched).
- D-13 (assumed, not a Phase 1 concern): ERRORED ranks above FAILED in the roll-up. Phase 2.

### Claude's Discretion
Exact error wording and reconciliation layout (provided D-07, D-08 are met); internal module split inside `testcase-reporting`.

### Deferred Ideas (OUT OF SCOPE)
Verdict roll-up (Phase 2); staleness vs `source_hashes` (v2); TPDG/HTML (v2); source hashes in run record; reviewer/independence report.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LOAD-01 | Fail on schema-invalid matrix/run, naming file | `load_and_validate_yaml` already names the file in its context chain (yaml_loader.rs:52-68); wrap with per-file collection |
| LOAD-02 | Reconciliation block, no silent drops | New `Reconciliation` struct; legacy `read_test_runs` silently skips (main.rs:513-525) so a NEW strict loader is required |
| LOAD-03 | Latest executed result wins | `select_results` over parsed `started_at` (section "Selection algorithm") |
| LOAD-04 | Duplicate `run_id` fails | Check after load, before selection (section "Where each decision is enforced") |
| CLI-01 | `test-executor report` exists with kind selection | New `Commands::Report` beside `TraceMatrix` (main.rs:140) |
</phase_requirements>

## Summary

No new third-party packages are needed. Everything uses what the workspace already has: `testcase-common::load_and_validate_yaml`, `testcase-verification::{TraceMatrix, TestRunRecord, Verdict, build_matrix}`, `chrono`, `anyhow`, `clap`, `serde_yaml`, `tempfile` (dev). Package Legitimacy Audit: not applicable (no external package is added; all dependencies are existing workspace members or workspace dependencies).

The central finding is a conflict between D-11 and D-06/D-07. The existing `read_test_runs` (main.rs:492-528) does NOT use `load_and_validate_yaml`, does not schema-validate, and silently skips (a) non-YAML files, (b) YAML whose `type` is not `test_run`, and (c) in a directory, any file that fails to deserialize (`Err(_) => {}`, main.rs:524). The existing test `runs_link_through_run_ids` (trace_matrix_cli.rs, around line 176-200) depends on `notes.yaml` being skipped. So the move must be two things: (1) move `read_test_runs` and `load_test_cases_strict` unchanged into `testcase-reporting` for `trace-matrix` (D-11, behaviour-preserving), and (2) write a NEW strict run loader for `report` (D-06/D-07). Do not "fix" `read_test_runs` in place; that would change `trace-matrix` behaviour and break the existing test.

**Primary recommendation:** Create `crates/testcase-reporting` with modules `legacy` (moved loaders), `load` (strict matrix + run loaders, reconciliation), `select` (D-01..D-05), and a `report` subcommand in `test-executor` that prints the reconciliation block to stdout (or `--output`). Keep `read_matrix_requirements` in test-executor.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary | Rationale |
|------------|-------------|-----------|-----------|
| Strict load of matrix and runs | Library crate `testcase-reporting` (L3) | `testcase-common` (L2) for validation | Domain logic, reusable by later render phases |
| Run selection (D-01..D-05) | `testcase-reporting` | — | Pure function over parsed records; unit-testable without CLI |
| Reconciliation block text | `testcase-reporting` (data + `Display`) | `test-executor` prints | Phase 2+ headers reuse it |
| CLI args, output file, exit code | `test-executor` (L5) | — | Binary owns I/O and process exit |

## Standard Stack

### Core (all already in the workspace, no installs)
| Library | Version | Purpose |
|---------|---------|---------|
| testcase-common | path | `load_and_validate_yaml` (yaml_loader.rs:38) [VERIFIED: codebase] |
| testcase-verification | path | `TraceMatrix` (trace_matrix.rs:151), `TestRunRecord`, `TestRunResult`, `Verdict` (test_run.rs) [VERIFIED: codebase] |
| testcase-models | path | `TestCase` for the moved `load_test_cases_strict` [VERIFIED: codebase] |
| chrono (workspace) | 0.4 | Parse `started_at` to compare instants, not strings [VERIFIED: Cargo.toml] |
| anyhow (workspace) | =1.0.99 | Errors [VERIFIED: Cargo.toml] |
| serde_yaml (workspace) | 0.9 | Type peek for directory entries [VERIFIED: Cargo.toml] |

### Alternatives Considered
`requirement-import` as a dependency of `testcase-reporting`: rejected. It pulls `jira-client` and `id-resolver` (requirement-import/Cargo.toml), which CONTEXT.md does not list for the new crate. Keep `read_matrix_requirements` (main.rs:580-611) and its three unit tests (main.rs:1548-1601) in test-executor.

## Crate Layout and Public API

New files:
```
crates/testcase-reporting/Cargo.toml      # template: crates/id-resolver/Cargo.toml
crates/testcase-reporting/src/lib.rs
crates/testcase-reporting/src/legacy.rs   # moved: read_test_runs, load_test_cases_strict, load_test_case
crates/testcase-reporting/src/load.rs     # strict loaders + Reconciliation
crates/testcase-reporting/src/select.rs   # D-01..D-05
crates/testcase-reporting/tests/          # fixture-based integration tests (optional)
crates/test-executor/tests/report_cli.rs  # CLI tests
```
Edits: root `Cargo.toml` members (add `"crates/testcase-reporting"`, root list at lines 2-33; keep alphabetical-ish order near `testcase-orchestration`), `crates/test-executor/Cargo.toml` (add `testcase-reporting = { path = "../testcase-reporting" }`), `crates/README.md` (Layer 3 table, ~lines 59-61 and 96-97, plus a crate section after `testcase-verification` at ~line 197), `Cargo.lock` (automatic).

`Cargo.toml` dependencies for the new crate: `testcase-models`, `testcase-common`, `testcase-verification` (path); `anyhow`, `chrono`, `serde_yaml` (workspace); dev: `tempfile`. Layering note: Layer 3 depending on Layer 3 (`testcase-verification`) is already precedent (`testcase-ui` consumes it, crates/README.md:205). CONTEXT.md lists exactly these dependencies. Nothing else may depend on `testcase-reporting` (D-09) [VERIFIED: codebase].

Proposed public API (names are Claude's discretion):
```rust
// legacy.rs  (moved verbatim; signatures unchanged except visibility pub)
pub fn read_test_runs(paths: &[PathBuf]) -> Result<Vec<TestRunRecord>>;
pub fn load_test_cases_strict(dir: &Path) -> Result<Vec<TestCase>>;
pub fn load_test_case(yaml_file: &PathBuf) -> Result<TestCase>;   // see note below

// load.rs
pub struct LoadedRun { pub path: PathBuf, pub record: TestRunRecord, pub started: DateTime<Utc> }
pub enum InputStatus { Loaded, Skipped(String) }
pub struct InputEntry { pub path: PathBuf, pub kind: InputKind /* Matrix | Run */, pub status: InputStatus }
pub struct Reconciliation { pub inputs: Vec<InputEntry>, pub notes: Vec<String> }
pub struct LoadedInputs { pub matrix: (PathBuf, TraceMatrix), pub runs: Vec<LoadedRun>, pub reconciliation: Reconciliation }
pub fn load_inputs(matrix: &Path, run_paths: &[PathBuf], schemas_root: &str) -> Result<LoadedInputs>;

// select.rs
pub struct SelectedResult { pub test_case_id: String, pub verdict: Verdict, pub run_id: String, pub build_id: String,
                            pub problem_report_id: Option<String>, pub verification_result_ref: Option<String> }
pub enum RunUse { Used { decided: usize }, Excluded { reason: String } }
pub struct RunSelection { pub results: BTreeMap<String, SelectedResult>, pub runs: Vec<RunReport>, pub builds: BTreeSet<String> }
pub fn select_results(runs: &[LoadedRun]) -> RunSelection;
```
`load_test_case` (main.rs:304-313) is used by about 12 call sites in main.rs (lines 448, 560, 842, 872, 1029, ...). Recommended: move its body into `legacy.rs` as the only implementation and leave a one-line wrapper in main.rs so none of the other call sites change. Keep the `"Failed to load and validate YAML file: {path}"` context string and the `validate_classification` mapping, because `an_unreadable_test_case_is_an_error_not_a_silent_gap` asserts on `"1 of 2 YAML files"` and `broken.yaml` [VERIFIED: codebase, trace_matrix_cli.rs:155-172]. `load_all_yaml_files_from_dir` (main.rs:431-462) is NOT a matrix/report loader (it warns and continues, used by dependency resolver at 470) and is not moved.

## Where Each Decision Is Enforced

| Decision | Enforcement point |
|----------|-------------------|
| D-01 | `select::select_results`: iterate runs sorted by (`started` ascending, then `run_id`); for each result with verdict != `NotExecuted`, overwrite `results[test_case_id]`. Last write wins = latest wins. Compare parsed `DateTime`, never strings (offsets like `+02:00` break lexical order). |
| D-02 | Same loop: overwrite is unconditional on pass/fail/error, so pass after fail replaces it and fail/error after pass replaces it. No `max()` on verdict rank. Add table-driven test. |
| D-03 | Same loop: `Verdict::NotExecuted` results `continue`; runs lacking the id are never visited. Record counts per run (`decided`, `superseded`, `not_executed`) for D-05. |
| D-04 | `RunSelection.builds` = set of `build_id` of runs with `decided > 0`; no build filter anywhere. Printed in reconciliation. |
| D-05 | `RunReport` per loaded run: Used (decided >= 1; also show how many of its results were superseded or not_executed) or Excluded with reason ("every result superseded by a later run", "only not_executed results", "no results"). A run can be Used for some cases and still list superseded cases. |
| D-06 | `load::load_inputs`: matrix via `load_and_validate_yaml::<TraceMatrix, _, _>(path, schemas_root)`; each run via `load_and_validate_yaml::<TestRunRecord, _, _>`. Collect ALL failures (like `load_test_cases_strict`, main.rs:557-574) then bail once, so the user sees every bad file; each message includes the path (the loader's context already does). |
| D-07 | `Reconciliation.inputs`: one entry per discovered file (matrix; each file under each `--runs` path). Loaded or Skipped(reason). Skipped reasons: "not a YAML file", "type is `X`, not test_run", "no `type` field". Printed to stdout/`--output` at the top of the report. |
| D-08 | `load_inputs`, after all runs load and before selection: build `BTreeMap<run_id, Vec<path>>`; any entry with 2+ paths bails with `duplicate run_id RUN-1 in A.yaml and B.yaml`. Dedupe by `fs::canonicalize` first so the same file reachable via two `--runs` arguments is not a false duplicate. |
| D-09 | New `Cargo.toml`; only `crates/test-executor/Cargo.toml` references it. Verify with `cargo tree -p testcase-reporting --invert`. |
| D-10 | New `Commands::Report` variant next to `Commands::TraceMatrix` (main.rs:140), dispatch arm next to main.rs:1277. |
| D-11 | `legacy.rs` bodies are copied from main.rs:492-528 and 532-576 unchanged; main.rs calls `testcase_reporting::legacy::*`. `trace_matrix()` (main.rs:613) and `regression_select` (main.rs:696) switch their calls. |

## Selection Algorithm (D-01..D-05)

```rust
// Source: derived from CONTEXT.md D-01..D-03; sketch only
let mut order: Vec<&LoadedRun> = runs.iter().collect();
order.sort_by(|a, b| a.started.cmp(&b.started).then_with(|| a.record.run_id.cmp(&b.record.run_id)));
for run in order {
    for r in &run.record.results {
        if r.verdict == Verdict::NotExecuted { /* count, continue */ continue; }
        results.insert(r.test_case_id.clone(), SelectedResult::from(run, r)); // later overwrites earlier
    }
}
// second pass: a run is Used if any selected result has its run_id, else Excluded with a reason
```
Demo case (CONTEXT specifics): case T1 pass in RUN-1 (t1), fail in RUN-2 (t2), not_executed in RUN-3 (t3). Result: T1 = fail from RUN-2; RUN-3 listed as not affecting the result. Note that if RUN-3 holds only that not_executed result, RUN-3 is Excluded ("only not_executed results").

## Strict Run Loader Behaviour (fail-closed vs. skip)

Existing behaviour, main.rs:492-528: explicit file that does not parse as a run -> bail; file inside a directory that does not parse -> skipped silently; `type != test_run` -> skipped silently; non-YAML files skipped silently. The new loader keeps directory scanning non-recursive and sorted (as main.rs:496-505) and:
1. Dedupe paths by canonical path, then sort.
2. Non-`.yml`/`.yaml` file: Skipped("not a YAML file") (listed, not dropped).
3. Peek the file with `serde_yaml::from_str::<serde_yaml::Value>`: YAML syntax error -> FAIL naming file. `type` present and not `test_run` -> Skipped("type is 'X', not test_run"). No `type` -> see Open Question Q1.
4. `type: test_run` -> `load_and_validate_yaml` (schema required: `run_id, build_id, environment, tester, started_at, results`, test-run.schema.json) -> failure makes the command fail.
5. Explicitly named file (not found via a directory) that is not a test_run -> FAIL (matches main.rs:521-523 spirit).
6. Parse `started_at` with `DateTime::parse_from_rfc3339`; failure -> FAIL naming file and value. Do not rely on the schema's `format: date-time` (this research did not verify whether the validator enforces `format`) [ASSUMED].

Matrix: single file, `type: trace_matrix`, schema `tcms/trace-matrix.schema.json` (trace_matrix.rs:286-287), required `id, links`. Schema files carry no `additionalProperties: false`, so the `type`/`schema` envelope fields are accepted [VERIFIED: ran python against both schemas]. Schemas root is the relative string `"schemas/"` (current pattern, main.rs:305), so the CLI must run with cwd at the repo root; tests do this via `.current_dir(CARGO_MANIFEST_DIR/../..)` (trace_matrix_cli.rs:23).

## CLI Skeleton (CLI-01)

Proposed, to sit beside `TraceMatrix`:
```
test-executor report --matrix <FILE> --runs <PATH>... [--kind <KIND>]... [--output <FILE>]
```
- `--matrix` required; `--runs` repeatable and required (>=1; a report with no runs would show everything NOT RUN, which Phase 2 handles, but a missing flag is more likely an error: confirm in Q3).
- `--kind` via `clap::ValueEnum`: `traceability`, `summary`, `gaps`, `problems`. Phase 1 validates and echoes the selection but renders no report body. Print an explicit line such as `Report kinds selected: traceability (renderer not yet available)` so output is not mistaken for a report.
- Output: reconciliation block goes to stdout or `--output` (CONTEXT: "in the report header, not only on stderr"). Errors go to stderr with exit 1 via `anyhow` return from `main` (main returns `Result<()>`, main.rs:999).
- CLI-02 (exit code 2) is Phase 5; do not add `--fail-on-gaps` now.
- The global flag `--no-audit` precedes the subcommand (tests call `["--no-audit", "trace-matrix"]`, trace_matrix_cli.rs:21); keep that convention in `report_cli.rs`.

## Test Plan

Must keep passing (unchanged): all of `crates/test-executor/tests/trace_matrix_cli.rs` (360 lines; notably `runs_link_through_run_ids` which requires `notes.yaml` be skipped, and the unreadable-test-case test), `regression_select_cli.rs`, `coverage_import_cli.rs`, `independence_cli.rs`, and the three unit tests at main.rs:1548-1601 (they stay with `read_matrix_requirements`).

New unit tests in `testcase-reporting` (table-driven where noted):
- `select`: demo case (pass/fail/not_executed gives fail from RUN-2, RUN-3 listed as non-affecting); pass-after-fail gives pass; error-after-pass gives error; fail-after-error; omitted case unchanged; not_executed-only run Excluded; out-of-order input files give same result (order independent); offsets (`+02:00` vs `Z`) compared by instant; equal `started_at` tie-break by `run_id` is deterministic; multiple builds all listed (D-04).
- `load`: malformed YAML fails naming the file; schema-invalid run (missing `tester`) fails naming file; bad `started_at` fails; duplicate `run_id` names both files (D-08); same file via dir and file argument is not a duplicate; non-run YAML in a directory appears as Skipped with reason; all failures collected in one message.
- `legacy`: move existing `read_test_runs`/`load_test_cases_strict` behaviour with a tiny regression test (silent skip still works there).

New CLI tests `crates/test-executor/tests/report_cli.rs` (model on trace_matrix_cli.rs helpers: `write`, `setup`, `current_dir` at workspace root): success prints reconciliation with discovered/loaded/skipped counts; invalid matrix exits 1 naming file; invalid run exits 1 naming file; duplicate run_id exits 1 naming both files; demo case output lists RUN-3 under excluded or "no effect"; `--output` writes the block to the file.

Framework: cargo test (existing). Quick: `cargo test -p testcase-reporting`; `cargo test -p test-executor --test report_cli --test trace_matrix_cli`. Full per CLAUDE.md: `make build`, `make lint`, `make test`, `make coverage-e2e`, `make acceptance-test`.

## Don't Hand-Roll

| Problem | Use Instead |
|---------|-------------|
| Schema validation and file-named errors | `load_and_validate_yaml` (D-06) |
| RFC 3339 parsing/ordering | `chrono::DateTime::parse_from_rfc3339`, compare instants |
| YAML output of structs (later phases) | `testcase_common::to_yaml_string` (yaml_output.rs:83) |
| Deterministic ordering | `BTreeMap`/`BTreeSet`, sorted file lists |

## Common Pitfalls

1. **Fixing `read_test_runs` in place** breaks `runs_link_through_run_ids` and changes `trace-matrix`. Move it unchanged; add a separate strict loader.
2. **String compare of timestamps** mis-orders mixed offsets. Parse first.
3. **False duplicate `run_id`** when a directory and a file inside it are both passed to `--runs`. Canonicalize and dedupe paths.
4. **Fail on first error only** hides other bad inputs. Collect all, bail once (pattern: main.rs:557-574).
5. **Relative `schemas/` root**: the command only works from the repo root, as the other subcommands do. Tests must set `current_dir`.
6. **Pulling `requirement-import` into the new crate** drags `jira-client` into Layer 3. Keep `read_matrix_requirements` in test-executor.
7. **Records the matrix names but that were not loaded**: `TraceLink.run_ids` (trace_matrix.rs:145) may name runs absent from `--runs`. Recommended: list as a reconciliation note ("matrix references run RUN-9, not loaded"), not an error. Likewise results for test case IDs absent from the matrix: list a count in the notes. Both keep with D-07's "nothing dropped silently" (see Q4).
8. **`coverage_import` reads the matrix with plain `serde_yaml::from_str` and no schema** (main.rs:761). Out of scope for this phase; note as follow-up so nobody assumes all matrix reads are strict.

## Open Questions

### Blocking planning
None. All decisions needed to plan are made; the items below have a recommended default the planner can adopt.

### Non-blocking (recommended defaults)
- **Q1: A YAML in a `--runs` directory with no `type` field** (e.g. the existing `notes.yaml: just: some other yaml`). Fail-closed says fail; D-07 permits "skipped with a reason". Recommended default: syntax error fails, a mapping without `type` or with another `type` is Skipped with reason, so a directory shared with other YAML still works and nothing is silent. Confidence MEDIUM; owner may prefer failing.
- **Q2: Equal `started_at` across runs.** D-01 is silent. Default: tie-break by `run_id` ascending (later id wins), deterministic, and list a reconciliation note when a tie decided a result.
- **Q3: Are `--runs` required, and the same test case twice inside one run?** Defaults: `--runs` required (>=1); a duplicate `test_case_id` within one run fails the command naming the file (conflicting verdicts in one record is invalid evidence). Neither is in CONTEXT.
- **Q4: Include the two extra reconciliation notes** (matrix-referenced runs not loaded; results for cases not in the matrix)? Default yes, as informational, not failures.
- **Q5: `--kind` list and default** until Phase 2/3 define renderers. Default: accept the four names, no default selection beyond "all", print the "not yet rendered" line.

## Environment Availability

| Dependency | Available | Notes |
|------------|-----------|-------|
| cargo 1.99.1 | yes | `cargo test -p test-executor --test trace_matrix_cli` FAILED to build in this session: `openssl-sys` needs `libssl-dev` (not installed). So the baseline of the existing tests could not be run here. Install `libssl-dev` (needs privileges; not done by this research) before planning verification steps depend on it. |
| uv | yes (/opt/python/bin/uv) | |
| shellcheck | no | Matters only when shell scripts are added (Phase 5) |
| strictdoc, test-plan-documentation-generator (TPDG), cargo-llvm-cov | no (not found on PATH) | So `make coverage-e2e` and `make acceptance-test` cannot run here; record as not run |

Gates runnable here once libssl-dev is installed: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`. Record the rest as not run in the phase summary.

## Validation Architecture

| Property | Value |
|----------|-------|
| Framework | cargo test (Rust, edition 2021) |
| Config | none beyond Cargo.toml |
| Quick run | `cargo test -p testcase-reporting` |
| Full | `cargo test --workspace --all-features` |

| Req | Behaviour | Test | Exists |
|-----|-----------|------|--------|
| LOAD-01 | invalid matrix/run fails naming file | unit in `load.rs` + `report_cli.rs` | Wave 0 |
| LOAD-02 | reconciliation lists discovered/loaded/skipped | `report_cli.rs` | Wave 0 |
| LOAD-03 | selection policy, demo case | unit in `select.rs` + CLI | Wave 0 |
| LOAD-04 | duplicate `run_id` fails naming both | unit + CLI | Wave 0 |
| CLI-01 | `report` subcommand accepts matrix, runs, kinds | `report_cli.rs` | Wave 0 |

Wave 0 gaps: `crates/testcase-reporting` (crate, lib, tests), `crates/test-executor/tests/report_cli.rs`, fixtures for runs and a matrix (build the matrix by writing YAML in the test, same style as trace_matrix_cli.rs; or generate it by running `trace-matrix` first).

## Security Domain

| ASVS | Applies | Control |
|------|---------|---------|
| V5 Input validation | yes | Schema validation through `load_and_validate_yaml` before deserialize; fail closed |
| V2/V3/V4/V6 | no | No auth, sessions, access control or crypto in this phase |

Threat: malformed or hostile YAML (tampering) -> schema validation plus typed deserialization; path traversal not relevant (user supplies paths to a local CLI). Directory scan is non-recursive, so no symlink-loop risk beyond what `fs::read_dir` already has. [ASSUMED: no hostile-input requirement beyond this]

## Assumptions Log

| # | Claim | Risk if wrong |
|---|-------|---------------|
| A1 | The JSON-schema validator may not enforce `format: date-time`, so explicit RFC 3339 parsing is needed | Low: extra check is harmless if it does |
| A2 | Q1-Q5 defaults are acceptable to the owner | Medium: Q1 changes whether mixed directories work |
| A3 | Layer 3 -> Layer 3 dependency on `testcase-verification` is acceptable (CONTEXT lists it; `testcase-ui` precedent) | Low |

## Sources

### Primary (HIGH, read in this session)
- crates/test-executor/src/main.rs (lines 140-176, 304-313, 431-462, 491-611, 613-678, 761, 1277, 1548-1601)
- crates/test-executor/tests/trace_matrix_cli.rs
- crates/testcase-common/src/yaml_loader.rs:38-88, envelope.rs:31-150
- crates/testcase-verification/src/test_run.rs, trace_matrix.rs (139-159, 208-214, 286-287), lib.rs:20-31
- schemas/tcms-schemas/tcms/test-run.schema.json, trace-matrix.schema.json
- Root Cargo.toml, crates/id-resolver/Cargo.toml, crates/requirement-import/Cargo.toml, crates/README.md

### Not consulted
No web or Context7 lookups were needed; no external libraries are involved.

## Metadata

**Confidence:** code facts HIGH; selection algorithm HIGH (direct from D-01..D-03); skip-vs-fail policy for untyped YAML MEDIUM (Q1).
**Research date:** 2026-10-30. **Valid until:** the next change to `main.rs` loaders or the schemas.
