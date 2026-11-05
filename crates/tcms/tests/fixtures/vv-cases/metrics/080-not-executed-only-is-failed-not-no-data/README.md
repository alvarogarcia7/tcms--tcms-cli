# Only not_executed results: FAILED with denominator 1, not NO_DATA

One case, not_executed. VV_METRICS.md: not_executed is in the denominator as not passed, so denominator 1, display 0, verdict FAILED. (report/README.md says a null passRate happens when no passed, failed or blocked result exists: that is inconsistent; see DISCREPANCES_TEST_CASES.md.)

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
