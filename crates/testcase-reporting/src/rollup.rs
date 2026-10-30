//! Verification-status roll-up for the traceability report (ROLL-02, ROLL-04).
//!
//! Pure functions over decided outcomes: no I/O, no clock. The rule lives here once so the
//! model, the renderer and later report kinds cannot disagree (D-02, D-03, D-05).

use crate::select::RunSelection;
use std::fmt;
use testcase_verification::Verdict;

/// The decided outcome of one linked live test case, as the roll-up sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CaseOutcome {
    Pass,
    Fail,
    Error,
    /// No executed result decides the case (missing, or only `not_executed`). Never a pass (D-05).
    NotRun,
}

impl CaseOutcome {
    pub fn label(self) -> &'static str {
        match self {
            CaseOutcome::Pass => "pass",
            CaseOutcome::Fail => "fail",
            CaseOutcome::Error => "error",
            CaseOutcome::NotRun => "NOT RUN",
        }
    }
}

/// The verification status of one requirement (D-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerificationStatus {
    Verified,
    Failed,
    Errored,
    NotRun,
    Partial,
    NoTests,
}

impl VerificationStatus {
    /// Every status, in declaration order.
    pub const ALL: [VerificationStatus; 6] = [
        VerificationStatus::Verified,
        VerificationStatus::Failed,
        VerificationStatus::Errored,
        VerificationStatus::NotRun,
        VerificationStatus::Partial,
        VerificationStatus::NoTests,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VerificationStatus::Verified => "VERIFIED",
            VerificationStatus::Failed => "FAILED",
            VerificationStatus::Errored => "ERRORED",
            VerificationStatus::NotRun => "NOT RUN",
            VerificationStatus::Partial => "PARTIAL",
            VerificationStatus::NoTests => "NO TESTS",
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Roll the outcomes of a requirement's linked live cases up to one status.
///
/// Callers pass only live (non-obsolete) cases (D-06, ROLL-03); obsolete cases are not this
/// function's concern. The rule is evaluated in this order (D-02, D-03):
///
/// 1. no linked case -> `NoTests`
/// 2. any `Error` -> `Errored` (error ranks above fail)
/// 3. any `Fail` -> `Failed` (a fail is never diluted by `NotRun`, nor reported as `Partial`)
/// 4. every case `Pass` -> `Verified`
/// 5. no case `Pass` (all `NotRun`) -> `NotRun`
/// 6. otherwise (some `Pass`, the rest `NotRun`) -> `Partial`
///
/// The result does not depend on the order of `outcomes`.
pub fn roll_up(outcomes: &[CaseOutcome]) -> VerificationStatus {
    if outcomes.is_empty() {
        VerificationStatus::NoTests
    } else if outcomes.contains(&CaseOutcome::Error) {
        VerificationStatus::Errored
    } else if outcomes.contains(&CaseOutcome::Fail) {
        VerificationStatus::Failed
    } else if outcomes.iter().all(|o| *o == CaseOutcome::Pass) {
        VerificationStatus::Verified
    } else if outcomes.contains(&CaseOutcome::Pass) {
        VerificationStatus::Partial
    } else {
        VerificationStatus::NotRun
    }
}

/// The roll-up input for one test case, read from the Phase 1 run selection (D-01, D-05).
///
/// A case with no entry in `selection.results` has no executed result and is `NotRun`. A
/// `not_executed` verdict is never a pass: it is also `NotRun` (ROLL-04).
pub fn case_outcome(selection: &RunSelection, test_case_id: &str) -> CaseOutcome {
    match selection.results.get(test_case_id).map(|r| r.verdict) {
        Some(Verdict::Pass) => CaseOutcome::Pass,
        Some(Verdict::Fail) => CaseOutcome::Fail,
        Some(Verdict::Error) => CaseOutcome::Error,
        Some(Verdict::NotExecuted) | None => CaseOutcome::NotRun,
    }
}

#[cfg(test)]
mod tests {
    use super::CaseOutcome::{Error, Fail, NotRun, Pass};
    use super::VerificationStatus as S;
    use super::*;
    use crate::run::test_support::record;
    use crate::run::LoadedRun;
    use crate::select::{select_results, SelectedResult};
    use std::path::PathBuf;
    use testcase_verification::Verdict;

    fn run(run_id: &str, started: &str, results: &[(&str, Verdict)]) -> LoadedRun {
        LoadedRun::new(
            PathBuf::from(format!("{run_id}.yaml")),
            record(run_id, "b1", started, results),
        )
        .expect("valid run")
    }

    #[test]
    fn selected_verdicts_map_to_outcomes() {
        let runs = vec![run(
            "RUN-1",
            "2026-10-28T10:00:00Z",
            &[
                ("T1", Verdict::Pass),
                ("T2", Verdict::Fail),
                ("T3", Verdict::Error),
            ],
        )];
        let sel = select_results(&runs);
        assert_eq!(case_outcome(&sel, "T1"), Pass);
        assert_eq!(case_outcome(&sel, "T2"), Fail);
        assert_eq!(case_outcome(&sel, "T3"), Error);
    }

    #[test]
    fn case_absent_from_selection_is_not_run() {
        let runs = vec![run(
            "RUN-1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::Pass)],
        )];
        let sel = select_results(&runs);
        assert_eq!(case_outcome(&sel, "T404"), NotRun);
    }

    #[test]
    fn case_with_only_not_executed_results_is_not_run() {
        let runs = vec![run(
            "RUN-1",
            "2026-10-28T10:00:00Z",
            &[("T1", Verdict::NotExecuted)],
        )];
        let sel = select_results(&runs);
        assert!(!sel.results.contains_key("T1"));
        assert_eq!(case_outcome(&sel, "T1"), NotRun);
    }

    #[test]
    fn demo_later_not_executed_does_not_override_fail() {
        // RUN-1 pass, RUN-2 fail, RUN-3 not_executed on T1: the executed fail decides (D-01).
        let runs = vec![
            run("RUN-1", "2026-10-28T10:00:00Z", &[("T1", Verdict::Pass)]),
            run("RUN-2", "2026-10-28T11:00:00Z", &[("T1", Verdict::Fail)]),
            run(
                "RUN-3",
                "2026-10-28T12:00:00Z",
                &[("T1", Verdict::NotExecuted)],
            ),
        ];
        let sel = select_results(&runs);
        assert_eq!(case_outcome(&sel, "T1"), Fail);
    }

    #[test]
    fn hand_built_not_executed_verdict_is_never_a_pass() {
        let mut sel = RunSelection::default();
        sel.results.insert(
            "T1".to_string(),
            SelectedResult {
                test_case_id: "T1".to_string(),
                verdict: Verdict::NotExecuted,
                run_id: "RUN-1".to_string(),
                build_id: "b1".to_string(),
                started_at: "2026-10-28T10:00:00Z".to_string(),
                problem_report_id: None,
                verification_result_ref: None,
            },
        );
        assert_eq!(case_outcome(&sel, "T1"), NotRun);
    }

    /// Every slice of length 0 to 4 over the four outcomes, built from base-4 digits.
    fn all_slices_up_to_four() -> Vec<Vec<CaseOutcome>> {
        let symbols = [Pass, Fail, Error, NotRun];
        let mut out = Vec::new();
        for len in 0..=4u32 {
            for code in 0..4usize.pow(len) {
                let mut rest = code;
                let slice: Vec<CaseOutcome> = (0..len)
                    .map(|_| {
                        let digit = rest % 4;
                        rest /= 4;
                        symbols[digit]
                    })
                    .collect();
                out.push(slice);
            }
        }
        out
    }

    fn permutations(items: &[CaseOutcome]) -> Vec<Vec<CaseOutcome>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }
        let mut out = Vec::new();
        for i in 0..items.len() {
            let mut rest = items.to_vec();
            let head = rest.remove(i);
            for mut tail in permutations(&rest) {
                tail.insert(0, head);
                out.push(tail);
            }
        }
        out
    }

