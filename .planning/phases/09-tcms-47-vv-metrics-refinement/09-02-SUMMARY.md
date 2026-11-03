# Phase 9 Plan 02: vv-config.yaml and --stability-window (TCMS-47.2) Summary

`tcms vv metrics` reads `<DATA_ROOT>/vv-config.yaml` (gate threshold, flaky threshold), takes `--stability-window N`
(default 12), and reports the value and source (file, flag or default) of each parameter in a `parameters` block.

## Commits

- `3468b08` feat(09-02): read vv-config.yaml and add --stability-window to tcms vv metrics [ci skip]
  (code, binary tests, config fixtures, golden output, ignored AC 7 test)
- `8b3131f` docs(09-02): document vv-config.yaml, stability window and parameters [ci skip]

The RED tests were not committed on their own: CLAUDE.md forbids committing failing tests. Test-first was kept in the
working tree (see Task record below), and the tests and code went into one commit.

## TCMS-47.2 acceptance criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Reads `<DATA_ROOT>/vv-config.yaml`; flag overrides file | MET | `file_value_used_when_no_flag`, `flag_overrides_file` |
| 2 | `--stability-window N` (default 12, at least 1); value in `parameters.stability_window_runs` | MET | `parameters_block_reports_window_source`, `stability_window_limits_the_history`, `stability_window_zero_is_refused`. The value is at `parameters.stability_window_runs.value`, per the plan's shape (see Notes) |
| 3 | Output reports source of each parameter | MET | `source` in every `parameters` entry; checked in the tests above |
| 4 | Missing file keeps behaviour; wrong type, unknown key, out-of-range name file and key | MET | `default_when_no_file_matches_previous_output` (golden), `wrong_type_names_file_and_key`, `unknown_key_names_file_and_key`, `out_of_range_names_file_and_key` |
| 5 | Tests for precedence, window in output, each error case | MET | `vv_config_cli.rs` (9 tests), unit tests in `vvconfig.rs` (11 tests) |
| 6 | `docs/VV_METRICS.md` documents file, flag and precedence | MET | new section "Per-product settings", "Stability window", options table |
| 7 | Validation against `tcms/vv-config.schema.json` | BLOCKED | No vv-config schema in the repository or any git ref (only the backlog task mentions it). Test `vv_config_fixtures_validate_against_the_schema` is `#[ignore]` with the reason. Fails when forced. No schema invented |

Definition of done (backlog): `cargo test -p tcms` passes, `cargo fmt --check` and `clippy -D warnings` are clean.

## Gates

- `cargo fmt --check -p tcms`: clean
- `cargo clippy -p tcms --no-deps --all-targets -- -D warnings`: clean
- `cargo test -p tcms`: all green (includes the 09-01 `vv_combine_cli` tests and the unchanged `vv_metrics_cli` tests); one ignored (AC 7)
- Not run, by instruction: `make test`, `make acceptance-test`, `make coverage-e2e`, `make lint`, `make build`. DEFERRED. No pipeline was awaited.

## Parameter sources

| Parameter | Flag | File | Default |
|-----------|------|------|---------|
| gate (`gate_threshold_percent`, 50 to 100) | `--gate` | yes | 90 |
| flaky (`flaky_flip_threshold`, at least 1) | `--flaky-threshold` | yes | 4 |
| stability window (`stability_window_runs`, at least 1) | `--stability-window` | no | 12 |

Precedence: flag over file over default. The window has no file key, so only flag over default applies to it.

## Task record (test first)

- Task 1 (RED): nine binary tests and four config fixtures, plus a golden file of the output of the pre-change binary
  for SUT-1. Run before any implementation: eight failed for the expected reasons (flag unknown, file ignored). The
  zero-window test passed for the wrong reason (clap rejected the unknown flag); I tightened it to check the
  "must be at least 1" message before implementing. That tightened version was not re-run in RED.
- Task 2 (GREEN): `vvconfig.rs` (load, parse, resolve, source labels), `vvcmd.rs` (optional flags, window, parameters
  block), `main.rs` (module). All nine tests pass.
- Task 3: AC 7 recorded as blocked (ignored test); docs updated.

## Deviations from the plan

- **Commit granularity:** one feat commit instead of separate test and feat commits (CLAUDE.md: no failing tests committed).
- **Task 3 as blocked, not a checkpoint:** per instructions, AC 7 is recorded as BLOCKED and does not stop execution.
- **Added `golden-sut1-default.json`** to `config/` to prove "the default output equals today's output" against
  a stored file, not against a re-run of the new code.
- **Accepted `type` and `schema` keys** in the config file (project YAML rule, CLAUDE.md). Not in the plan.
- **Window history change (CONTEXT Q6):** the history is now the last N runs, not all runs. Documented in
  `docs/VV_METRICS.md` and in the doc comment of `stability_section`. Data sets of 12 runs or fewer are unchanged.
- **Flag checks moved** into `vvconfig::resolve` (same messages). They now run before the run records are loaded.
- **`--gate` and `--flaky-threshold`** no longer have a clap default. The help text names the default. Effective defaults unchanged.
- **`parameters` shape:** each entry is `{"value", "source"}`, per the plan's Task 2. Not re-checked against tcms-schemas, which is absent.

## Notes and open questions for the user

1. **`stability_window_runs` in the file is refused** as an unknown key. The backlog's "Why" section says Verdict keeps the
   window in `vv-config.yaml`, but its "What" section lists only the gate and flaky keys, and the user's brief matches that.
   If Verdict's real file has the window key, this must be decided before 09-03 or any CI use.
2. **No upper bound on the window.** tcms-web's `ReportService` has `MAX_WINDOW = 50`. The backlog gives only "at least 1", so none is enforced here.
3. **The AC 2 reading.** The plan's shape puts the number under `value`. A literal reading of "the value appears in `parameters.stability_window_runs`" would put the number directly there. The two cannot both hold; I followed the plan.

## Self-Check

Self-Check: PASSED. Files present: `crates/tcms/src/vvconfig.rs`, `crates/tcms/tests/vv_config_cli.rs`,
`crates/tcms/tests/fixtures/vv-metrics/config/golden-sut1-default.json`, `docs/VV_METRICS.md`. Commits present: `3468b08`, `8b3131f`.
