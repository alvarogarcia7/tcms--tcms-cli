# A YAML of another type under the root is ignored

result.yml declares type test_result: it is not a test case, is not listed as schemaErrors and does not count in distinctCases.

**Expected:** exit 0.

Arguments and expected values are in `case.json`; see the index `../README.md`.
