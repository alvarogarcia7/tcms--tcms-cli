# Domain Pitfalls

**Domain:** Traceability and verification reports added to an existing YAML test harness (TCMS)
**Researched:** 2026-10-30
**Scope:** What goes wrong when ADDING reports on top of the trace matrix (`trace_matrix.rs`) and test-run records (`test_run.rs`). Existing CONCERNS.md items are not repeated.
**Confidence:** HIGH for pitfalls grounded in the current code (read directly); MEDIUM for general domain patterns (traceability practice, drawn from training knowledge, not re-verified online).

Key observation: the current `build_matrix` records *structural* coverage only (a test case of the right type exists). It carries no verdict. The new reports are the first place PASS/FAIL meets requirements, so they are where the "false covered" claims will be introduced.

## Critical Pitfalls

### Pitfall 1: "Covered" means "a test case exists", not "a test passed"
**What goes wrong:** `normal_range_covered` / `robustness_covered` are true when any non-obsolete linked test case has that type, whatever its run outcome. A report that reuses these flags shows a requirement as covered when its only tests failed, errored, or never ran. `run_ids` has the same flaw: a run is attached if any linked case has a verdict other than `not_executed`, including `fail` and `error`.
**Why it happens:** The flags answer the glossary rule (a test must exist). The report author reads "covered" as "verified".
**Consequences:** Direct violation of the Core Value ("gaps shown explicitly rather than hidden"). Reviewers sign off on unverified requirements.
**Prevention:**
- Define two separate, named concepts in the report model: *test-coverage* (a test exists, from the matrix) and *verification status* (per-requirement roll-up of verdicts). Never print a bare "covered".
- Roll-up rule, written down and unit-tested: requirement is VERIFIED only if every required test type (normal-range, robustness) has at least one linked case whose latest-run verdict is `pass` AND no linked live case is `fail`/`error`/`not_executed`/absent. Any other state gets its own label (FAILED, NOT RUN, PARTIAL, NO TESTS).
- Table-driven tests: one row per combination of verdicts across normal-range and robustness (pass/pass, pass/fail, pass/not-run, pass/missing, error/pass, ...). The case "normal passes, robustness fails" must not read as PASS.
- Treat `error` as distinct from `fail` (hook abort, not product failure) but never as pass.
**Detection:** A requirement with a failed case shown green; summary counts where covered + gaps != total requirements.
**Phase:** Verdict roll-up/domain model phase (first phase, before any rendering).

### Pitfall 2: Results attached to the wrong run (no run selection policy)
**What goes wrong:** The matrix takes every run passed on the command line and unions their `run_id`s per requirement. A report that derives a PASS/FAIL from "the runs for this link" will mix results from different builds, environments or baselines: an old passing run masks a new failure, or vice versa. Directory input (`read_test_runs`) picks up every YAML in a folder, including stale runs from earlier builds.
**Why it happens:** `TestRunRecord` has `build_id`, `environment`, `started_at`, but the matrix has only an optional `baseline_id` and nothing checks that runs belong to it. No rule says which run "wins" when a test case appears in several.
**Consequences:** Non-reproducible reports (result depends on what else sits in the folder); false PASS from a superseded run.
**Prevention:**
- Make run selection an explicit, printed input: report header lists each run_id, build_id, environment, started_at, tester that was used, and every run that was found but excluded and why.
- Choose one policy and document it (recommended: all runs in a report must share one `build_id`, otherwise fail with a clear error unless `--allow-mixed-builds`; within a build, the latest `started_at` per test case wins and earlier ones are listed as superseded, not silently dropped).
- Duplicate `run_id` across files is an error, not a merge.
- Do not infer the run from file mtime or directory order (BSD/GNU `ls` ordering differs; `files.sort()` is lexical, not chronological).
- Parse `started_at` as RFC 3339 for ordering; never compare strings from different offsets.
**Detection:** Same inputs, different file order, different report; a test case with two verdicts in one report.
**Phase:** Run-selection/data-loading phase (same phase as Pitfall 1, must precede rendering).

