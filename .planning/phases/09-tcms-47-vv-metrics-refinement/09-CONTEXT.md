# Phase 9 context: TCMS-47 V&V metrics refinement

Source of truth: the acceptance criteria of TCMS-47, TCMS-47.1, TCMS-47.2 and TCMS-47.3 in
`backlog/tasks/`. This phase adds no scope beyond them.

## Goal

Finish the TCMS-47 family in tcms-cli so that a CI job, a document pipeline and Verdict produce the same
V&V figures from the command line:

1. TCMS-47.3 (plan 09-01, producer): combine the per-case results of one run into one run record
   (`runs/<RUN_ID>/test-run.yaml` and `verification-results.yaml`) that `tcms vv metrics` reads.
2. TCMS-47.2 (plan 09-02, consumer): `tcms vv metrics` reads `<DATA_ROOT>/vv-config.yaml`, adds
   `--stability-window <N>`, and reports where each parameter came from. Depends on 09-01.
3. TCMS-47.1 (plan 09-03, report): a new crate `tcms-vv-report` renders the metrics JSON as Markdown or
   AsciiDoc. It computes nothing. Depends on 09-02.

Plan order is the dependency order: 09-01 -> 09-02 -> 09-03. Each plan is test-first.

## Scope

In scope:
- `crates/tcms/src/vvcmd.rs`, `crates/tcms/src/vvmetrics.rs`, new modules for the combinator and the
  config file, `crates/tcms/tests/` and fixtures under `crates/tcms/tests/fixtures/`.
- New workspace member `crates/tcms-vv-report/` (TCMS-47.1).
- `docs/VV_METRICS.md`, `docs/data_flow/DATA_FLOW.md` section 4.5 (TCMS-47.1 AC 11 and 12).

Out of scope (from the backlog, not to be added):
- Verdict (tcms-web) switching to read the CLI output (ORCH-087.09).
- Browser-side calculations, PDF rendering, report endpoints in tcms-web.
- The failing-since rule (D-98).
- Traceability report (stays in `test-executor report`, TCMS-43).
- Publishing or validating against tcms-schemas result objects (TCMS-47 AC 11 stays unchecked until the
  published schema exists).

## What is done

- TCMS-47 (base): `tcms vv metrics` exists in `crates/tcms/src/vvcmd.rs` with `--root`, `--run`, `--plan`,
  `--gate` (default 90), `--flaky-threshold` (default 4), `--out`, `--now`. Gate and delta (D-95, D-96),
  V&V run figures, requirement coverage (D-116.3), stability (D-97), load summary are implemented in
  `crates/tcms/src/vvmetrics.rs` and `vvcmd.rs`.
- TCMS-47 ACs 1 to 10, 12 to 15 are checked in the backlog. AC 11 (schema validation in CI) is open.
- Fixtures: `crates/tcms/tests/fixtures/vv-metrics/sut-1/` (recorded, not edited) and
  `derived/` (seven labelled variants with `DERIVED_FROM`).
- Documentation: `docs/VV_METRICS.md`.
- The git history contains the merge of the TCMS-47.2 backlog branch (commit 1c90974). That merge
  added the task document only. No code for 47.2 is in the tree: there is no `vv-config` reader and no
  `--stability-window` flag.

## What is open

Each open item is either decided in the plans below or blocks a named acceptance criterion. Blocking
items are not silently dropped; the plan that owns them records the outcome.

- Q1 (naming, owner 09-02). TCMS-47.2 AC 2 names `parameters.stability_window_runs` (snake case), while
  TCMS-47 AC 10 asks for camelCase JSON. 09-02 uses the literal TCMS-47.2 spelling `stability_window_runs`.
- Q2 (missing schema, owner 09-02). `vv-config.schema.json` is not in the repo. `schemas/tcms-schemas/`
  has `tcms/test-run.schema.json` (used by 09-01) but no `vv-config` schema. TCMS-47.2 AC 7 is blocked
  until the schema is vendored. 09-02 Task 3 is a checkpoint for this.
- Q3 (missing schema, owner 09-01). `verification-results.yaml` has no schema in `schemas/`. 09-01
  validates `test-run.yaml` against the test-run schema and checks `verification-results.yaml` against
  the fields that `tcms vv metrics` reads (`test_results` with `test_case_id`, `total_steps`,
  `passed_steps`). The gap is recorded in the summary.
- Q4 (input layout, owner 09-01). The 63 per-case files come from
  `scripts/convert_verification_to_result_yaml.py` (type `test_result`). The repo has no 63-file fixture.
  09-01 derives a fixture from that script's output contract and records the derivation in the fixture
  README. The output path and file naming must be read from the script first.
- Q5 (command name, owner 09-01). The backlog does not name the combinator command. Planner choice:
  `tcms vv combine --root <DATA_ROOT> --run <RUN_ID>`, reviewed by the user at the checkpoint in 09-01.
- Q6 (behaviour change, owner 09-02). The stability window (default 12) limits the history to the last N
  runs up to and including the selected run. Today the history is all runs. Flaky counts can therefore
  change for data sets with more than 12 runs. The derived fixtures have fewer runs, so their expected
  figures do not change; SUT-1 has no flips, so it stays STABLE.
- Q7 (dependency, owner 09-03). TCMS-47.1 lists no backlog dependencies. The phase makes 09-03 depend on
  09-02 because the metrics JSON it reads gains the parameters block from 09-02.
- Q8 (gate default, owner 09-02). `--gate` currently has `default_value_t = 90`, so a file value cannot be
  told apart from the default. 09-02 changes it to an optional flag, keeping the same effective default.

## Quality gates

Per CLAUDE.md, each plan ends with `make build`, `make lint`, `make test`, and `make coverage-e2e`
(70 percent threshold). Nothing is committed in this phase; the user reviews the working tree.
