# Phase 4 Plan 04: Report formats in the CLI Summary

`test-executor report --format markdown|asciidoc|json|yaml` renders all four kinds; JSON and YAML are validated before any write, `--shape a|b` is exit 2 outside Markdown traceability.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | cc12e71 | test(04-04): add Markdown baseline golden and failing CLI tests for report formats [ci skip] (from the previous executor) |
| 2 | 1c1951e | feat(04-04): render report kinds in asciidoc, json and yaml with --format [ci skip] |
| 3 | 7ccd74e | feat(04-04): document output formats, layout and validation in TRACEABILITY.md [ci skip] |

## Task 2 changes on resume

- `asciidoc_all_kinds` failed: the AsciiDoc Inputs section printed the `Reconciliation` heading of the input summary, so stdout contained the word. `inputs_block` in `render_asciidoc.rs` now skips that heading line. The Markdown renderers and the test are unchanged, so D-04-5 holds: the summary is on stderr, not stdout.
- Lint and format fixes: rustfmt on the touched files; a needless `'a` lifetime in `report_formats_cli.rs` removed for clippy `-D warnings`.
- `.planning/DECISIONS.md` (D-04-5, uncommitted on resume) is committed with Task 2.

## Gates

- `cargo fmt --check -p testcase-reporting -p test-executor`: pass
- `cargo clippy ... --all-targets -- -D warnings`: pass
- `cargo test -p testcase-reporting -p test-executor`: pass (213 + 40 library and schema tests; report_formats_cli 12/12; report_cli 56/56; trace_matrix_cli 11/11; others pass)
- `make build`, `make lint`, `make test`, `make validate-envelope-schemas`, `make coverage-e2e`, `make acceptance-test`: not run (instruction for this resume: cargo gates only)

## Deviations

- **Rule 1 (bug):** AsciiDoc Inputs leaked the `Reconciliation` heading to stdout (see Task 2 changes).
- **Doc correction:** the sentence "AsciiDoc, JSON and YAML output are not available yet." in `docs/TRACEABILITY.md` was stale and was replaced. This is the only removed line in that file.
- **Not verified here:** the Markdown golden `report_markdown_pre_04-04.md` was captured by the previous executor; I did not regenerate it from commit 5862408. `git diff 5862408` shows no change to `render_markdown.rs` or `render_shapes.rs`, and `default_format_is_markdown_unchanged` passes.

## Red run

Not re-recorded. When resumed, only `asciidoc_all_kinds` failed (11 of 12 passed), for the reason above.

## Requirements

OUT-01 and OUT-02 stay Pending in REQUIREMENTS.md until the verifier passes this plan. They were not edited.

## Self-Check: PASSED

- Files: `04-04-SUMMARY.md`, `docs/TRACEABILITY.md`, `report_formats_cli.rs`, `render_asciidoc.rs`, `render_data.rs`, `main.rs` present.
- Commits: cc12e71, 1c1951e, 7ccd74e found in `git log`.
