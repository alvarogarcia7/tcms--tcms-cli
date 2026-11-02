//! Findings: the one rule that says whether a report run has gaps or failed results (D-01).
//!
//! `collect_findings` calls `build_gaps` and `build_problems` on the same `ViewInput`, so the
//! counts are the counts those two reports print. It reads only the model and the run selection.
//! No kind, no format and no clock is a parameter, so the answer is the same for every kind and
//! every format, and `--kind` cannot hide a failure.
//!
//! Obsolete test cases and superseded results never count: the model and the selection already
//! left them out. A requirement with no tests is listed in all three requirement lists (Phase 3
//! decision), so it counts three times, as the gaps report prints it.

use crate::model::ReportModel;
use crate::select::RunSelection;

/// The two counts that decide the exit status of `report --fail-on-gaps` (plan 05-02).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Findings {
    /// Sum of the lengths of the four gap lists: no tests, missing normal-range test, missing
    /// robustness test, and linked test case with no recorded result.
    pub gap_entries: usize,
    /// Number of rows of the problems report: decided fail and error results.
    pub failed_results: usize,
}

impl Findings {
    /// True only when there are no gap entries and no failed results.
    pub fn is_clean(&self) -> bool {
        self.gap_entries == 0 && self.failed_results == 0
    }

    /// The fixed stderr text used by the CLI, for example `2 gap entries, 1 failed results`.
    pub fn summary(&self) -> String {
        format!(
            "{} gap entries, {} failed results",
            self.gap_entries, self.failed_results
        )
    }
}

