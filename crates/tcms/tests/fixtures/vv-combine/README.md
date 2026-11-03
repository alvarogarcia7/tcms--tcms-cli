# tcms vv combine fixtures

Input of `tcms vv combine` (TCMS-47.3) for one run. Used by `crates/tcms/tests/vv_combine_cli.rs`.

## Source of the layout

No recorded 63-file run exists in this repository, so the layout is derived from the output contract
of `scripts/convert_verification_to_result_yaml.py`:

- `write_multiple_result_files` writes one file per case named `{test_case_id}_result.yaml` (line 534).
- `convert_test_case_to_result` writes `type: test_result` with `test_case_id`, `total_steps`,
  `passed_steps`, `failed_steps`, `not_executed_steps` and `overall_pass` (lines 407-417).
- The script reads `{test_case_id}_execution_log.json`; each entry carries a `timestamp`
  (the executor writes it, see `docs/RUN_METADATA.md`). The combinator takes `started_at` and
  `finished_at` from these timestamps, because the result file itself has none.

The script output has no run metadata (`run_id`, `build_id`, `tester`, `environment`) and no
"missing run" marker. These come from the command line (`--build-id`, `--tester`, `--environment`,
`--plan`) and from the rule that a case with no step executed is `not_executed`.

## Layout in this folder

The folder mirrors one run, `runs/<RUN_ID>/`:

| Path here | Copied to by the tests |
|-----------|------------------------|
| `per-case/*.yaml` (63 files) | `runs/<RUN_ID>/per-case/` |
| `per-case/execution_logs/*.json` (60 files) | `runs/<RUN_ID>/execution_logs/` |

File key: the file name without `_result.yaml`. The execution log of a key is
`execution_logs/<key>_execution_log.json`. Ordinary files use the test case id as key. The duplicate
pair uses `TC_FIX_010__a1` and `TC_FIX_010__a2`.

## Expected figures (run RUN-FIX-001)

| Figure | Value |
|--------|-------|
| Files in `per-case/` | 63 (61 ordinary files + 2 duplicate files) |
| Distinct `test_case_id` | 62 |
| Results after combining | 62 |
| `pass` | 49 |
| `fail` | 10 (9 ordinary + `TC_FIX_010`) |
| `not_executed` | 3 (`TC_FIX_060`, `TC_FIX_061`, `TC_FIX_062`: every step NotExecuted, no execution log) |
| `total_steps` per case | 2 |
| Run `started_at` / `finished_at` | 2026-11-02T06:00:00Z (`TC_FIX_010__a1`, earliest) / 2026-11-02T09:58:05Z (`TC_FIX_059`, latest) |

## Duplicate case

`TC_FIX_010` appears twice:

| File | started_at (first log entry) | overall_pass | Verdict kept |
|------|------------------------------|--------------|--------------|
| `TC_FIX_010__a1_result.yaml` | 2026-11-02T06:00:00Z | true | no |
| `TC_FIX_010__a2_result.yaml` | 2026-11-02T09:00:00Z | false | yes: `fail`, passed_steps 1 |

The later `started_at` wins, so the expected verdict is `fail`.
