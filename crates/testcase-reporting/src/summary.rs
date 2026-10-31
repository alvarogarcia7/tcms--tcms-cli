//! Summary view (SUM-01, D-02, D-06): counts of `pass`, `fail`, `error` and `not_executed` results
//! per run and per requirement, plus the total, rendered as Markdown.
//!
//! A pure function of `ViewInput`: no clock, no file access. Per-run buckets come from the
//! decided results of the selection, so a superseded result is never counted (D-02).

use crate::model::Header;
use crate::render_markdown::{cell, header_markdown};
use crate::rollup::{CaseOutcome, VerificationStatus};
use crate::select::RunUse;
use crate::view::{linked_cases, ResultCounts, ViewInput};
use testcase_verification::{RequirementLevel, Verdict};

/// One run of the selection, in selection order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummaryRow {
    pub run_id: String,
    pub build_id: String,
    pub used: bool,
    /// Why the run contributed nothing; `None` for a used run.
    pub excluded_reason: Option<String>,
    pub counts: ResultCounts,
}

/// One requirement of the model, in model order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementSummaryRow {
    pub requirement_id: String,
    pub level: RequirementLevel,
    pub status: VerificationStatus,
    pub counts: ResultCounts,
}

/// The whole summary, ready for a renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryReport {
    pub header: Header,
    pub runs: Vec<RunSummaryRow>,
    pub requirements: Vec<RequirementSummaryRow>,
    /// Distinct live linked test cases, counted once each.
    pub linked: ResultCounts,
}

/// Build the summary from the model and the selection it was built from.
pub fn build_summary(input: ViewInput) -> SummaryReport {
    let selection = input.selection;

    let runs = selection
        .runs
        .iter()
        .map(|run| {
            let mut counts = ResultCounts::default();
            // Only decided results are in `results`; a superseded result is not there (D-02).
            for result in selection
                .results
                .values()
                .filter(|r| r.run_id == run.run_id)
            {
                match result.verdict {
                    Verdict::Pass => counts.pass += 1,
                    Verdict::Fail => counts.fail += 1,
                    Verdict::Error => counts.error += 1,
                    // `select_results` never records a not_executed verdict as a decided result.
                    Verdict::NotExecuted => {}
                }
            }
            counts.not_executed = run.not_executed;
            // Total from the record size, buckets from the selection: two sources that must agree.
            counts.total = run.total.saturating_sub(run.superseded);
            debug_assert_eq!(
                counts.bucket_sum(),
                counts.total,
                "run {}: buckets disagree with the record size",
                run.run_id
            );

            let (used, excluded_reason) = match &run.use_ {
                RunUse::Used => (true, None),
                RunUse::Excluded(reason) => (false, Some(reason.clone())),
            };
            RunSummaryRow {
                run_id: run.run_id.clone(),
                build_id: run.build_id.clone(),
                used,
                excluded_reason,
                counts,
            }
        })
        .collect();

    let requirements = input
        .model
        .documents
        .iter()
        .flat_map(|section| section.rows.iter())
        .map(|row| {
            let outcomes: Vec<CaseOutcome> = row.cases.iter().map(|case| case.outcome).collect();
            RequirementSummaryRow {
                requirement_id: row.requirement_id.clone(),
                level: row.level,
                status: row.status,
                counts: ResultCounts::from_outcomes(&outcomes),
            }
        })
        .collect();

    let linked_outcomes: Vec<CaseOutcome> = linked_cases(input.model)
        .iter()
        .map(|case| case.outcome)
        .collect();

    SummaryReport {
        header: input.model.header.clone(),
        runs,
        requirements,
        linked: ResultCounts::from_outcomes(&linked_outcomes),
    }
}

const RUN_TABLE_HEADER: &str =
    "| Run | Build | Used | pass | fail | not_executed | error | Total |";
const REQUIREMENT_TABLE_HEADER: &str =
    "| Requirement | Level | Status | pass | fail | not_executed | error | Total |";
