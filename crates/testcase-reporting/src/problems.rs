//! Problem reports view (PRB-01, PRB-02, D-04, D-05, D-06): every decided fail and error result,
//! with its `problem_report_id`, and a visible flag where that id is missing or blank.
//!
//! A pure function of `ViewInput`: no clock, no file access. Only `selection.results` is read, so
//! a superseded result never appears (only the decided result of a test case is listed).
//!
//! The list covers every decided fail or error result, including those of test cases that no
//! requirement links. It can therefore be longer than the traceability report's FAILED and
//! ERRORED requirement rows.

use crate::model::Header;
use crate::non_blank;
use crate::render_markdown::{cell, header_markdown};
use crate::view::{linked_cases, ViewInput};
use std::collections::BTreeMap;
use testcase_verification::Verdict;

/// One decided fail or error result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemRow {
    pub test_case_id: String,
    pub verdict: Verdict,
    pub run_id: String,
    pub build_id: String,
    /// `None` when the source was absent or blank after trimming; otherwise the trimmed id.
    pub problem_report_id: Option<String>,
    /// Sorted, without duplicates; empty when no requirement links the test case.
    pub requirement_ids: Vec<String>,
}

impl ProblemRow {
    /// True when the result has no usable problem report ID (D-05).
    pub fn is_missing(&self) -> bool {
        self.problem_report_id.is_none()
    }
}

/// The problem list plus the report header (D-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemsReport {
    pub header: Header,
    /// Ordered by test case ID.
    pub rows: Vec<ProblemRow>,
    /// Number of rows with `is_missing()`.
    pub missing_count: usize,
}

