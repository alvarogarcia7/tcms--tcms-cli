# Phase 4 Plan 03: Validated JSON and YAML output for the traceability report Summary

**render_json and render_yaml serialise the traceability ReportModel and return text only after the document passed validate_document; YAML validates the value parsed back from the returned text.**

## Commits

- 8da8585 test(04-03): add failing render_json and render_yaml tests [ci skip]
- bf326dc feat(04-03): render_json and render_yaml validate before returning text [ci skip]
- db84c55 test(04-03): hostile round trip, determinism and golden tests for render_json and render_yaml [ci skip]

Task 1 was red then green (9 of 10 tests failed on the stubs before the implementation). Task 2's tests were added after the Task 1 implementation and passed on first run, so no Task 2 implementation commit was needed. The hostile round-trip and determinism tests therefore verify existing behaviour; they were not seen failing first.

## Test counts

- `crates/testcase-reporting/src/render_data.rs`: 21 passed (10 for Task 1, 11 for Task 2).
- `cargo test -p testcase-reporting -p test-executor`: all test binaries pass; testcase-reporting lib 203 passed.

## Gates

- `cargo fmt --check -p testcase-reporting -p test-executor`: exit 0
- `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings`: exit 0
- `cargo test -p testcase-reporting -p test-executor`: exit 0

Not run, per instruction: `make test`, `make acceptance-test`, `make coverage-e2e` (tcms-45 and tcms-46 are open).

## Accomplishments

- `OutputError { Schema(Vec<String>), Serialize(String) }` with the Display text from the plan and `std::error::Error`.
- `render_json`: `build_document`, `serde_json::to_value`, `validate_document`, then `to_string_pretty` of the typed document plus one newline. Key order is the struct declaration order.
- `render_yaml`: `serde_yaml::to_string`, parse the returned text back with `serde_yaml::from_str`, `validate_document` on the parsed value, then return the text.
- Hostile fixture (about 50 strings: pipes, newlines, carriage return, quotes, backslashes, YAML and JSON specials, look-alike scalars, markup, a bell character, non-ASCII, a 2000 character string) round-trips through `select_results`, `build_model` and both renderers. Parsed documents equal `build_document` of the model, and the requirement, case and document counts equal the model's.
- Determinism: separately built identical inputs give identical bytes; reversed links, test case IDs, runs and document names give identical bytes; a different `generated_at` changes exactly one line in each format.
- Golden JSON of the small fixture (full literal) and golden YAML (first 12 lines plus total count of 46). The literals were produced by the renderer, then checked by hand against the Phase 2 rules: REQ-1 and REQ-2 roll up to FAILED, and cases without a test-case definition fall into `other`.

## Decisions Made

- **D-04-3 and D-04-4 respected:** no AsciiDoc or Markdown renderer and no Phase 3 view file changed.
- Only the files in `files_modified` were changed: `render_data.rs` (new) and `lib.rs` (one `pub mod` line).
- The header drops whitespace-only run, build, environment and tester values (existing model behaviour via `sorted_recorded`). The single-space hostile string is therefore used only in requirement and test case IDs, which keep it.

## Deviations from Plan

None to the plan's behaviour. The plan asked for Task 2 implementation changes "only as the tests reveal"; none were needed.

## Known Stubs

None.

## Threat Flags

None. No new network, auth, file-access or schema-trust surface; the functions read no files and use no clock.

## Self-Check: PASSED

- `crates/testcase-reporting/src/render_data.rs`, `crates/testcase-reporting/src/lib.rs`: present and committed.
- Commits 8da8585, bf326dc, db84c55: present on `dev`.
- `git diff --stat e329c94..HEAD`: only `lib.rs` (1 line) and `render_data.rs` changed.
- Acceptance greps: `serde_yaml::from_str` 4 matches; `validate_document(` 4 matches; forbidden pattern (`std::fs`, `Utc::now`, `SystemTime`, `preserve_order`) outside comments 0; the six named determinism and hostile tests present.
