---
phase: 09-tcms-47-vv-metrics-refinement
plan: 02
subsystem: tcms-cli
tags: [vv-metrics, vv-config, stability-window]
requires: ["09-01"]
provides: ["vv-config.yaml reader", "--stability-window", "parameters block in vv metrics output"]
affects: [crates/tcms, docs/VV_METRICS.md]
key-files:
  created: [crates/tcms/src/vvconfig.rs, crates/tcms/tests/vv_config_cli.rs]
  modified: [crates/tcms/src/main.rs, crates/tcms/src/vvcmd.rs, docs/VV_METRICS.md]
requirements-completed: [TCMS-47.2]
metrics:
  completed: 2026-11-03
---

# Phase 9 Plan 02: vv-config.yaml and stability window Summary

`tcms vv metrics` reads `<DATA_ROOT>/vv-config.yaml` (gate and flaky threshold), resolves flag over file over default, limits the stability history to `--stability-window` runs (default 12), and reports each value with its source in a new `parameters` block.

## Commits
- e2f1fac test: nine failing tests and fixtures (RED, previous executor)
- 186d80d feat: vvconfig.rs, flags, parameters block, window (GREEN)
- addda99 docs: VV_METRICS.md, ignored AC 7 test

## Acceptance criteria
- TCMS-47.2 ACs 1 to 6: covered by the nine passing tests in `vv_config_cli.rs`, 5 unit tests in `vvconfig.rs`, and docs.
- **AC 7 (schema validation): BLOCKED.** No vv-config schema in the repo (CONTEXT Q2); none invented. Recorded as the ignored test `vv_config_validates_against_schema` with the reason string, and in docs. Only Rust checks (types, unknown keys, ranges) apply.

## Behaviour change
Stability history is now the last N runs (default 12) instead of all runs; documented in docs and the `stability_section` doc comment.

## Verification
- `cargo test -p tcms`: all pass except `vv_combine_cli` (8 failures, see below). vv_config_cli 9 passed, 1 ignored; vv_metrics_cli 19 passed.
- `make lint` (fmt + clippy): passed. `cargo build -p tcms`: passed.
- Not run: `make coverage-e2e` (cargo-llvm-cov not installed), `make acceptance-test` (test-plan-documentation-generator not installed), full-workspace `make test` / `cargo test --all-features` (only `-p tcms` run, time limits). Run these in CI.
- Environment note: `CARGO_HOME=/opt/cargo` is not writable here; used `CARGO_HOME=$HOME/.cargo cargo ... --offline`.

## Deviations from Plan
None in implementation. Plan's `<verification>` says "no commit made" (template text); tasks were committed individually as instructed.

## Deferred Issues (pre-existing, out of scope)
`vv_combine_cli` (from 09-01) fails 8 tests: fixture `crates/tcms/tests/fixtures/vv-combine/per-case/execution_logs/` is not in the repository (commit e34cc07 did not include it), so `stage_fixture` panics with NotFound. Needs fixing in 09-01 scope.

## Known Stubs
None.

## Self-Check: PASSED