const TABLE_RULE: &str = "| --- | --- | --- | --- | --- | --- | --- | --- |";
const NO_RUNS: &str = "No runs loaded";
const NO_REQUIREMENTS: &str = "No requirements";

/// Render the summary as Markdown (SUM-01, D-06). Reads the report only: no clock, no hash-ordered
/// iteration, and every model-derived string goes through `cell`. Ends with exactly one newline.
pub fn render_summary_markdown(report: &SummaryReport) -> String {
    let runs_body = table_or(
        NO_RUNS,
        RUN_TABLE_HEADER,
        report.runs.iter().map(run_line).collect(),
    );
    let requirements_body = table_or(
        NO_REQUIREMENTS,
        REQUIREMENT_TABLE_HEADER,
        report.requirements.iter().map(requirement_line).collect(),
    );

    let blocks = [
        "# Run Summary".to_string(),
        header_markdown(&report.header),
        format!("## Results per run\n\n{runs_body}"),
        format!("## Results per requirement\n\n{requirements_body}"),
        format!("## Linked test cases\n\n{}", counts_line(&report.linked)),
    ];
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// A Markdown table with its header and rule, or the placeholder when there are no rows.
fn table_or(placeholder: &str, header: &str, rows: Vec<String>) -> String {
    if rows.is_empty() {
        placeholder.to_string()
    } else {
        let mut lines = vec![header.to_string(), TABLE_RULE.to_string()];
        lines.extend(rows);
        lines.join("\n")
    }
}

fn run_line(row: &RunSummaryRow) -> String {
    let used = if row.used {
        "yes".to_string()
    } else {
        let reason = row.excluded_reason.as_deref().unwrap_or("excluded");
        format!("no: {}", cell(reason))
    };
    format!(
        "| {} | {} | {} | {} |",
        cell(&row.run_id),
        cell(&row.build_id),
        used,
        count_cells(&row.counts)
    )
}

fn requirement_line(row: &RequirementSummaryRow) -> String {
    format!(
        "| {} | {} | {} | {} |",
        cell(&row.requirement_id),
        row.level,
        row.status.label(),
        count_cells(&row.counts)
    )
}

/// The five count columns in table order: pass, fail, not_executed, error, total.
fn count_cells(counts: &ResultCounts) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        counts.pass, counts.fail, counts.not_executed, counts.error, counts.total
    )
}

