//! Gaps view (GAP-01, D-03, D-06): which requirements lack tests, which lack a normal-range or a
//! robustness test, and which linked test cases have no recorded result.
//!
//! A pure function of `ViewInput`: no clock, no file access. The coverage flags are read from the
//! model rows as the matrix gave them; they are not recomputed here.
//!
//! Test cases that no requirement links are not known to the model, so they cannot be listed.

use crate::model::{Header, ReportModel};
use crate::non_blank;
use crate::render_markdown::{cell, header_markdown};
use crate::rollup::{CaseOutcome, VerificationStatus};
use crate::view::{linked_cases, ViewInput};
use testcase_verification::RequirementLevel;

/// A requirement listed in one of the three requirement lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapRequirement {
    pub requirement_id: String,
    pub level: RequirementLevel,
    /// The source document section of the row; `None` when not recorded.
    pub document: Option<String>,
}

/// A linked live test case with no recorded result, with every requirement that links it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapCase {
    pub test_case_id: String,
    /// Sorted, without duplicates.
    pub requirement_ids: Vec<String>,
}

/// The four GAP-01 lists plus the report header (D-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapsReport {
    pub header: Header,
    pub no_tests: Vec<GapRequirement>,
    pub missing_normal_range: Vec<GapRequirement>,
    pub missing_robustness: Vec<GapRequirement>,
    pub no_result: Vec<GapCase>,
}

/// Build the gaps report. Every list is sorted by requirement ID (or test case ID), in plain
/// string order, and does not depend on the order of the input.
///
/// Decision: a requirement with no tests has both coverage flags false, so it appears in
/// `no_tests` and in both coverage lists. Omitting it from the coverage lists would make them
/// look complete when they are not (T-03-06).
pub fn build_gaps(input: ViewInput) -> GapsReport {
    let model: &ReportModel = input.model;

    let mut no_tests = Vec::new();
    let mut missing_normal_range = Vec::new();
    let mut missing_robustness = Vec::new();
    for section in &model.documents {
        for row in &section.rows {
            let gap = GapRequirement {
                requirement_id: row.requirement_id.clone(),
                level: row.level,
                document: section.name.clone(),
            };
            if row.status == VerificationStatus::NoTests {
                no_tests.push(gap.clone());
            }
            if !row.normal_range_covered {
                missing_normal_range.push(gap.clone());
            }
            if !row.robustness_covered {
                missing_robustness.push(gap);
            }
        }
    }
    for list in [
        &mut no_tests,
        &mut missing_normal_range,
        &mut missing_robustness,
    ] {
        list.sort_by(|a, b| {
            (&a.requirement_id, &a.document).cmp(&(&b.requirement_id, &b.document))
        });
    }

    let no_result = linked_cases(input.model)
        .into_iter()
        .filter(|case| case.outcome == CaseOutcome::NotRun)
        .map(|case| GapCase {
            test_case_id: case.test_case_id,
            requirement_ids: case.requirement_ids,
        })
        .collect();

    GapsReport {
        header: model.header.clone(),
        no_tests,
        missing_normal_range,
        missing_robustness,
        no_result,
    }
}

const NONE: &str = "None";
const NOT_RECORDED: &str = "not recorded";

