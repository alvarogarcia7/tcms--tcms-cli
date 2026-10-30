//! Run selection policy: latest executed result per test case decides (D-01..D-05, D-16).
//!
//! Pure function over loaded runs: no I/O, no clock. Runs are ordered by their parsed
//! `started` instant, then by `run_id`, so the input order never changes the result.

use crate::run::LoadedRun;
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use testcase_verification::Verdict;

/// The result that decides one test case, and the run it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedResult {
    pub test_case_id: String,
    pub verdict: Verdict,
    pub run_id: String,
    pub build_id: String,
    pub started_at: String,
    pub problem_report_id: Option<String>,
    pub verification_result_ref: Option<String>,
}

/// Whether a run contributed to the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunUse {
    Used,
    Excluded(String),
}

/// One loaded run, with how its results were used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub run_id: String,
    pub build_id: String,
    /// The run's original `started_at` text, as recorded.
    pub started_at: String,
    pub path: PathBuf,
    /// Number of results in the run record.
    pub total: usize,
    /// Results of this run that decide a test case in the selection.
    pub decided: usize,
    /// Executed results of this run replaced by a later run.
    pub superseded: usize,
    /// Results of this run recorded as `not_executed`.
    pub not_executed: usize,
    pub use_: RunUse,
}

/// The outcome of run selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunSelection {
    /// Deciding result per test case, keyed by test case ID.
    pub results: BTreeMap<String, SelectedResult>,
    /// Every loaded run, in selection order (started ascending, then run_id).
    pub runs: Vec<RunReport>,
    /// Build IDs of the used runs (D-04).
    pub builds: BTreeSet<String>,
    /// Ties and other facts the reader needs to trace a result.
    pub notes: Vec<String>,
}

impl RunSelection {
    pub fn used(&self) -> impl Iterator<Item = &RunReport> {
        self.runs.iter().filter(|r| r.use_ == RunUse::Used)
    }

    pub fn excluded(&self) -> impl Iterator<Item = &RunReport> {
        self.runs
            .iter()
            .filter(|r| matches!(r.use_, RunUse::Excluded(_)))
    }
}

impl fmt::Display for RunSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = f;
        Ok(())
    }
}

/// Per-run counters, indexed by position in selection order.
#[derive(Debug, Clone, Copy, Default)]
struct Counts {
    total: usize,
    decided: usize,
    superseded: usize,
    not_executed: usize,
}

/// The current deciding result for one test case, with the position of its run.
struct Winner {
    index: usize,
    started: DateTime<Utc>,
    result: SelectedResult,
}

