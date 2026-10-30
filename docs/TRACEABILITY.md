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

## Report inputs and run selection (`test-executor report`)

`test-executor report` reads one trace matrix and the test run records, checks every input
strictly, chooses one result per test case, and prints what it used and what it skipped.

```bash
test-executor report --matrix trace-matrix.yaml --runs test-runs/ --kind traceability
```

| Option | Meaning |
|--------|---------|
| `--matrix FILE` | Trace matrix YAML file (required) |
| `--runs PATH` | Test run record file or directory of them. Repeatable. Required |
| `--kind KIND` | Report kind: `traceability`, `summary`, `gaps` or `problems`. Repeatable. Default: all four |
| `--output FILE` | Write the text to a file instead of stdout |

Report bodies are not rendered yet. Each selected kind prints
`<kind>: renderer not yet available`.

### Inputs are strict

- Every matrix and every run record is validated against its schema before use. A file that
  fails validation, or a YAML syntax error, makes the command exit with status 1. The message
  names the file. Nothing is written when an input fails.
- A file named on the command line must be a test run record. Otherwise the command fails.
- In a runs directory, YAML files without a `type` field, or with another `type`, are skipped,
  and so are non-YAML files. Each skipped file is listed with its reason. The directory is not
  searched recursively.
- Two run records with the same `run_id` fail the command, naming both files.
- A test case that appears twice in one run fails the command.

### Reconciliation block

The output starts with the matrix line and a reconciliation of every discovered input:

```
Trace matrix: TM-1 (baseline: not recorded)

Reconciliation
  Inputs discovered: 4 (loaded: 2, skipped: 2)
  loaded   matrix  trace-matrix.yaml
  loaded   run     test-runs/run1.yaml (RUN-1)
  skipped  run     test-runs/notes.yaml: no `type` field
  skipped  run     test-runs/readme.txt: not a YAML file
  Notes: none
```

### Run selection

For each test case the result that decides is the one from the latest run that has an
executed result for it:

1. Runs are ordered by `started_at`, compared as instants, so `+02:00` and `Z` offsets compare
   correctly.
2. Runs with the same `started_at` are ordered by `run_id`. The later run wins, and a note
   names both runs.
3. A `not_executed` result never decides a test case and never replaces an earlier result.
4. A later `pass`, `fail` or `error` replaces an earlier one. There is no ranking of verdicts.
5. The builds of the used runs are listed. Builds are not restricted.

Each run is listed as used or excluded. An excluded run has one of these reasons:
`only not_executed results`, or `every executed result superseded by a later run`.

```
Run selection (per test case, the latest run with an executed result decides)
  Builds used: b2
  Runs used: 1
    RUN-2  build b2  started 2026-10-28T11:00:00Z  decided 1, superseded 0, not_executed 0
  Runs excluded: 2
    RUN-1  build b1  started 2026-10-28T10:00:00Z: every executed result superseded by a later run
    RUN-3  build b3  started 2026-10-28T12:00:00Z: only not_executed results
  Selected results: 1
    T1  fail  RUN-2
  Notes: none
```

### Notes

Notes never fail the command:

- A run that the matrix names in `run_ids` but that was not loaded.
- Results for test cases that are not in the matrix.
- Ties on `started_at`.

### Exit status

0 on success. 1 when any input is invalid, a `run_id` repeats, or the output cannot be
written. The three-state exit code is a later phase.

`trace-matrix --runs` keeps its previous lenient behaviour. It does not use the strict loader.
