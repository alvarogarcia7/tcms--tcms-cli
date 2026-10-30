# Phase 2 Plan 05: Blank-value rule for baseline and matrix id Summary

Blank or whitespace-only `baseline_id` and matrix `id` now render as `not recorded` through one shared `non_blank` rule, closing gap G1 from 02-VERIFICATION.md.

## Tasks

| Task | Name | Commit | Result |
|------|------|--------|--------|
| 1 | Failing unit tests for the blank-value rule (RED) | fd3a1f8 | Tests added; compile fails only on the missing `non_blank` symbol |
| 2 | One blank-value rule used by model, renderer and input summary (GREEN) | 300744f | `non_blank` added and wired in lib.rs, model.rs, render_markdown.rs |
| 3 | CLI tests for a blank baseline_id and a blank matrix id | 330e3ed | Two real-binary tests added to report_cli.rs |

## Gate Results

- `CARGO_HOME=$HOME/.cargo-user cargo fmt -p testcase-reporting -p test-executor -- --check`: exit 0
- `CARGO_HOME=$HOME/.cargo-user cargo clippy -p testcase-reporting -p test-executor -- -D warnings`: exit 0
- `CARGO_HOME=$HOME/.cargo-user cargo test -p testcase-reporting -p test-executor`: exit 0, no FAILED line
  - testcase-reporting unit tests: `test result: ok. 79 passed; 0 failed` (6 new tests added by this plan)
  - test-executor report_cli: `test result: ok. 35 passed; 0 failed` (baseline 33 plus 2 new blank tests)

## Manual Spot Check

Running the real binary (`test-executor --no-audit report`) on a matrix with `baseline_id: ""` printed:

```
Trace matrix: TM-1 (baseline: not recorded)
| Matrix | TM-1 |
| Baseline | not recorded |
```

Exit code 0.

## Acceptance Criteria

- Blank or whitespace-only `baseline_id` renders `| Baseline | not recorded |` and `(baseline: not recorded)`: met, proven by unit and CLI tests.
- Blank or whitespace-only matrix `id` renders `| Matrix | not recorded |`: met, proven by unit test and by `blank_matrix_id_renders_not_recorded` run against the real binary.
- report_cli has 33 test functions before the plan and 35 after: met.
- Valid inputs produce the same output: all pre-existing tests pass unchanged (golden layout and byte-identical snapshot tests included).
- Plans 02-01 to 02-04 and their summaries are unchanged; STATE.md and ROADMAP.md are not written.

## Deviations from Plan

**1. [Process - commit state] RED commit does not compile**
- **Found during:** Task 1
- **Issue:** The plan requires the RED step to fail on the missing `non_blank` symbol, so commit fd3a1f8 leaves `testcase-reporting` tests uncompilable. CLAUDE.md says code with failing tests must not be committed.
- **Fix:** Committed per the orchestrator's per-task commit instruction. Commit 300744f restores a passing state. Anyone bisecting should use 300744f or later.

**2. [Acceptance grep false positive] `grep -c 'fn non_blank'` prints 1, not 0**
- **Found during:** Task 1 acceptance check
- **Issue:** The unit test `non_blank_keeps_visible_values_and_drops_missing_or_blank` matches the pattern `fn non_blank`.
- **Fix:** None to the code. The helper itself was absent at RED (confirmed by the E0425 compile error) and is present at GREEN as `pub fn non_blank`. Test name kept as written in the plan's behavior list.

**3. [Scope addition] Matrix id normalised in input summary**
- Plan Task 2 step 2 asked for `non_blank` on the matrix id in `input_summary`. Added one extra unit test (`summary_renders_blank_matrix_id_as_not_recorded`) for it; not listed in the plan's behavior list.

**4. [Tooling] `cargo fmt -p` ran over the two crates**
- Only the lines edited by this plan changed; the `git diff` at Task 2 showed no unrelated reformatting.

## Self-Check

- Files modified (all in `files_modified`): `crates/testcase-reporting/src/lib.rs`, `crates/testcase-reporting/src/model.rs`, `crates/testcase-reporting/src/render_markdown.rs`, `crates/test-executor/tests/report_cli.rs`.
- Commits present: fd3a1f8, 300744f, 330e3ed.
- Not run in this step (per instructions): `make acceptance-test`, `make coverage-e2e`, `make test`, `make lint`. Phase-level gates run later.
- Not pushed.
