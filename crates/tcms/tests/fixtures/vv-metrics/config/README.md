# vv-config fixtures (TCMS-47.2)

Each folder holds one `vv-config.yaml`. The tests copy `../sut-1` into a temp dir and add the file there;
`sut-1` itself is never changed.

| Folder | Content | Expected outcome |
|--------|---------|------------------|
| `valid/` | gate 95, flaky_flip_threshold 3 | success; both values come from the file. The only file used by success tests |
| `wrong-type/` | `gate_threshold_percent: "high"` | exit non-zero, message names `vv-config.yaml` and `gate_threshold_percent` |
| `unknown-key/` | extra key `stability_window` | exit non-zero, message names `stability_window` |
| `out-of-range/` | `gate_threshold_percent: 49` | exit non-zero, message names `gate_threshold_percent` (range 50 to 100) |

`sut-1-default-output.json` is the output of `tcms vv metrics --root sut-1 --now 2026-01-01T00:00:00Z`
recorded before TCMS-47.2 was implemented. The default-case test compares every top-level key except
`parameters` against it.

The stability window test builds its 14-run history from `sut-1` (14 runs) in a temp dir: the earliest run's
verdict for `SUT1_PASS_001` is changed to fail, giving one flip between the first and second run.
