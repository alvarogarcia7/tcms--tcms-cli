# One invalid test case file among valid ones

BAD_001.yml declares type test_case but lacks required fields. It goes to loadSummary.schemaErrors and is not counted: distinctCases counts only the valid case. Exit 0.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