### Pitfall 3: Stale results after test cases change
**What goes wrong:** A test-run result records `test_case_id` only. If the test case YAML is edited (steps, expected values, requirements list, test_type, status) after the run, the old PASS is still attached to the new definition. The report says "REQ-1 verified by TC-5: PASS" for a test that was never run in its current form. Also the reverse: requirements added to a test case after the run make a requirement look verified by a run that never targeted it.
**Why it happens:** The run record carries `verification_result_ref` but no content hash or version of the test case; trace links are computed from today's test case files while verdicts come from yesterday's run.
**Consequences:** Silent staleness. Auditable evidence is the main selling point of TCMS, and this undermines it. Changing the shared test-run schema is out of scope, so the fix has to be done without it.
**Prevention:**
- Compare timestamps as a minimum: if a test case file (or its git last-commit time) is newer than the run's `started_at`, flag the result STALE and do not count it toward VERIFIED. Prefer git commit time or a content hash over mtime (checkouts and worktrees reset mtimes).
- Better: if the verification result referenced by `verification_result_ref` embeds the test case snapshot or a digest, compare it to the current definition. Verify during research of the verifier output whether such data exists; if not, record a digest in the report-side manifest and flag as a gap for a later schema change request (do not change the shared schema in this milestone).
- Report lists "results with no matching current test case" (test case deleted/renamed/obsolete) as ORPHANED results, not dropped.
- Test case present in the run with `status: obsolete` must still be shown (as excluded), not just omitted as `build_matrix` does today.
**Detection:** Edit a test case after a run, regenerate: report must change state. Make this an E2E test.
**Phase:** Staleness/freshness phase after the roll-up model; flag for deeper research (what evidence of "which version ran" exists in the verifier output).

### Pitfall 4: Unreadable or unrecognised inputs silently dropped
**What goes wrong:** Three silent-drop paths exist or will be copied:
1. `load_all_yaml_files_from_dir` (used for dependency resolution) prints a warning and continues on a failed load. If a report author reuses it instead of `load_test_cases_strict`, a broken test case vanishes and its requirement shows as "no tests" (or, worse, the requirement is simply absent from counts).
2. `read_test_runs` swallows parse errors for files found via a directory (`Err(_) => {}`) and skips any record whose type is not `test_run`. A corrupted or schema-drifted run file means its results are missing and the tests appear NOT RUN with no explanation.
3. `read_requirements` (YAML branch) skips records lacking `id` or with a `type` other than HLR/LLR, and in a directory it reads non-YAML files (README, `.sdoc`) as ID-list text. A malformed requirement is dropped and the requirement count shrinks, which makes coverage percentages look better.
**Why it happens:** Tolerant loading is convenient for mixed directories; reports need the opposite (fail closed).
**Consequences:** The denominator shrinks or evidence disappears without a trace; coverage percentage is inflated.
**Prevention:**
- Report layer must use strict loaders and have a reconciliation block in every report: files discovered, files loaded, files skipped with reason. Fail the run (non-zero) if any file matching the input glob failed to parse, unless the user passes an explicit `--allow-skip` that is echoed in the report header.
- Add an "ignored records" count for requirements of other types (e.g. a SYS or TEST record) so skipping is intentional and visible.
- Invariant test: `requirements_in_input == requirements_in_report` and `loaded_test_cases + failed == discovered_files`. Assert this in every report builder and in E2E.
- Negative fixtures: a corrupt test case YAML, a corrupt run file, a requirement record missing `id`, and a non-YAML file in the requirements directory; each must produce a visible error or a listed skip, never a clean report.
**Detection:** Compare counts in the report against `ls | wc -l` of the inputs; any difference must be explained in the report itself.
**Phase:** Data-loading phase (with Pitfall 2). Add the invariants early; they protect every later phase.

### Pitfall 5: Report output drifts from its schema
**What goes wrong:** The machine-readable report is written by hand-assembled structs, and nothing validates it before it is written. Structs get fields added for rendering convenience; `skip_serializing_if` omits required fields; enums gain variants (new verdict) that the vendored schema does not know. The project already has a precedent for validate-on-load (`load_and_validate_yaml`) but the trace-matrix command writes YAML with `to_yaml_string` and no validation of its own output.
**Why it happens:** Validation is only on the read path; the schemas are shared with other tools and are vendored (`schemas/tcms-schemas`), so a local struct and the vendored schema can diverge when the vendor version updates (`TCMS_SCHEMAS_VERSION`).
**Consequences:** Downstream consumers (TPDG, other tooling) reject or misread the report; failures appear in someone else's pipeline.
**Prevention:**
- Validate every generated machine-readable report against its schema before writing; refuse to write an invalid file (as `TestRunRecord::build` already does: "never writing an invalid record"). Test this by feeding builder output through the validator in unit tests for every report type.
- Decide per report whether it has a schema at all. Human-readable summary/gap/problem lists are projections of existing schema-validated documents; do not invent new top-level schemas (out of scope: no schema changes). Keep derived fields (roll-up status, staleness) in a separate, locally owned report document with its own schema in the repo, clearly not one of the shared schemas.
- Include `type` and `schema` envelope fields (project rule) in every YAML output.
- Pin a test that fails when `TCMS_SCHEMAS_VERSION` changes without re-running schema-conformance tests; round-trip sample files from `schemas/tcms-schemas/samples`.
- Beware `doc_type`: the matrix writes `trace_matrix` while the schema file is named `trace-matrix`; keep the exact strings under test, not a copy-pasted literal.
**Detection:** CI step that runs the generator on fixtures and validates output with `validate-yaml`; failure on any enum or required-field mismatch.
**Phase:** Report data-model phase (define) and every output phase (enforce). Standard patterns; no deeper research needed beyond reading the vendored schemas.

