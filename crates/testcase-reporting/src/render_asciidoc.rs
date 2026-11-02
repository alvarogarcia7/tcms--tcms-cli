//! AsciiDoc renderer for the traceability report and the Phase 3 views (OUT-01, D-02, D-05, D-06).
//!
//! Reads the report model and the Phase 3 view structs only: no clock, no environment lookups,
//! no hash-ordered iteration, so the same model always gives the same bytes (D-05). Every string
//! that comes from the model passes through `escape_adoc` (D-02, D-06).

use crate::model::{DocumentSection, Header, ReportModel, RequirementRow, TestColumn};
use crate::gaps::GapsReport;
use crate::non_blank;
use crate::problems::ProblemsReport;
use crate::summary::SummaryReport;

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

/// Summary view as one AsciiDoc document (SUM-01, OUT-01). Stub for the RED commit.
pub fn render_asciidoc_summary(report: &SummaryReport) -> String {
    let _ = report;
    unimplemented!("04-01 Task 2: render_asciidoc_summary")
}

/// Gaps view as one AsciiDoc document (GAP-01, OUT-01). Stub for the RED commit.
pub fn render_asciidoc_gaps(report: &GapsReport) -> String {
    let _ = report;
    unimplemented!("04-01 Task 2: render_asciidoc_gaps")
}

