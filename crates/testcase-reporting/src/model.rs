//! Report model for the traceability report (D-04..D-12).
//!
//! Pure: no I/O, no clock. The generation time is injected through `ReportInputs`. The model
//! holds data only; the renderer decides how a missing value is written.

use crate::input_summary;
use crate::load::LoadedInputs;
use crate::rollup::{case_outcome, roll_up, CaseOutcome, VerificationStatus};
use crate::run::LoadedRun;
use crate::select::{RunReport, RunSelection};
use std::collections::{BTreeMap, BTreeSet};
use testcase_models::{TestCase, TestCaseStatus, TestType};
use testcase_verification::RequirementLevel;

/// The test-type column a linked case is shown under (D-08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestColumn {
    NormalRange,
    Robustness,
    Other,
}

impl TestColumn {
    /// Every column, in report order.
    pub const ALL: [TestColumn; 3] = [
        TestColumn::NormalRange,
        TestColumn::Robustness,
        TestColumn::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TestColumn::NormalRange => "normal_range",
            TestColumn::Robustness => "robustness",
            TestColumn::Other => "other",
        }
    }
}

/// Which source document each requirement comes from (D-07). Optional input: without it the
/// report has one section with no document name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RequirementDocuments {
    /// Every document path of the requirements source, in any order.
    pub documents: Vec<String>,
    /// Requirement ID to the document path it is declared in.
    pub requirement_document: BTreeMap<String, String>,
}

/// Everything the model is built from. `test_cases` and `documents` are optional extra inputs:
/// the trace matrix carries neither obsolete flags, test types nor source documents.
#[derive(Debug, Clone, Copy)]
pub struct ReportInputs<'a> {
    pub inputs: &'a LoadedInputs,
    pub selection: &'a RunSelection,
    pub test_cases: Option<&'a [TestCase]>,
    pub documents: Option<&'a RequirementDocuments>,
    pub generated_at: &'a str,
}

/// Report header (D-06, D-11). An empty list or `None` means "not recorded".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub matrix_id: String,
    pub baseline_id: Option<String>,
    pub run_ids: Vec<String>,
    pub build_ids: Vec<String>,
    pub environments: Vec<String>,
    pub testers: Vec<String>,
    pub generated_at: String,
    /// Distinct obsolete test cases left out of the roll-up; `None` when no test cases were
    /// supplied, so the count is unknown.
    pub obsolete_excluded: Option<usize>,
}

/// One linked live test case on a requirement row (D-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseEntry {
    pub test_case_id: String,
    pub column: TestColumn,
    pub outcome: CaseOutcome,
    /// The run that decided the outcome; `None` when the case has no executed result.
    pub run_id: Option<String>,
}

/// One normative requirement (D-09) with its roll-up status and coverage flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementRow {
    pub requirement_id: String,
    pub level: RequirementLevel,
    pub status: VerificationStatus,
    /// Coverage is shown apart from the status (D-04).
    pub normal_range_covered: bool,
    pub robustness_covered: bool,
    pub cases: Vec<CaseEntry>,
}

/// The rows of one source document. `name` is `None` when the document is not recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSection {
    pub name: Option<String>,
    pub rows: Vec<RequirementRow>,
}

/// The Phase 1 input and run-selection text, stored verbatim (D-14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputsSection {
    pub details: String,
}

/// The whole report, ready for any renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportModel {
    pub inputs: InputsSection,
    pub header: Header,
    pub documents: Vec<DocumentSection>,
}

