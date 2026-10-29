# Traceability matrix and coverage check

`test-executor trace-matrix` builds the requirement to test case to run view
(tcms-schemas `trace-matrix`) and checks the glossary rule that every high-level and low-level
requirement has both a normal-range and a robustness test (TCMS-40.5, TCMS-40.6).

```bash
test-executor trace-matrix testcases/ \
    --requirements requirements/ \
    --runs test-runs/ \
    --id TM-2026-10 --baseline-id BL-001 \
    --output trace-matrix.yaml
```

| Option | Meaning |
|--------|---------|
| `TEST_CASE_DIR` | Test cases, searched recursively. Every YAML file below it must be a valid test case; one that does not load fails the command, because a coverage claim must not rest on missing data |
| `--requirements PATH` | An ID list, a requirement YAML file, a directory of requirement records, or a StrictDoc JSON export (a `.json` file, below) |
| `--runs PATH` | Test run records (`verifier --test-run`), files or directories; other YAML files in a directory are skipped. Repeatable |
| `--id`, `--baseline-id` | Matrix ID (default `TM-<UTC time>`) and the baseline it belongs to |
| `--output` | Output file; stdout by default |
| `--no-coverage-check` | Do not fail when a requirement lacks a normal-range or robustness test |

## Requirements

Requirement IDs come from the requirements tool (StrictDoc or Jira), never from the CLI.

- ID list (`.txt`): one `ID`, `ID:HLR` or `ID,LLR` per line, `#` for comments. A level is guessed
  from an `HLR` or `LLR` prefix of the ID, otherwise the line must say it.
- Requirement records (`.yml`, `.yaml`, or a directory): the tcms-schemas `requirement` records,
  one per file or a list per file. Only `type: HLR` and `type: LLR` count.
- StrictDoc JSON export (`.json`): the items of the export are the requirement set. Only items of
  level `HLR` and `LLR` count; items of other levels (`system`, `SFR`, ...) are ignored. An item
  with a level that is not a requirement type, a UID that appears twice, or a file that is not
  JSON in the expected shape fails the command, naming the file and item.

  **ASSUMPTION (TCMS-42.3, not verified):** StrictDoc is not installed in this environment and no
  real export sample exists. The layout read is the one assumed by `requirement-import`
  (`crates/requirement-import/src/strictdoc.rs`): a JSON object with a `documents` array, each
  document with a `path` and a `requirements` array of items with `uid` and `level`. The same
  parser serves `requirement-import`, so the assumption lives in one place. The fixture
  `crates/test-executor/tests/fixtures/strictdoc_export.json` is a crafted file written to that
  assumed layout, not a StrictDoc output. Check the layout against a real export before relying on it.

  ```bash
  test-executor trace-matrix testcases/ --requirements strictdoc_export.json
  ```

## What the matrix says

- A test case traces to a requirement through its `requirements:` list. The legacy
  `requirement:` string is ignored. Obsolete test cases (`status: obsolete`) do not count.
- `normal_range_covered` and `robustness_covered` are true when a linked test case has that
  `test_type`. A security test or a test case without `test_type` covers neither.
- `run_ids` lists the runs (`test-run.run_id`) with a result for a linked test case that is not
  `not_executed`. Verification results have no ID of their own.
- `level` is `HLR` or `LLR`, the level of the requirement. `integration` is a test level.

## The coverage check

After writing the matrix the command exits with status 1 when a requirement lacks a
normal-range or a robustness test case, naming each requirement and what is missing:

```
Requirements without test cases: LLR-2
Coverage check failed: every HLR and LLR needs a normal-range and a robustness test ...:
HLR-1 (HLR): no robustness test
LLR-2 (LLR): no normal_range and no robustness test
```

Test cases that name requirements missing from the requirement set are listed as warnings.