/// Render the gaps report as Markdown ending with exactly one newline. Every empty list prints
/// `None` under its heading, and the heading always shows the count (D-03, T-03-07).
pub fn render_gaps_markdown(report: &GapsReport) -> String {
    let blocks = [
        "# Coverage Gaps".to_string(),
        header_markdown(&report.header),
        requirement_block("Requirements with no test cases", &report.no_tests),
        requirement_block(
            "Requirements missing a normal-range test",
            &report.missing_normal_range,
        ),
        requirement_block(
            "Requirements missing a robustness test",
            &report.missing_robustness,
        ),
        case_block("Test cases with no recorded result", &report.no_result),
    ];
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

fn heading(title: &str, count: usize) -> String {
    format!("## {title} ({count})")
}

fn requirement_block(title: &str, items: &[GapRequirement]) -> String {
    let heading = heading(title, items.len());
    if items.is_empty() {
        return format!("{heading}\n\n{NONE}");
    }
    let mut lines = vec![
        heading,
        String::new(),
        "| Requirement | Level | Document |".to_string(),
        "| --- | --- | --- |".to_string(),
    ];
    for item in items {
        let document =
            non_blank(item.document.as_deref()).map_or_else(|| NOT_RECORDED.to_string(), cell);
        lines.push(format!(
            "| {} | {} | {} |",
            cell(&item.requirement_id),
            item.level,
            document
        ));
    }
    lines.join("\n")
}

fn case_block(title: &str, items: &[GapCase]) -> String {
    let heading = heading(title, items.len());
    if items.is_empty() {
        return format!("{heading}\n\n{NONE}");
    }
    let mut lines = vec![
        heading,
        String::new(),
        "| Test case | Requirements |".to_string(),
        "| --- | --- |".to_string(),
    ];
    for item in items {
        let requirements = item
            .requirement_ids
            .iter()
            .map(|id| cell(id))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "| {} | {} |",
            cell(&item.test_case_id),
            requirements
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, RequirementDocuments};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection};
    use crate::view::test_support::fixture;
    use std::collections::BTreeMap;
    use testcase_models::{TestCase, TestCaseStatus};
    use testcase_verification::{TraceLink, TraceMatrix, Verdict};

    const GENERATED: &str = "2026-10-31T09:00:00Z";

    fn link(
        id: &str,
        level: RequirementLevel,
        cases: &[&str],
        normal: bool,
        robustness: bool,
    ) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: normal,
            robustness_covered: robustness,
            run_ids: Vec::new(),
        }
    }

    fn run(run_id: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
        LoadedRun::new(
            format!("{run_id}.yaml").into(),
            record(run_id, "b1", started, results),
        )
        .expect("valid run")
    }

    /// Full pipeline: links, runs and optional test cases -> selection -> model.
    fn pipeline(
        links: Vec<TraceLink>,
        runs: Vec<LoadedRun>,
        test_cases: Option<&[TestCase]>,
        documents: Option<&RequirementDocuments>,
    ) -> (ReportModel, RunSelection) {
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
            test_cases,
            documents,
            generated_at: GENERATED,
        });
        (model, selection)
    }

    fn gaps_of(model: &ReportModel, selection: &RunSelection) -> GapsReport {
        build_gaps(ViewInput { model, selection })
    }

    fn ids(list: &[GapRequirement]) -> Vec<&str> {
        list.iter().map(|g| g.requirement_id.as_str()).collect()
    }

    fn case_pairs(list: &[GapCase]) -> Vec<(&str, Vec<&str>)> {
        list.iter()
            .map(|c| {
                (
                    c.test_case_id.as_str(),
                    c.requirement_ids.iter().map(String::as_str).collect(),
                )
            })
            .collect()
    }

    /// Fixture links: HLR-1 (T1, T2), LLR-2 (T3), HLR-3 (none), LLR-4 (T4).
    fn fixture_links() -> Vec<TraceLink> {
        vec![
            link("HLR-1", RequirementLevel::Hlr, &["T1", "T2"], true, true),
            link("LLR-2", RequirementLevel::Llr, &["T3"], true, false),
            link("HLR-3", RequirementLevel::Hlr, &[], false, false),
            link("LLR-4", RequirementLevel::Llr, &["T4"], false, true),
        ]
    }

    fn fixture_runs() -> Vec<LoadedRun> {
        vec![
            run(
                "RUN-1",
                "2026-10-01T10:00:00Z",
                &[
                    ("T1", Verdict::Pass),
                    ("T2", Verdict::Fail),
                    ("T3", Verdict::Error),
                ],
            ),
            run(
                "RUN-2",
                "2026-10-02T10:00:00Z",
                &[("T2", Verdict::NotExecuted), ("T5", Verdict::Pass)],
            ),
        ]
    }

    struct Expected {
        no_tests: Vec<&'static str>,
        missing_normal_range: Vec<&'static str>,
        missing_robustness: Vec<&'static str>,
        no_result: Vec<(&'static str, Vec<&'static str>)>,
    }

    #[test]
    fn gap_lists_match_expected_for_every_scenario() {
        let (fixture_model, fixture_selection) = fixture();

        let (full_model, full_selection) = pipeline(
            vec![
                link("HLR-1", RequirementLevel::Hlr, &["T1"], true, true),
                link("LLR-2", RequirementLevel::Llr, &["T2"], true, true),
            ],
            vec![run(
                "RUN-1",
                "2026-10-01T10:00:00Z",
                &[("T1", Verdict::Pass), ("T2", Verdict::Pass)],
            )],
            None,
            None,
        );

        let mut obsolete = TestCase::new("LEGACY".into(), 1, 1, "T9".into(), "d".into());
        obsolete.status = Some(TestCaseStatus::Obsolete);
        let (obsolete_model, obsolete_selection) = pipeline(
            vec![link("HLR-5", RequirementLevel::Hlr, &["T9"], true, true)],
            Vec::new(),
            Some(&[obsolete]),
            None,
        );

        let (shared_model, shared_selection) = pipeline(
            vec![
                link("HLR-1", RequirementLevel::Hlr, &["T1"], true, true),
                link("LLR-2", RequirementLevel::Llr, &["T1"], true, true),
            ],
            vec![run(
                "RUN-1",
                "2026-10-01T10:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            )],
            None,
            None,
        );

        let scenarios: Vec<(&str, ReportModel, RunSelection, Expected)> = vec![
            (
                "fixture",
                fixture_model,
                fixture_selection,
                Expected {
                    no_tests: vec!["HLR-3"],
                    missing_normal_range: vec!["HLR-3", "LLR-4"],
                    missing_robustness: vec!["HLR-3", "LLR-2"],
                    no_result: vec![("T4", vec!["LLR-4"])],
                },
            ),
            (
                "fully covered and fully run",
                full_model,
                full_selection,
                Expected {
                    no_tests: vec![],
                    missing_normal_range: vec![],
                    missing_robustness: vec![],
                    no_result: vec![],
                },
            ),
            (
                "requirement whose cases are all obsolete",
                obsolete_model,
                obsolete_selection,
                Expected {
                    no_tests: vec!["HLR-5"],
                    missing_normal_range: vec![],
                    missing_robustness: vec![],
                    no_result: vec![],
                },
            ),
            (
                "case shared by two requirements and never run",
                shared_model,
                shared_selection,
                Expected {
                    no_tests: vec![],
                    missing_normal_range: vec![],
                    missing_robustness: vec![],
                    no_result: vec![("T1", vec!["HLR-1", "LLR-2"])],
                },
            ),
        ];

        for (name, model, selection, expected) in scenarios {
            let report = gaps_of(&model, &selection);
            assert_eq!(report.header, model.header, "{name}: header");
            assert_eq!(ids(&report.no_tests), expected.no_tests, "{name}: no tests");
            assert_eq!(
                ids(&report.missing_normal_range),
                expected.missing_normal_range,
                "{name}: missing normal-range"
            );
            assert_eq!(
                ids(&report.missing_robustness),
                expected.missing_robustness,
                "{name}: missing robustness"
            );
            assert_eq!(
                case_pairs(&report.no_result),
                expected.no_result,
                "{name}: no result"
            );
        }
    }

    #[test]
    fn fixture_requirement_with_no_tests_is_in_all_three_requirement_lists() {
        let (model, selection) = fixture();
        let report = gaps_of(&model, &selection);
        let hlr3 = |list: &[GapRequirement]| list.iter().any(|g| g.requirement_id == "HLR-3");
        assert!(hlr3(&report.no_tests));
        assert!(hlr3(&report.missing_normal_range));
        assert!(hlr3(&report.missing_robustness));
    }

    #[test]
    fn requirement_entries_carry_level_and_document() {
        let (model, selection) = fixture();
        let report = gaps_of(&model, &selection);
        let hlr3 = &report.no_tests[0];
        assert_eq!(hlr3.level, RequirementLevel::Hlr);
        assert_eq!(hlr3.document, None);
    }

    #[test]
    fn document_section_name_is_carried_into_gap_entries() {
        let documents = RequirementDocuments {
            documents: vec!["docs/a.sdoc".to_string()],
            requirement_document: BTreeMap::from([(
                "HLR-3".to_string(),
                "docs/a.sdoc".to_string(),
            )]),
        };
        let (model, selection) = pipeline(
            vec![link("HLR-3", RequirementLevel::Hlr, &[], false, false)],
            Vec::new(),
            None,
            Some(&documents),
        );
        let report = gaps_of(&model, &selection);
        assert_eq!(report.no_tests[0].document.as_deref(), Some("docs/a.sdoc"));
    }

    #[test]
    fn shuffled_input_gives_equal_report() {
        let forward_model_selection = pipeline(fixture_links(), fixture_runs(), None, None);
        let forward = gaps_of(&forward_model_selection.0, &forward_model_selection.1);

        let mut links = fixture_links();
        links.reverse();
        for l in &mut links {
            l.test_case_ids.reverse();
        }
        let mut runs = fixture_runs();
        runs.reverse();
        let backward_model_selection = pipeline(links, runs, None, None);
        let backward = gaps_of(&backward_model_selection.0, &backward_model_selection.1);

        assert_eq!(forward, backward);
        assert_eq!(ids(&forward.no_tests), ["HLR-3"]);
    }

    #[test]
    fn requirement_lists_are_sorted_by_plain_string_order() {
        let (model, selection) = pipeline(
            vec![
                link("LLR-2", RequirementLevel::Llr, &[], false, false),
                link("LLR-10", RequirementLevel::Llr, &[], false, false),
                link("HLR-1", RequirementLevel::Hlr, &[], false, false),
            ],
            Vec::new(),
            None,
            None,
        );
        let report = gaps_of(&model, &selection);
        assert_eq!(ids(&report.no_tests), ["HLR-1", "LLR-10", "LLR-2"]);
    }

    #[test]
    fn case_decided_by_later_run_is_absent_from_no_result() {
        let (model, selection) = pipeline(
            vec![link("HLR-1", RequirementLevel::Hlr, &["T1"], true, true)],
            vec![
                run(
                    "RUN-1",
                    "2026-10-01T10:00:00Z",
                    &[("T1", Verdict::NotExecuted)],
                ),
                run("RUN-2", "2026-10-05T10:00:00Z", &[("T1", Verdict::Pass)]),
            ],
            None,
            None,
        );
        let report = gaps_of(&model, &selection);
        assert!(report.no_result.is_empty(), "{:?}", report.no_result);
    }

    mod render {
        use super::*;

        fn gaps_fixture() -> GapsReport {
            let (model, selection) = fixture();
            gaps_of(&model, &selection)
        }

        #[test]
        fn golden_gaps_markdown_matches_exact_text() {
            let expected = r#"# Coverage Gaps

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

## Requirements with no test cases (1)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-3 | HLR | not recorded |

## Requirements missing a normal-range test (2)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-3 | HLR | not recorded |
| LLR-4 | LLR | not recorded |

## Requirements missing a robustness test (2)

| Requirement | Level | Document |
| --- | --- | --- |
| HLR-3 | HLR | not recorded |
| LLR-2 | LLR | not recorded |

## Test cases with no recorded result (1)

| Test case | Requirements |
| --- | --- |
| T4 | LLR-4 |
"#;
            assert_eq!(render_gaps_markdown(&gaps_fixture()), expected);
        }

        #[test]
        fn empty_report_prints_none_four_times_with_zero_counts() {
            let (model, selection) = pipeline(Vec::new(), Vec::new(), None, None);
            let text = render_gaps_markdown(&gaps_of(&model, &selection));
            assert_eq!(text.matches("\n\nNone").count(), 4, "{text}");
            for heading in [
                "## Requirements with no test cases (0)",
                "## Requirements missing a normal-range test (0)",
                "## Requirements missing a robustness test (0)",
                "## Test cases with no recorded result (0)",
            ] {
                assert!(text.contains(heading), "{text}");
            }
            assert!(!text.contains("| Requirement |"), "{text}");
        }

        #[test]
        fn identical_inputs_render_byte_identically() {
            let first = render_gaps_markdown(&gaps_fixture());
            let second = render_gaps_markdown(&gaps_fixture());
            assert_eq!(first, second);
        }

        #[test]
        fn output_ends_with_exactly_one_newline() {
            let text = render_gaps_markdown(&gaps_fixture());
            assert!(text.ends_with('\n'));
            assert!(!text.ends_with("\n\n"));
        }

        #[test]
        fn hostile_ids_stay_on_one_line_with_pipe_escaped() {
            let (model, selection) = pipeline(
                vec![link(
                    "HLR|1\nforged",
                    RequirementLevel::Hlr,
                    &[],
                    false,
                    false,
                )],
                Vec::new(),
                None,
                None,
            );
            let text = render_gaps_markdown(&gaps_of(&model, &selection));
            let row = text
                .lines()
                .find(|line| line.contains("forged"))
                .expect("row present");
            assert_eq!(row, "| HLR\\|1 forged | HLR | not recorded |");
        }
    }
}
