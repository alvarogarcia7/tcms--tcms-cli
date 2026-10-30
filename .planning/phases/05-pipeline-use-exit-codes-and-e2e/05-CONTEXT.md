# Phase 5: Pipeline Use: Exit Codes and E2E - Context

**Gathered:** 2026-10-31
**Status:** Ready for planning
**Source:** REQUIREMENTS.md (CLI-02), ROADMAP Phase 5, and the exit-code rules of the requirement lookup (06-CONTEXT D-06)

<domain>
## Phase Boundary

`test-executor report` can run in a script or CI job and its exit status is reliable. The report is
written before any non-zero exit caused by gaps or failed results. The end-to-end behaviour is tested
with a bash script, and the new E2E is wired into the Make targets and the acceptance suite.

</domain>

<decisions>
## Implementation Decisions

- **D-01 (exit codes, CLI-02):** 0 on success; 1 on an input or validation error; 2 when
  `--fail-on-gaps` is given and at least one gap or failed result exists. No other codes are used
  by `report`.
- **D-02:** The report file (or stdout) is written before the process exits with 2. The exit does not
  discard output.
- **D-03:** Phase 1's existing exit 2 from clap (a missing `--runs`) is changed to 1 for `report`, so
  the three-state rule holds (01-04 SUMMARY deviation). Other tools keep their codes.
- **D-04 (E2E, ROADMAP Phase 5 criterion 3):** A bash 3.2 and BSD/GNU safe script, using
  `scripts/lib/logger.sh`, covers the exit codes and the write-before-exit rule.
- **D-05 (wiring):** The E2E is mapped in `mk/incremental.mk` for the `test-executor` crate, and added
  to the acceptance suite (`test-acceptance/`).
- **D-06 (gates):** Record the CLAUDE.md gates that run and those that cannot run locally (missing
  tools), as the earlier phases did.

### Claude's Discretion
- Script structure and test-case fixtures, provided D-04 and D-05 hold.

</decisions>

<canonical_refs>
- `.planning/REQUIREMENTS.md` - CLI-02
- `.planning/phases/01-strict-input-loading-and-reconciliation/01-04-SUMMARY.md` - the clap exit-code deviation
- `crates/test-executor/tests/report_cli.rs` - existing CLI tests
- `mk/incremental.mk`, `test-acceptance/` - wiring points
- `scripts/lib/logger.sh` - logging library
- `.planning/phases/02-traceability-report-markdown/02-CONTEXT.md`, `03-CONTEXT.md`, `04-CONTEXT.md` - report kinds and formats the exit codes apply to
</canonical_refs>

<deferred>
- Changing the exit codes of other commands: out of scope.
</deferred>
