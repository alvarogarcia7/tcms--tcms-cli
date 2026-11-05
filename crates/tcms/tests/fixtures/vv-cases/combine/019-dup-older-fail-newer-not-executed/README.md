# 019-dup-older-fail-newer-not-executed

TC_D has several attempts; the newest attempt that is pass or fail decides (expected fail). not_executed attempts never decide (TCMS-50.1, VV_METRICS.md).

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
