# Phase 2 Plan 03: Markdown renderer Summary

`render_markdown(&ReportModel) -> String` renders the traceability report as one Markdown document: a title, an Inputs section holding the Phase 1 text verbatim in a fence, a report header table, and one table per source document with fixed test-type columns and honest per-case verdicts.

## Performance

- **Tasks:** 2 of 2 completed
- **Commits:** 2 task commits plus this metadata commit
- **Files touched:** `crates/testcase-reporting/src/render_markdown.rs` (new), `crates/testcase-reporting/src/lib.rs`

## Accomplishments

- Golden layout test: the whole output is compared with `assert_eq!` against an exact literal. It covers the title, the fenced Inputs block, the header table, a VERIFIED row, a FAILED row with a NOT RUN case, and an empty document showing `No traceable content`.
- The Inputs section keeps the Phase 1 text unchanged inside a fence. The fence is one backtick longer than the longest backtick run in the text, and at least three, so input text cannot close it early.
- The header table has the fixed rows Matrix, Baseline, Runs, Builds, Environments, Testers, Obsolete test cases excluded, and Generated. Missing values print `not recorded`; obsolete count `None` prints `not recorded`.
- Each document gets its own `## Document:` heading in model order. A document with no rows prints `No traceable content`.
- Row columns are ID, Level, Status, Normal-range test, Robustness test (yes/no), then the `normal_range`, `robustness` and `other` cells in that order. An empty column shows `-`.
- A case cell reads `<id> (<outcome>, <run>)`, and a case with no result reads `<id> (NOT RUN, no result)`. Several cases in a cell are joined with `<br>`.
- Table safety: model-derived values pass through `cell()`, which escapes `|` as `\|` and turns line breaks into a space. This is table safety only; full hostile-string handling is Phase 4.
- Pipeline tests (`render_markdown::tests::pipeline`) run the real chain `select_results -> build_model -> render_markdown` over loaded runs:
  - two separate runs give byte-identical output (`as_bytes()` equal);
  - changing only `generated_at` changes only the `| Generated |` line;
  - a missing result renders NOT RUN, and a partly-run requirement is PARTIAL, never pass;
  - a requirement with no results renders NOT RUN;
  - FAILED and ERRORED render their words;
  - VERIFIED rows contain only `(pass, ...)` entries;
  - a pipe and a newline in a requirement ID stay on one table line;
  - a requirement with no cases renders NO TESTS with three `-` cells;
  - shuffled links, test cases and runs render the same bytes.

## Task Commits

1. **Task 1: markdown layout for inputs, header and document tables** - `d6cedad` (`feat(02-03): markdown renderer with golden layout [ci skip]`)
2. **Task 2: status honesty, escaping and byte-identical rendering** - `7ac9f1b` (`test(02-03): pipeline snapshot, status honesty and escaping for markdown [ci skip]`)

## Files Created/Modified

- `crates/testcase-reporting/src/render_markdown.rs` - the renderer (`render_markdown`, private helpers `inputs_block`, `fence_len`, `header_block`, `document_block`, `row_line`, `column_cell`, `cell`, `list`), a golden layout test, and 10 pipeline tests.
- `crates/testcase-reporting/src/lib.rs` - adds `pub mod render_markdown;`.

## Decisions Made

- The renderer reads `ReportModel` only. It has no clock, no environment lookups and no hash-ordered iteration (D-12, D-13). The Generated value is the one injected through the model.
- The Inputs fence is a plain ``` fence with no info string.
- One trailing newline on `inputs.details` is dropped before the closing fence, so a details text that already ends in a line break does not leave a blank line inside the fence. All other characters of the text are kept exactly.
- `cell()` also applies to document names and the Generated value, so no model string can break a table row or a heading.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Tests and implementation committed together per task**
- **Found during:** Task 1
- **Issue:** The plan asks for a RED test commit before the implementation. CLAUDE.md requires that all tests pass before any code change is committed, so a failing-test commit is not allowed.
- **Fix:** The golden test and the renderer landed in one green commit. The Task 2 tests are a separate commit, and they also pass.
- **Commits:** `d6cedad`, `7ac9f1b`

**2. [Rule 3 - Blocking] rustfmt applied to the new module**
- **Found during:** Task 1 and Task 2
- **Issue:** `cargo fmt` reformatted the long table constant and the test code.
- **Fix:** Ran `cargo fmt -p testcase-reporting`. The change is included in the task commits.

### TDD gate note

The plan is `type: tdd`, but no RED state was observed for either task. Both the golden test and the pipeline tests were written against the behavior list and passed on the first run. No `test(02-03)` commit precedes the `feat(02-03)` commit, so the RED evidence is missing from history.

## Verification Gates

| Gate | Result |
| ---- | ------ |
| `cargo test -p testcase-reporting -p test-executor` | passed: 73 unit tests in `testcase-reporting` (62 earlier, plus 1 golden and 10 pipeline tests), and all `test-executor` integration tests green |
| `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings` | passed |
| `cargo fmt -p testcase-reporting -- --check` | passed |
| `grep -c 'HashMap\|Utc::now\|env::var' render_markdown.rs` | 0 |
| `make build`, `make lint`, `make test`, `make coverage-e2e` | NOT RUN: only the cargo equivalents above were run. `cargo-llvm-cov` is not installed and the environment forbids installing packages |
| `make acceptance-test` | NOT RUN: `test-plan-documentation-generator` (TPDG) is not installed and the environment forbids installing packages |

## Issues Encountered

- None blocking.
- The `backlog` MCP server failed to connect (`ENOENT`), so the Backlog workflow in CLAUDE.md was not used. This plan does not depend on Backlog.

## Known Stubs

None. The renderer writes every value from the model. Empty values are shown as `not recorded`, `No traceable content` or `-` by design.

## Threat Flags

None beyond the planned register. The renderer adds no I/O, no new dependency and no new trust boundary. T-02-06 (cell forgery) is mitigated by `cell()` and covered by `pipe_and_newline_in_requirement_id_stay_on_one_line`. T-02-07 (status display) is covered by `missing_result_is_not_run_never_pass` and `verified_rows_show_only_pass_entries`.

## Next Phase Readiness

- `render_markdown` is ready for the CLI in plan 02-04 to call for `report --kind traceability`.
- The CLI-level byte-identical check (D-12) is the remaining proof and belongs to 02-04.

## Self-Check: PASSED

- Created file: `crates/testcase-reporting/src/render_markdown.rs` FOUND.
- Commits: `d6cedad` FOUND; `7ac9f1b` FOUND.
- Tests: `cargo test -p testcase-reporting -p test-executor` exits 0.
