# Traceability Report

## Inputs

```
Trace matrix: TM-1 (baseline: not recorded)

Reconciliation
  Inputs discovered: 2 (loaded: 2, skipped: 0)
  loaded   matrix  crates/test-executor/tests/golden/report_fixture_matrix.yaml
  loaded   run     crates/test-executor/tests/golden/report_fixture_run.yaml (RUN-1)
  Notes: none

Run selection (per test case, the latest run with an executed result decides)
  Builds used: build-1
  Runs used: 1
    RUN-1  build build-1  started 2026-10-30T10:00:00Z  decided 2, superseded 0, not_executed 0
  Runs excluded: 0
  Selected results: 2
    T1  pass  RUN-1
    T2  fail  RUN-1
  Notes: none
```

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1 |
| Builds | build-1 |
| Environments | lab |
| Testers | alice |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |

## Document: not recorded

| Requirement | Level | Status | Normal-range test | Robustness test | normal_range | robustness | other |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HLR-1 | HLR | VERIFIED | yes | no | - | - | T1 (pass, RUN-1) |
| HLR-2 | HLR | NO TESTS | no | no | - | - | - |
| LLR-1 | LLR | FAILED | yes | yes | - | - | T2 (fail, RUN-1)<br>T3 (NOT RUN, no result) |

Report kind summary: rendered below

# Run Summary

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1 |
| Builds | build-1 |
| Environments | lab |
| Testers | alice |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |

## Results per run

| Run | Build | Used | pass | fail | not_executed | error | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RUN-1 | build-1 | yes | 1 | 1 | 0 | 0 | 2 |

## Results per requirement

| Requirement | Level | Status | pass | fail | not_executed | error | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HLR-1 | HLR | VERIFIED | 1 | 0 | 0 | 0 | 1 |
| HLR-2 | HLR | NO TESTS | 0 | 0 | 0 | 0 | 0 |
| LLR-1 | LLR | FAILED | 0 | 1 | 1 | 0 | 2 |

## Linked test cases

pass 1, fail 1, not_executed 1, error 0, total 3

Report kind gaps: rendered below

# Coverage Gaps

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1 |
| Builds | build-1 |
| Environments | lab |
| Testers | alice |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |

## Requirements with no test cases (1)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-2 | HLR | not recorded |

## Requirements missing a normal-range test (1)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-2 | HLR | not recorded |

## Requirements missing a robustness test (2)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-1 | HLR | not recorded |
| HLR-2 | HLR | not recorded |

## Test cases with no recorded result (1)

| Test case | Requirements |
| --- | --- |
| T3 | LLR-1 |

Report kind problems: rendered below

# Problem Reports

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1 |
| Builds | build-1 |
| Environments | lab |
| Testers | alice |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |

Failed or errored results: 1

Without problem report: 1

| Test case | Verdict | Run | Build | Problem report | Requirements |
| --- | --- | --- | --- | --- | --- |
| T2 | fail | RUN-1 | build-1 | **MISSING problem_report_id** | LLR-1 |
