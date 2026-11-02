# Phase 8 Plan 01: V&V data flow diagram with invocations and file map Summary

**Diagram now shows plan, execute, verify, test-run record, trace matrix and report (Markdown and AsciiDoc), with 7 fake-data invocations and 6 step file maps checked by a row-counting script.**

## Commits

- `26a351b` docs(08-01): data-flow diagram with invocations and file map [ci skip]
- Summary commit: `docs(08-01): summary [ci skip]` (this file)

Not pushed.

## Files changed

- `docs/data_flow/2026-01-20_diagram_steps.mermaid`: V&V participants Plan, Exec, Verif, RunRec, Matrix, Report replace the TODO_??? participant. Arrow markers `%% vv-arrow A1`..`A7`, example notes, and step notes S1..S6. Removed `schema=TODO`.
- `docs/data_flow/DATA_FLOW.md`: section 4.5 "V&V data flow (Phase 8)" with arrow table (7 rows) and step table (6 rows).
- `test-acceptance/scenarios/tests/test_diagram_mermaid.sh` (new): nine checks. Exits 0 on the new diagram, exits 1 on the old one (checks 1 to 7 failed first, as red-green requires).

## Criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| V&V path shown: validate, execute, verifier, test-run record, trace-matrix, report (md and adoc) | MET | Diagram participants and arrows A1..A7; check 1 |
| Each of 7 arrows has exactly one fake-data example | MET | Check 2 and 2b (7 notes), check 3 (each line names exactly one tool) |
| Each example uses real subcommands and flags checked against `--help` | MET | 38 flag uses across the 7 examples were found in the captured `--help` output of verifier, `test-executor execute`, `trace-matrix`, `report` and `validate-yaml`. The match was a substring match, so each flag was also confirmed by hand. No schema path is used |
| Each of 6 steps lists input and output paths, using only plans/, runs/, requirements/, tcms-config/, reports/ | MET | Check 5 (6 step notes), 6 (five directories present), 6b (no other paths in step notes) |
| Mermaid syntax check passes, or is recorded as not run with the reason | DEFERRED | `mmdc` is not installed. Check 9 logs "mermaid syntax not checked: mmdc not installed". No install was run (threat T-08-SC). The diagram was not syntax-validated |
| DATA_FLOW.md mirrors the arrow and step tables | MET | Check 7a (7 rows), 7b (6 rows) |
| Row counts enforced by the check, not by grep > 0 | MET | Checks compare exact counts against 7 and 6 |
| `make build` / `cargo build --workspace` | MET | Exit 0 |
| `cargo fmt --check` | MET | Exit 0 (no Rust code changed) |
| `make lint` (clippy) | NOT MET | Not run. Only fmt was run |
| Workspace tests (`cargo test --workspace --all-features`) | NOT MET | 1225 passed, 1 failed. The failure is `test_command_escaping_for_json_with_backslashes` in `crates/testcase-manager/tests/executor_test.rs`. `git diff HEAD -- crates/` is empty, so the code under test is unchanged by this plan and the failure predates it. Not fixed (outside files_modified) |
| Coverage (`make coverage-e2e`, 70% threshold) | DEFERRED | Not run. No Rust code changed by this plan |
| Acceptance (`make acceptance-test`) | DEFERRED | Not run. `test-plan-documentation-generator` is not installed and `TEST_PLAN_DOC_GEN` is unset |

## Deviations

- **[Rule 2 - Correctness] A3 example adds `--no-test-run`.** Without it, verifier writes a default test-run record next to the output, which would duplicate A4's record write at `runs/RUN-FAKE-001.yml`. This keeps the file map exact.
- **[Scope note] Plan key_link to `test-acceptance/scenarios/run_scenario.sh`.** That file does not exist yet; it is 08-02's output. No change made here.

## Open question (for the user)

`test-executor execute --help` has no output-path flag. The execution log location for S2 (`runs/logs/`) is unconfirmed. The diagram and DATA_FLOW.md mark it "to be confirmed". Resolving it needs either a flag or a confirmed run output path.

## Self-Check

Verified after writing: diagram commit `26a351b` exists in `git log`, the three files exist, and the check script exits 0.