fn counts_line(counts: &ResultCounts) -> String {
    format!(
        "pass {}, fail {}, not_executed {}, error {}, total {}",
        counts.pass, counts.fail, counts.not_executed, counts.error, counts.total
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, ReportModel};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection};
    use crate::view::test_support::{fixture, GENERATED};
    use std::path::PathBuf;
    use testcase_verification::{TraceLink, TraceMatrix};

    fn counts(
        pass: usize,
        fail: usize,
        not_executed: usize,
        error: usize,
        total: usize,
    ) -> ResultCounts {
        ResultCounts {
            pass,
            fail,
            not_executed,
            error,
            total,
        }
    }

    fn loaded(run_id: &str, build: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
        LoadedRun::new(
            PathBuf::from(format!("{run_id}.yaml")),
            record(run_id, build, started, results),
        )
        .expect("valid run")
    }

    /// A model and selection built through the real pipeline from the given runs. The matrix
    /// has one requirement, HLR-1, linked to T1 and T2.
    fn scenario(runs: Vec<LoadedRun>) -> (ReportModel, RunSelection) {
        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-2".into(),
                baseline_id: None,
                links: vec![TraceLink {
                    requirement_id: "HLR-1".into(),
                    level: RequirementLevel::Hlr,
                    test_case_ids: vec!["T1".into(), "T2".into()],
                    normal_range_covered: true,
                    robustness_covered: false,
                    run_ids: Vec::new(),
                }],
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
            generated_at: GENERATED,
        });
        (model, selection)
    }

    fn summary_of(model: &ReportModel, selection: &RunSelection) -> SummaryReport {
        build_summary(ViewInput { model, selection })
    }

    fn run_row<'a>(report: &'a SummaryReport, run_id: &str) -> &'a RunSummaryRow {
        report
            .runs
            .iter()
            .find(|r| r.run_id == run_id)
            .unwrap_or_else(|| panic!("no run row for {run_id}"))
    }

    fn req_row<'a>(report: &'a SummaryReport, requirement_id: &str) -> &'a RequirementSummaryRow {
        report
            .requirements
            .iter()
            .find(|r| r.requirement_id == requirement_id)
            .unwrap_or_else(|| panic!("no requirement row for {requirement_id}"))
    }

    fn assert_buckets_sum(report: &SummaryReport) {
        for row in &report.runs {
            assert_eq!(
                row.counts.bucket_sum(),
                row.counts.total,
                "run {}",
                row.run_id
            );
        }
        for row in &report.requirements {
            assert_eq!(
                row.counts.bucket_sum(),
                row.counts.total,
                "requirement {}",
                row.requirement_id
            );
        }
        assert_eq!(report.linked.bucket_sum(), report.linked.total, "linked");
    }

    #[test]
    fn fixture_counts_match_the_expected_table() {
        let (model, selection) = fixture();
        let report = summary_of(&model, &selection);

        let runs: Vec<(&str, ResultCounts)> = report
            .runs
            .iter()
            .map(|r| (r.run_id.as_str(), r.counts.clone()))
            .collect();
        assert_eq!(
            runs,
            [
                ("RUN-1", counts(1, 1, 0, 1, 3)),
                ("RUN-2", counts(1, 0, 1, 0, 2)),
            ]
        );

        let requirements: Vec<(&str, ResultCounts)> = report
            .requirements
            .iter()
            .map(|r| (r.requirement_id.as_str(), r.counts.clone()))
            .collect();
        assert_eq!(
            requirements,
            [
                ("HLR-1", counts(1, 1, 0, 0, 2)),
                ("HLR-3", counts(0, 0, 0, 0, 0)),
                ("LLR-2", counts(0, 0, 0, 1, 1)),
                ("LLR-4", counts(0, 0, 1, 0, 1)),
            ]
        );

        assert_eq!(report.linked, counts(1, 1, 1, 1, 4));
        assert!(run_row(&report, "RUN-1").used);
        assert_eq!(run_row(&report, "RUN-1").excluded_reason, None);
        assert_buckets_sum(&report);
    }

    /// Golden output for the shared fixture. Runs in selection order; requirements in model
    /// order (document, requirement ID).
    const FIXTURE_GOLDEN: &str = r#"# Run Summary

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

## Results per run

| Run | Build | Used | pass | fail | not_executed | error | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RUN-1 | b1 | yes | 1 | 1 | 0 | 1 | 3 |
| RUN-2 | b2 | yes | 1 | 0 | 1 | 0 | 2 |

## Results per requirement

| Requirement | Level | Status | pass | fail | not_executed | error | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HLR-1 | HLR | FAILED | 1 | 1 | 0 | 0 | 2 |
| HLR-3 | HLR | NO TESTS | 0 | 0 | 0 | 0 | 0 |
| LLR-2 | LLR | ERRORED | 0 | 0 | 0 | 1 | 1 |
| LLR-4 | LLR | NOT RUN | 0 | 0 | 1 | 0 | 1 |

## Linked test cases

