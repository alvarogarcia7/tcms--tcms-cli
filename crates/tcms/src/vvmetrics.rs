//! Pure V&V metric rules (TCMS-47). No I/O.
//!
//! The rules are those of the tcms-web reference snapshot in `docs/reference/tcms-web/`, with the
//! decisions of TCMS-50 applied on top:
//! - pass rate and gate verdict: DECISIONS D-95 (`report/PassRate.java`), with the buckets of TCMS-50.2
//! - run-over-run delta: D-96, as changed by TCMS-50.3 (difference of the printed rates)
//! - flaky / failing / stable: D-97 (`report/StabilityClassifier.java`)
//! - requirement coverage: D-116.3 (`trace/CoverageClassifier.java`)
//! - V&V run figures: `vv/TestRunDataService.java`
//!
//! Percentages are kept as exact ratios (`Ratio`) and only turned into numbers at the edge, so the
//! gate comparison uses integer arithmetic.

/// Result status of one test case in one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Passed,
    /// `fail`, `error` (TCMS-50.2: error is a failure for the gate) and the legacy `blocked`.
    Failed,
    /// `not_executed` (and the legacy `untested`): the case did not run. In the gate denominator as
    /// not passed, never counted as Failed.
    NotExecuted,
    /// `skipped`: outside the gate denominator.
    Skipped,
}

impl Status {
    /// Map a `verdict` value of a test-run record to a status. Any other value is `None`, so a typo
    /// is an error, not a count.
    ///
    /// `pass`, `fail`, `error` and `not_executed` are the test-run schema enum (`error` is a failure
    /// for the gate, TCMS-50.2). `skipped` stays as a legacy value outside the denominator. `blocked`
    /// and `untested` are legacy tcms-web spellings: `blocked` counts as a failure, as in Kiwi TCMS,
    /// and `untested` as not executed.
    pub fn from_verdict(verdict: &str) -> Option<Status> {
        match verdict {
            "pass" => Some(Status::Passed),
            "fail" | "error" | "blocked" => Some(Status::Failed),
            "not_executed" | "untested" => Some(Status::NotExecuted),
            "skipped" => Some(Status::Skipped),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Passed => "PASSED",
            Status::Failed => "FAILED",
            Status::NotExecuted => "NOT_EXECUTED",
            Status::Skipped => "SKIPPED",
        }
    }
}

/// Number of results per status in one run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub passed: u64,
    pub failed: u64,
    pub not_executed: u64,
    pub skipped: u64,
}

impl Counts {
    pub fn add(&mut self, status: Status) {
        match status {
            Status::Passed => self.passed += 1,
            Status::Failed => self.failed += 1,
            Status::NotExecuted => self.not_executed += 1,
            Status::Skipped => self.skipped += 1,
        }
    }

    /// Passed + Failed + NotExecuted: the pass-rate denominator (TCMS-50.2). Skipped is outside it.
    pub fn denominator(&self) -> u64 {
        self.passed + self.failed + self.not_executed
    }
}

/// A part of a whole, kept exact. `whole == 0` means there is no data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    pub part: u64,
    pub whole: u64,
}

impl Ratio {
    /// The ratio, or `None` when there is nothing to divide by.
    pub fn new(part: u64, whole: u64) -> Option<Ratio> {
        (whole > 0).then_some(Ratio { part, whole })
    }

    /// Exact percentage, not rounded.
    pub fn exact(&self) -> f64 {
        self.part as f64 * 100.0 / self.whole as f64
    }

    /// Percentage rounded down (the gate display: a badge never reads "90% FAILED").
    pub fn floor_percent(&self) -> u64 {
        self.part * 100 / self.whole
    }

    /// Percentage rounded half up (the V&V view figure, as `Math.round` in TestRunDataService).
    pub fn round_percent(&self) -> u64 {
        (2 * self.part * 100 + self.whole) / (2 * self.whole)
    }
}

