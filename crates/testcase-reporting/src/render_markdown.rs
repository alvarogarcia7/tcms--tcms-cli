//! Markdown renderer for the traceability report (D-07..D-14).
//!
//! Reads `ReportModel` only. No clock, no environment lookups, no hash-ordered iteration, so the
//! same model always gives the same bytes (D-12). Markdown is the only format in Phase 2 (D-13).

use crate::model::{DocumentSection, Header, ReportModel, RequirementRow, TestColumn};
use crate::non_blank;

const NOT_RECORDED: &str = "not recorded";
const NO_CONTENT: &str = "No traceable content";
const EMPTY_CELL: &str = "-";
const TABLE_HEADER: &str = "| Requirement | Level | Status | Normal-range test | Robustness test | normal_range | robustness | other |";
const TABLE_RULE: &str = "| --- | --- | --- | --- | --- | --- | --- | --- |";

/// Render the whole report as one Markdown document ending with exactly one newline.
pub fn render_markdown(model: &ReportModel) -> String {
    let mut blocks: Vec<String> = vec![
        "# Traceability Report".to_string(),
        inputs_block(&model.inputs.details),
        header_markdown(&model.header),
    ];
    blocks.extend(model.documents.iter().map(document_block));

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// The Phase 1 input text, verbatim, inside a fence longer than any backtick run in it (D-14).
fn inputs_block(details: &str) -> String {
    let fence = "`".repeat(fence_len(details));
    // One trailing newline is structural (the line break before the fence), not content.
    let body = details.strip_suffix('\n').unwrap_or(details);
    format!("## Inputs\n\n{fence}\n{body}\n{fence}")
}

/// Fence length: at least three, and one more than the longest run of backticks in `text`.
fn fence_len(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    (longest + 1).max(3)
}

/// The `## Report header` section (D-06): shared by the traceability report and the Phase 3 views.
pub fn header_markdown(header: &Header) -> String {
    let rows = [
        (
            "Matrix",
            non_blank(Some(&header.matrix_id)).map_or_else(|| NOT_RECORDED.to_string(), cell),
        ),
        (
            "Baseline",
            non_blank(header.baseline_id.as_deref()).map_or_else(|| NOT_RECORDED.to_string(), cell),
        ),
        ("Runs", list(&header.run_ids)),
        ("Builds", list(&header.build_ids)),
        ("Environments", list(&header.environments)),
        ("Testers", list(&header.testers)),
        (
            "Obsolete test cases excluded",
            header
                .obsolete_excluded
                .map_or_else(|| NOT_RECORDED.to_string(), |n| n.to_string()),
        ),
        ("Generated", cell(&header.generated_at)),
    ];

    let mut lines = vec![
        "## Report header".to_string(),
        String::new(),
        "| Field | Value |".to_string(),
        "| --- | --- |".to_string(),
    ];
    lines.extend(
        rows.iter()
            .map(|(field, value)| format!("| {field} | {value} |")),
    );
    lines.join("\n")
}

fn document_block(section: &DocumentSection) -> String {
    let name = section
        .name
        .as_deref()
        .map_or_else(|| NOT_RECORDED.to_string(), cell);
    let body = if section.rows.is_empty() {
        NO_CONTENT.to_string()
    } else {
        let mut lines = vec![TABLE_HEADER.to_string(), TABLE_RULE.to_string()];
        lines.extend(section.rows.iter().map(row_line));
        lines.join("\n")
    };
    format!("## Document: {name}\n\n{body}")
}

fn row_line(row: &RequirementRow) -> String {
    let mut cells = vec![
        cell(&row.requirement_id),
        row.level.to_string(),
        row.status.label().to_string(),
        yes_no(row.normal_range_covered).to_string(),
        yes_no(row.robustness_covered).to_string(),
    ];
    cells.extend(
        TestColumn::ALL
            .iter()
            .map(|&column| column_cell(row, column)),
    );
    format!("| {} |", cells.join(" | "))
}

/// The cases of one test-type column; `-` when the column has none (TRC-04).
fn column_cell(row: &RequirementRow, column: TestColumn) -> String {
    let entries: Vec<String> = row
        .cases
        .iter()
        .filter(|case| case.column == column)
        .map(|case| {
            let run = case
                .run_id
                .as_deref()
                .map_or_else(|| "no result".to_string(), cell);
            format!(
                "{} ({}, {run})",
                cell(&case.test_case_id),
                case.outcome.label()
            )
        })
        .collect();
    if entries.is_empty() {
        EMPTY_CELL.to_string()
    } else {
        entries.join("<br>")
    }
}

fn yes_no(covered: bool) -> &'static str {
    if covered {
        "yes"
    } else {
        "no"
    }
}