/// Build the problem list. Iterates `selection.results` (ordered by test case ID), keeps the
/// decided fail and error results (D-04), and attaches the requirements that link each case.
/// A missing or blank problem report ID is kept as `None` and counted (D-05).
pub fn build_problems(input: ViewInput) -> ProblemsReport {
    let requirements: BTreeMap<String, Vec<String>> = linked_cases(input.model)
        .into_iter()
        .map(|case| (case.test_case_id, case.requirement_ids))
        .collect();

    let rows: Vec<ProblemRow> = input
        .selection
        .results
        .values()
        .filter(|result| matches!(result.verdict, Verdict::Fail | Verdict::Error))
        .map(|result| ProblemRow {
            test_case_id: result.test_case_id.clone(),
            verdict: result.verdict,
            run_id: result.run_id.clone(),
            build_id: result.build_id.clone(),
            problem_report_id: non_blank(result.problem_report_id.as_deref())
                .map(|id| id.trim().to_string()),
            requirement_ids: requirements
                .get(&result.test_case_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect();

    let missing_count = rows.iter().filter(|row| row.is_missing()).count();
    ProblemsReport {
        header: input.model.header.clone(),
        rows,
        missing_count,
    }
}

const MISSING_FLAG: &str = "**MISSING problem_report_id**";
const NO_REQUIREMENTS: &str = "-";
const TABLE_HEADER: &str = "| Test case | Verdict | Run | Build | Problem report | Requirements |";
const TABLE_RULE: &str = "| --- | --- | --- | --- | --- | --- |";

/// Render the problem list as one Markdown document ending with exactly one newline.
/// Every model-derived string goes through `cell`; a missing report ID is the visible flag text.
pub fn render_problems_markdown(report: &ProblemsReport) -> String {
    let mut blocks: Vec<String> = vec![
        "# Problem Reports".to_string(),
        header_markdown(&report.header),
        format!("Failed or errored results: {}", report.rows.len()),
        format!("Without problem report: {}", report.missing_count),
    ];
    if report.rows.is_empty() {
        blocks.push("No failed or errored results".to_string());
    } else {
        let mut lines = vec![TABLE_HEADER.to_string(), TABLE_RULE.to_string()];
        lines.extend(report.rows.iter().map(row_line));
        blocks.push(lines.join("\n"));
    }

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

fn row_line(row: &ProblemRow) -> String {
    let problem_report = row
        .problem_report_id
        .as_deref()
        .map_or_else(|| MISSING_FLAG.to_string(), cell);
    let requirements = if row.requirement_ids.is_empty() {
        NO_REQUIREMENTS.to_string()
    } else {
        row.requirement_ids
            .iter()
            .map(|id| cell(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "| {} | {} | {} | {} | {} | {} |",
        cell(&row.test_case_id),
        row.verdict.as_str(),
        cell(&row.run_id),
        cell(&row.build_id),
        problem_report,
        requirements
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, ReportModel};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection, SelectedResult};
    use crate::view::test_support::fixture;
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix};

    fn link(id: &str, cases: &[&str]) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level: RequirementLevel::Hlr,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: true,
            robustness_covered: false,
            run_ids: Vec::new(),
        }
    }

    /// A loaded run; each result carries an optional problem report ID.
    fn run(
        run_id: &str,
        build: &str,
        started: &str,
        results: &[(&str, Verdict, Option<&str>)],
    ) -> LoadedRun {
        let plain: Vec<(&str, Verdict)> = results.iter().map(|(id, v, _)| (*id, *v)).collect();
        let mut rec = record(run_id, build, started, &plain);
        for result in rec.results.iter_mut() {
            if let Some((_, _, pr)) = results.iter().find(|(id, _, _)| *id == result.test_case_id) {
                result.problem_report_id = pr.map(str::to_string);
            }
        }
        LoadedRun::new(format!("{run_id}.yaml").into(), rec).expect("valid run")
    }

    /// Loaded inputs -> run selection -> report model, the same pipeline the CLI uses.
    fn pipeline(links: Vec<TraceLink>, runs: Vec<LoadedRun>) -> (ReportModel, RunSelection) {
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
        let model = build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: None,
            documents: None,
            generated_at: "2026-10-31T09:00:00Z",
        });
        (model, selection)
    }

    fn report_of(model: &ReportModel, selection: &RunSelection) -> ProblemsReport {
        build_problems(ViewInput { model, selection })
    }

    #[test]
    fn verdict_and_report_id_decide_listing_and_flag() {
        let cases: [(Verdict, Option<&str>, bool, bool); 8] = [
            (Verdict::Fail, Some("PR-1"), true, false),
            (Verdict::Error, Some("PR-2"), true, false),
            (Verdict::Fail, None, true, true),
            (Verdict::Error, None, true, true),
            (Verdict::Fail, Some(""), true, true),
            (Verdict::Error, Some("   "), true, true),
            (Verdict::Pass, None, false, false),
            (Verdict::Pass, Some("PR-3"), false, false),
        ];
        for (verdict, pr, listed, flagged) in cases {
            let (model, selection) = pipeline(
                vec![link("HLR-1", &["T1"])],
                vec![run(
                    "RUN-1",
                    "b1",
                    "2026-10-01T10:00:00Z",
                    &[("T1", verdict, pr)],
                )],
            );
            let report = report_of(&model, &selection);
            let label = format!("{verdict:?} {pr:?}");
            assert_eq!(report.rows.len(), usize::from(listed), "{label}");
            assert_eq!(report.missing_count, usize::from(flagged), "{label}");
            if listed {
                assert_eq!(report.rows[0].is_missing(), flagged, "{label}");
            }
        }
    }

    #[test]
    fn not_executed_selection_entry_is_not_listed() {
        // select_results never decides on not_executed; this hand-built entry checks the filter.
        let (model, mut selection) = pipeline(vec![link("HLR-1", &["T1"])], Vec::new());
        selection.results.insert(
            "T1".to_string(),
            SelectedResult {
                test_case_id: "T1".to_string(),
                verdict: Verdict::NotExecuted,
                run_id: "RUN-1".to_string(),
                build_id: "b1".to_string(),
                started_at: "2026-10-01T10:00:00Z".to_string(),
                problem_report_id: None,
                verification_result_ref: None,
            },
        );
        let report = report_of(&model, &selection);
        assert!(report.rows.is_empty());
        assert_eq!(report.missing_count, 0);
    }

    #[test]
    fn fixture_rows_are_t2_then_t3() {
        let (model, selection) = fixture();
        let report = build_problems(ViewInput {
            model: &model,
            selection: &selection,
        });
        let ids: Vec<&str> = report
            .rows
            .iter()
            .map(|r| r.test_case_id.as_str())
            .collect();
        assert_eq!(ids, ["T2", "T3"]);
        assert_eq!(report.missing_count, 1);

        let t2 = &report.rows[0];
        assert_eq!(t2.problem_report_id.as_deref(), Some("PR-7"));
        assert!(!t2.is_missing());
        assert_eq!(t2.run_id, "RUN-1");
        assert_eq!(t2.build_id, "b1");
        assert_eq!(t2.requirement_ids, ["HLR-1"]);

        let t3 = &report.rows[1];
        assert!(t3.is_missing());
        assert_eq!(t3.verdict, Verdict::Error);
        assert_eq!(t3.requirement_ids, ["LLR-2"]);
    }

    #[test]
    fn superseded_fail_gives_empty_rows() {
        let (model, selection) = pipeline(
            vec![link("HLR-1", &["T1"])],
            vec![
                run(
                    "RUN-A",
                    "b1",
                    "2026-10-01T10:00:00Z",
                    &[("T1", Verdict::Fail, Some("PR-9"))],
                ),
                run(
                    "RUN-B",
                    "b2",
                    "2026-10-02T10:00:00Z",
                    &[("T1", Verdict::Pass, None)],
                ),
            ],
        );
        let report = report_of(&model, &selection);
        assert!(report.rows.is_empty());
        assert_eq!(report.missing_count, 0);
    }

    #[test]
    fn unlinked_failure_is_listed_with_no_requirements() {
        let (model, selection) = pipeline(
            vec![link("HLR-1", &["T1"])],
            vec![run(
                "RUN-1",
                "b1",
                "2026-10-01T10:00:00Z",
                &[
                    ("T1", Verdict::Pass, None),
                    ("T9", Verdict::Fail, Some("PR-4")),
                ],
            )],
        );
        let report = report_of(&model, &selection);
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].test_case_id, "T9");
        assert!(report.rows[0].requirement_ids.is_empty());
        assert_eq!(report.missing_count, 0);
    }

    #[test]
    fn shuffled_inputs_give_equal_report() {
        let a = pipeline(
            vec![link("HLR-1", &["T1"]), link("HLR-2", &["T1", "T2"])],
            vec![
                run(
                    "RUN-1",
                    "b1",
                    "2026-10-01T10:00:00Z",
                    &[("T1", Verdict::Fail, Some("PR-1"))],
                ),
                run(
                    "RUN-2",
                    "b1",
                    "2026-10-02T10:00:00Z",
                    &[("T2", Verdict::Error, None)],
                ),
            ],
        );
        let b = pipeline(
            vec![link("HLR-2", &["T2", "T1"]), link("HLR-1", &["T1"])],
            vec![
                run(
                    "RUN-2",
                    "b1",
                    "2026-10-02T10:00:00Z",
                    &[("T2", Verdict::Error, None)],
                ),
                run(
                    "RUN-1",
                    "b1",
                    "2026-10-01T10:00:00Z",
                    &[("T1", Verdict::Fail, Some("PR-1"))],
                ),
            ],
        );
        assert_eq!(report_of(&a.0, &a.1), report_of(&b.0, &b.1));
    }

    #[test]
    fn selection_results_map_is_keyed_for_the_fixture() {
        // Guards the fixture assumption used above: T2 is decided by RUN-1, not by RUN-2.
        let (_, selection) = fixture();
        assert_eq!(selection.results["T2"].run_id, "RUN-1");
    }

    // Task 2: Markdown rendering. Golden tests are written before the renderer exists.
    mod markdown {
        use super::*;
        use crate::render_markdown::header_markdown;

        #[test]
        fn fixture_renders_exact_golden_markdown() {
            let (model, selection) = fixture();
            let report = report_of(&model, &selection);
            let expected = r#"# Problem Reports

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1, RUN-2 |
| Builds | b1, b2 |
| Environments | e |
| Testers | t |
| Obsolete test cases excluded | not recorded |
| Generated | 2026-10-31T09:00:00Z |

Failed or errored results: 2

Without problem report: 1

| Test case | Verdict | Run | Build | Problem report | Requirements |
| --- | --- | --- | --- | --- | --- |
| T2 | fail | RUN-1 | b1 | PR-7 | HLR-1 |
| T3 | error | RUN-1 | b1 | **MISSING problem_report_id** | LLR-2 |
"#;
            assert_eq!(render_problems_markdown(&report), expected);
        }

        #[test]
        fn fixture_rows_carry_the_flag_text_and_counts() {
            let (model, selection) = fixture();
            let out = render_problems_markdown(&report_of(&model, &selection));
            assert!(
                out.contains("| T2 | fail | RUN-1 | b1 | PR-7 | HLR-1 |"),
                "{out}"
            );
            assert!(
                out.contains("| T3 | error | RUN-1 | b1 | **MISSING problem_report_id** | LLR-2 |"),
                "{out}"
            );
            assert!(out.contains("Failed or errored results: 2"), "{out}");
            assert!(out.contains("Without problem report: 1"), "{out}");
        }

        #[test]
        fn empty_report_says_so_and_counts_zero() {
            let (model, _) = fixture();
            let report = ProblemsReport {
                header: model.header.clone(),
                rows: Vec::new(),
                missing_count: 0,
            };
            let out = render_problems_markdown(&report);
            assert!(out.contains("No failed or errored results"), "{out}");
            assert!(out.contains("Without problem report: 0"), "{out}");
            assert!(out.contains("Failed or errored results: 0"), "{out}");
            assert!(!out.contains("| Test case |"), "{out}");
            // The header is still the shared Phase 2 header.
            assert!(out.contains(&header_markdown(&model.header)), "{out}");
        }

        #[test]
        fn hostile_problem_report_id_is_escaped_and_stays_on_one_line() {
            let (model, selection) = fixture();
            let mut report = report_of(&model, &selection);
            report.rows[0].problem_report_id = Some("PR|1\n9".to_string());
            report.rows[0].requirement_ids.clear();
            let out = render_problems_markdown(&report);
            let matches: Vec<&str> = out.lines().filter(|l| l.contains("PR\\|1 9")).collect();
            assert_eq!(matches.len(), 1, "{out}");
            assert_eq!(
                matches[0], "| T2 | fail | RUN-1 | b1 | PR\\|1 9 | - |",
                "{out}"
            );
        }

        #[test]
        fn identical_inputs_give_identical_bytes_and_one_trailing_newline() {
            let (model_a, selection_a) = fixture();
            let (model_b, selection_b) = fixture();
            let a = render_problems_markdown(&report_of(&model_a, &selection_a));
            let b = render_problems_markdown(&report_of(&model_b, &selection_b));
            assert_eq!(a.as_bytes(), b.as_bytes());
            assert!(a.ends_with('\n'));
            assert!(!a.ends_with("\n\n"));
        }
    }
}
