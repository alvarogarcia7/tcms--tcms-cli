//! Shared input and helpers for the three Phase 3 views (summary, gaps, problems).
//!
//! Every view is a pure function of `ViewInput`: the Phase 2 `ReportModel` and the Phase 1
//! `RunSelection` the model was built from. No clock, no file access, so the numbers in the
//! three views cannot disagree (D-01). `model.rs`, `rollup.rs` and `select.rs` are not changed
//! by this module.

use crate::model::ReportModel;
use crate::rollup::CaseOutcome;
use crate::select::RunSelection;
use std::collections::BTreeMap;

/// The only input a view reads.
#[derive(Debug, Clone, Copy)]
pub struct ViewInput<'a> {
    pub model: &'a ReportModel,
    pub selection: &'a RunSelection,
}

/// Counts of case outcomes. `total` is the number of outcomes counted, not the sum of the
/// buckets, so `bucket_sum() == total` is a real check and not a tautology.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResultCounts {
    pub pass: usize,
    pub fail: usize,
    pub not_executed: usize,
    pub error: usize,
    pub total: usize,
}

impl ResultCounts {
    /// Count the outcomes. `NotRun` is the not-executed bucket (D-05).
    pub fn from_outcomes(outcomes: &[CaseOutcome]) -> Self {
        let mut counts = ResultCounts {
            total: outcomes.len(),
            ..ResultCounts::default()
        };
        for outcome in outcomes {
            match outcome {
                CaseOutcome::Pass => counts.pass += 1,
                CaseOutcome::Fail => counts.fail += 1,
                CaseOutcome::Error => counts.error += 1,
                CaseOutcome::NotRun => counts.not_executed += 1,
            }
        }
        counts
    }

    /// Sum of the four buckets. Equals `total` for every count built by `from_outcomes`.
    pub fn bucket_sum(&self) -> usize {
        self.pass + self.fail + self.not_executed + self.error
    }
}

/// One distinct live test case linked from the model, with every requirement that links it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedCase {
    pub test_case_id: String,
    /// Sorted, without duplicates.
    pub requirement_ids: Vec<String>,
    pub outcome: CaseOutcome,
    pub run_id: Option<String>,
}