### Pitfall 6: Percentages and counts that hide the denominator
**What goes wrong:** "92% covered" with the denominator silently excluding obsolete tests, orphan results, skipped requirements, or not-executed cases. A summary counting only results present in runs (not the full set of live test cases) reports 100% pass when half the suite has no result at all. `Verdict::of` also maps "not overall_pass and no failed steps" to `not_executed`, so partially-run cases fall into that bucket.
**Prevention:**
- Always compute counts against the *expected* universe (live test cases from the test-case directory, requirements from the requirement source), not against what appears in runs. Test cases with no result row are counted as NOT RUN.
- Show the four verdicts plus "no result" and "obsolete/excluded" so the buckets sum to the total; assert sum == total in code.
- Report pass-rate as `pass / expected` and also show `pass / executed` only if labelled.
- Per-requirement counts count each test case once even if it links to several requirements; a global summary must not double-count a case linked to N requirements. Label which unit (test cases vs. requirement-links) each number uses.
**Phase:** Summary-report phase.

## Moderate Pitfalls

### Pitfall 7: Reports regenerate non-deterministically
**What goes wrong:** Timestamps in IDs (`TM-<now>`), HashMap iteration, locale-dependent sorting, and `fs::read_dir` order (unspecified, differs on BSD/GNU filesystems) make two runs over the same artifacts differ. Diff-based review and snapshot tests become noisy and flaky.
**Prevention:** Sort everything explicitly (BTreeMap/BTreeSet as the matrix does); isolate "generated at" into one header field excluded from snapshot comparison; accept `--id` and `--generated-at` for reproducible output; use stable ordering by requirement ID, then test case ID.
**Phase:** Data-model phase; assert in E2E by running twice and diffing.

### Pitfall 8: Problem-report list loses or invents links
**What goes wrong:** `problem_report_id` is optional and only meaningful for fail/error. A list built by iterating runs omits failed cases with no problem report (the most important ones: unreported failures), repeats a case once per run, or shows a problem report from an older run for a case that has since passed.
**Prevention:** Drive the list from failed/errored *effective* verdicts (after run selection), show "NONE RAISED" explicitly where the ID is missing, and flag a `problem_report_id` attached to a pass verdict as inconsistent rather than ignoring it. Do not resolve IDs against Jira in this milestone (TCMS-42 owns that); print the ID verbatim.
**Phase:** Problem-report-list phase.

### Pitfall 9: ID normalisation mismatches between sources
**What goes wrong:** Requirement IDs come from StrictDoc/Jira/text lists and test cases carry `requirements` lists; matching is exact string equality. Case, whitespace or prefix differences (`HLR-1` vs `hlr-1`, trailing space) turn a link into an "unknown reference" and the requirement into "untraced". Conversely, the `HLR`/`LLR` level is guessed from the ID prefix in text lists, so a mis-prefixed ID gets the wrong level.
**Prevention:** Normalise once, at the loader (trim, consistent case policy documented), keep the original for display. Surface `unknown_references` in the report body rather than only on stderr (today they are only printed as warnings and are not in the matrix). Use the legacy `requirement` string never; keep the current rule and say so in the report footer.
**Phase:** Data-loading phase.

### Pitfall 10: Second renderer and unvalidated rendering path
**What goes wrong:** The project decision is to reuse the TPDG render path, but TPDG consumes container YAML shaped for verification results, not requirement-centric tables. Shoe-horning a traceability table into that container, or writing a quick Markdown formatter beside it, creates two pipelines whose output disagrees. TPDG is also an external, unpinned binary that is not installed in the dev environment, so the rendering step can't be verified locally (CONCERNS.md notes this dependency; this pitfall is specifically that new report formats make it load-bearing).
**Prevention:** In the first phase, check what TPDG can actually render (research flag) before committing to the reuse decision. Keep report logic as a pure function: model -> structured document; rendering as a thin final step; test the model and a plain-text/Markdown projection without TPDG, so the correctness tests do not depend on an uninstalled tool. Escape requirement titles and test descriptions when emitting AsciiDoc/Markdown (pipes in tables, leading `=`/`#`).
**Phase:** Rendering phase; flag for deeper research.

