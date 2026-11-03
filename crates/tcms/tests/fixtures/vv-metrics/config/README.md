# vv-config fixtures (TCMS-47.2)

Used by `crates/tcms/tests/vv_config_cli.rs`. Each test copies the SUT-1 data set (`../sut-1`) to a
temporary directory and, where needed, copies one of these files to `<DATA_ROOT>/vv-config.yaml`.

| Folder | File | Used for | Expected outcome |
|--------|------|----------|------------------|
| `valid/` | `vv-config.yaml` | file value used, flag over file | gate 95 and flaky 3 from `source: file` (no flag) |
| `wrong-type/` | `vv-config.yaml` | wrong type | exit non-zero; message names the file and `gate_threshold_percent` |
| `unknown-key/` | `vv-config.yaml` | unknown key | exit non-zero; message names the file and `gate_threshold_pct` |
| `out-of-range/` | `vv-config.yaml` | out-of-range value | exit non-zero; message names the file and `gate_threshold_percent` (49 is below 50) |
| `window/` | `vv-config.yaml` | file window used, flag over file (D-47-4) | `stability_window_runs` 14 from `source: file`; flag `--stability-window 12` gives `source: flag` |
| `window-zero/` | `vv-config.yaml` | window below 1 (D-47-4) | exit non-zero; message names the file and `stability_window_runs` |
| `gate-50/` | `vv-config.yaml` | file gate changes the verdict; flag overrides it | gate 50 from `source: file` gives PASSED for a 50 percent run; `--gate 90` gives FAILED |

`golden-sut1-default.json` is the output of `tcms vv metrics --root sut-1 --now 2026-01-01T00:00:00Z`
from the binary before TCMS-47.2 (no `parameters` block). The default-case test requires that every other
top-level key of the new output still equals it.

The valid file carries the `type` and `schema` header that every YAML file in the project must have (CLAUDE.md,
"Required YAML Fields"). The schema `tcms/vv-config.schema.v1.json` is not in the repository (TCMS-47.2 AC 7 is blocked).