/// Build the report model from loaded inputs and the run selection (D-02..D-06, D-09..D-11).
pub fn build_model(input: &ReportInputs) -> ReportModel {
    let selection = input.selection;
    let matrix = &input.inputs.matrix;

    let by_id: BTreeMap<&str, &TestCase> = input
        .test_cases
        .unwrap_or(&[])
        .iter()
        .map(|tc| (tc.id.as_str(), tc))
        .collect();
    let obsolete: BTreeSet<&str> = input
        .test_cases
        .unwrap_or(&[])
        .iter()
        .filter(|tc| tc.status == Some(TestCaseStatus::Obsolete))
        .map(|tc| tc.id.as_str())
        .collect();

    let rows: Vec<(Option<String>, RequirementRow)> = matrix
        .links
        .iter()
        .map(|link| {
            let mut ids: Vec<&str> = link
                .test_case_ids
                .iter()
                .map(String::as_str)
                .filter(|id| !obsolete.contains(id))
                .collect();
            ids.sort_unstable();
            ids.dedup();

            let cases: Vec<CaseEntry> = ids
                .iter()
                .map(|id| CaseEntry {
                    test_case_id: id.to_string(),
                    column: column_of(by_id.get(id).copied()),
                    outcome: case_outcome(selection, id),
                    run_id: selection.results.get(*id).map(|r| r.run_id.clone()),
                })
                .collect();

            let outcomes: Vec<CaseOutcome> = cases.iter().map(|c| c.outcome).collect();
            let document = input
                .documents
                .and_then(|d| d.requirement_document.get(&link.requirement_id).cloned());
            let row = RequirementRow {
                requirement_id: link.requirement_id.clone(),
                level: link.level,
                status: roll_up(&outcomes),
                normal_range_covered: link.normal_range_covered,
                robustness_covered: link.robustness_covered,
                cases,
            };
            (document, row)
        })
        .collect();

    ReportModel {
        inputs: InputsSection {
            details: input_summary(input.inputs, selection),
        },
        header: header(input, selection, obsolete.len()),
        documents: sections(input.documents, rows),
    }
}

/// Group rows under their source documents (D-07).
///
/// Without a document list there is one section with no name. With one, every listed document
/// gets a section even when it has no rows; a row whose document is not listed still gets a
/// section of its own; rows with no mapped document go to a trailing unnamed section. Sections
/// are sorted by name, rows by requirement ID (D-12).
fn sections(
    documents: Option<&RequirementDocuments>,
    rows: Vec<(Option<String>, RequirementRow)>,
) -> Vec<DocumentSection> {
    let Some(documents) = documents else {
        return vec![DocumentSection {
            name: None,
            rows: sorted_rows(rows.into_iter().map(|(_, row)| row).collect()),
        }];
    };

    let mut named: BTreeMap<String, Vec<RequirementRow>> = documents
        .documents
        .iter()
        .map(|path| (path.clone(), Vec::new()))
        .collect();
    let mut unnamed: Vec<RequirementRow> = Vec::new();
    for (document, row) in rows {
        match document {
            Some(path) => named.entry(path).or_default().push(row),
            None => unnamed.push(row),
        }
    }

    let mut sections: Vec<DocumentSection> = named
        .into_iter()
        .map(|(name, rows)| DocumentSection {
            name: Some(name),
            rows: sorted_rows(rows),
        })
        .collect();
    if !unnamed.is_empty() {
        sections.push(DocumentSection {
            name: None,
            rows: sorted_rows(unnamed),
        });
    }
    sections
}

fn sorted_rows(mut rows: Vec<RequirementRow>) -> Vec<RequirementRow> {
    rows.sort_by(|a, b| a.requirement_id.cmp(&b.requirement_id));
    rows
}

/// The column of a case. Unknown test types, and cases not in the supplied list, go to `Other`.
fn column_of(test_case: Option<&TestCase>) -> TestColumn {
    match test_case.and_then(|tc| tc.test_type) {
        Some(TestType::NormalRange) => TestColumn::NormalRange,
        Some(TestType::Robustness) => TestColumn::Robustness,
        Some(TestType::Security) | None => TestColumn::Other,
    }
}

fn header(input: &ReportInputs, selection: &RunSelection, obsolete_count: usize) -> Header {
    let used: Vec<&RunReport> = selection.used().collect();
    let loaded: Vec<&LoadedRun> = used
        .iter()
        .filter_map(|r| {
            input
                .inputs
                .runs
                .iter()
                .find(|lr| lr.record.run_id == r.run_id)
        })
        .collect();

    Header {
        matrix_id: input.inputs.matrix.id.clone(),
        baseline_id: input.inputs.matrix.baseline_id.clone(),
        run_ids: sorted_recorded(used.iter().map(|r| r.run_id.as_str())),
        build_ids: sorted_recorded(used.iter().map(|r| r.build_id.as_str())),
        environments: sorted_recorded(loaded.iter().map(|r| r.record.environment.as_str())),
        testers: sorted_recorded(loaded.iter().map(|r| r.record.tester.as_str())),
        generated_at: input.generated_at.to_string(),
        obsolete_excluded: input.test_cases.map(|_| obsolete_count),
    }
}

