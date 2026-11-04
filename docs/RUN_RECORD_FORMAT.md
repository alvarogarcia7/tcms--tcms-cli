# Combined run record format (TCMS-47.3, decided 2026-11-03)

One run = one folder: `runs/<RUN_ID>/` under the data root.

## Files
- `test-run.yaml` (required): follows the tcms-schemas test-run schema (`schemas/tcms/test-run.schema.json`). Keys used by `tcms vv metrics`: `run_id`, `plan_id`, `started_at`, `finished_at`, and `results:` with one entry per test case: `test_case_id`, `verdict` (pass | fail | not_executed | error), optional `problem_report_id`.
- `verification-results.yaml` (optional): per test case step counts and step results, keyed by `test_case_id`. Header `schema: tcms/verification-results.schema.json` (schema file `schemas/tcms/verification-results.schema.json`). Required keys: `type`, `schema`, `run_id`, and `test_results`, a list with one entry per test case. Each entry requires `test_case_id` (non-empty string), `total_steps` and `passed_steps` (integers >= 0). Step results are optional.

## Build rules (from the per-case `type: test_result` files)
- `run_id`, `plan_id`: from the shared run metadata; one run per combine. `plan_id` is always written: an empty string `""` when no `--plan` is given, because the test-run schema types it as a string and `null` is not allowed (TCMS-47.4 AC 1).
- `started_at` = earliest case start; `finished_at` = latest case finish (execution logs under `10_test_results/execution_logs/`).
- `overall_pass: true` -> `pass`; `false` -> `fail`.
- `not_executed` means a per-case file that reports no step executed (no execution recorded).
- A case with no per-case file is absent from the record: it gets no entry and is not emitted as `not_executed`. The combinator does not read the plan's case list (D-47-3).
- Duplicate `test_case_id` (TCMS-50.1): attempts are ordered by `started_at`, then by file key; walking from the newest, the first `pass` or `fail` decides. `not_executed` and `skipped` never decide. A case with no decided attempt is `not_executed`.
- Validate both files against their schemas before writing; on failure write nothing.

## Consumers
- `tcms vv metrics --root <data root>` reads `runs/*/test-run.yaml` (TCMS-47.2 adds `vv-config.yaml`).
- The legacy Java path reads the folder as it is (single run per folder, no history).

Implementation: TCMS-47.3, later.
