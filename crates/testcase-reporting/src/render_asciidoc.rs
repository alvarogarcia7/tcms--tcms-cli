//! AsciiDoc renderer for the traceability report and the Phase 3 views (OUT-01, D-02, D-05, D-06).
//!
//! Reads the report model and the Phase 3 view structs only: no clock, no environment lookups,
//! no hash-ordered iteration, so the same model always gives the same bytes (D-05). Every string
//! that comes from the model passes through `escape_adoc` (D-02, D-06).

use crate::model::{DocumentSection, Header, ReportModel, RequirementRow, TestColumn};
use crate::non_blank;

/// Start-of-line characters that are written as numeric references at the start of a cell or a
/// paragraph line, so a value cannot begin a block, a heading, a list or an attribute entry.
pub(crate) const LINE_START_CHARS: &[char] =
    &['.', '=', ':', '-', '*', '>', '|', '[', '+', '\'', '`'];

/// Characters that are always written as numeric references (D-02).
const MARKUP_CHARS: &[char] = &[
    '&', '<', '>', '*', '_', '`', '#', '+', '^', '~', '[', ']', '{', '}', '\\', '|', '@',
];

/// Admonition words: a colon right after one of them would start an admonition block.
const ADMONITIONS: [&str; 5] = ["NOTE", "TIP", "WARNING", "CAUTION", "IMPORTANT"];

const NOT_RECORDED: &str = "not recorded";
const NO_CONTENT: &str = "No traceable content";
const EMPTY_CELL: &str = "-";
const TABLE_HEADER: [&str; 8] = [
    "Requirement",
    "Level",
    "Status",
    "Normal-range test",
    "Robustness test",
    "normal_range",
    "robustness",
    "other",
];

/// Escape one model value for an AsciiDoc table cell or paragraph (D-02, D-06).
///
/// Line breaks and other control characters become one space (`\r\n` gives one space), so a value
/// cannot add a row or a line. Markup characters and the colon forms that start a macro or an
/// admonition are written as decimal references `&#NN;`. The start-of-line rule then applies.
pub fn escape_adoc(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut body = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\r' {
            body.push(' ');
            if chars.get(i + 1) == Some(&'\n') {
                i += 1;
            }
        } else if c.is_control() {
            body.push(' ');
        } else if MARKUP_CHARS.contains(&c) || (c == ':' && colon_is_markup(&chars, i)) {
            push_reference(&mut body, c);
        } else {
            body.push(c);
        }
        i += 1;
    }
    encode_line_start(&body)
}

/// Escape one free-text line of a paragraph (the Inputs block). Same rules as `escape_adoc`;
/// the caller passes one line at a time, so the start-of-line rule applies to that line.
pub fn escape_adoc_line_start(line: &str) -> String {
    escape_adoc(line)
}

/// A colon is written as a reference when it is next to another colon, when a slash follows it
/// (`http://`), or when it directly follows an admonition word.
fn colon_is_markup(chars: &[char], i: usize) -> bool {
    let before = i.checked_sub(1).map(|j| chars[j]);
    let after = chars.get(i + 1).copied();
    if before == Some(':') || matches!(after, Some(':') | Some('/')) {
        return true;
    }
    ADMONITIONS.iter().any(|word| {
        let word: Vec<char> = word.chars().collect();
        i >= word.len() && chars[i - word.len()..i] == word[..]
    })
}

fn push_reference(out: &mut String, c: char) {
    out.push_str(&format!("&#{};", c as u32));
}

/// The start-of-line rule, applied to the first non-space character of a value. A leading run of
/// `=` is encoded in full (so `== Heading` cannot start a section); any other listed character is
/// encoded alone.
fn encode_line_start(text: &str) -> String {
    let body = text.trim_start_matches(' ');
    let indent = &text[..text.len() - body.len()];
    let Some(first) = body.chars().next() else {
        return text.to_string();
    };
    let mut out = String::with_capacity(text.len() + 8);
    out.push_str(indent);
    if first == '=' {
        let run = body.len() - body.trim_start_matches('=').len();
        for _ in 0..run {
            push_reference(&mut out, '=');
        }
        out.push_str(&body[run..]);
    } else if LINE_START_CHARS.contains(&first) {
        push_reference(&mut out, first);
        out.push_str(&body[first.len_utf8()..]);
    } else {
        out.push_str(body);
    }
    out
}

