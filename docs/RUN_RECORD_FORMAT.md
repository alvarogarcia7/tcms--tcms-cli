# Combined run record format (TCMS-47.3, decided 2026-11-03)

One run = one folder: `runs/<RUN_ID>/` under the data root.

## Files
- `test-run.yaml` (required): follows the tcms-schemas test-run schema (`schemas/tcms/test-run.schema.json`). Keys used by `tcms vv metrics`: `run_id`, `plan_id`, `started_at`, `finished_at`, and `results:` with one entry per test case: `test_case_id`, `verdict` (pass | fail | not_executed | error), optional `problem_report_id`.
- `verification-results.yaml` (optional): per test case step counts and step results, keyed by `test_case_id`.

## Build rules (from the per-case `type: test_result` files)
- `run_id`, `plan_id`: from the shared run metadata; one run per combine.
- `started_at` = earliest case start; `finished_at` = latest case finish (execution logs under `10_test_results/execution_logs/`).
- `overall_pass: true` -> `pass`; `false` -> `fail`; no execution -> `not_executed`.
- Duplicate `test_case_id`: latest `started_at` wins; `not_executed` never overrides an executed result.
- Validate both files against their schemas before writing; on failure write nothing.

## Consumers
- `tcms vv metrics --root <data root>` reads `runs/*/test-run.yaml` (TCMS-47.2 adds `vv-config.yaml`).
- The legacy Java path reads the folder as it is (single run per folder, no history).

Implementation: TCMS-47.3, later.