/// Problem reports view as one AsciiDoc document (PRB-01, OUT-01). Stub for the RED commit.
pub fn render_asciidoc_problems(report: &ProblemsReport) -> String {
    let _ = report;
    unimplemented!("04-01 Task 2: render_asciidoc_problems")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CaseEntry, DocumentSection, Header, InputsSection, RequirementRow, TestColumn,
    };
    use crate::gaps::{GapCase, GapRequirement};
    use crate::problems::ProblemRow;
    use crate::summary::{RequirementSummaryRow, RunSummaryRow};
    use crate::view::ResultCounts;
    use testcase_verification::Verdict;
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

    fn summary_fixture() -> SummaryReport {
        SummaryReport {
            header: golden_model().header,
            runs: vec![
                RunSummaryRow {
                    run_id: "RUN|1".to_string(),
                    build_id: "b1".to_string(),
                    used: true,
                    excluded_reason: None,
                    counts: ResultCounts {
                        pass: 1,
                        fail: 1,
                        not_executed: 0,
                        error: 1,
                        total: 3,
                    },
                },
                RunSummaryRow {
                    run_id: "RUN-2".to_string(),
                    build_id: "b2".to_string(),
                    used: false,
                    excluded_reason: Some("superseded".to_string()),
                    counts: ResultCounts {
                        pass: 0,
                        fail: 0,
                        not_executed: 2,
                        error: 0,
                        total: 2,
                    },
                },
            ],
            requirements: vec![
                RequirementSummaryRow {
                    requirement_id: "HLR-1".to_string(),
                    level: RequirementLevel::Hlr,
                    status: VerificationStatus::Verified,
                    counts: ResultCounts {
                        pass: 2,
                        fail: 0,
                        not_executed: 0,
                        error: 0,
                        total: 2,
                    },
                },
                RequirementSummaryRow {
                    requirement_id: "LLR-2".to_string(),
                    level: RequirementLevel::Llr,
                    status: VerificationStatus::Failed,
                    counts: ResultCounts {
                        pass: 0,
                        fail: 1,
                        not_executed: 1,
                        error: 0,
                        total: 2,
                    },
                },
            ],
            linked: ResultCounts {
                pass: 2,
                fail: 1,
                not_executed: 1,
                error: 0,
                total: 4,
            },
        }
    }

    #[test]
    fn summary_golden_matches_exact_text() {
        let expected = r##"= Run Summary

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

== Results per run

[cols="8",options="header"]
|===
| Run
| Build
| Used
| pass
| fail
| not_executed
| error
| Total
| RUN&#124;1
| b1
| yes
| 1
| 1
| 0
| 1
| 3
| RUN-2
| b2
| no: superseded
| 0
| 0
| 2
| 0
| 2
|===

== Results per requirement

[cols="8",options="header"]
|===
| Requirement
| Level
| Status
| pass
| fail
| not_executed
| error
| Total
| HLR-1
| HLR
| VERIFIED
| 2
| 0
| 0
| 0
| 2
| LLR-2
| LLR
| FAILED
| 0
| 1
| 1
| 0
| 2
|===

== Linked test cases

pass 2, fail 1, not_executed 1, error 0, total 4
"##;
        assert_eq!(render_asciidoc_summary(&summary_fixture()), expected);
    }

    #[test]
    fn summary_empty_tables_use_the_markdown_placeholders() {
        let mut report = summary_fixture();
        report.runs.clear();
        report.requirements.clear();
        let text = render_asciidoc_summary(&report);
        assert!(text.contains("\nNo runs loaded\n"), "{text}");
        assert!(text.contains("\nNo requirements\n"), "{text}");
        assert!(!text.contains("== Inputs"), "{text}");
    }

    fn gaps_fixture() -> GapsReport {
        GapsReport {
            header: golden_model().header,
            no_tests: vec![GapRequirement {
                requirement_id: "HLR-3".to_string(),
                level: RequirementLevel::Hlr,
                document: Some("docs/srs.sdoc".to_string()),
            }],
            missing_normal_range: Vec::new(),
            missing_robustness: vec![GapRequirement {
                requirement_id: "LLR-2".to_string(),
                level: RequirementLevel::Llr,
                document: None,
            }],
            no_result: vec![GapCase {
                test_case_id: "T4".to_string(),
                requirement_ids: vec!["LLR-2".to_string()],
            }],
        }
    }

    #[test]
    fn gaps_golden_matches_exact_text() {
        let expected = r##"= Coverage Gaps

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

== Requirements with no test cases (1)

[cols="3",options="header"]
|===
| Requirement
| Level
| Document
| HLR-3
| HLR
| docs/srs.sdoc
|===

== Requirements missing a normal-range test (0)

None

== Requirements missing a robustness test (1)

[cols="3",options="header"]
|===
| Requirement
| Level
| Document
| LLR-2
| LLR
| not recorded
|===

== Test cases with no recorded result (1)

[cols="2",options="header"]
|===
| Test case
| Requirements
| T4
| LLR-2
|===
"##;
        assert_eq!(render_asciidoc_gaps(&gaps_fixture()), expected);
    }

    fn problems_fixture() -> ProblemsReport {
        ProblemsReport {
            header: golden_model().header,
            rows: vec![
                ProblemRow {
                    test_case_id: "T3".to_string(),
                    verdict: Verdict::Fail,
                    run_id: "RUN-2".to_string(),
                    build_id: "b2".to_string(),
                    problem_report_id: Some("PR-7".to_string()),
                    requirement_ids: vec!["LLR-2".to_string()],
                },
                ProblemRow {
                    test_case_id: "T5".to_string(),
                    verdict: Verdict::Error,
                    run_id: "RUN-1".to_string(),
                    build_id: "b1".to_string(),
                    problem_report_id: None,
                    requirement_ids: Vec::new(),
                },
            ],
            missing_count: 1,
        }
    }

    #[test]
    fn problems_golden_matches_exact_text() {
        let expected = r##"= Problem Reports

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

Failed or errored results: 2

Without problem report: 1

[cols="6",options="header"]
|===
| Test case
| Verdict
| Run
| Build
| Problem report
| Requirements
| T3
| fail
| RUN-2
| b2
| PR-7
| LLR-2
| T5
| error
| RUN-1
| b1
| **MISSING problem_report_id**
| -
|===
"##;
        assert_eq!(render_asciidoc_problems(&problems_fixture()), expected);
    }

    #[test]
    fn problems_flag_appears_only_on_the_missing_row() {
        let text = render_asciidoc_problems(&problems_fixture());
        let flagged: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("**MISSING problem_report_id**"))
            .collect();
        assert_eq!(flagged, vec!["| **MISSING problem_report_id**"]);
        assert!(text.contains("Without problem report: 1"), "{text}");
    }

    #[test]
    fn problems_empty_list_uses_the_markdown_placeholder() {
        let mut report = problems_fixture();
        report.rows.clear();
        report.missing_count = 0;
        let text = render_asciidoc_problems(&report);
        assert!(text.contains("\nNo failed or errored results\n"), "{text}");
        assert!(text.contains("Failed or errored results: 0"), "{text}");
    }
}