/// Select the deciding result for every test case (D-01..D-05, D-16).
pub fn select_results(runs: &[LoadedRun]) -> RunSelection {
    let mut order: Vec<&LoadedRun> = runs.iter().collect();
    order.sort_by(|a, b| {
        a.started
            .cmp(&b.started)
            .then_with(|| a.record.run_id.cmp(&b.record.run_id))
    });

    let mut counts = vec![Counts::default(); order.len()];
    let mut winners: BTreeMap<String, Winner> = BTreeMap::new();
    let mut notes: Vec<String> = Vec::new();

    for (index, run) in order.iter().enumerate() {
        counts[index].total = run.record.results.len();
        for r in &run.record.results {
            if r.verdict == Verdict::NotExecuted {
                counts[index].not_executed += 1;
                continue;
            }
            let candidate = Winner {
                index,
                started: run.started,
                result: SelectedResult {
                    test_case_id: r.test_case_id.clone(),
                    verdict: r.verdict,
                    run_id: run.record.run_id.clone(),
                    build_id: run.record.build_id.clone(),
                    started_at: run.record.started_at.clone(),
                    problem_report_id: r.problem_report_id.clone(),
                    verification_result_ref: r.verification_result_ref.clone(),
                },
            };
            if let Some(previous) = winners.insert(r.test_case_id.clone(), candidate) {
                counts[previous.index].superseded += 1;
                if previous.started == run.started {
                    notes.push(format!(
                        "{}: runs {} and {} share started_at {}; {} wins by run_id order",
                        r.test_case_id,
                        order[previous.index].record.run_id,
                        run.record.run_id,
                        run.record.started_at,
                        run.record.run_id
                    ));
                }
            }
        }
    }

    for winner in winners.values() {
        counts[winner.index].decided += 1;
    }

    let runs_report: Vec<RunReport> = order
        .iter()
        .zip(counts.iter())
        .map(|(run, c)| {
            let use_ = if c.decided > 0 {
                RunUse::Used
            } else if c.total == 0 {
                RunUse::Excluded("no results".to_string())
            } else if c.superseded == 0 {
                RunUse::Excluded("only not_executed results".to_string())
            } else {
                RunUse::Excluded("every executed result superseded by a later run".to_string())
            };
            RunReport {
                run_id: run.record.run_id.clone(),
                build_id: run.record.build_id.clone(),
                started_at: run.record.started_at.clone(),
                path: run.path.clone(),
                total: c.total,
                decided: c.decided,
                superseded: c.superseded,
                not_executed: c.not_executed,
                use_,
            }
        })
        .collect();

    let builds: BTreeSet<String> = runs_report
        .iter()
        .filter(|r| r.use_ == RunUse::Used)
        .map(|r| r.build_id.clone())
        .collect();

    let results: BTreeMap<String, SelectedResult> =
        winners.into_iter().map(|(id, w)| (id, w.result)).collect();

    RunSelection {
        results,
        runs: runs_report,
        builds,
        notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::test_support::record;

    type RunSpec = (
        &'static str,
        &'static str,
        &'static str,
        Vec<(&'static str, Verdict)>,
    );
    type Expect = (&'static str, Verdict, &'static str);

    fn run(run_id: &str, build: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
        LoadedRun::new(
            PathBuf::from(format!("{run_id}.yaml")),
            record(run_id, build, started, results),
        )
        .unwrap()
    }

    fn build_set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    fn report<'a>(sel: &'a RunSelection, run_id: &str) -> &'a RunReport {
        sel.runs
            .iter()
            .find(|r| r.run_id == run_id)
            .unwrap_or_else(|| panic!("no run report for {run_id}"))
    }

    #[test]
    fn policy_table() {
        use Verdict::*;
        let cases: Vec<(&str, Vec<RunSpec>, Vec<Expect>)> = vec![
            (
                "pass then fail: later fail wins",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Pass)]),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Fail)]),
                ],
                vec![("T1", Fail, "RUN-2")],
            ),
            (
                "fail then pass: later pass overrides (D-02)",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Fail)]),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Pass)]),
                ],
                vec![("T1", Pass, "RUN-2")],
            ),
            (
                "error then pass: later pass overrides (D-02)",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Error)]),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Pass)]),
                ],
                vec![("T1", Pass, "RUN-2")],
            ),
            (
                "pass then error: later error overrides (D-02)",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Pass)]),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Error)]),
                ],
                vec![("T1", Error, "RUN-2")],
            ),
            (
                "error then fail: later fail overrides",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Error)]),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Fail)]),
                ],
                vec![("T1", Fail, "RUN-2")],
            ),
            (
                "later run omits T1: earlier result kept (D-03)",
                vec![
                    (
                        "RUN-1",
                        "b1",
                        "2026-10-28T10:00:00Z",
                        vec![("T1", Pass), ("T2", Pass)],
                    ),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T2", Fail)]),
                ],
                vec![("T1", Pass, "RUN-1"), ("T2", Fail, "RUN-2")],
            ),
            (
                "later not_executed does not change the result (D-03)",
                vec![
                    ("RUN-1", "b1", "2026-10-28T10:00:00Z", vec![("T1", Pass)]),
                    (
                        "RUN-2",
                        "b2",
                        "2026-10-28T11:00:00Z",
                        vec![("T1", NotExecuted)],
                    ),
                ],
                vec![("T1", Pass, "RUN-1")],
            ),
            (
                "only not_executed anywhere: absent from results",
                vec![(
                    "RUN-1",
                    "b1",
                    "2026-10-28T10:00:00Z",
                    vec![("T1", NotExecuted)],
                )],
                vec![],
            ),
            (
                "offsets compare as instants (D-01): 12:30+02:00 is 10:30Z, earlier than 11:00Z",
                vec![
                    (
                        "RUN-1",
                        "b1",
                        "2026-10-28T12:30:00+02:00",
                        vec![("T1", Fail)],
                    ),
                    ("RUN-2", "b2", "2026-10-28T11:00:00Z", vec![("T1", Pass)]),
                ],
                vec![("T1", Pass, "RUN-2")],
            ),
            (
                "equal started_at: later run_id wins (D-16)",
                vec![
                    ("RUN-A", "b1", "2026-10-28T10:00:00Z", vec![("T1", Pass)]),
                    ("RUN-B", "b2", "2026-10-28T10:00:00Z", vec![("T1", Fail)]),
                ],
                vec![("T1", Fail, "RUN-B")],
            ),
        ];

        for (name, specs, expect) in cases {
            let runs: Vec<LoadedRun> = specs.iter().map(|(id, b, s, r)| run(id, b, s, r)).collect();
            let sel = select_results(&runs);

            let mut reversed = runs.clone();
            reversed.reverse();
            assert_eq!(
                select_results(&reversed),
                sel,
                "{name}: input order changed the selection"
            );

            assert_eq!(
                sel.results.len(),
                expect.len(),
                "{name}: wrong number of selected results"
            );
            for (tc, verdict, run_id) in expect {
                let got = sel
                    .results
                    .get(tc)
                    .unwrap_or_else(|| panic!("{name}: {tc} not selected"));
                assert_eq!(
                    (got.verdict, got.run_id.as_str()),
                    (verdict, run_id),
                    "{name}: {tc}"
                );
            }
        }
    }

    #[test]
    fn demo_case_resolves_to_fail_from_run_2() {
        let runs = vec![
            run(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
            run(
                "RUN-2",
                "b2",
                "2026-10-28T11:00:00Z",
                &[("T1", Verdict::Fail)],
            ),
            run(
                "RUN-3",
                "b3",
                "2026-10-28T12:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            ),
        ];
        let sel = select_results(&runs);

        let t1 = &sel.results["T1"];
        assert_eq!(t1.verdict, Verdict::Fail);
        assert_eq!(t1.run_id, "RUN-2");

        assert_eq!(
            report(&sel, "RUN-3").use_,
            RunUse::Excluded("only not_executed results".into())
        );
        assert_eq!(
            report(&sel, "RUN-1").use_,
            RunUse::Excluded("every executed result superseded by a later run".into())
        );
        let r2 = report(&sel, "RUN-2");
        assert_eq!(r2.use_, RunUse::Used);
        assert_eq!(r2.decided, 1);
        assert_eq!(sel.builds, build_set(&["b2"]));

        let mut reversed = runs.clone();
        reversed.reverse();
        assert_eq!(select_results(&reversed), sel);
    }

    #[test]
    fn equal_started_at_is_noted_and_later_run_id_wins() {
        let runs = vec![
            run(
                "RUN-B",
                "b2",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Fail)],
            ),
            run(
                "RUN-A",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
        ];
        let sel = select_results(&runs);
        assert_eq!(sel.results["T1"].run_id, "RUN-B");
        assert_eq!(
            sel.notes,
            vec![
                "T1: runs RUN-A and RUN-B share started_at 2026-10-28T10:00:00Z; RUN-B wins by run_id order"
                    .to_string()
            ]
        );
    }

    #[test]
    fn run_without_results_is_excluded_with_reason() {
        let runs = vec![
            run("RUN-0", "b0", "2026-10-28T09:00:00Z", &[]),
            run(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
        ];
        let sel = select_results(&runs);
        assert_eq!(
            report(&sel, "RUN-0").use_,
            RunUse::Excluded("no results".into())
        );
        assert_eq!(sel.used().count(), 1);
        assert_eq!(sel.excluded().count(), 1);
    }

    #[test]
    fn builds_are_those_of_used_runs_only() {
        let runs = vec![
            run(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
            run(
                "RUN-2",
                "b2",
                "2026-10-28T11:00:00Z",
                &[("T2", Verdict::Fail)],
            ),
            run(
                "RUN-3",
                "b3",
                "2026-10-28T12:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            ),
        ];
        let sel = select_results(&runs);
        assert_eq!(sel.builds, build_set(&["b1", "b2"]));
        assert_eq!(
            report(&sel, "RUN-3").use_,
            RunUse::Excluded("only not_executed results".into())
        );
    }

    #[test]
    fn mixed_run_stays_used_and_counts_each_outcome() {
        let runs = vec![
            run(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[
                    ("T1", Verdict::Pass),
                    ("T2", Verdict::Fail),
                    ("T3", Verdict::NotExecuted),
                ],
            ),
            run(
                "RUN-2",
                "b2",
                "2026-10-28T11:00:00Z",
                &[("T2", Verdict::Pass)],
            ),
        ];
        let sel = select_results(&runs);
        let r1 = report(&sel, "RUN-1");
        assert_eq!(r1.use_, RunUse::Used);
        assert_eq!(r1.total, 3);
        assert_eq!(r1.decided, 1);
        assert_eq!(r1.superseded, 1);
        assert_eq!(r1.not_executed, 1);
        let r2 = report(&sel, "RUN-2");
        assert_eq!(r2.use_, RunUse::Used);
        assert_eq!(r2.decided, 1);
        assert_eq!(sel.results["T2"].run_id, "RUN-2");
    }

    fn demo_runs() -> Vec<LoadedRun> {
        vec![
            run(
                "RUN-1",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
            run(
                "RUN-2",
                "b2",
                "2026-10-28T11:00:00Z",
                &[("T1", Verdict::Fail)],
            ),
            run(
                "RUN-3",
                "b3",
                "2026-10-28T12:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            ),
        ]
    }

    #[test]
    fn demo_display_matches_layout() {
        let sel = select_results(&demo_runs());
        let expected = [
            "Run selection (per test case, the latest run with an executed result decides)",
            "  Builds used: b2",
            "  Runs used: 1",
            "    RUN-2  build b2  started 2026-10-28T11:00:00Z  decided 1, superseded 0, not_executed 0",
            "  Runs excluded: 2",
            "    RUN-1  build b1  started 2026-10-28T10:00:00Z: every executed result superseded by a later run",
            "    RUN-3  build b3  started 2026-10-28T12:00:00Z: only not_executed results",
            "  Selected results: 1",
            "    T1  fail  RUN-2",
            "  Notes: none",
        ]
        .join("\n")
            + "\n";
        assert_eq!(sel.to_string(), expected);
    }

    #[test]
    fn empty_selection_renders_none_and_zero_counts() {
        let expected = [
            "Run selection (per test case, the latest run with an executed result decides)",
            "  Builds used: none",
            "  Runs used: 0",
            "  Runs excluded: 0",
            "  Selected results: 0",
            "  Notes: none",
        ]
        .join("\n")
            + "\n";
        assert_eq!(RunSelection::default().to_string(), expected);
    }

    #[test]
    fn tie_note_is_rendered_as_a_block_entry() {
        let runs = vec![
            run(
                "RUN-A",
                "b1",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Pass)],
            ),
            run(
                "RUN-B",
                "b2",
                "2026-10-28T10:00:00Z",
                &[("T1", Verdict::Fail)],
            ),
        ];
        let text = select_results(&runs).to_string();
        assert!(text.contains(
            "  Notes:\n    - T1: runs RUN-A and RUN-B share started_at 2026-10-28T10:00:00Z; RUN-B wins by run_id order\n"
        ));
    }
}