    #[test]
    fn status_table() {
        let cases: Vec<(Vec<CaseOutcome>, S)> = vec![
            (vec![], S::NoTests),
            (vec![NotRun], S::NotRun),
            (vec![NotRun, NotRun], S::NotRun),
            (vec![Pass], S::Verified),
            (vec![Pass, Pass, Pass], S::Verified),
            (vec![Pass, NotRun], S::Partial),
            (vec![Fail], S::Failed),
            (vec![Pass, Fail], S::Failed),
            (vec![Fail, NotRun], S::Failed),
            (vec![Pass, Fail, NotRun], S::Failed),
            (vec![Error], S::Errored),
            (vec![Pass, Error], S::Errored),
            (vec![Error, Fail], S::Errored),
            (vec![Fail, Error, Pass, NotRun], S::Errored),
            (vec![Error, NotRun], S::Errored),
        ];
        for (outcomes, expected) in cases {
            assert_eq!(roll_up(&outcomes), expected, "outcomes {outcomes:?}");
        }
    }

    #[test]
    fn every_permutation_of_mixed_outcomes_gives_the_same_status() {
        let base = [Pass, Fail, NotRun, Error];
        for order in permutations(&base) {
            assert_eq!(roll_up(&order), S::Errored, "order {order:?}");
        }
    }

    #[test]
    fn verified_iff_non_empty_and_all_pass() {
        let slices = all_slices_up_to_four();
        assert_eq!(slices.len(), 1 + 4 + 16 + 64 + 256);
        for outcomes in slices {
            let expected_verified = !outcomes.is_empty() && outcomes.iter().all(|o| *o == Pass);
            let is_verified = roll_up(&outcomes) == S::Verified;
            assert_eq!(is_verified, expected_verified, "outcomes {outcomes:?}");
        }
    }

    #[test]
    fn labels_match_report_wording() {
        let labels: Vec<&str> = S::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(
            labels,
            ["VERIFIED", "FAILED", "ERRORED", "NOT RUN", "PARTIAL", "NO TESTS"]
        );
    }
}
