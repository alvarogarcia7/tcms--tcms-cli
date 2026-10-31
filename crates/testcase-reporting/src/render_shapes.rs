//! Traceability report shapes A and B (`report --kind traceability --shape a|b`).
//!
//! Both shapes are Markdown views over the same `ReportModel` as the default grid. They read the
//! model only: no clock, no environment lookups, no hash-ordered iteration. Every cell value goes
//! through `render_markdown::cell`.

use crate::model::{DocumentSection, ReportModel, RequirementRow};
use crate::render_markdown::{cell, header_markdown, inputs_block};

const NOT_RECORDED: &str = "not recorded";
const NO_CONTENT: &str = "No traceable content";
const EMPTY_CELL: &str = "-";

/// Shape A: one row per requirement with its test cases as `ID (status)` and the final verdict.
pub fn render_shape_a(model: &ReportModel) -> String {
    render(model, table_a)
}

/// Shape B: per requirement, a `REQUIREMENT | final verdict` row, then `TESTCASE ID | status` rows.
pub fn render_shape_b(model: &ReportModel) -> String {
    render(model, tables_b)
}

/// The shared frame: title, Inputs, header, then one block per document (D-07, D-14).
fn render(model: &ReportModel, body: fn(&[RequirementRow]) -> String) -> String {
    let mut blocks: Vec<String> = vec![
        "# Traceability Report".to_string(),
        inputs_block(&model.inputs.details),
        header_markdown(&model.header),
    ];
    blocks.extend(
        model
            .documents
            .iter()
            .map(|section| document_block(section, body)),
    );

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// Same heading and empty-document rule as the grid's `document_block`.
fn document_block(section: &DocumentSection, body: fn(&[RequirementRow]) -> String) -> String {
    let name = section
        .name
        .as_deref()
        .map_or_else(|| NOT_RECORDED.to_string(), cell);
    let text = if section.rows.is_empty() {
        NO_CONTENT.to_string()
    } else {
        body(&section.rows)
    };
    format!("## Document: {name}\n\n{text}")
}

/// Shape A body: one table per document, one row per requirement.
fn table_a(rows: &[RequirementRow]) -> String {
    let mut lines = vec![
        "| Requirement | Test cases (ID, status) | Final verdict |".to_string(),
        "| --- | --- | --- |".to_string(),
    ];
    lines.extend(rows.iter().map(|row| {
        format!(
            "| {} | {} | {} |",
            cell(&row.requirement_id),
            case_list(row),
            row.status.label()
        )
    }));
    lines.join("\n")
}

/// `ID (outcome)` entries in model order, joined with `<br>`; `-` when the requirement has none.
fn case_list(row: &RequirementRow) -> String {
    if row.cases.is_empty() {
        return EMPTY_CELL.to_string();
    }
    row.cases
        .iter()
        .map(|case| format!("{} ({})", cell(&case.test_case_id), case.outcome.label()))
        .collect::<Vec<_>>()
        .join("<br>")
}

/// Shape B body: per requirement, a verdict table, then a case-status table when it has cases.
/// Tables are separated by a blank line; a requirement without cases gets only the verdict table.
fn tables_b(rows: &[RequirementRow]) -> String {
    rows.iter()
        .map(|row| {
            let mut tables = vec![format!(
                "| Requirement | Final verdict |\n| --- | --- |\n| {} | {} |",
                cell(&row.requirement_id),
                row.status.label()
            )];
            if !row.cases.is_empty() {
                let mut lines = vec![
                    "| TESTCASE ID | Status |".to_string(),
                    "| --- | --- |".to_string(),
                ];
                lines.extend(row.cases.iter().map(|case| {
                    format!(
                        "| {} | {} |",
                        cell(&case.test_case_id),
                        case.outcome.label()
                    )
                }));
                tables.push(lines.join("\n"));
            }
            tables.join("\n\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{
        build_model, CaseEntry, DocumentSection, Header, InputsSection, ReportInputs,
        RequirementRow, TestColumn,
    };
    use crate::render_markdown::header_markdown;
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::select_results;
    use testcase_models::{TestCase, TestType};
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    const GENERATED: &str = "2026-10-31T09:00:00Z";
    const DETAILS: &str = "Trace matrix: TM-1 (baseline: not recorded)\nRuns selected: RUN-1";

    fn header() -> Header {
        Header {
            matrix_id: "TM-1".to_string(),
            baseline_id: Some("BL-1".to_string()),
            run_ids: vec!["RUN-1".to_string()],
            build_ids: vec!["b1".to_string()],
            environments: vec!["lab-1".to_string()],
            testers: vec!["alice".to_string()],
            generated_at: GENERATED.to_string(),
            obsolete_excluded: Some(0),
        }
    }

    fn model(documents: Vec<DocumentSection>) -> ReportModel {
        ReportModel {
            inputs: InputsSection {
                details: DETAILS.to_string(),
            },
            header: header(),
            documents,
        }
    }

    fn doc(name: Option<&str>, rows: Vec<RequirementRow>) -> DocumentSection {
        DocumentSection {
            name: name.map(str::to_string),
            rows,
        }
    }

    fn row(id: &str, status: VerificationStatus, cases: Vec<CaseEntry>) -> RequirementRow {
        RequirementRow {
            requirement_id: id.to_string(),
            level: RequirementLevel::Hlr,
            status,
            normal_range_covered: true,
            robustness_covered: false,
            cases,
        }
    }

    fn case(id: &str, outcome: CaseOutcome) -> CaseEntry {
        CaseEntry {
            test_case_id: id.to_string(),
            column: TestColumn::NormalRange,
            outcome,
            run_id: Some("RUN-1".to_string()),
        }
    }

    /// Two requirements: R-1 verified by two passing cases, R-2 with no tests.
    fn verified_and_no_tests() -> ReportModel {
        model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![
                row(
                    "R-1",
                    VerificationStatus::Verified,
                    vec![
                        case("T-1", CaseOutcome::Pass),
                        case("T-2", CaseOutcome::Pass),
                    ],
                ),
                row("R-2", VerificationStatus::NoTests, Vec::new()),
            ],
        )])
    }

    // ---- Shape A ----

    #[test]
    fn a1_rows_list_cases_and_verdicts() {
        let out = render_shape_a(&verified_and_no_tests());
        assert!(
            out.contains(
                "| Requirement | Test cases (ID, status) | Final verdict |\n| --- | --- | --- |\n"
            ),
            "{out}"
        );
        assert!(
            out.contains("| R-1 | T-1 (pass)<br>T-2 (pass) | VERIFIED |\n"),
            "{out}"
        );
        assert!(out.contains("| R-2 | - | NO TESTS |\n"), "{out}");
    }

    #[test]
    fn a2_failed_and_not_run_cases_both_listed_in_id_order() {
        let m = model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![row(
                "R-3",
                VerificationStatus::Failed,
                vec![
                    case("T-1", CaseOutcome::Fail),
                    case("T-2", CaseOutcome::NotRun),
                ],
            )],
        )]);
        let out = render_shape_a(&m);
        assert!(
            out.contains("| R-3 | T-1 (fail)<br>T-2 (NOT RUN) | FAILED |\n"),
            "{out}"
        );
    }

    #[test]
    fn a3_pipe_and_newline_in_case_id_keep_the_row_on_one_line() {
        let m = model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![row(
                "R-4",
                VerificationStatus::Verified,
                vec![case("T|1\nx", CaseOutcome::Pass)],
            )],
        )]);
        let out = render_shape_a(&m);
        let rows: Vec<&str> = out.lines().filter(|l| l.contains("R-4")).collect();
        assert_eq!(rows, vec!["| R-4 | T\\|1 x (pass) | VERIFIED |"], "{out}");
    }

    // ---- Shape B ----

    #[test]
    fn b1_requirement_verdict_then_case_status_rows() {
        let m = model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![row(
                "R-1",
                VerificationStatus::Partial,
                vec![
                    case("T-1", CaseOutcome::Pass),
                    case("T-2", CaseOutcome::NotRun),
                ],
            )],
        )]);
        let out = render_shape_b(&m);
        let expected = "| Requirement | Final verdict |\n| --- | --- |\n| R-1 | PARTIAL |\n\n\
                        | TESTCASE ID | Status |\n| --- | --- |\n| T-1 | pass |\n| T-2 | NOT RUN |\n";
        assert!(out.ends_with(&format!("\n\n{expected}")), "{out}");
    }

    #[test]
    fn b2_requirement_without_cases_has_no_case_table() {
        let out = render_shape_b(&verified_and_no_tests());
        assert!(out.contains("| R-2 | NO TESTS |\n"), "{out}");
        // R-1 still gets its case table; R-2 must not add a second one after its verdict row.
        assert_eq!(out.matches("| TESTCASE ID | Status |").count(), 1, "{out}");
        assert!(
            !out.contains("| R-2 | NO TESTS |\n\n| TESTCASE ID"),
            "{out}"
        );
    }

    // ---- Both shapes ----

    fn assert_empty_document_marked(out: &str) {
        assert!(
            out.contains("## Document: not recorded\n\nNo traceable content"),
            "{out}"
        );
    }

    #[test]
    fn c1_document_without_requirements_says_no_traceable_content_a() {
        let m = model(vec![doc(None, Vec::new())]);
        assert_empty_document_marked(&render_shape_a(&m));
    }

    #[test]
    fn c1_document_without_requirements_says_no_traceable_content_b() {
        let m = model(vec![doc(None, Vec::new())]);
        assert_empty_document_marked(&render_shape_b(&m));
    }

    fn assert_top_matches_grid(out: &str, m: &ReportModel) {
        assert!(
            out.starts_with("# Traceability Report\n\n## Inputs\n\n```\n"),
            "{out}"
        );
        assert!(
            out.contains(
                "```\nTrace matrix: TM-1 (baseline: not recorded)\nRuns selected: RUN-1\n```"
            ),
            "{out}"
        );
        assert!(out.contains(&header_markdown(&m.header)), "{out}");
    }

    #[test]
    fn c2_top_of_document_matches_the_grid_a() {
        let m = verified_and_no_tests();
        assert_top_matches_grid(&render_shape_a(&m), &m);
    }

    #[test]
    fn c2_top_of_document_matches_the_grid_b() {
        let m = verified_and_no_tests();
        assert_top_matches_grid(&render_shape_b(&m), &m);
    }

    fn blank_matrix_model() -> ReportModel {
        let mut m = verified_and_no_tests();
        m.header.matrix_id = String::new();
        m.header.baseline_id = None;
        m
    }

    #[test]
    fn c3_missing_header_values_show_not_recorded_a() {
        let out = render_shape_a(&blank_matrix_model());
        assert!(out.contains("| Matrix | not recorded |"), "{out}");
        assert!(out.contains("| Baseline | not recorded |"), "{out}");
    }

    #[test]
    fn c3_missing_header_values_show_not_recorded_b() {
        let out = render_shape_b(&blank_matrix_model());
        assert!(out.contains("| Matrix | not recorded |"), "{out}");
        assert!(out.contains("| Baseline | not recorded |"), "{out}");
    }

    // ---- Determinism: the same content from shuffled inputs gives the same bytes ----

    fn case_def(id: &str) -> TestCase {
        let mut tc = TestCase::new("LEGACY".into(), 1, 1, id.into(), "d".into());
        tc.test_type = Some(TestType::NormalRange);
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

    fn build(links: Vec<TraceLink>, runs: Vec<LoadedRun>, tcs: &[TestCase]) -> ReportModel {
        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-1".into(),
                baseline_id: Some("BL-1".into()),
                links,
            },
            runs,
            reconciliation: Reconciliation::default(),
        };
        let selection = select_results(&inputs.runs);
        build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: Some(tcs),
            documents: None,
            generated_at: GENERATED,
        })
    }

    fn sorted_model() -> ReportModel {
        build(
            vec![link("HLR-1", &["T1", "T2"]), link("HLR-2", &["T2"])],
            vec![
                run("RUN-1", "2026-10-28T10:00:00Z", &[("T1", Verdict::Pass)]),
                run("RUN-2", "2026-10-28T11:00:00Z", &[("T2", Verdict::Fail)]),
            ],
            &[case_def("T1"), case_def("T2")],
        )
    }

    fn shuffled_model() -> ReportModel {
        build(
            vec![link("HLR-2", &["T2"]), link("HLR-1", &["T2", "T1"])],
            vec![
                run("RUN-2", "2026-10-28T11:00:00Z", &[("T2", Verdict::Fail)]),
                run("RUN-1", "2026-10-28T10:00:00Z", &[("T1", Verdict::Pass)]),
            ],
            &[case_def("T2"), case_def("T1")],
        )
    }

    #[test]
    fn d1_same_model_gives_same_bytes_a() {
        let m = sorted_model();
        assert_eq!(render_shape_a(&m), render_shape_a(&m));
    }

    #[test]
    fn d1_shuffled_inputs_give_same_bytes_a() {
        assert_eq!(
            render_shape_a(&sorted_model()),
            render_shape_a(&shuffled_model())
        );
    }

    #[test]
    fn d1_same_model_gives_same_bytes_b() {
        let m = sorted_model();
        assert_eq!(render_shape_b(&m), render_shape_b(&m));
    }

    #[test]
    fn d1_shuffled_inputs_give_same_bytes_b() {
        assert_eq!(
            render_shape_b(&sorted_model()),
            render_shape_b(&shuffled_model())
        );
    }

    #[test]
    fn d2_document_ends_with_exactly_one_newline_a() {
        let out = render_shape_a(&verified_and_no_tests());
        assert!(out.ends_with('\n') && !out.ends_with("\n\n"), "{out:?}");
    }

    #[test]
    fn d2_document_ends_with_exactly_one_newline_b() {
        let out = render_shape_b(&verified_and_no_tests());
        assert!(out.ends_with('\n') && !out.ends_with("\n\n"), "{out:?}");
    }
}