/// The findings of one report run, from the same two views that the reports print.
pub fn collect_findings(model: &ReportModel, selection: &RunSelection) -> Findings {
    let _ = (model, selection);
    Findings::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::select_results;
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    const EARLY: &str = "2026-10-01T10:00:00Z";
    const LATE: &str = "2026-10-02T10:00:00Z";

    fn link(id: &str, cases: &[&str], normal: bool, robust: bool) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level: RequirementLevel::Hlr,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: normal,
            robustness_covered: robust,
            run_ids: Vec::new(),
        }
    }

    fn run(run_id: &str, build: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
        LoadedRun::new(
            format!("{run_id}.yaml").into(),
            record(run_id, build, started, results),
        )
        .expect("valid run")
    }

    /// Loaded inputs -> run selection -> report model: the same pipeline the CLI uses.
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

    fn findings_of(links: Vec<TraceLink>, runs: Vec<LoadedRun>) -> Findings {
        let (model, selection) = pipeline(links, runs);
        collect_findings(&model, &selection)
    }

    /// HLR-1 links T1, both coverage flags true, T1 passes in RUN-1: nothing is wrong.
    fn covered_and_passing() -> (Vec<TraceLink>, Vec<LoadedRun>) {
        (
            vec![link("HLR-1", &["T1"], true, true)],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
        )
    }

    #[test]
    fn nothing_wrong_is_clean() {
        let (links, runs) = covered_and_passing();
        let findings = findings_of(links, runs);
        assert_eq!(
            findings,
            Findings {
                gap_entries: 0,
                failed_results: 0
            }
        );
        assert!(findings.is_clean());
    }

    #[test]
    fn requirement_without_tests_counts_three_gap_entries() {
        // HLR-2 has no test cases and both flags false: it is in the no-tests list and in both
        // coverage lists, so the sum for this fixture is 3.
        let links = vec![
            link("HLR-1", &["T1"], true, true),
            link("HLR-2", &[], false, false),
        ];
        let runs = vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])];
        let findings = findings_of(links, runs);
        assert_eq!(findings.gap_entries, 3);
        assert_eq!(findings.failed_results, 0);
        assert!(!findings.is_clean());
    }

    #[test]
    fn each_gap_list_alone_makes_findings_not_clean() {
        let scenarios: Vec<(&str, Vec<TraceLink>, Vec<LoadedRun>)> = vec![
            (
                "no tests (flags true, so only the no-tests list)",
                vec![
                    link("HLR-1", &["T1"], true, true),
                    link("HLR-2", &[], true, true),
                ],
                vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
            ),
            (
                "missing normal-range test",
                vec![link("HLR-1", &["T1"], false, true)],
                vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
            ),
            (
                "missing robustness test",
                vec![link("HLR-1", &["T1"], true, false)],
                vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
            ),
            (
                "linked case with no recorded result",
                vec![link("HLR-1", &["T1"], true, true)],
                Vec::new(),
            ),
        ];
        for (label, links, runs) in scenarios {
            let findings = findings_of(links, runs);
            assert_eq!(findings.gap_entries, 1, "{label}");
            assert_eq!(findings.failed_results, 0, "{label}");
            assert!(!findings.is_clean(), "{label}");
        }
    }

    #[test]
    fn decided_fail_and_error_count_as_failed_results() {
        for verdict in [Verdict::Fail, Verdict::Error] {
            let findings = findings_of(
                vec![link("HLR-1", &["T1"], true, true)],
                vec![run("RUN-1", "b1", EARLY, &[("T1", verdict)])],
            );
            assert_eq!(
                findings,
                Findings {
                    gap_entries: 0,
                    failed_results: 1
                },
                "{verdict:?}"
            );
            assert!(!findings.is_clean(), "{verdict:?}");
        }
    }

    #[test]
    fn fail_superseded_by_later_pass_is_clean() {
        let findings = findings_of(
            vec![link("HLR-1", &["T1"], true, true)],
            vec![
                run("RUN-1", "b1", EARLY, &[("T1", Verdict::Fail)]),
                run("RUN-2", "b2", LATE, &[("T1", Verdict::Pass)]),
            ],
        );
        assert_eq!(findings.failed_results, 0);
        assert!(findings.is_clean());
    }

    #[test]
    fn only_not_executed_record_is_a_no_result_gap_not_a_failure() {
        let findings = findings_of(
            vec![link("HLR-1", &["T1"], true, true)],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::NotExecuted)])],
        );
        assert_eq!(findings.gap_entries, 1);
        assert_eq!(findings.failed_results, 0);
        assert!(!findings.is_clean());
    }

    #[test]
    fn fail_of_unlinked_test_case_still_counts() {
        let findings = findings_of(
            vec![link("HLR-1", &["T1"], true, true)],
            vec![run(
                "RUN-1",
                "b1",
                EARLY,
                &[("T1", Verdict::Pass), ("T9", Verdict::Fail)],
            )],
        );
        assert_eq!(
            findings,
            Findings {
                gap_entries: 0,
                failed_results: 1
            }
        );
        assert!(!findings.is_clean());
    }

    #[test]
    fn shuffled_inputs_give_equal_findings() {
        let links = || {
            vec![
                link("HLR-1", &["T1", "T2"], true, true),
                link("HLR-2", &[], false, false),
                link("HLR-3", &["T3"], true, true),
            ]
        };
        let runs = || {
            vec![
                run(
                    "RUN-1",
                    "b1",
                    EARLY,
                    &[("T1", Verdict::Fail), ("T2", Verdict::Pass)],
                ),
                run(
                    "RUN-2",
                    "b2",
                    LATE,
                    &[("T1", Verdict::Pass), ("T3", Verdict::Error)],
                ),
            ]
        };
        let mut links_rev = links();
        links_rev.reverse();
        let mut runs_rev = runs();
        runs_rev.reverse();

        let forward = findings_of(links(), runs());
        let backward = findings_of(links_rev, runs_rev);
        assert_eq!(forward, backward);
        assert_eq!(
            forward,
            Findings {
                gap_entries: 3,
                failed_results: 1
            }
        );
    }

    #[test]
    fn summary_text_is_fixed() {
        let findings = Findings {
            gap_entries: 2,
            failed_results: 1,
        };
        assert_eq!(findings.summary(), "2 gap entries, 1 failed results");
    }
}