### Pitfall 11: Exit codes that say success for a failing or incomplete report
**What goes wrong:** The trace-matrix command exits 1 on coverage gaps unless `--no-coverage-check`; a new report command that always exits 0 lets CI pass with gaps and failures. Conversely, making gaps a hard failure on every use blocks users who just want to read the report.
**Prevention:** Distinguish exit codes: usage/IO/schema error, report generated with gaps, report generated clean. Always write the report before exiting non-zero. Document in `--help`. Test exit codes in E2E.
**Phase:** CLI integration phase.

### Pitfall 12: Quality gates not runnable, so bugs ship unverified
**What goes wrong:** `make test` cannot run in this environment (uv, shellcheck, StrictDoc, TPDG missing) and the pre-commit rules demand acceptance tests and 70% E2E coverage. New report code ends up tested only by unit tests, with E2E and acceptance coverage claimed but never run.
**Prevention:** Put correctness-critical logic (roll-up, selection, staleness, reconciliation) in a library crate with plain `cargo test` coverage, independent of external tools. Add new crates/commands to `mk/incremental.mk` E2E mapping. Record in each phase verification which gates actually ran.
**Phase:** All; plan in phase 1.

## Minor Pitfalls

### Pitfall 13: Layering violations when placing report code
**What goes wrong:** Putting report logic in the binary (`test-executor/main.rs`, already large) or having `testcase-verification` depend on UI/render crates. Existing code already deviates; do not add to it.
**Prevention:** Model and roll-up in `testcase-verification` (layer 3); orchestration/rendering glue in layer 4 or the binary; check `cargo tree -p <crate> --invert` before adding dependencies. Do not copy into `testcase-manager` duplicates.

### Pitfall 14: Bash 3.2 / BSD-GNU breakage in helper scripts
**What goes wrong:** E2E wrapper scripts for the reports use `readlink -f`, `sed -r`, `grep -P`, associative arrays or GNU `date`. **Prevention:** Follow the AGENTS.md compatibility list, use `scripts/lib/logger.sh`, run `make verify-scripts`.

### Pitfall 15: Timestamps with mixed time zones in headers
**What goes wrong:** `started_at` / `finished_at` are RFC 3339 strings; displayed durations or ordering computed on raw strings break across offsets. **Prevention:** Parse with chrono, display UTC.

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Data loading (requirements, test cases, runs) | Silent drops (4), ID mismatches (9), duplicate run IDs (2) | Strict loaders, reconciliation block, invariant tests |
| Run selection and freshness | Wrong run (2), stale results (3) | Single build per report, latest-wins rule printed, staleness flag via git time/digest |
| Verdict roll-up model | False "covered" (1), hidden denominator (6) | Separate test-coverage vs verification status; table-driven tests |
| Traceability report | Covered vs verified wording (1), orphans (3) | Explicit labels, orphan section |
| Run summary report | Denominator and double counting (6) | Buckets sum to expected universe |
| Coverage gaps report | Gaps defined by existence only, obsolete tests invisible (1, 3) | Show gap type and obsolete exclusions |
| Problem report list | Missing/invented links (8) | Drive from effective verdicts, show "none raised" |
| Output/schema | Drift from schema (5), nondeterminism (7) | Validate before write, local schema for derived fields, sorted output |
| Rendering (TPDG) | Second renderer, unavailable tool (10) | Research TPDG capability first; model/render split |
| CLI/CI integration | Exit codes (11), unrunnable gates (12), layering (13) | Library-first, E2E mapping, documented exit codes |

## Research Flags

- Staleness (Pitfall 3): needs phase-level research into what the verifier output and `verification_result_ref` actually record about the executed test case version.
- Rendering (Pitfall 10): needs confirmation of what TPDG can render for non-verification documents; not installed here, so unverified.
- Run selection policy (Pitfall 2) is a product decision for the owner; surface it as an open question before planning.

## Sources

- `crates/testcase-verification/src/trace_matrix.rs` (build_matrix, read_requirements): HIGH
- `crates/testcase-verification/src/test_run.rs` (Verdict::of, TestRunRecord fields): HIGH
- `crates/test-executor/src/main.rs` (`read_test_runs`, `load_test_cases_strict`, `load_all_yaml_files_from_dir`, `trace_matrix`): HIGH
- `.planning/PROJECT.md`, `.planning/codebase/CONCERNS.md`: HIGH
- General traceability practice (coverage vs verification status, stale evidence, baselines): MEDIUM, training knowledge, not re-verified online
