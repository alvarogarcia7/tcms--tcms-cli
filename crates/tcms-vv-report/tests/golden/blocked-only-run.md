# V&V metrics report

## Run

| Item | Value |
| --- | --- |
| Run | RUN-DERIVED-BLOCKED-01 |
| Started | 2026-10-30T12:00:00Z |
| Finished | 2026-10-30T12:00:00Z |
| Provisional | no |
| Generated | 2026-01-01T00:00:00Z |
| Gate threshold | 90% (default) |

## Gate verdict

| Measure | Value |
| --- | --- |
| Verdict | FAILED |
| Gate pass rate | 0% (exact 0.0%) |
| Denominator (passed + failed + not executed) | 2 |
| Counts: passed / failed / not executed / skipped | 0 / 2 / 0 / 0 |
| Delta from previous run | -100 |
| Previous run | RUN-32921 |

## V&V run figures

| Measure | Value |
| --- | --- |
| V&V test-case pass rate | 0% (exact 0.0%) |
| Test cases passed / total | 0 / 2 |
| Step coverage | 0% (exact 0.0%) |
| Steps passed / total | 0 / 6 |

## Requirement coverage

| Measure | Value |
| --- | --- |
| Requirements | 2 |
| Covered | 0 |
| Failing | 2 |
| Not run | 0 |
| No cases | 0 |
| With problem report | 0 |
| Open problem reports | 0 |

| Requirement | Coverage | Executed / total | Problem reports | Open problem reports |
| --- | --- | --- | --- | --- |
| SUT1-1 | FAILING | 1 / 1 | - | - |
| SUT1-2 | FAILING | 1 / 1 | - | - |

## Stability

| Measure | Value |
| --- | --- |
| Flaky threshold (flips) | 4 (default) |
| Flaky cases | 0 |

Cases with stability FLAKY or FAILING:

| Case | Stability | Flips |
| --- | --- | --- |
| SUT1_CALC_001 | FAILING | 1 |
| SUT1_PASS_001 | FAILING | 1 |

## Load summary

| Measure | Value |
| --- | --- |
| Distinct cases | 2 |
| Runs recorded | 2 |
| Schema errors | 0 |
| First schema error file | none |
| First schema error | none |

