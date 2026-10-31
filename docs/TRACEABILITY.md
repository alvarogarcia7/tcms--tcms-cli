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

All four kinds are rendered as Markdown, from one model built from the same inputs, so their
numbers agree. The kinds are printed in the fixed order traceability, summary, gaps, problems,
whatever order `--kind` was given in. The traceability kind is one Markdown document that starts
with `# Traceability Report` (see [Traceability report](#traceability-report-markdown)); its
first section, `## Inputs`, holds the input and run-selection details described below. When
traceability is not selected, the input block and a `Report kinds` list come first, and each
selected kind follows as its own Markdown document. AsciiDoc, JSON and YAML output are not
available yet.

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

### Report shapes

`--shape` picks the layout of the traceability report. Both shapes are views over the same
model as the grid, so every verdict comes from the same roll-up. The default is `grid`, the
layout described above, and `--shape grid` prints the same bytes as no `--shape`.

`--shape a` and `--shape b` need `--kind traceability` as the only `--kind`. Any other use (no
`--kind`, or `--kind summary`, `gaps` or `problems`) is a usage error: exit code 2, nothing on
stdout, and stderr names `--shape` and `--kind`. An unknown shape value is also exit code 2.

Both shapes keep the document top (title, `## Inputs`, `## Report header`) and one
`## Document: <path>` section per document, with `No traceable content` for a document without
requirements. They do not show coverage columns or the fixed test-type columns.

Shape A: one row per requirement. Test cases are `ID (status)`, joined by `<br>`; `-` when the
requirement has none.

```
| Requirement | Test cases (ID, status) | Final verdict |
| --- | --- | --- |
| HLR-1 | T1 (pass)<br>T2 (NOT RUN) | PARTIAL |
```

Shape B: per requirement, a verdict table and then a case table. The requirement verdict is
the roll-up. Each case row shows that case's own status, not the requirement verdict. A
requirement with no cases has no case table.

```
| Requirement | Final verdict |
| --- | --- |
| HLR-1 | PARTIAL |

| TESTCASE ID | Status |
| --- | --- |
| T1 | pass |
| T2 | NOT RUN |
```

Shape output is not yet validated against the report schema; that is Phase 4.

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

### Run summary

`--kind summary` prints `# Run Summary` with two tables and a count line.

**Results per run.** One row per run of the selection, with the columns `Run`, `Build`, `Used`,
`pass`, `fail`, `not_executed`, `error` and `Total`.

- `pass`, `fail` and `error` count the decided results of that run. A superseded result is not
  counted.
- `not_executed` is the run's own `not_executed` records.
- `Total` is the size of the run's record minus its superseded results.

**Results per requirement.** One row per HLR or LLR, with the columns `Requirement`, `Level`,
`Status` and the same five count columns. The counts cover the live linked test cases only. A
linked case with no executed result is counted in `not_executed`. A requirement with no test
cases shows `NO TESTS` and zeros. The status column uses the rule in
[Verification status rule](#verification-status-rule).

In both tables the four buckets always sum to `Total`.

**Linked test cases.** A count line for all distinct live linked test cases, each counted once,
even when several requirements link it.

The per-run table can include test cases that the matrix does not link, while the per-requirement
table counts only linked live cases. The two tables therefore need not have equal totals.

### Coverage gaps

`--kind gaps` prints `# Coverage Gaps` with four lists. Each heading shows its count:

| Heading | Lists |
|---------|-------|
| `## Requirements with no test cases (N)` | HLR and LLR requirements with no linked live test case |
| `## Requirements missing a normal-range test (N)` | Requirements whose matrix row has no normal-range coverage |
| `## Requirements missing a robustness test (N)` | Requirements whose matrix row has no robustness coverage |
| `## Test cases with no recorded result (N)` | Linked live test cases with no executed result in any selected run |

- A requirement with no test cases also appears in both coverage lists. Leaving it out would make
  those lists look complete when they are not.
- The coverage flags are read from the matrix as written; the report does not recompute them.
- A test case that no requirement links is unknown to the report, so it cannot appear in any list.
- An empty list prints `None` under its heading.

### Problem reports

`--kind problems` prints `# Problem Reports`. It lists every decided `fail` and `error` result:

| Column | Content |
|--------|---------|
| Test case | The test case ID |
| Verdict | `fail` or `error` |
| Run, Build | The run that decides the result, and its build |
| Problem report | The `problem_report_id` of the result |
| Requirements | The HLR and LLR requirements that link the case; empty for a case outside the matrix |

- The list includes test cases that no requirement links. It can therefore be longer than the
  FAILED and ERRORED rows of the traceability report.
- A superseded fail or error is not listed.
- A missing or blank `problem_report_id` is shown as `**MISSING problem_report_id**` in the
  Problem report cell. The id is trimmed when it is kept.
- Two count lines come before the table: `Failed or errored results: N` and
  `Without problem report: N`. When the list is empty the section prints
  `No failed or errored results` instead of a table.

### Exit status

0 on success. 1 when any input is invalid, a `run_id` repeats, `--generated-at` is not an
RFC 3339 timestamp, `--test-cases` or `--requirements-export` cannot be read, or the output cannot
be written. An error names the input at fault and nothing is written. The three-state exit code is
a later phase.

`trace-matrix --runs` keeps its previous lenient behaviour. It does not use the strict loader.
