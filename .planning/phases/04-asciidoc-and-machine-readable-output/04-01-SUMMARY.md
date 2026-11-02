# Phase 4 Plan 01: AsciiDoc renderers at library level Summary

**Shapes A and B, the Phase 3 views and the traceability grid render as AsciiDoc; hostile model strings cannot change table or heading structure in any of the six renderers.**

## Performance

- **Tasks completed in this session:** 1 of 3 (Task 3). Tasks 1 and 2 were committed earlier (ac5d5f2, 4e2f94f, 3d93f89, 56edd66).
- **Commits this session:** a4c468f (docs), 7c32c70 (test), 4e07928 (feat).

## Accomplishments

- `render_asciidoc_shape_a` and `render_asciidoc_shape_b` in `crates/testcase-reporting/src/render_asciidoc_shapes.rs`, with the Markdown shapes' frame (title, Inputs, report header, one document section each, `No traceable content` for an empty document) and columns.
- Shape A header is the single line `| Requirement | Test cases (ID, status) | Final verdict |`. Shape B headers are `| Requirement | Final verdict |` and `| TESTCASE ID | Status |`.
- Golden tests for both shapes on a hand-built fixture: R-1 verified by two passing cases, R-2 with no tests, R-3 partial with one NOT RUN case.
- Structure classifier, safety check and value check, run over all six AsciiDoc renderers (grid, shape A, shape B, summary, gaps, problems) on a hostile fixture built through `select_results`, `build_model` and the view builders.
- Byte-identical, shuffled-input (links, test case IDs, runs) and `generated_at` tests over all six renderers.
- NOT RUN never renders as pass; VERIFIED rows list only pass entries.

## Files

- Created: `crates/testcase-reporting/src/render_asciidoc_shapes.rs`
- Modified: `crates/testcase-reporting/src/lib.rs` (`pub mod render_asciidoc_shapes;`)
- Not modified: `render_asciidoc.rs`, `summary.rs`, `gaps.rs`, `problems.rs`, `render_markdown.rs`, `render_shapes.rs`, `model.rs`

## Decisions Made

- **D-04-4 applied:** the one-pipe check skips the table header row (the first cell line after each `[cols=...]` attribute line). Body cell lines keep the exactly-one-pipe check. The header rows are asserted by literal in the golden tests.
- **D-04-2 (branch):** Phase 4 work is on `dev`.
- **D-04-3 (no Inputs on views):** the AsciiDoc summary, gaps and problems documents have no Inputs section, as in Markdown. The grid and shapes A and B keep it.
- Shape table header cells are counted by their cell count in the classifier (`| A | B | C |` counts 3), so the multiple-of-columns rule holds for the single-line header.

## Hostile fixture: escaped output

Each value below is passed through `escape_adoc` and appears in the output in that escaped form. The forms were derived from the `escape_adoc` rules. `a&#92;|b` and `&#58;attr: value` were also seen in the output during a mutation check.

| Value | Escaped |
|---|---|
| `A\|B` | `A&#124;B` |
| `x` + newline + `y` | `x y` |
| `*bold* and _it_` | `&#42;bold&#42; and &#95;it&#95;` |
| backtick pair | `&#96;code&#96;` |
| `[[anchor]]` | `&#91;&#91;anchor&#93;&#93;` |
| `<<xref>>` | `&#60;&#60;xref&#62;&#62;` |
| `{attr}` | `&#123;attr&#125;` |
| `link:http://example.com[click]` | `link:http&#58;//example.com&#91;click&#93;` |
| `+++<b>raw</b>+++` | `&#43;&#43;&#43;&#60;b&#62;raw&#60;/b&#62;&#43;&#43;&#43;` |
| `include::/etc/passwd[]` | `include&#58;&#58;/etc/passwd&#91;&#93;` |
| `image::x.png[]` | `image&#58;&#58;x.png&#91;&#93;` |
| `----` | `&#45;---` |
| `\|===` | `&#124;===` |
| `== Fake heading` | `&#61;&#61; Fake heading` |
| `.Title` | `&#46;Title` |
| `NOTE: x` | `NOTE&#58; x` |
| `ifdef::x[]` | `ifdef&#58;&#58;x&#91;&#93;` |
| `:attr: value` | `&#58;attr: value` |
| backslash before pipe | `a&#92;&#124;b` |
| 2000 characters | unchanged |
| `Ünïcode 日本語 🙂` | unchanged |

**Rules beyond `escape_adoc` and `escape_adoc_line_start`:** none were needed. The shape renderers add no escaping rule. The only addition is the header-row exemption in the test classifier, which is the D-04-4 rule.

Fixed words are not escaped by design. `not_executed` in count lines and the `-` and `**MISSING problem_report_id**` literals are the only exemptions in the classifier.

## Verification

- `cargo fmt --check -p testcase-reporting -p test-executor`: exit 0
- `cargo clippy -p testcase-reporting -p test-executor --all-targets -- -D warnings`: exit 0 (after changing `cells % n == 0` to `cells.is_multiple_of(n)`, as clippy requires)
- `cargo test -p testcase-reporting -p test-executor`: exit 0. testcase-reporting 158 passed, including the 11 new shape tests. Other suites passed as well.
- Mutation check: with `|` removed from `escape_adoc`'s markup set, the hostile test failed at the one-pipe check. The file was restored with `git checkout` before committing.
- `asciidoctor` on a sample shape B document with a case ID containing an escaped pipe: no warnings; 3 tables, 9 body cells, 7 table rows, as expected.

## TDD Gate Compliance

Test commit `7c32c70` precedes feat commit `4e07928`. The red stage had 9 of 11 tests failing against a stub. `byte_identical...` and `shuffled...` passed in the red stage because the four non-shape renderers are real and deterministic; they cover the shapes once the shapes exist.

## Deviations from Plan

- **Header row is one line in the shape tables.** This is what the plan requires (`| Requirement | Test cases (ID, status) | Final verdict |`). The grid and the Phase 3 views keep their per-cell header lines from Tasks 1 and 2, unchanged.
- **Test-only helpers.** The classifier and fixture helpers live in `render_asciidoc_shapes.rs` tests, because the plan lists only the three files and the hostile check spans all six renderers.

## Known Stubs

None.

## Threat Flags

None. The threat model's T-04-01 to T-04-04 mitigations are covered by the tests above.

## Self-Check: PASSED

- Commit `a4c468f` (docs), `7c32c70` (test), `4e07928` (feat) exist on `dev`.
- `crates/testcase-reporting/src/render_asciidoc_shapes.rs` and this SUMMARY exist.
- Gates above exit 0.
