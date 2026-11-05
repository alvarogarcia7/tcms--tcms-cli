# tcms vv test cases (TCMS-50 branch)

One folder per case, for each V&V binary. Every case folder holds its own inputs, a `README.md` saying what the case
covers, and `case.json` (the arguments and the expected output, computed by an independent reference model of the rules
in `docs/VV_METRICS.md`, not by running the binary).

| Binary | Cases | Focus |
|--------|-------|-------|
| `metrics/` (`tcms vv metrics`) | 81 | rounding (floor display, half-up V&V rate), gate boundaries incl. 100 %, zero-blocked, deltas, stability, requirements, plan filter, vv-config precedence, invalid inputs |
| `combine/` (`tcms vv combine`) | 52 | verdict rules, duplicate attempts (TCMS-50.1), run times, plan flag, missing required flags, malformed input |
| `history/` (`tcms vv history`) | 50 | every verdict kind, gaps, plans, long histories |
| `trends/` (`tcms vv trends`) | 51 | run window, gate and threshold variants, heatmap labels, rounding rows |
| `report/` (`tcms-vv-report`) | 62 | Markdown and AsciiDoc, exit codes (0/1/2), --fail-on-gate, missing fields, escaping |

Run all of them (copies each folder to a scratch directory, runs the binary, compares):

    python3 scripts/vv/vv_cases/run_vv_cases.py            # all binaries
    python3 scripts/vv/vv_cases/run_vv_cases.py metrics    # one binary

Regenerate the folders from the generator: `scripts/vv/vv_cases/gen_metrics.py` and `gen_rest.py`.
Discrepancies between the binary and the docs are logged in `DISCREPANCES_TEST_CASES.md` at the repository root.