/// Render the traceability report as one AsciiDoc document ending with exactly one newline
/// (OUT-01). Section order and table columns follow `render_markdown::render_markdown`.
pub fn render_asciidoc(model: &ReportModel) -> String {
    let mut blocks: Vec<String> = vec![
        "= Traceability Report".to_string(),
        inputs_block(&model.inputs.details),
        header_block(&model.header),
    ];
    blocks.extend(model.documents.iter().map(document_block));

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// The mandatory `== Inputs` section: the Phase 1 text as one paragraph per run of non-blank
/// lines, the lines joined by a hard line break (D-14). No code fence.
pub(crate) fn inputs_block(details: &str) -> String {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in details.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                paragraphs.push(current.join(" +\n"));
                current.clear();
            }
        } else {
            current.push(escape_adoc_line_start(line));
        }
    }
    if !current.is_empty() {
        paragraphs.push(current.join(" +\n"));
    }
    if paragraphs.is_empty() {
        paragraphs.push(NOT_RECORDED.to_string());
    }
    format!("== Inputs\n\n{}", paragraphs.join("\n\n"))
}

/// One AsciiDoc table: an attribute line with the column count, a fence, a header row and the
/// data rows, one cell per `| ` line. A cell may hold hard line breaks (` +`), never a pipe.
pub(crate) fn table(columns: usize, header: &[&str], rows: &[Vec<String>]) -> String {
    let mut lines = vec![
        format!("[cols=\"{columns}\",options=\"header\"]"),
        "|===".to_string(),
    ];
    lines.extend(header.iter().map(|cell| format!("| {cell}")));
    for row in rows {
        lines.extend(row.iter().map(|cell| format!("| {cell}")));
    }
    lines.push("|===".to_string());
    lines.join("\n")
}

/// The `== Report header` section (D-06): the same eight field rows as the Markdown header.
pub(crate) fn header_block(header: &Header) -> String {
    let list = |values: &[String]| {
        if values.is_empty() {
            NOT_RECORDED.to_string()
        } else {
            values
                .iter()
                .map(|v| escape_adoc(v))
                .collect::<Vec<_>>()
                .join(", ")
        }
    };
    let fields: Vec<Vec<String>> = vec![
        vec![
            "Matrix".to_string(),
            non_blank(Some(&header.matrix_id))
                .map_or_else(|| NOT_RECORDED.to_string(), escape_adoc),
        ],
        vec![
            "Baseline".to_string(),
            non_blank(header.baseline_id.as_deref())
                .map_or_else(|| NOT_RECORDED.to_string(), escape_adoc),
        ],
        vec!["Runs".to_string(), list(&header.run_ids)],
        vec!["Builds".to_string(), list(&header.build_ids)],
        vec!["Environments".to_string(), list(&header.environments)],
        vec!["Testers".to_string(), list(&header.testers)],
        vec![
            "Obsolete test cases excluded".to_string(),
            header
                .obsolete_excluded
                .map_or_else(|| NOT_RECORDED.to_string(), |n| n.to_string()),
        ],
        vec!["Generated".to_string(), escape_adoc(&header.generated_at)],
    ];
    format!(
        "== Report header\n\n{}",
        table(2, &["Field", "Value"], &fields)
    )
}

/// One `== Document:` section: the requirement table, or `No traceable content` when empty.
fn document_block(section: &DocumentSection) -> String {
    let name = section
        .name
        .as_deref()
        .map_or_else(|| NOT_RECORDED.to_string(), escape_adoc);
    let body = if section.rows.is_empty() {
        NO_CONTENT.to_string()
    } else {
        let rows: Vec<Vec<String>> = section.rows.iter().map(row_cells).collect();
        table(TABLE_HEADER.len(), &TABLE_HEADER, &rows)
    };
    format!("== Document: {name}\n\n{body}")
}