pass 1, fail 1, not_executed 1, error 1, total 4
"#;

    #[test]
    fn fixture_renders_the_golden_markdown() {
        let (model, selection) = fixture();
        let rendered = render_summary_markdown(&summary_of(&model, &selection));
        assert_eq!(rendered, FIXTURE_GOLDEN);
    }

    #[test]
    fn rendering_is_byte_identical_for_independent_builds() {
        let (model_a, selection_a) = fixture();
        let (model_b, selection_b) = fixture();
        assert_eq!(
            render_summary_markdown(&summary_of(&model_a, &selection_a)),
            render_summary_markdown(&summary_of(&model_b, &selection_b))
        );
    }

    #[test]
    fn hostile_run_id_is_escaped_and_stays_on_one_line() {
        let (model, mut selection) = fixture();
        for run in selection.runs.iter_mut().filter(|r| r.run_id == "RUN-1") {
            run.run_id = "R|1\n".to_string();
        }
        for result in selection
            .results
            .values_mut()
            .filter(|r| r.run_id == "RUN-1")
        {
            result.run_id = "R|1\n".to_string();
        }
        let rendered = render_summary_markdown(&summary_of(&model, &selection));
        assert!(
            rendered.contains("| R\\|1  | b1 | yes | 1 | 1 | 0 | 1 | 3 |\n"),
            "{rendered}"
        );
        assert!(!rendered.contains("R|1"), "{rendered}");
    }

    #[test]
    fn no_runs_prints_the_placeholder_instead_of_the_table() {
        let (model, mut selection) = fixture();
        selection.runs.clear();
        let rendered = render_summary_markdown(&summary_of(&model, &selection));
        assert!(
            rendered.contains("## Results per run\n\nNo runs loaded\n\n"),
            "{rendered}"
        );
        assert!(!rendered.contains("| Run | Build |"), "{rendered}");
    }

    #[test]
    fn no_requirements_prints_the_placeholder_instead_of_the_table() {
        let (mut model, selection) = fixture();
        model.documents.clear();
        let rendered = render_summary_markdown(&summary_of(&model, &selection));
        assert!(
            rendered.contains("## Results per requirement\n\nNo requirements\n\n"),
            "{rendered}"
        );
        assert!(!rendered.contains("| Requirement | Level |"), "{rendered}");
    }

    #[test]
    fn fixture_header_is_the_model_header() {
        let (model, selection) = fixture();
        let report = summary_of(&model, &selection);
        assert_eq!(report.header, model.header);
    }

    #[test]
    fn superseded_result_is_never_counted() {
        let (model, selection) = scenario(vec![
            loaded(
                "RUN-A",
                "bA",
                "2026-10-01T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
            loaded(
                "RUN-B",
                "bB",
                "2026-10-02T10:00:00Z",
                &[("T1", Verdict::Fail)],
            ),
        ]);
        let report = summary_of(&model, &selection);

        let superseded = run_row(&report, "RUN-A");
        assert_eq!(superseded.counts, counts(0, 0, 0, 0, 0));
        assert_eq!(superseded.counts.pass, 0);
        assert!(!superseded.used);
        assert_eq!(
            superseded.excluded_reason.as_deref(),
            Some("every executed result superseded by a later run")
        );

        assert_eq!(report.runs.len(), 2, "the superseded run stays listed");
        assert_eq!(run_row(&report, "RUN-B").counts, counts(0, 1, 0, 0, 1));
        // HLR-1: T1 fails (RUN-B), T2 has no result.
        assert_eq!(req_row(&report, "HLR-1").counts, counts(0, 1, 1, 0, 2));
        assert_buckets_sum(&report);
    }

    #[test]
    fn run_with_only_not_executed_results_is_listed_with_its_reason() {
        let (model, selection) = scenario(vec![
            loaded(
                "RUN-N",
                "bN",
                "2026-10-01T10:00:00Z",
                &[("T1", Verdict::NotExecuted), ("T2", Verdict::NotExecuted)],
            ),
            loaded(
                "RUN-P",
                "bP",
                "2026-10-02T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
        ]);
        let report = summary_of(&model, &selection);

        let excluded = run_row(&report, "RUN-N");
        assert!(!excluded.used);
        assert_eq!(
            excluded.excluded_reason.as_deref(),
            Some("only not_executed results")
        );
        assert_eq!(excluded.counts, counts(0, 0, 2, 0, 2));
        assert_eq!(run_row(&report, "RUN-P").counts, counts(1, 0, 0, 0, 1));
        assert_buckets_sum(&report);
    }
}
