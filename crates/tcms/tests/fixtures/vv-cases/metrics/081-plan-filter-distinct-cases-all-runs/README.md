# distinctCases counts all runs, not only the plan's

DOCS (VV_METRICS.md Output): loadSummary.distinctCases counts the case files and the test cases of the verification results 'in all runs'. Here --plan P2 selects RUN-1a, but RUN-2b (no plan) has TC_02 and must still count: distinctCases 2. Logged as D-08 in DISCREPANCES_TEST_CASES.md.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
