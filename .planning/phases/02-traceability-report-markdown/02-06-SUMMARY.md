# Phase 2 Plan 06: Traceability report shapes A and B Summary

Markdown shapes A (requirement, cases as `ID (status)`, final verdict) and B (requirement verdict, then per-case status tables) as views over the existing `ReportModel`, selected by `test-executor report --kind traceability --shape a|b`.

## Commits

| Commit | Message |
|--------|---------|
| 03e7a26 | feat(02-06): add traceability shapes A and B over ReportModel |
| c88f81e | feat(02-06): add --shape a|b to report with CLI tests and docs [ci skip] |

## What changed

- `crates/testcase-reporting/src/render_shapes.rs` (new): `render_shape_a`, `render_shape_b`. Same document top and `## Document:` headings as the grid; `No traceable content` for an empty document; every cell passes through `render_markdown::cell`.
- `crates/testcase-reporting/src/render_markdown.rs`: `inputs_block` made `pub(crate)`. No other change; the grid output is unchanged.
- `crates/testcase-reporting/src/lib.rs`: `pub mod render_shapes;`.
- `crates/test-executor/src/main.rs`: `--shape grid|a|b` (default `grid`), `ReportShape` enum, usage check before any input is read.
- `crates/test-executor/tests/report_cli.rs`: CLI tests for shape output, grid identity, determinism, unknown shape, and grid-only usage errors. Existing tests unchanged (additive diff only).
- `docs/TRACEABILITY.md`: "Report shapes" section.

## Decisions applied

- D-02-6-1: D-04 and D-08 apply to the grid only; shapes keep headings, Inputs and the empty-document rule.
- D-02-6-2: `--shape a|b` is a usage error (exit 2) unless `--kind traceability` is the only kind.
- D-02-6-3: shape B case rows show each case's own outcome, not the requirement verdict.
- D-02-6-4: grid-only options (every `--kind` other than traceability) are rejected with the `--shape` option; the error names both `--shape` and `--kind`.

## Deviations

- Tasks 1 and 2 were committed together as 03e7a26. Task 1 alone leaves failing tests, which CLAUDE.md forbids committing.
- Task 3 was committed as c88f81e, after the workspace test run.
- `unknown_shape_value_is_a_usage_error` asserts that stderr names `xyz`, so it does not pass only because clap rejects unknown arguments.

## Gates

- `cargo fmt --check -p testcase-reporting -p test-executor`: pass.
- `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings`: pass.
- `cargo test -p testcase-reporting -p test-executor`: pass (137 unit tests in testcase-reporting; 56 in report_cli).
- `cargo test --all-features --workspace`: exit 0.
- Not run, per instructions: `make test`, `make acceptance-test`, `make coverage-e2e`.

## Acceptance criteria

All met: shape A and B CLI output, grid byte identity, determinism for the same `--generated-at`, `not recorded` in both shapes, and exit code 2 for usage errors. `model.rs` and the grid code are untouched.

## Known Stubs

None.

## Self-Check: PASSED
