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

use crate::gaps::build_gaps;
use crate::model::ReportModel;
use crate::problems::build_problems;
use crate::select::RunSelection;
use crate::view::ViewInput;

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
    let input = ViewInput { model, selection };
    let gaps = build_gaps(input);
    let problems = build_problems(input);
    Findings {
        gap_entries: gaps.no_tests.len()
            + gaps.missing_normal_range.len()
            + gaps.missing_robustness.len()
            + gaps.no_result.len(),
        failed_results: problems.rows.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gaps::{build_gaps, render_gaps_markdown};
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs};
    use crate::problems::{build_problems, render_problems_markdown};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::select_results;
    use crate::view::ViewInput;
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

        // Control: the same requirement with a decided fail in a later run is not clean and has
        // one failed result. Without it, a function that always returns zeros would pass above.
        let failing = findings_of(
            vec![link("HLR-1", &["T1"], true, true)],
            vec![
                run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)]),
                run("RUN-2", "b2", LATE, &[("T1", Verdict::Fail)]),
            ],
        );
        assert_eq!(
            failing,
            Findings {
                gap_entries: 0,
                failed_results: 1
            }
        );
        assert!(!failing.is_clean());
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

    /// Asserts a fixture whose only finding is exactly one entry in one gap list.
    fn assert_one_gap_entry(links: Vec<TraceLink>, runs: Vec<LoadedRun>) {
        let findings = findings_of(links, runs);
        assert_eq!(
            findings,
            Findings {
                gap_entries: 1,
                failed_results: 0
            }
        );
        assert!(!findings.is_clean());
    }

    #[test]
    fn gap_list_no_tests_alone_makes_findings_not_clean() {
        // Flags are true, so the only list the requirement appears in is the no-tests list.
        assert_one_gap_entry(
            vec![
                link("HLR-1", &["T1"], true, true),
                link("HLR-2", &[], true, true),
            ],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
        );
    }

    #[test]
    fn gap_list_missing_normal_range_alone_makes_findings_not_clean() {
        assert_one_gap_entry(
            vec![link("HLR-1", &["T1"], false, true)],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
        );
    }

    #[test]
    fn gap_list_missing_robustness_alone_makes_findings_not_clean() {
        assert_one_gap_entry(
            vec![link("HLR-1", &["T1"], true, false)],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)])],
        );
    }

    #[test]
    fn gap_list_no_result_alone_makes_findings_not_clean() {
        // T1 is linked and has no record in any run.
        assert_one_gap_entry(vec![link("HLR-1", &["T1"], true, true)], Vec::new());
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
        // T1 fails in RUN-1 and passes in RUN-2: the plan says this is clean.
        let findings = findings_of(
            vec![link("HLR-1", &["T1"], true, true)],
            vec![
                run("RUN-1", "b1", EARLY, &[("T1", Verdict::Fail)]),
                run("RUN-2", "b2", LATE, &[("T1", Verdict::Pass)]),
            ],
        );
        assert_eq!(
            findings,
            Findings {
                gap_entries: 0,
                failed_results: 0
            }
        );
        assert!(findings.is_clean());

        // Control: add a second linked case, T2, that fails in RUN-2. Only T2 is a failed
        // result. If the superseded T1 fail were counted, failed_results would be 2.
        let with_live_fail = findings_of(
            vec![link("HLR-1", &["T1", "T2"], true, true)],
            vec![
                run("RUN-1", "b1", EARLY, &[("T1", Verdict::Fail)]),
                run(
                    "RUN-2",
                    "b2",
                    LATE,
                    &[("T1", Verdict::Pass), ("T2", Verdict::Fail)],
                ),
            ],
        );
        assert_eq!(
            with_live_fail,
            Findings {
                gap_entries: 0,
                failed_results: 1
            }
        );
        assert!(!with_live_fail.is_clean());
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
        let literal = Findings {
            gap_entries: 2,
            failed_results: 1,
        };
        assert_eq!(literal.summary(), "2 gap entries, 1 failed results");

        // The summary of the function's own output: a requirement with no tests (3 gap entries)
        // and one decided fail (1 failed result). A stub that returns zeros cannot produce this.
        let findings = findings_of(
            vec![
                link("HLR-1", &["T1"], true, true),
                link("HLR-2", &[], false, false),
            ],
            vec![run("RUN-1", "b1", EARLY, &[("T1", Verdict::Fail)])],
        );
        assert_eq!(findings.summary(), "3 gap entries, 1 failed results");
    }

    /// The integer at the end of each `## ... (N)` heading of the gaps Markdown. The `## Report
    /// header` heading has no count and is skipped.
    fn gap_heading_counts(markdown: &str) -> Vec<usize> {
        markdown
            .lines()
            .filter(|line| line.starts_with("## ") && line.ends_with(')'))
            .map(|line| {
                let open = line.rfind('(').expect("count opens");
                let close = line.rfind(')').expect("count closes");
                line[open + 1..close].parse().expect("count is an integer")
            })
            .collect()
    }

    /// The N of the `Failed or errored results: N` line of the problems Markdown.
    fn rendered_failed_count(markdown: &str) -> usize {
        const PREFIX: &str = "Failed or errored results: ";
        let line = markdown
            .lines()
            .find(|line| line.starts_with(PREFIX))
            .expect("problems report prints the count line");
        line[PREFIX.len()..]
            .trim()
            .parse()
            .expect("count is an integer")
    }

    #[test]
    fn collect_findings_agrees_with_rendered_reports() {
        let covered = || link("HLR-1", &["T1"], true, true);
        let gap = || link("HLR-2", &[], false, false);
        let pass = || run("RUN-1", "b1", EARLY, &[("T1", Verdict::Pass)]);
        let fail = || run("RUN-1", "b1", EARLY, &[("T1", Verdict::Fail)]);
        let error = || run("RUN-1", "b1", EARLY, &[("T1", Verdict::Error)]);

        // (name, links, runs, expected gap entries, expected failed results)
        type Scenario = (&'static str, Vec<TraceLink>, Vec<LoadedRun>, usize, usize);
        let scenarios: Vec<Scenario> = vec![
            ("clean", vec![covered()], vec![pass()], 0, 0),
            ("gap_only", vec![covered(), gap()], vec![pass()], 3, 0),
            ("fail_only", vec![covered()], vec![fail()], 0, 1),
            ("error_only", vec![covered()], vec![error()], 0, 1),
            ("gap_and_fail", vec![covered(), gap()], vec![fail()], 3, 1),
        ];
        for (name, links, runs, gap_expected, failed_expected) in scenarios {
            let (model, selection) = pipeline(links, runs);
            let input = ViewInput {
                model: &model,
                selection: &selection,
            };
            let gaps = render_gaps_markdown(&build_gaps(input));
            let problems = render_problems_markdown(&build_problems(input));

            let headings = gap_heading_counts(&gaps);
            assert_eq!(headings.len(), 4, "{name}: four gap headings\n{gaps}");
            let gap_sum: usize = headings.iter().sum();
            let failed = rendered_failed_count(&problems);

            let findings = collect_findings(&model, &selection);
            assert_eq!(findings.gap_entries, gap_sum, "{name}\n{gaps}");
            assert_eq!(findings.failed_results, failed, "{name}\n{problems}");
            assert_eq!(gap_sum, gap_expected, "{name}\n{gaps}");
            assert_eq!(failed, failed_expected, "{name}\n{problems}");
            assert_eq!(
                findings.is_clean(),
                headings.iter().all(|n| *n == 0) && failed == 0,
                "{name}"
            );
        }
    }
}
