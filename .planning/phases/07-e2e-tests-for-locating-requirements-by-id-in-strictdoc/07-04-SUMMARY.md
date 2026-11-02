# Phase 7 Plan 04: StrictDoc export-ids e2e tests Summary

**18 e2e tests run the real `tcms requirements export-ids` binary on the checked-in StrictDoc 0.20.0 exports: valid IDs written in file order, exit 1 with the valid IDs still written, and exit 2 with nothing written for product mismatch, unknown fields and every input refusal.**

## Performance

- **Tasks:** 2 (both in one test file; committed together)
- **Files created:** 1 (`crates/tcms/tests/requirements_export_ids_strictdoc_e2e.rs`)

## Accomplishments

- Exit 0 with every ID valid: stdout is `{"product_id":"PROD-002","requirements":[HLR-1, LLR-1, SYS-1]}` in file order (test 1).
- Default `requirements.json` of the current directory is read when `--requirements` is absent (test 2).
- One invalid ID: exit 1, valid IDs still written, `invalid: HLR-99: not found in strictdoc (strictdoc)` and `invalid: 1 of 3 requirement IDs` on stderr (test 3).
- Every ID invalid: exit 1 with an empty list (test 4). Each level (system, HLR, LLR) is valid, IDs absent from the export are not (test 5). A repeated invalid ID is reported once and the count is over the de-duplicated list (test 6).
- `--output` writes the file and leaves stdout empty on exit 0 and exit 1 (tests 7, 8). The requirements file and the export are byte-identical after a run (test 9).
- Exit 2 with empty stdout and no output file for: product_id mismatch (test 10), unknown field inside a requirement (test 11), unknown top-level field (test 12), unparseable file (test 13), missing `requirements.json` (test 14), StrictDoc ID without `--input` (test 15), refused level `chapter` (test 17), item without LEVEL (test 18).
- Unknown ID format `PAY-1` exits 2 with empty stdout even though `HLR-1` in the same file is valid (test 16).

## Commits

- `2cd4ad5` test(07-04): e2e tests of requirements export-ids on real StrictDoc exports [ci skip]

## Gates

- `env -u CARGO_TARGET_DIR cargo fmt -p tcms -- --check`: pass
- `env -u CARGO_TARGET_DIR cargo clippy -p tcms --all-targets -- -D warnings`: pass, no warnings (no `--no-deps` fallback needed)
- `env -u CARGO_TARGET_DIR cargo test -p tcms`: pass; new file 18/18; requirements_export_ids_cli 18/18; requirements_locate_strictdoc_e2e 21/21; all other tcms test binaries pass
- `env -u CARGO_TARGET_DIR cargo test -p tcms --all-features`: pass, 217 passed, 0 failed across tcms
- `make test`, `make acceptance-test`, `make coverage-e2e`: not run (per instruction)

## Criteria

- Task 1 test list (7 named tests): MET. All reported ok.
- Task 1 `current_dir(` at least 1: MET (1)
- Task 1 `status.code(), Some(0)` at least 3: MET (3)
- Task 1 `status.code(), Some(1)` at least 5: MET (6)
- Task 1 `invalid: HLR-99: not found in strictdoc (strictdoc)` at least 1: MET (1)
- Task 1 `export_items` / `strictdoc_export.json` count 0: MET (0)
- Task 2 at least 18 tests ok, including the five named tests: MET (18/18)
- Task 2 `status.code(), Some(2)` at least 9: MET (9)
- Task 2 `does not match product_id 'PROD-002' of the integration config` at least 1: MET (1)
- Task 2 `cargo test -p tcms --all-features` exit 0: MET
- Task 2 `cargo clippy -p tcms --all-targets -- -D warnings` exit 0: MET
- Must-have: tests run the real binary on the real StrictDoc export: MET
- Must-have: default `requirements.json` or `--requirements`, sample shape: MET
- Must-have: valid IDs to stdout or `--output` in file order, exit 0: MET
- Must-have: one invalid gives exit 1, valid IDs still written, invalid on stderr: MET
- Must-have: product mismatch exits 2 and writes nothing: MET
- Must-have: unknown field exits 2 and writes nothing, top level and inside a requirement: MET
- Must-have: literal expectations, no network, no Python at test time: MET. Tests use the StrictDoc-only config (no Jira system), read checked-in exports, and never call StrictDoc.
- Must-have: StrictDoc is never called by the tests: MET

## Phase 6 contract check

No difference found. Every literal string the plan asserts (`invalid: HLR-99: not found in strictdoc (strictdoc)`, `invalid: 1 of 3 requirement IDs`, the product mismatch text, `parse requirements file`, `read requirements file` with `requirements.json`, `--input`, `is not a requirement type`, `has no LEVEL`) matched the real output on first run.

## Deviations from Plan

- Both tasks were written into one file and committed in a single commit, not one commit per task.
- `cargo fmt -p tcms` reformatted the new file once before the first test run (formatting only).

## Not covered

- `export-ids` help text still says "assumed layout" (`crates/tcms/src/export_ids.rs`); that wording is owned by 07-05 (D-07-7).
- No coverage figure is claimed; `make coverage-e2e` was not run.

## Self-Check: PASSED

File `crates/tcms/tests/requirements_export_ids_strictdoc_e2e.rs` exists; commit `2cd4ad5` exists on `dev`; all plan criteria above are MET.
