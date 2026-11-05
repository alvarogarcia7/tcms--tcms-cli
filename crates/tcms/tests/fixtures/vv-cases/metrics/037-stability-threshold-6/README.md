# Stability: threshold 6 makes TC_FLIP not flaky

Same six runs as the previous case with --flaky-threshold 6: 5 flips are below 6, so TC_FLIP is FAILING (its last kept result is FAILED).

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
