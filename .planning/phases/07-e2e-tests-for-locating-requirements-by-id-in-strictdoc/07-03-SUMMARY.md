# Phase 7 Plan 03: StrictDoc locate e2e tests Summary

**21 e2e tests run the real `tcms requirements locate` binary on the checked-in real StrictDoc 0.20.0 exports, with literal expectations for found, not-found, batch, output formats and exit 2 cases.**

## Performance

- **Tasks:** 2 (both in one test file; committed together)
- **Files created:** 1 (`crates/tcms/tests/requirements_locate_strictdoc_e2e.rs`)

## Accomplishments

- Found HLR-1, LLR-1 (nested two sections deep) and SYS-1 each proven by the exact output line, including the StrictDoc level, title, status and URL.
- Not-found gives exit 1 with `not found: N of M requirement IDs` on stderr.
- Batch: one result per ID in command-line order, repeats reported once, exit 1 when any ID is missing. The tests do not check how many times the export is read.
- Text, json and plaintext formats pinned.
- Exit 2 for: no system format matching an ID (nothing on stdout), missing `--input`, missing file, refused level `chapter` (`is not a requirement type`), item without LEVEL (`has no LEVEL`), old lowercase layout (`DOCUMENTS`), non-JSON input, unknown `--format`.
- `locate_writes_nothing` checks the tempdir and the fixture bytes are unchanged.

## Commits

- `303fe01` test(07-03): e2e tests of requirements locate on real StrictDoc exports [ci skip]

## Gates

- `env -u CARGO_TARGET_DIR cargo fmt -p tcms -- --check`: pass
- `env -u CARGO_TARGET_DIR cargo clippy -p tcms --all-targets -- -D warnings`: pass, no warnings
- `env -u CARGO_TARGET_DIR cargo test -p tcms`: pass; new file 21/21; requirements_locate_cli 23/23, requirements_cli 7/7, requirements_export_ids_cli 18/18, others pass
- `make test`, `make acceptance-test`, `make coverage-e2e`: not run (per instruction)

## Criteria

- Real binary on real StrictDoc exports (D-01): MET
- Expected answers are literals, not computed from the export (D-02): MET
- Found HLR, LLR, system item each proven with exact line (D-03): MET
- Absent ID gives exit 1 and `not found` line (D-03): MET
- Unknown format gives exit 2, names ID, empty stdout (D-03): MET
- Missing input, missing file, refused level, no LEVEL, old layout give exit 2 (D-03): MET. An unreadable file (chmod 000) is NOT tested.
- Batch rule: command-line order, repeats once, exit 1 on any missing: MET. Single export read is NOT asserted.
- No network, no Python, no fixture writes: MET
- Task 1 grep criteria: `status.code(), Some(0)` / `Some(1)` literal patterns: NOT MET as literal text. Measured on the file: the literal greps return 0 each. The tests assert exit codes via a `code(&o)` helper, and the helper-form `code(&o), Some(0)` returns 5 and `code(&o), Some(1)` returns 6. Behaviour is asserted; wording differs.
- Task 2 grep criterion `status.code(), Some(2)` at least 8: same literal-text difference. Measured: literal `status.code(), Some(2)` returns 0; helper-form `code(&o), Some(2)` returns 9; literal `status.code()` returns 1 (inside the helper body).
- Task 2 clippy and fmt criteria: MET
- Task 2 `Command::new("` count 0: MET

## Deviations from Plan

- Both tasks were written into one file and committed in a single commit, not one commit per task; the plan's two verification steps both ran against the final file.
- `cargo fmt -p tcms` reformatted the new file once before the first test run (formatting only).

## Contract check

No difference found between `locate`'s real output and the Phase 6 contract. Every literal line in the file matched on first run.

## Self-Check: PARTIAL

Files and commit verified present. Two grep-text criteria are NOT MET as literal text (see Criteria); the plan is not reported as PASSED.