/// Distinct, non-blank values in sorted order.
fn sorted_recorded<'a>(values: impl Iterator<Item = &'a str>) -> Vec<String> {
    values
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::Reconciliation;
    use crate::run::test_support::record;
    use crate::select::select_results;
    use testcase_verification::{TestRunRecord, TraceLink, TraceMatrix, Verdict};

    const GENERATED: &str = "2026-10-31T00:00:00Z";

    fn case(id: &str, test_type: Option<TestType>, status: Option<TestCaseStatus>) -> TestCase {
        let mut tc = TestCase::new("LEGACY".into(), 1, 1, id.into(), "d".into());
        tc.test_type = test_type;
        tc.status = status;
        tc
    }

    fn link(
        id: &str,
        level: RequirementLevel,
        cases: &[&str],
        normal: bool,
        robust: bool,
    ) -> TraceLink {
        TraceLink {
            requirement_id: id.into(),
            level,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: normal,
            robustness_covered: robust,
            run_ids: Vec::new(),
        }
    }

    fn loaded(rec: TestRunRecord) -> LoadedRun {
        let path = format!("{}.yaml", rec.run_id).into();
        LoadedRun::new(path, rec).expect("valid run")
    }

    fn inputs(links: Vec<TraceLink>, baseline: Option<&str>, runs: Vec<LoadedRun>) -> LoadedInputs {
        LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-1".into(),
                baseline_id: baseline.map(str::to_string),
                links,
            },
            runs,
            reconciliation: Reconciliation::default(),
        }
    }

    fn build(inputs: &LoadedInputs, test_cases: Option<&[TestCase]>) -> ReportModel {
        let selection = select_results(&inputs.runs);
        build_model(&ReportInputs {
            inputs,
            selection: &selection,
            test_cases,
            documents: None,
            generated_at: GENERATED,
        })
    }

    fn rows(model: &ReportModel) -> &[RequirementRow] {
        &model.documents[0].rows
    }

    /// Phase 1 demo: RUN-1 pass, RUN-2 fail, RUN-3 not_executed on T1.
    fn demo_runs() -> Vec<LoadedRun> {
        vec![
            loaded(record(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            )),
            loaded(record(
                "RUN-2",
                "b1",
                "2026-10-28T11:00:00Z",
                &[("T1", Verdict::Fail)],
            )),
            loaded(record(
                "RUN-3",
                "b1",
                "2026-10-28T12:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            )),
        ]
    }

    #[test]
    fn test_column_follows_test_type() {
        let tcs = vec![
            case("T1", Some(TestType::NormalRange), None),
            case("T2", Some(TestType::Robustness), None),
            case("T3", Some(TestType::Security), None),
            case("T4", None, None),
        ];
        let inputs = inputs(
            vec![link(
                "R1",
                RequirementLevel::Hlr,
                &["T1", "T2", "T3", "T4", "T404"],
                true,
                true,
            )],
            None,
            Vec::new(),
        );
        let model = build(&inputs, Some(&tcs));
        let cols: Vec<(&str, TestColumn)> = rows(&model)[0]
            .cases
            .iter()
            .map(|c| (c.test_case_id.as_str(), c.column))
            .collect();
        assert_eq!(
            cols,
            vec![
                ("T1", TestColumn::NormalRange),
                ("T2", TestColumn::Robustness),
                ("T3", TestColumn::Other),
                ("T4", TestColumn::Other),
                ("T404", TestColumn::Other),
            ]
        );
    }

    #[test]
    fn test_column_is_other_when_no_test_cases_supplied() {
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1"], false, false)],
            None,
            Vec::new(),
        );
        let model = build(&inputs, None);
        assert_eq!(rows(&model)[0].cases[0].column, TestColumn::Other);
    }

    #[test]
    fn obsolete_case_is_dropped_from_row_and_roll_up_and_counted() {
        let tcs = vec![
            case(
                "T1",
                Some(TestType::NormalRange),
                Some(TestCaseStatus::Approved),
            ),
            case(
                "T2",
                Some(TestType::Robustness),
                Some(TestCaseStatus::Obsolete),
            ),
        ];
        let runs = vec![loaded(record(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass), ("T2", Verdict::Fail)],
        ))];
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1", "T2"], true, true)],
            None,
            runs,
        );
        let model = build(&inputs, Some(&tcs));
        let row = &rows(&model)[0];
        let ids: Vec<&str> = row.cases.iter().map(|c| c.test_case_id.as_str()).collect();
        assert_eq!(ids, vec!["T1"]);
        assert_eq!(row.status, VerificationStatus::Verified);
        assert_eq!(model.header.obsolete_excluded, Some(1));
    }

    #[test]
    fn obsolete_count_is_none_without_test_cases() {
        let runs = vec![loaded(record(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass), ("T2", Verdict::Fail)],
        ))];
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1", "T2"], true, true)],
            None,
            runs,
        );
        let model = build(&inputs, None);
        assert_eq!(model.header.obsolete_excluded, None);
        assert_eq!(rows(&model)[0].cases.len(), 2);
        assert_eq!(rows(&model)[0].status, VerificationStatus::Failed);
    }

    #[test]
    fn link_without_cases_is_no_tests_and_unrun_case_is_not_run() {
        let runs = vec![loaded(record(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass)],
        ))];
        let inputs = inputs(
            vec![
                link("R-EMPTY", RequirementLevel::Llr, &[], false, false),
                link("R-UNRUN", RequirementLevel::Llr, &["T9"], false, false),
            ],
            None,
            runs,
        );
        let model = build(&inputs, None);
        let r = rows(&model);
        assert_eq!(r[0].status, VerificationStatus::NoTests);
        assert!(r[0].cases.is_empty());
        assert_eq!(r[1].status, VerificationStatus::NotRun);
        assert_eq!(r[1].cases[0].outcome, CaseOutcome::NotRun);
        assert_eq!(r[1].cases[0].run_id, None);
    }

    #[test]
    fn coverage_flags_stay_apart_from_status() {
        let tcs = vec![case("T1", Some(TestType::NormalRange), None)];
        let runs = vec![loaded(record(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass)],
        ))];
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1"], true, false)],
            None,
            runs,
        );
        let model = build(&inputs, Some(&tcs));
        let row = &rows(&model)[0];
        assert_eq!(row.status, VerificationStatus::Verified);
        assert!(row.normal_range_covered);
        assert!(!row.robustness_covered);
    }

    #[test]
    fn hlr_and_llr_keep_their_level() {
        let inputs = inputs(
            vec![
                link("HLR-1", RequirementLevel::Hlr, &[], false, false),
                link("LLR-1", RequirementLevel::Llr, &[], false, false),
            ],
            None,
            Vec::new(),
        );
        let model = build(&inputs, None);
        assert_eq!(rows(&model)[0].level, RequirementLevel::Hlr);
        assert_eq!(rows(&model)[1].level, RequirementLevel::Llr);
    }

    #[test]
    fn decided_case_names_the_deciding_run() {
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1"], false, false)],
            None,
            demo_runs(),
        );
        let model = build(&inputs, None);
        let case = &rows(&model)[0].cases[0];
        assert_eq!(case.outcome, CaseOutcome::Fail);
        assert_eq!(case.run_id, Some("RUN-2".to_string()));
    }

    #[test]
    fn inputs_section_is_the_phase_one_text_verbatim() {
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &["T1"], false, false)],
            Some("BL-7"),
            demo_runs(),
        );
        let selection = select_results(&inputs.runs);
        let model = build(&inputs, None);
        assert_eq!(model.inputs.details, input_summary(&inputs, &selection));
        assert_eq!(
            model.inputs.details.lines().next(),
            Some("Trace matrix: TM-1 (baseline: BL-7)")
        );
        assert_eq!(model, build(&inputs, None));
    }

    #[test]
    fn header_lists_only_used_runs_and_their_environment_and_tester() {
        let mut r1 = record(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass)],
        );
        r1.environment = "staging".into();
        r1.tester = "alice".into();
        let mut r2 = record(
            "RUN-2",
            "b2",
            "2026-10-28T11:00:00Z",
            &[("T2", Verdict::Pass)],
        );
        r2.environment = "staging".into();
        r2.tester = "bob".into();
        // Only not_executed: excluded, so its build, environment and tester stay out.
        let mut r9 = record(
            "RUN-9",
            "b9",
            "2026-10-28T12:00:00Z",
            &[("T1", Verdict::NotExecuted)],
        );
        r9.environment = "lab".into();
        r9.tester = "carol".into();
        let runs = vec![loaded(r2), loaded(r9), loaded(r1)];
        let inputs = inputs(
            vec![link(
                "R1",
                RequirementLevel::Hlr,
                &["T1", "T2"],
                false,
                false,
            )],
            None,
            runs,
        );
        let model = build(&inputs, None);
        let h = &model.header;
        assert_eq!(h.matrix_id, "TM-1");
        assert_eq!(h.baseline_id, None);
        assert_eq!(h.run_ids, vec!["RUN-1", "RUN-2"]);
        assert_eq!(h.build_ids, vec!["b1", "b2"]);
        assert_eq!(h.environments, vec!["staging"]);
        assert_eq!(h.testers, vec!["alice", "bob"]);
        assert_eq!(h.generated_at, GENERATED);
    }

    #[test]
    fn header_run_ids_are_empty_when_no_run_is_used() {
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &[], false, false)],
            None,
            Vec::new(),
        );
        let model = build(&inputs, None);
        assert!(model.header.run_ids.is_empty());
        assert!(model.header.build_ids.is_empty());
        assert!(model.header.environments.is_empty());
    }

    #[test]
    fn blank_baseline_is_not_recorded_in_header() {
        for blank in ["", "  "] {
            let inputs = inputs(
                vec![link("R1", RequirementLevel::Hlr, &[], false, false)],
                Some(blank),
                Vec::new(),
            );
            let model = build(&inputs, None);
            assert_eq!(model.header.baseline_id, None, "baseline {blank:?}");
        }
        let inputs = inputs(
            vec![link("R1", RequirementLevel::Hlr, &[], false, false)],
            Some("B-1"),
            Vec::new(),
        );
        let model = build(&inputs, None);
        assert_eq!(model.header.baseline_id, Some("B-1".to_string()));
    }

    #[test]
    fn one_row_per_link() {
        let inputs = inputs(
            vec![
                link("R1", RequirementLevel::Hlr, &["T1"], false, false),
                link("R2", RequirementLevel::Llr, &["T1"], false, false),
                link("R3", RequirementLevel::Llr, &[], false, false),
            ],
            None,
            Vec::new(),
        );
        let model = build(&inputs, None);
        assert_eq!(rows(&model).len(), inputs.matrix.links.len());
    }

    fn build_docs(inputs: &LoadedInputs, documents: Option<&RequirementDocuments>) -> ReportModel {
        let selection = select_results(&inputs.runs);
        build_model(&ReportInputs {
            inputs,
            selection: &selection,
            test_cases: None,
            documents,
            generated_at: GENERATED,
        })
    }

    fn docs(list: &[&str], mapping: &[(&str, &str)]) -> RequirementDocuments {
        RequirementDocuments {
            documents: list.iter().map(|d| d.to_string()).collect(),
            requirement_document: mapping
                .iter()
                .map(|(r, d)| (r.to_string(), d.to_string()))
                .collect(),
        }
    }

    fn section_names(model: &ReportModel) -> Vec<Option<&str>> {
        model.documents.iter().map(|s| s.name.as_deref()).collect()
    }

    fn row_ids(section: &DocumentSection) -> Vec<&str> {
        section
            .rows
            .iter()
            .map(|r| r.requirement_id.as_str())
            .collect()
    }

    #[test]
    fn sections_are_sorted_by_document_with_unnamed_last() {
        let inputs = inputs(
            vec![
                link("R-B", RequirementLevel::Hlr, &[], false, false),
                link("R-X", RequirementLevel::Llr, &[], false, false),
                link("R-A", RequirementLevel::Llr, &[], false, false),
            ],
            None,
            Vec::new(),
        );
        let d = docs(
            &["docs/b.sdoc", "docs/a.sdoc"],
            &[("R-B", "docs/b.sdoc"), ("R-A", "docs/a.sdoc")],
        );
        let model = build_docs(&inputs, Some(&d));
        assert_eq!(
            section_names(&model),
            vec![Some("docs/a.sdoc"), Some("docs/b.sdoc"), None]
        );
        assert_eq!(row_ids(&model.documents[0]), vec!["R-A"]);
        assert_eq!(row_ids(&model.documents[1]), vec!["R-B"]);
        assert_eq!(row_ids(&model.documents[2]), vec!["R-X"]);
        let total: usize = model.documents.iter().map(|s| s.rows.len()).sum();
        assert_eq!(total, inputs.matrix.links.len());
    }

    #[test]
    fn document_without_requirements_is_kept_as_empty_section() {
        let inputs = inputs(
            vec![link("R-A", RequirementLevel::Hlr, &[], false, false)],
            None,
            Vec::new(),
        );
        let d = docs(
            &["docs/a.sdoc", "docs/system.sdoc"],
            &[("R-A", "docs/a.sdoc")],
        );
        let model = build_docs(&inputs, Some(&d));
        assert_eq!(
            section_names(&model),
            vec![Some("docs/a.sdoc"), Some("docs/system.sdoc")]
        );
        assert!(model.documents[1].rows.is_empty());
    }

    #[test]
    fn requirement_in_undeclared_document_gets_its_own_section() {
        let inputs = inputs(
            vec![link("R-A", RequirementLevel::Hlr, &[], false, false)],
            None,
            Vec::new(),
        );
        let d = docs(&["docs/a.sdoc"], &[("R-A", "docs/other.sdoc")]);
        let model = build_docs(&inputs, Some(&d));
        assert_eq!(
            section_names(&model),
            vec![Some("docs/a.sdoc"), Some("docs/other.sdoc")]
        );
        assert_eq!(row_ids(&model.documents[1]), vec!["R-A"]);
    }

    #[test]
    fn without_documents_there_is_one_unnamed_section_with_every_row() {
        let inputs = inputs(
            vec![
                link("R-2", RequirementLevel::Llr, &[], false, false),
                link("R-1", RequirementLevel::Hlr, &[], false, false),
            ],
            None,
            Vec::new(),
        );
        let model = build_docs(&inputs, None);
        assert_eq!(section_names(&model), vec![None]);
        assert_eq!(row_ids(&model.documents[0]), vec!["R-1", "R-2"]);
    }

    #[test]
    fn shuffled_links_and_case_ids_give_the_sorted_model() {
        let runs = || {
            vec![loaded(record(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass), ("T2", Verdict::Fail)],
            ))]
        };
        let sorted = inputs(
            vec![
                link("R-1", RequirementLevel::Hlr, &["T1", "T2"], true, true),
                link("R-2", RequirementLevel::Llr, &["T2"], false, false),
            ],
            Some("BL-1"),
            runs(),
        );
        let shuffled = inputs(
            vec![
                link("R-2", RequirementLevel::Llr, &["T2"], false, false),
                link("R-1", RequirementLevel::Hlr, &["T2", "T1"], true, true),
            ],
            Some("BL-1"),
            runs(),
        );
        assert_eq!(build_docs(&sorted, None), build_docs(&shuffled, None));
    }

    #[test]
    fn cases_within_a_row_are_sorted_by_test_case_id() {
        let inputs = inputs(
            vec![link(
                "R-1",
                RequirementLevel::Hlr,
                &["T3", "T1", "T2"],
                false,
                false,
            )],
            None,
            Vec::new(),
        );
        let model = build_docs(&inputs, None);
        let ids: Vec<&str> = rows(&model)[0]
            .cases
            .iter()
            .map(|c| c.test_case_id.as_str())
            .collect();
        assert_eq!(ids, vec!["T1", "T2", "T3"]);
    }

    #[test]
    fn identical_inputs_give_equal_model_and_identical_text() {
        let inputs = inputs(
            vec![
                link("R-1", RequirementLevel::Hlr, &["T1"], true, false),
                link("R-2", RequirementLevel::Llr, &["T1", "T2"], false, false),
            ],
            Some("BL-1"),
            demo_runs(),
        );
        let d = docs(&["docs/a.sdoc"], &[("R-1", "docs/a.sdoc")]);
        let first = build_docs(&inputs, Some(&d));
        let second = build_docs(&inputs, Some(&d));
        assert_eq!(first, second);
        assert_eq!(format!("{first:?}"), format!("{second:?}"));
    }
}
