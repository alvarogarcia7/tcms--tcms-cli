# V&V metrics fixtures

Shared fixtures for the V&V metric tests (TCMS-47). The tests in tcms-cli and the expected
figures derived from them must agree with the rules in `docs/reference/tcms-web/`.

## sut-1/

Recorded V&V data of SUT 1 (`alvarogarcia7/tcms--sut-1`, branch `main`, commit recorded in
`sut-1/SOURCE_COMMIT`). Copied unchanged, except that the audit and execution logs are left out.

| Path | Content |
|------|---------|
| `index.jsonl` | One line per recorded run (14 runs) |
| `runs/<RUN_ID>/test-run.yaml` | Run record: one result per test case with its verdict |
| `runs/<RUN_ID>/verification-results.yaml` | Step-level verification results of the run |
| `requirements.json` | Requirement IDs `SUT1-1`, `SUT1-2` |
| `SUT1_PASS_001.yml`, `SUT1_CALC_001.yml` | The two test cases the runs execute (`SUT1_PASS_001` cites `SUT1-2`) |

### What SUT-1 covers

- Passing runs only: all 28 results across the 14 runs have verdict `pass`.
- Every run has a pass rate of 100 percent, so run-over-run deltas are zero and no run sits at a gate boundary.

### derived/

The cases SUT-1 lacks (failing, no data, blocked only, skipped outside the denominator) are derived in
`derived/`, one folder each with a `DERIVED_FROM` file. See `derived/README.md`. `sut-1/` is unchanged.

## What SUT-1 does not cover (must be derived)

SUT-1 has no failing result, no `NO_DATA` run, no skipped or untested result, no blocked
result, and no requirement with `NOT_RUN` or `FAILING` coverage. These cases are needed by the
acceptance criteria of TCMS-47. Add them as separate, clearly named fixture folders (for example
`derived/failing-run/`), each with a `DERIVED_FROM` file naming the SUT-1 run it was copied from
and the exact edits made. Do not edit the `sut-1/` records themselves.
