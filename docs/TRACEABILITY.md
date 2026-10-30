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
| `--test-cases DIR` | Test cases, searched recursively. Supplies each case's test type (the report columns) and obsolete status. Without it every case is shown in the `other` column and the obsolete count is `not recorded` |
| `--requirements-export FILE` | StrictDoc JSON export (same assumed layout as the matrix's `.json` requirements). Supplies the source document of each requirement. Without it every row sits under `## Document: not recorded` |
| `--generated-at TIMESTAMP` | Generation time, RFC 3339 (for example `2026-10-31T09:00:00Z`), written into the report header. Without it the current UTC time is used |

The traceability kind is rendered as one Markdown document (see
[Traceability report](#traceability-report-markdown)). Its first section, `## Inputs`, holds the
input and run-selection details described below. The summary, gaps and problems kinds print
`<kind>: renderer not yet available` until their renderers arrive in a later phase. AsciiDoc,
JSON and YAML output are not available yet.

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

### Traceability report (Markdown)

`--kind traceability` prints one Markdown document. Line 1 is `# Traceability Report`; nothing
comes before it.

1. `## Inputs`: the matrix line, the reconciliation and the run selection from above, verbatim,
   inside a fenced block.
2. `## Report header`: a `Field | Value` table with these rows in this order: Matrix, Baseline,
   Runs, Builds, Environments, Testers, Obsolete test cases excluded, Generated. A value that is
   not recorded reads `not recorded`. Runs, builds, environments and testers list only the runs
   the selection used, sorted and without duplicates.
3. One `## Document: <path>` section per source document, in path order.

Each document section holds a table with one row per requirement link in the matrix. Only HLR
and LLR requirements have rows. The columns are:

| Column | Content |
|--------|---------|
| Requirement | Requirement ID |
| Level | `HLR` or `LLR` |
| Status | The verification status (table below) |
| Normal-range test, Robustness test | `yes` or `no`: the coverage flags from the matrix, separate from the status |
| normal_range, robustness, other | The live linked cases of that test type, in that fixed order |

A case cell reads `<test case ID> (<verdict>, <run ID>)`, for example `T1 (pass, RUN-2)`. Several
cases in one cell are separated by `<br>`. A case with no executed result reads
`T1 (NOT RUN, no result)`. An empty test-type column shows `-`.

Grouping and order:

- Documents come from `--requirements-export`. A document with no HLR or LLR rows shows
  `No traceable content`. Without the export every row is under `## Document: not recorded`.
- Sections are sorted by document path. Rows are sorted by requirement ID, and the cases in a
  cell by test case ID.
- Obsolete test cases are not shown in any cell. They are counted in the header.

**Determinism:** the same inputs and the same `--generated-at` give identical bytes. The only
clock read is the default generation time, the current UTC time, used when `--generated-at` is
absent.

### Verification status rule

A requirement's status is computed over its linked live test cases. Each case's verdict is the
one chosen by the run selection, and a case with no executed result counts as `NOT RUN`. The
rules are checked in this order:

| Status | Condition |
|--------|-----------|
| `NO TESTS` | No live test case is linked to the requirement |
| `ERRORED` | At least one linked case has an `error` result. This ranks above `FAILED` |
| `FAILED` | At least one linked case has a `fail` result, and none has `error` |
| `VERIFIED` | Every linked live case has an executed result, and all are `pass` |
| `NOT RUN` | Linked cases exist, and none has an executed result |
| `PARTIAL` | Some linked cases have a `pass` result and the rest have no executed result. None has `fail` or `error` |

Consequences:

- A `fail` with a missing run is `FAILED`, never `PARTIAL`.
- A linked case with no executed result is shown as `NOT RUN` on its own row. It is never shown
  as a pass.
- Coverage (the normal-range and robustness flags) does not change the status. A requirement can
  be `VERIFIED` and still show `no` in the robustness column.
- Obsolete cases do not count toward the status. The header shows how many were excluded.

These rules were confirmed by the project owner on 2026-10-31.

### Exit status

0 on success. 1 when any input is invalid, a `run_id` repeats, `--generated-at` is not an
RFC 3339 timestamp, `--test-cases` or `--requirements-export` cannot be read, or the output cannot
be written. An error names the input at fault and nothing is written. The three-state exit code is
a later phase.

`trace-matrix --runs` keeps its previous lenient behaviour. It does not use the strict loader.
