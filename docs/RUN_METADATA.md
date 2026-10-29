# Run metadata on execution logs and verification results

A test result states what ran, on which build, in which environment, by whom and when
(TCMS-40.3). Execution logs and verification results carry these values so that results of one
execution can be grouped into a run (TCMS-40.4) and attributed.

## Execution log entries

`test-executor generate --json-log` makes the script write these keys on every entry, hook
failures included:

| Key | Value |
|-----|-------|
| `test_case_id` | The test case `id` (always) |
| `run_id` | `TCMS_RUN_ID` of the process that runs the script, else the `--run-id` given to `generate`; omitted when neither is set |

```bash
test-executor generate --json-log --run-id RUN-42 TC_001.yaml --output tc_001.sh   # default run ID
TCMS_RUN_ID=RUN-43 ./tc_001.sh                                                     # wins at run time
```

## Verification results

`verifier` stamps these keys on each result of the container (`run_id`, `build_id`,
`environment`, `tester`, `date`):

| Key | Flag | Environment variable | Config file key | Otherwise |
|-----|------|----------------------|-----------------|-----------|
| `run_id` | `--run-id` | `TCMS_RUN_ID` | `run_id` | the `run_id` found in the execution log |
| `build_id` | `--build-id` | `TCMS_BUILD_ID` | `build_id` | omitted |
| `tester` | `--tester` | `TCMS_TESTER` | `tester` | omitted |
| `environment` | `--environment` | n/a | `environment` | omitted |
| `date` | n/a | n/a | n/a | the report's generation time (RFC 3339) |

Precedence: flag, then environment variable, then config file, then the value in the log.
Empty and blank values count as absent, and absent values are never written empty.

```bash
TCMS_RUN_ID=RUN-42 TCMS_BUILD_ID=fw-1.4.2 verifier -f logs/ --tester ci-bot --output results.yaml
```

Both outputs validate against the extended `execution-log` and `verification-result` schemas of
tcms-schemas (`docs/TCMS_SCHEMAS.md`).

## Test run record

The verifier writes the test run (tcms-schemas `test-run`) of the verified execution: one
result per test case with its verdict. It does so **by default** whenever the run ID, build ID,
environment and tester are known.

```bash
TCMS_RUN_ID=RUN-42 TCMS_BUILD_ID=fw-1.4 TCMS_TESTER=ci-bot \
  verifier -f logs/ --environment Staging --reviewer bob \
    --problem-report TC_FAIL_001=PR-057 \
    --output results.yaml --test-run run-42.yaml
```

| Field | Source |
|-------|--------|
| `run_id`, `build_id`, `environment`, `tester` | The run metadata above. All four are required: the verifier refuses to write an invalid record and names what is missing |
| `started_at`, `finished_at` | First and last timestamp in the execution logs (the report time when the logs have none) |
| `reviewer` | `--reviewer` or `TCMS_REVIEWER`; optional |
| `results[].verdict` | `pass`; `fail` when a step failed; `not_executed` when no step failed but some did not run; `error` when a hook failed and ended the script |
| `results[].problem_report_id` | `--problem-report TEST_CASE_ID=PROBLEM_REPORT_ID` (repeatable); only for `fail` and `error`, and only for a test case in the run |
| `results[].verification_result_ref` | `<--output path>#<test_case_id>`, when `--output` is given |

### Where it goes, and when it is skipped

| Situation | Result |
|-----------|--------|
| Run ID, build ID, environment and tester known | Written next to `--output` as `<name>.test-run.yaml` (`results.yaml` gives `results.test-run.yaml`); `test-run.yaml` in the current directory without `--output` |
| `--test-run PATH` | Written to `PATH`. The four values become required: a missing one is an error naming it |
| `--reviewer` or `--problem-report` given | The record was asked for: the four values are required, like with `--test-run` |
| Some of the four unknown, nothing asked for | No record; a warning names what is missing and the run still succeeds, so existing invocations keep working |
| `--no-test-run` | No record (cannot be combined with `--test-run`) |

The independence check of `docs/INDEPENDENCE.md` applies whenever a record would be written: a
run with complete metadata and a test case that requires independence fails without a suitable
`--reviewer`. Pass `--no-test-run` for a run that must not produce a record.