/// Every distinct test case the model links, once each, sorted by test case ID.
///
/// Obsolete cases never appear: the model already left them out (ROLL-03).
pub fn linked_cases(model: &ReportModel) -> Vec<LinkedCase> {
    let mut by_id: BTreeMap<String, LinkedCase> = BTreeMap::new();
    for section in &model.documents {
        for row in &section.rows {
            for case in &row.cases {
                let entry = by_id
                    .entry(case.test_case_id.clone())
                    .or_insert_with(|| LinkedCase {
                        test_case_id: case.test_case_id.clone(),
                        requirement_ids: Vec::new(),
                        outcome: case.outcome,
                        run_id: case.run_id.clone(),
                    });
                entry.requirement_ids.push(row.requirement_id.clone());
            }
        }
    }
    by_id
        .into_values()
        .map(|mut case| {
            case.requirement_ids.sort();
            case.requirement_ids.dedup();
            case
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod test_support {
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, ReportModel};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection};
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    pub(crate) const GENERATED: &str = "2026-10-31T09:00:00Z";

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

    /// The shared fixture for the three views, built through the real pipeline: loaded runs ->
    /// run selection -> report model. No test cases and no documents are supplied.
    ///
    /// Matrix TM-1: HLR-1 (T1, T2; normal and robustness covered), LLR-2 (T3; normal covered),
    /// HLR-3 (no cases), LLR-4 (T4; robustness covered).
    /// RUN-1 (build b1): T1 pass, T2 fail with problem report PR-7, T3 error.
    /// RUN-2 (build b2): T2 not_executed, T5 pass.
    /// Selection: T1 pass, T2 fail (RUN-1), T3 error (RUN-1), T5 pass (RUN-2). T4 has no result.
    /// Statuses: HLR-1 FAILED, LLR-2 ERRORED, HLR-3 NO TESTS, LLR-4 NOT RUN.
    pub(crate) fn fixture() -> (ReportModel, RunSelection) {
        let mut rec1 = record(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[
                ("T1", Verdict::Pass),
                ("T2", Verdict::Fail),
                ("T3", Verdict::Error),
            ],
        );
        for result in rec1.results.iter_mut() {
            if result.test_case_id == "T2" {
                result.problem_report_id = Some("PR-7".to_string());
            }
        }
        let rec2 = record(
            "RUN-2",
            "b2",
            "2026-10-02T10:00:00Z",
            &[("T2", Verdict::NotExecuted), ("T5", Verdict::Pass)],
        );
        let runs = vec![
            LoadedRun::new("RUN-1.yaml".into(), rec1).expect("valid run"),
            LoadedRun::new("RUN-2.yaml".into(), rec2).expect("valid run"),
        ];
        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-1".into(),
                baseline_id: None,
                links: vec![
                    link("HLR-1", RequirementLevel::Hlr, &["T1", "T2"], true, true),
                    link("LLR-2", RequirementLevel::Llr, &["T3"], true, false),
                    link("HLR-3", RequirementLevel::Hlr, &[], false, false),
                    link("LLR-4", RequirementLevel::Llr, &["T4"], false, true),
                ],
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
}

#[cfg(test)]
mod tests {
    use super::test_support::fixture;
    use super::*;
    use crate::rollup::CaseOutcome::{Error, Fail, NotRun, Pass};

    fn all_outcomes() -> [CaseOutcome; 4] {
        [Pass, Fail, Error, NotRun]
    }

    #[test]
    fn counts_from_mixed_outcomes() {
        let counts = ResultCounts::from_outcomes(&[Pass, Pass, Fail, Error, NotRun]);
        assert_eq!(
            counts,
            ResultCounts {
                pass: 2,
                fail: 1,
                not_executed: 1,
                error: 1,
                total: 5,
            }
        );
    }

    #[test]
    fn empty_counts_are_all_zero() {
        assert_eq!(ResultCounts::from_outcomes(&[]), ResultCounts::default());
    }

    #[test]
    fn buckets_sum_to_total_over_every_small_slice() {
        // Every slice of length 0..=4 over the four outcomes: 1 + 4 + 16 + 64 + 256 slices.
        for len in 0..=4u32 {
            for code in 0..4usize.pow(len) {
                let mut rest = code;
                let outcomes: Vec<CaseOutcome> = (0..len)
                    .map(|_| {
                        let outcome = all_outcomes()[rest % 4];
                        rest /= 4;
                        outcome
                    })
                    .collect();
                let counts = ResultCounts::from_outcomes(&outcomes);
                assert_eq!(counts.bucket_sum(), counts.total, "{outcomes:?}");
                assert_eq!(counts.total, outcomes.len(), "{outcomes:?}");
            }
        }
    }

    #[test]
    fn linked_cases_of_fixture_are_t1_to_t4_in_order() {
        let (model, _) = fixture();
        let cases = linked_cases(&model);
        let ids: Vec<&str> = cases.iter().map(|c| c.test_case_id.as_str()).collect();
        assert_eq!(ids, ["T1", "T2", "T3", "T4"]);

        let t2 = cases.iter().find(|c| c.test_case_id == "T2").unwrap();
        assert_eq!(t2.outcome, Fail);
        assert_eq!(t2.run_id.as_deref(), Some("RUN-1"));
        assert_eq!(t2.requirement_ids, ["HLR-1"]);

        let t4 = cases.iter().find(|c| c.test_case_id == "T4").unwrap();
        assert_eq!(t4.outcome, NotRun);
        assert_eq!(t4.run_id, None);
    }

    #[test]
    fn case_linked_to_two_requirements_appears_once_with_sorted_ids() {
        let (mut model, _) = fixture();
        let mut extra = model.documents[0].rows[0].clone();
        extra.requirement_id = "HLR-0".to_string();
        model.documents[0].rows.push(extra);
        let cases = linked_cases(&model);
        let t1: Vec<&LinkedCase> = cases.iter().filter(|c| c.test_case_id == "T1").collect();
        assert_eq!(t1.len(), 1);
        assert_eq!(t1[0].requirement_ids, ["HLR-0", "HLR-1"]);
    }

    #[test]
    fn fixture_statuses_match_the_expected_words() {
        let (model, _) = fixture();
        let rows: Vec<(&str, &str)> = model.documents[0]
            .rows
            .iter()
            .map(|r| (r.requirement_id.as_str(), r.status.label()))
            .collect();
        assert_eq!(
            rows,
            [
                ("HLR-1", "FAILED"),
                ("HLR-3", "NO TESTS"),
                ("LLR-2", "ERRORED"),
                ("LLR-4", "NOT RUN"),
            ]
        );
    }

    #[test]
    fn fixture_selection_keeps_problem_report_and_run() {
        let (_, selection) = fixture();
        let t2 = &selection.results["T2"];
        assert_eq!(t2.run_id, "RUN-1");
        assert_eq!(t2.problem_report_id.as_deref(), Some("PR-7"));
        assert!(!selection.results.contains_key("T4"));
        assert_eq!(selection.used().count(), 2);
    }
}