/// Gate verdict of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Passed,
    Failed,
    NoData,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Passed => "PASSED",
            Verdict::Failed => "FAILED",
            Verdict::NoData => "NO_DATA",
        }
    }
}

/// Pass rate and gate verdict of one run (D-95).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gate {
    pub counts: Counts,
    pub threshold: u64,
    pub verdict: Verdict,
    /// `None` when the denominator is zero.
    pub pass_rate: Option<Ratio>,
}

/// Gate of a run: passed when `passed * 100 >= threshold * denominator`, exact integer arithmetic.
pub fn gate(counts: Counts, threshold: u64) -> Gate {
    let pass_rate = Ratio::new(counts.passed, counts.denominator());
    let verdict = match pass_rate {
        None => Verdict::NoData,
        Some(r) if r.part * 100 >= threshold * r.whole => Verdict::Passed,
        Some(_) => Verdict::Failed,
    };
    Gate {
        counts,
        threshold,
        verdict,
        pass_rate,
    }
}

/// Change of the printed gate pass rate from the previous run, in whole percentage points (D-96 as
/// changed by TCMS-50.3): the difference of the two floored rates, so every printed delta equals the
/// difference of the printed rates. `None` when either side has no data.
pub fn delta(current: Option<Ratio>, previous: Option<Ratio>) -> Option<i64> {
    let (c, p) = (current?, previous?);
    Some(c.floor_percent() as i64 - p.floor_percent() as i64)
}

/// Stability of one case over the runs up to the selected one (D-97).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stability {
    Stable,
    Failing,
    Flaky,
}

impl Stability {
    pub fn as_str(self) -> &'static str {
        match self {
            Stability::Stable => "STABLE",
            Stability::Failing => "FAILING",
            Stability::Flaky => "FLAKY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StabilityResult {
    pub stability: Stability,
    pub flips: u64,
}

/// Classify a case from its statuses, oldest first. Only Passed and Failed count; a flip is a change
/// between two neighbouring kept results. Flips at or above `flaky_threshold` make the case Flaky,
/// which outranks Failing; otherwise a last kept result of Failed is Failing; anything else is Stable.
pub fn stability(history_oldest_first: &[Status], flaky_threshold: u64) -> StabilityResult {
    let mut flips = 0u64;
    let mut previous: Option<Status> = None;
    for status in history_oldest_first {
        if !matches!(status, Status::Passed | Status::Failed) {
            continue;
        }
        if let Some(prev) = previous {
            if prev != *status {
                flips += 1;
            }
        }
        previous = Some(*status);
    }
    let stability = if flips >= flaky_threshold {
        Stability::Flaky
    } else if previous == Some(Status::Failed) {
        Stability::Failing
    } else {
        Stability::Stable
    };
    StabilityResult { stability, flips }
}

/// Requirement coverage class in one run (D-116.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    Covered,
    Failing,
    NotRun,
    NoCases,
}

impl Coverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Coverage::Covered => "COVERED",
            Coverage::Failing => "FAILING",
            Coverage::NotRun => "NOT_RUN",
            Coverage::NoCases => "NO_CASES",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageResult {
    pub coverage: Coverage,
    /// Linked cases that have a result in the run other than NotExecuted.
    pub executed: u64,
    pub total: u64,
}