fn row_cells(row: &RequirementRow) -> Vec<String> {
    let mut cells = vec![
        escape_adoc(&row.requirement_id),
        escape_adoc(&row.level.to_string()),
        row.status.label().to_string(),
        yes_no(row.normal_range_covered).to_string(),
        yes_no(row.robustness_covered).to_string(),
    ];
    cells.extend(
        TestColumn::ALL
            .iter()
            .map(|&column| column_cell(row, column)),
    );
    cells
}

/// The cases of one test-type column: `ID (outcome, run)` per case, separated by hard line
/// breaks; `-` when the column has none (TRC-04).
fn column_cell(row: &RequirementRow, column: TestColumn) -> String {
    let entries: Vec<String> = row
        .cases
        .iter()
        .filter(|case| case.column == column)
        .map(|case| {
            let run = case
                .run_id
                .as_deref()
                .map_or_else(|| "no result".to_string(), escape_adoc);
            format!(
                "{} ({}, {run})",
                escape_adoc(&case.test_case_id),
                case.outcome.label()
            )
        })
        .collect();
    if entries.is_empty() {
        EMPTY_CELL.to_string()
    } else {
        entries.join(" +\n")
    }
}

fn yes_no(covered: bool) -> &'static str {
    if covered {
        "yes"
    } else {
        "no"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CaseEntry, DocumentSection, Header, InputsSection, RequirementRow, TestColumn,
    };
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use testcase_verification::RequirementLevel;

    /// The same hand-built model as the Phase 2 Markdown golden test (render_markdown.rs).
    fn golden_model() -> ReportModel {
        ReportModel {
            inputs: InputsSection {
                details: "Trace matrix: TM-1 (baseline: not recorded)\nRuns selected: RUN-1, RUN-2"
                    .to_string(),
            },
            header: Header {
                matrix_id: "TM-1".to_string(),
                baseline_id: None,
                run_ids: vec!["RUN-1".to_string(), "RUN-2".to_string()],
                build_ids: vec!["b1".to_string(), "b2".to_string()],
                environments: vec!["lab-1".to_string()],
                testers: vec!["alice".to_string()],
                generated_at: "2026-10-31T09:00:00Z".to_string(),
                obsolete_excluded: Some(2),
            },
            documents: vec![
                DocumentSection {
                    name: Some("docs/srs.sdoc".to_string()),
                    rows: vec![
                        RequirementRow {
                            requirement_id: "HLR-1".to_string(),
                            level: RequirementLevel::Hlr,
                            status: VerificationStatus::Verified,
                            normal_range_covered: true,
                            robustness_covered: true,
                            cases: vec![
                                CaseEntry {
                                    test_case_id: "T1".to_string(),
                                    column: TestColumn::NormalRange,
                                    outcome: CaseOutcome::Pass,
                                    run_id: Some("RUN-1".to_string()),
                                },
                                CaseEntry {
                                    test_case_id: "T2".to_string(),
                                    column: TestColumn::Robustness,
                                    outcome: CaseOutcome::Pass,
                                    run_id: Some("RUN-2".to_string()),
                                },
                            ],
                        },
                        RequirementRow {
                            requirement_id: "LLR-2".to_string(),
                            level: RequirementLevel::Llr,
                            status: VerificationStatus::Failed,
                            normal_range_covered: true,
                            robustness_covered: false,
                            cases: vec![
                                CaseEntry {
                                    test_case_id: "T3".to_string(),
                                    column: TestColumn::Other,
                                    outcome: CaseOutcome::Fail,
                                    run_id: Some("RUN-2".to_string()),
                                },
                                CaseEntry {
                                    test_case_id: "T4".to_string(),
                                    column: TestColumn::Other,
                                    outcome: CaseOutcome::NotRun,
                                    run_id: None,
                                },
                            ],
                        },
                    ],
                },
                DocumentSection {
                    name: Some("docs/empty.sdoc".to_string()),
                    rows: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn escape_table_maps_each_listed_input_exactly() {
        let pairs: &[(&str, &str)] = &[
            ("HLR-1", "HLR-1"),
            ("Some title 2026", "Some title 2026"),
            ("a|b", "a&#124;b"),
            ("line1\nline2", "line1 line2"),
            ("line1\r\nline2", "line1 line2"),
            ("a\tb", "a b"),
            ("*bold*", "&#42;bold&#42;"),
            ("_it_", "&#95;it&#95;"),
            ("`x`", "&#96;x&#96;"),
            ("#mark#", "&#35;mark&#35;"),
            ("+++raw+++", "&#43;&#43;&#43;raw&#43;&#43;&#43;"),
            ("^sup^", "&#94;sup&#94;"),
            ("~sub~", "&#126;sub&#126;"),
            ("[[anchor]]", "&#91;&#91;anchor&#93;&#93;"),
            ("<<xref>>", "&#60;&#60;xref&#62;&#62;"),
            ("{attr}", "&#123;attr&#125;"),
            ("a\\b", "a&#92;b"),
            ("&amp;", "&#38;amp;"),
            ("http://x.y", "http&#58;//x.y"),
            (
                "include::/etc/passwd[]",
                "include&#58;&#58;/etc/passwd&#91;&#93;",
            ),
            ("image::x.png[]", "image&#58;&#58;x.png&#91;&#93;"),
            ("ifdef::x[]", "ifdef&#58;&#58;x&#91;&#93;"),
            ("a::b", "a&#58;&#58;b"),
            ("09:00:00Z", "09:00:00Z"),
            ("link:x", "link:x"),
            ("a@b.c", "a&#64;b.c"),
            ("NOTE: x", "NOTE&#58; x"),
            ("TIP: x", "TIP&#58; x"),
            ("WARNING: x", "WARNING&#58; x"),
            ("CAUTION: x", "CAUTION&#58; x"),
            ("IMPORTANT: x", "IMPORTANT&#58; x"),
            (".Title", "&#46;Title"),
            ("== Fake heading", "&#61;&#61; Fake heading"),
            (":attr: value", "&#58;attr: value"),
            ("- item", "&#45; item"),
            ("> quote", "&#62; quote"),
            ("'x", "&#39;x"),
            ("`code", "&#96;code"),
            ("", ""),
            ("x\r", "x "),
        ];
        for (input, expected) in pairs {
            assert_eq!(escape_adoc(input), *expected, "input: {input:?}");
        }
    }

    #[test]
    fn escape_line_start_maps_each_listed_line_exactly() {
        let pairs: &[(&str, &str)] = &[
            ("-----", "&#45;----"),
            (".Title", "&#46;Title"),
            (":attr: v", "&#58;attr: v"),
            ("Trace matrix: x", "Trace matrix: x"),
            ("== Heading", "&#61;&#61; Heading"),
            ("* star", "&#42; star"),
        ];
        for (input, expected) in pairs {
            assert_eq!(escape_adoc_line_start(input), *expected, "input: {input:?}");
        }
    }

    #[test]
    fn golden_layout_matches_exact_text() {
        let expected = r##"= Traceability Report

== Inputs

Trace matrix: TM-1 (baseline: not recorded) +
Runs selected: RUN-1, RUN-2

== Report header

[cols="2",options="header"]
|===
| Field
| Value
| Matrix
| TM-1
| Baseline
| not recorded
| Runs
| RUN-1, RUN-2
| Builds
| b1, b2
| Environments
| lab-1
| Testers
| alice
| Obsolete test cases excluded
| 2
| Generated
| 2026-10-31T09:00:00Z
|===

== Document: docs/srs.sdoc

[cols="8",options="header"]
|===
| Requirement
| Level
| Status
| Normal-range test
| Robustness test
| normal_range
| robustness
| other
| HLR-1
| HLR
| VERIFIED
| yes
| yes
| T1 (pass, RUN-1)
| T2 (pass, RUN-2)
| -
| LLR-2
| LLR
| FAILED
| yes
| no
| -
| -
| T3 (fail, RUN-2) +
T4 (NOT RUN, no result)
|===

== Document: docs/empty.sdoc

No traceable content
"##;
        assert_eq!(render_asciidoc(&golden_model()), expected);
    }

    #[test]
    fn golden_inputs_paragraph_has_no_code_fence() {
        let text = render_asciidoc(&golden_model());
        assert!(
            !text.lines().any(|line| line == "----" || line == "```"),
            "{text}"
        );
        assert!(text.starts_with("= Traceability Report\n\n"));
        assert!(text.contains("NOT RUN"));
        assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
    }
}