/// Values joined with `, `; an empty list is "not recorded" (D-11).
fn list(values: &[String]) -> String {
    if values.is_empty() {
        NOT_RECORDED.to_string()
    } else {
        values
            .iter()
            .map(|v| cell(v))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Make one model-derived value safe inside a table cell: `|` is escaped, and line breaks become
/// a space so a value cannot forge extra columns or rows (T-02-06). This is table safety only.
pub fn cell(text: &str) -> String {
    text.replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CaseEntry, DocumentSection, Header, ReportModel, RequirementRow};
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use testcase_verification::RequirementLevel;

    /// Hand-built model: one populated document, one empty document, every header field set
    /// except the baseline, and one obsolete count.
    fn golden_model() -> ReportModel {
        ReportModel {
            inputs: crate::model::InputsSection {
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
    fn golden_layout_matches_exact_text() {
        let expected = r#"# Traceability Report

## Inputs

```
Trace matrix: TM-1 (baseline: not recorded)
Runs selected: RUN-1, RUN-2
```

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1, RUN-2 |
| Builds | b1, b2 |
| Environments | lab-1 |
| Testers | alice |
| Obsolete test cases excluded | 2 |
| Generated | 2026-10-31T09:00:00Z |

## Document: docs/srs.sdoc

| Requirement | Level | Status | Normal-range test | Robustness test | normal_range | robustness | other |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HLR-1 | HLR | VERIFIED | yes | yes | T1 (pass, RUN-1) | T2 (pass, RUN-2) | - |
| LLR-2 | LLR | FAILED | yes | no | - | - | T3 (fail, RUN-2)<br>T4 (NOT RUN, no result) |

## Document: docs/empty.sdoc

No traceable content
"#;
        assert_eq!(render_markdown(&golden_model()), expected);
    }

    #[test]
    fn blank_matrix_id_and_baseline_render_not_recorded() {
        for blank in ["", "   "] {
            let mut header = golden_model().header;
            header.matrix_id = blank.to_string();
            header.baseline_id = Some(blank.to_string());
            let block = header_markdown(&header);
            assert!(block.contains("| Matrix | not recorded |"), "{block}");
            assert!(block.contains("| Baseline | not recorded |"), "{block}");
            assert!(!block.contains("| Baseline |  |"), "{block}");
        }
    }

    #[test]
    fn header_markdown_has_exact_eight_rows() {
        let header = Header {
            matrix_id: "TM-9".to_string(),
            baseline_id: None,
            run_ids: vec!["RUN-1".to_string()],
            build_ids: Vec::new(),
            environments: vec!["lab|1".to_string()],
            testers: Vec::new(),
            generated_at: "2026-10-31T09:00:00Z".to_string(),
            obsolete_excluded: None,
        };
        let expected = "## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-9 |
| Baseline | not recorded |
| Runs | RUN-1 |
| Builds | not recorded |
| Environments | lab\\|1 |
| Testers | not recorded |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |";
        assert_eq!(header_markdown(&header), expected);
    }

    mod pipeline {
        use super::super::*;
        use crate::load::{LoadedInputs, Reconciliation};
        use crate::model::{build_model, ReportInputs, ReportModel};
        use crate::run::{test_support::record, LoadedRun};
        use crate::select::select_results;
        use testcase_models::{TestCase, TestType};
        use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

        const GENERATED: &str = "2026-10-31T09:00:00Z";

        fn case(id: &str, test_type: Option<TestType>) -> TestCase {
            let mut tc = TestCase::new("LEGACY".into(), 1, 1, id.into(), "d".into());
            tc.test_type = test_type;
            tc
        }

        fn link(id: &str, cases: &[&str]) -> TraceLink {
            TraceLink {
                requirement_id: id.into(),
                level: RequirementLevel::Hlr,
                test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
                normal_range_covered: true,
                robustness_covered: false,
                run_ids: Vec::new(),
            }
        }

        fn run(run_id: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
            let rec = record(run_id, "b1", started, results);
            LoadedRun::new(format!("{run_id}.yaml").into(), rec).expect("valid run")
        }

        /// Full pipeline: loaded inputs -> run selection -> model -> Markdown.
        fn render(
            links: Vec<TraceLink>,
            runs: Vec<LoadedRun>,
            test_cases: Option<&[TestCase]>,
            generated_at: &str,
        ) -> String {
            let inputs = LoadedInputs {
                matrix_path: "matrix.yaml".into(),
                matrix: TraceMatrix {
                    doc_type: "trace_matrix".into(),
                    schema: "tcms/trace-matrix.schema.json".into(),
                    id: "TM-1".into(),
                    baseline_id: None,
                    links,
                },
                runs,
                reconciliation: Reconciliation::default(),
            };
            let selection = select_results(&inputs.runs);
            let model: ReportModel = build_model(&ReportInputs {
                inputs: &inputs,
                selection: &selection,
                test_cases,
                documents: None,
                generated_at,
            });
            render_markdown(&model)
        }

        fn mixed_runs() -> Vec<LoadedRun> {
            vec![
                run(
                    "RUN-1",
                    "2026-10-28T10:00:00Z",
                    &[("T1", Verdict::Pass), ("T2", Verdict::Fail)],
                ),
                run("RUN-2", "2026-10-28T11:00:00Z", &[("T3", Verdict::Error)]),
            ]
        }

        fn mixed_links() -> Vec<TraceLink> {
            vec![
                link("HLR-1", &["T1"]),
                link("HLR-2", &["T1", "T9"]),
                link("HLR-3", &["T2"]),
                link("HLR-4", &["T3"]),
                link("HLR-5", &[]),
            ]
        }

        fn line_with<'a>(out: &'a str, needle: &str) -> &'a str {
            out.lines()
                .find(|l| l.starts_with("| ") && l.contains(needle))
                .unwrap_or_else(|| panic!("no row containing {needle}\n{out}"))
        }

        #[test]
        fn byte_identical_snapshot_from_separate_pipeline_runs() {
            let tcs = [case("T1", Some(TestType::NormalRange))];
            let first = render(
                vec![link("HLR-1", &["T1"])],
                mixed_runs(),
                Some(&tcs),
                GENERATED,
            );
            let second = render(
                vec![link("HLR-1", &["T1"])],
                mixed_runs(),
                Some(&tcs),
                GENERATED,
            );
            assert_eq!(first, second);
            assert_eq!(first.as_bytes(), second.as_bytes());
        }

        #[test]
        fn changed_generated_time_changes_only_the_generated_row() {
            let a = render(mixed_links(), mixed_runs(), None, GENERATED);
            let b = render(mixed_links(), mixed_runs(), None, "2026-11-01T00:00:00Z");
            let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
            assert_eq!(la.len(), lb.len());
            let differing: Vec<(&str, &str)> = la
                .iter()
                .zip(lb.iter())
                .filter(|(x, y)| x != y)
                .map(|(x, y)| (*x, *y))
                .collect();
            assert_eq!(differing.len(), 1);
            assert!(differing[0].0.starts_with("| Generated |"));
            assert!(differing[0].1.starts_with("| Generated |"));
        }

        #[test]
        fn inputs_fence_grows_past_backtick_runs() {
            let model = ReportModel {
                inputs: crate::model::InputsSection {
                    details: "line one\n```\nline with ````` five\nend".to_string(),
                },
                header: Header {
                    matrix_id: "TM-1".into(),
                    baseline_id: None,
                    run_ids: Vec::new(),
                    build_ids: Vec::new(),
                    environments: Vec::new(),
                    testers: Vec::new(),
                    generated_at: GENERATED.into(),
                    obsolete_excluded: None,
                },
                documents: Vec::new(),
            };
            let out = render_markdown(&model);
            // Longest run is five backticks, so the fence is six.
            assert!(out.contains("\n``````\nline one\n```\nline with ````` five\nend\n``````\n"));
            assert!(out.contains("## Report header"));
        }

        #[test]
        fn missing_result_is_not_run_never_pass() {
            let out = render(
                vec![link("HLR-1", &["T1", "T2"])],
                vec![run(
                    "RUN-1",
                    "2026-10-28T10:00:00Z",
                    &[("T1", Verdict::Pass)],
                )],
                None,
                GENERATED,
            );
            let row = line_with(&out, "HLR-1");
            assert!(row.contains("| PARTIAL |"), "{row}");
            assert!(row.contains("T2 (NOT RUN, no result)"), "{row}");
            assert!(!row.contains("T2 (pass"), "{row}");
        }

        #[test]
        fn all_missing_results_render_not_run_status() {
            let out = render(vec![link("HLR-1", &["T1"])], Vec::new(), None, GENERATED);
            let row = line_with(&out, "HLR-1");
            assert!(row.contains("| NOT RUN |"), "{row}");
            assert!(row.contains("T1 (NOT RUN, no result)"), "{row}");
        }

        #[test]
        fn errored_and_failed_render_their_words() {
            let out = render(mixed_links(), mixed_runs(), None, GENERATED);
            assert!(line_with(&out, "HLR-4").contains("| ERRORED |"));
            assert!(line_with(&out, "HLR-3").contains("| FAILED |"));
            assert!(line_with(&out, "HLR-3").contains("T2 (fail, RUN-1)"));
            assert!(line_with(&out, "HLR-4").contains("T3 (error, RUN-2)"));
        }

        #[test]
        fn verified_rows_show_only_pass_entries() {
            let out = render(
                vec![
                    link("HLR-1", &["T1"]),
                    link("HLR-2", &["T1", "T2"]),
                    link("HLR-3", &["T2"]),
                ],
                mixed_runs(),
                None,
                GENERATED,
            );
            let verified: Vec<&str> = out
                .lines()
                .filter(|l| l.starts_with("| ") && l.contains("| VERIFIED |"))
                .collect();
            assert_eq!(verified.len(), 1, "{out}");
            for line in verified {
                assert!(!line.contains("fail"), "{line}");
                assert!(!line.contains("error"), "{line}");
                assert!(!line.contains("NOT RUN"), "{line}");
                assert!(line.contains("(pass, RUN-1)"), "{line}");
            }
        }

        #[test]
        fn pipe_and_newline_in_requirement_id_stay_on_one_line() {
            let out = render(
                vec![link("HLR|1\nx", &["T1"])],
                mixed_runs(),
                None,
                GENERATED,
            );
            let matches: Vec<&str> = out.lines().filter(|l| l.contains("HLR\\|1 x")).collect();
            assert_eq!(matches.len(), 1, "{out}");
            assert!(matches[0].starts_with("| HLR\\|1 x |"), "{}", matches[0]);
        }

        #[test]
        fn requirement_without_cases_is_no_tests_with_dash_cells() {
            let out = render(vec![link("HLR-5", &[])], Vec::new(), None, GENERATED);
            let row = line_with(&out, "HLR-5");
            assert!(row.contains("| NO TESTS |"), "{row}");
            assert!(row.ends_with("| - | - | - |"), "{row}");
        }

        #[test]
        fn shuffled_inputs_render_the_same_bytes() {
            let tcs_sorted = [
                case("T1", Some(TestType::NormalRange)),
                case("T2", Some(TestType::Robustness)),
            ];
            let tcs_shuffled = [
                case("T2", Some(TestType::Robustness)),
                case("T1", Some(TestType::NormalRange)),
            ];
            let sorted = render(
                vec![link("HLR-1", &["T1", "T2"]), link("HLR-2", &["T2"])],
                vec![
                    run("RUN-1", "2026-10-28T10:00:00Z", &[("T1", Verdict::Pass)]),
                    run("RUN-2", "2026-10-28T11:00:00Z", &[("T2", Verdict::Pass)]),
                ],
                Some(&tcs_sorted),
                GENERATED,
            );
            let shuffled = render(
                vec![link("HLR-2", &["T2"]), link("HLR-1", &["T2", "T1"])],
                vec![
                    run("RUN-2", "2026-10-28T11:00:00Z", &[("T2", Verdict::Pass)]),
                    run("RUN-1", "2026-10-28T10:00:00Z", &[("T1", Verdict::Pass)]),
                ],
                Some(&tcs_shuffled),
                GENERATED,
            );
            assert_eq!(sorted, shuffled);
        }
    }
}