/// Classify a requirement from the statuses of its linked cases in the run (`None`: not in the run).
/// No linked case is NoCases; any Failed is Failing; all Passed is Covered; anything else is NotRun.
pub fn coverage(statuses: &[Option<Status>]) -> CoverageResult {
    let total = statuses.len() as u64;
    if statuses.is_empty() {
        return CoverageResult {
            coverage: Coverage::NoCases,
            executed: 0,
            total: 0,
        };
    }
    let executed = statuses
        .iter()
        .filter(|s| matches!(s, Some(st) if *st != Status::NotExecuted))
        .count() as u64;
    let class = if statuses.contains(&Some(Status::Failed)) {
        Coverage::Failing
    } else if statuses.iter().all(|s| *s == Some(Status::Passed)) {
        Coverage::Covered
    } else {
        Coverage::NotRun
    };
    CoverageResult {
        coverage: class,
        executed,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(passed: u64, failed: u64, not_executed: u64, skipped: u64) -> Counts {
        Counts {
            passed,
            failed,
            not_executed,
            skipped,
        }
    }

    #[test]
    fn gate_reached_exactly_is_passed() {
        let g = gate(counts(9, 1, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Passed);
    }

    #[test]
    fn one_below_the_gate_is_failed_and_display_rounds_down() {
        // 89 of 100 at gate 90: display 89, verdict FAILED.
        let g = gate(counts(89, 11, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Failed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 89);
    }

    #[test]
    fn two_thirds_displays_66_and_keeps_exact_value() {
        let r = Ratio::new(2, 3).unwrap();
        assert_eq!(r.floor_percent(), 66);
        assert!((r.exact() - 66.666).abs() < 0.01);
    }

    #[test]
    fn zero_denominator_is_no_data() {
        let g = gate(Counts::default(), 90);
        assert_eq!(g.verdict, Verdict::NoData);
        assert!(g.pass_rate.is_none());
    }

    #[test]
    fn all_skipped_is_no_data() {
        let g = gate(counts(0, 0, 0, 5), 90);
        assert_eq!(g.verdict, Verdict::NoData);
    }

    #[test]
    fn only_failed_is_failed_with_display_zero() {
        let g = gate(counts(0, 3, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Failed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 0);
    }

    #[test]
    fn skipped_is_outside_the_denominator() {
        // 9 passed, 1 failed, plus 5 skipped: still 90 percent.
        let g = gate(counts(9, 1, 0, 5), 90);
        assert_eq!(g.counts.denominator(), 10);
        assert_eq!(g.verdict, Verdict::Passed);
    }

    #[test]
    fn not_executed_is_in_the_denominator_as_not_passed() {
        // 9 passed and 1 not executed: 90 of 10 in the denominator, so the gate passes at 90 but a
        // single not-executed case lowers the rate that a passing set would show.
        let g = gate(counts(9, 0, 1, 0), 90);
        assert_eq!(g.counts.denominator(), 10);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 90);
        let lower = gate(counts(8, 0, 2, 0), 90);
        assert_eq!(lower.verdict, Verdict::Failed);
    }

    #[test]
    fn not_executed_only_is_failed_not_no_data() {
        let g = gate(counts(0, 0, 4, 0), 90);
        assert_eq!(g.verdict, Verdict::Failed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 0);
    }

    #[test]
    fn gate_threshold_is_a_parameter() {
        assert_eq!(gate(counts(8, 2, 0, 0), 80).verdict, Verdict::Passed);
        assert_eq!(gate(counts(8, 2, 0, 0), 81).verdict, Verdict::Failed);
    }

    #[test]
    fn round_percent_is_half_up() {
        assert_eq!(Ratio::new(1, 8).unwrap().round_percent(), 13);
        assert_eq!(Ratio::new(1, 2).unwrap().round_percent(), 50);
    }

    #[test]
    fn delta_first_run_is_none() {
        assert_eq!(delta(Ratio::new(1, 1), None), None);
    }

    #[test]
    fn delta_with_no_data_on_either_side_is_none() {
        assert_eq!(delta(None, Ratio::new(1, 1)), None);
        assert_eq!(delta(Ratio::new(1, 1), None), None);
    }

    #[test]
    fn delta_is_the_difference_of_the_printed_rates() {
        // TCMS-50.3: 0% to 12.5% prints 0 and 12, so the delta is +12, not +13.
        let zero = Ratio::new(0, 8);
        let eighth = Ratio::new(1, 8); // 12.5 percent, printed 12
        assert_eq!(delta(eighth, zero), Some(12));
        assert_eq!(delta(zero, eighth), Some(-12));
        assert_eq!(delta(Ratio::new(1, 1), Ratio::new(1, 2)), Some(50));
    }

    #[test]
    fn delta_zero_when_printed_rates_are_equal() {
        // 12.5 and 12.9 both print 12.
        assert_eq!(delta(Ratio::new(1, 8), Ratio::new(129, 1000)), Some(0));
    }

    #[test]
    fn stability_flip_boundaries_at_default_threshold() {
        use Status::*;
        // Three flips with a threshold of 4: stable, last kept result PASSED.
        let three = [Passed, Failed, Passed, Failed];
        assert_eq!(stability(&three, 4).flips, 3);
        assert_eq!(stability(&three, 4).stability, Stability::Failing);
        // Four flips reach the threshold: flaky.
        let four = [Passed, Failed, Passed, Failed, Passed];
        assert_eq!(stability(&four, 4).stability, Stability::Flaky);
    }

    #[test]
    fn stability_threshold_is_a_parameter() {
        use Status::*;
        let two = [Passed, Failed, Passed];
        assert_eq!(stability(&two, 2).stability, Stability::Flaky);
        assert_eq!(stability(&two, 3).stability, Stability::Stable);
    }

    #[test]
    fn stability_ignores_skipped_and_not_executed() {
        use Status::*;
        let h = [Passed, Skipped, NotExecuted, Passed];
        let s = stability(&h, 1);
        assert_eq!(s.flips, 0);
        assert_eq!(s.stability, Stability::Stable);
    }

    #[test]
    fn error_counts_as_failed_in_stability() {
        // error is Failed (TCMS-50.2), so Passed then error is one flip.
        let h = [
            Status::from_verdict("pass").unwrap(),
            Status::from_verdict("error").unwrap(),
        ];
        assert_eq!(stability(&h, 4).flips, 1);
        assert_eq!(stability(&h, 4).stability, Stability::Failing);
    }

    #[test]
    fn stability_failing_when_last_kept_result_is_failed() {
        use Status::*;
        assert_eq!(
            stability(&[Passed, Failed], 4).stability,
            Stability::Failing
        );
    }

    #[test]
    fn stability_no_results_is_stable() {
        assert_eq!(stability(&[], 4).stability, Stability::Stable);
    }

    #[test]
    fn flaky_outranks_failing() {
        use Status::*;
        let h = [Failed, Passed, Failed, Passed, Failed];
        assert_eq!(stability(&h, 4).stability, Stability::Flaky);
    }

    #[test]
    fn coverage_classes() {
        use Status::*;
        assert_eq!(coverage(&[]).coverage, Coverage::NoCases);
        assert_eq!(
            coverage(&[Some(Passed), Some(Passed)]).coverage,
            Coverage::Covered
        );
        assert_eq!(coverage(&[Some(Passed), None]).coverage, Coverage::NotRun);
        assert_eq!(coverage(&[Some(NotExecuted)]).coverage, Coverage::NotRun);
    }

    #[test]
    fn coverage_failing_outranks_not_run_and_counts_executed() {
        use Status::*;
        let c = coverage(&[Some(Failed), Some(NotExecuted), Some(Passed), None]);
        assert_eq!(c.coverage, Coverage::Failing);
        assert_eq!(c.executed, 2);
        assert_eq!(c.total, 4);
    }

    #[test]
    fn status_mapping_from_verdicts() {
        assert_eq!(Status::from_verdict("pass"), Some(Status::Passed));
        assert_eq!(Status::from_verdict("fail"), Some(Status::Failed));
        assert_eq!(Status::from_verdict("error"), Some(Status::Failed));
        assert_eq!(Status::from_verdict("blocked"), Some(Status::Failed));
        assert_eq!(Status::from_verdict("skipped"), Some(Status::Skipped));
        assert_eq!(
            Status::from_verdict("not_executed"),
            Some(Status::NotExecuted)
        );
        assert_eq!(Status::from_verdict("untested"), Some(Status::NotExecuted));
        assert_eq!(Status::from_verdict("PASS"), None);
    }
}
