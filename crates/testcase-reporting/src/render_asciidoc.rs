//! AsciiDoc renderer for the traceability report and the Phase 3 views (OUT-01, D-02, D-05, D-06).
//!
//! Reads the report model and the Phase 3 view structs only: no clock, no environment lookups,
//! no hash-ordered iteration, so the same model always gives the same bytes (D-05). Every string
//! that comes from the model passes through `escape_adoc` (D-02, D-06).

use crate::model::ReportModel;

/// Start-of-line characters that are written as numeric references at the start of a cell or a
/// paragraph line, so a value cannot begin a block, a heading, a list or an attribute entry.
pub(crate) const LINE_START_CHARS: &[char] = &[
    '.', '=', ':', '-', '*', '>', '|', '[', '+', '\'', '`',
];

/// Escape one model value for an AsciiDoc table cell or paragraph (D-02, D-06). Stub for the
/// RED commit; replaced by the implementation commit.
pub fn escape_adoc(text: &str) -> String {
    let _ = text;
    unimplemented!("04-01 Task 1: escape_adoc")
}

/// Escape one free-text line of a paragraph (the Inputs block). Stub for the RED commit.
pub fn escape_adoc_line_start(line: &str) -> String {
    let _ = line;
    unimplemented!("04-01 Task 1: escape_adoc_line_start")
}

/// Render the traceability report as one AsciiDoc document. Stub for the RED commit.
pub fn render_asciidoc(model: &ReportModel) -> String {
    let _ = model;
    unimplemented!("04-01 Task 1: render_asciidoc")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CaseEntry, DocumentSection, Header, InputsSection, RequirementRow, TestColumn};
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
            ("include::/etc/passwd[]", "include&#58;&#58;/etc/passwd&#91;&#93;"),
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
        assert!(!text.lines().any(|line| line == "----" || line == "```"), "{text}");
        assert!(text.starts_with("= Traceability Report\n\n"));
        assert!(text.contains("NOT RUN"));
        assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
    }
}
