//! Pure V&V metric rules (TCMS-47). No I/O.
//!
//! The rules are those of the tcms-web reference snapshot in `docs/reference/tcms-web/`:
//! - pass rate and gate verdict: DECISIONS D-95 (`report/PassRate.java`)
//! - run-over-run delta: D-96
//! - flaky / failing / stable: D-97 (`report/StabilityClassifier.java`)
//! - requirement coverage: D-116.3 (`trace/CoverageClassifier.java`)
//! - V&V run figures: `vv/TestRunDataService.java`
//!
//! Percentages are kept as exact ratios (`Ratio`) and only turned into numbers at the edge, so the
//! gate comparison and the delta use integer arithmetic.

/// Result status of one test case in one run (the tcms-web vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Passed,
    Failed,
    Blocked,
    Skipped,
    Untested,
}

impl Status {
    /// Map a `verdict` value of a test-run record to a status.
    ///
    /// `pass`, `fail` and `not_executed` come from the tcms-schemas test-run enum; `error` and
    /// `blocked` mean the test could not reach a verdict (Blocked); `skipped` and `untested` are the
    /// remaining tcms-web statuses. Any other value is `None`, so a typo is an error, not a count.
    pub fn from_verdict(verdict: &str) -> Option<Status> {
        match verdict {
            "pass" => Some(Status::Passed),
            "fail" => Some(Status::Failed),
            "error" | "blocked" => Some(Status::Blocked),
            "skipped" => Some(Status::Skipped),
            "not_executed" | "untested" => Some(Status::Untested),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Passed => "PASSED",
            Status::Failed => "FAILED",
            Status::Blocked => "BLOCKED",
            Status::Skipped => "SKIPPED",
            Status::Untested => "UNTESTED",
        }
    }
}

/// Number of results per status in one run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub passed: u64,
    pub failed: u64,
    pub blocked: u64,
    pub skipped: u64,
    pub untested: u64,
}

impl Counts {
    pub fn add(&mut self, status: Status) {
        match status {
            Status::Passed => self.passed += 1,
            Status::Failed => self.failed += 1,
            Status::Blocked => self.blocked += 1,
            Status::Skipped => self.skipped += 1,
            Status::Untested => self.untested += 1,
        }
    }

    /// Passed + Failed + Blocked: the D-95 pass-rate denominator. Skipped and Untested are outside it.
    pub fn denominator(&self) -> u64 {
        self.passed + self.failed + self.blocked
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

/// Change of the exact pass rate from the previous run, in percentage points, rounded to an integer
/// with halves rounded up (D-96). `None` when either side has no data.
pub fn delta(current: Option<Ratio>, previous: Option<Ratio>) -> Option<i64> {
    let (c, p) = (current?, previous?);
    // (c.part/c.whole - p.part/p.whole) * 100 = n / m, with m > 0.
    let n = 100 * (c.part as i128 * p.whole as i128 - p.part as i128 * c.whole as i128);
    let m = c.whole as i128 * p.whole as i128;
    // round half up of n/m is floor((2n + m) / 2m); div_euclid is floor for a positive divisor.
    Some((2 * n + m).div_euclid(2 * m) as i64)
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
    /// Linked cases that have a result in the run other than Untested.
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
        .filter(|s| matches!(s, Some(st) if *st != Status::Untested))
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

    fn counts(passed: u64, failed: u64, blocked: u64, skipped: u64, untested: u64) -> Counts {
        Counts {
            passed,
            failed,
            blocked,
            skipped,
            untested,
        }
    }

    #[test]
    fn gate_reached_exactly_is_passed() {
        // 9 of 10 at gate 90: 900 >= 90 * 10.
        let g = gate(counts(9, 1, 0, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Passed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 90);
    }

    #[test]
    fn one_below_the_gate_is_failed_and_display_rounds_down() {
        // 89 of 100 at gate 90: display 89, verdict FAILED.
        let g = gate(counts(89, 11, 0, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Failed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 89);
    }

    #[test]
    fn two_thirds_displays_66_and_keeps_exact_value() {
        let r = gate(counts(2, 1, 0, 0, 0), 90).pass_rate.unwrap();
        assert_eq!(r.floor_percent(), 66);
        assert!((r.exact() - 66.666_666).abs() < 1e-4);
    }

    #[test]
    fn zero_denominator_is_no_data() {
        let g = gate(counts(0, 0, 0, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::NoData);
        assert!(g.pass_rate.is_none());
    }

    #[test]
    fn all_skipped_is_no_data() {
        let g = gate(counts(0, 0, 0, 4, 0), 90);
        assert_eq!(g.verdict, Verdict::NoData);
        assert!(g.pass_rate.is_none());
    }

    #[test]
    fn all_untested_is_no_data() {
        let g = gate(counts(0, 0, 0, 0, 5), 90);
        assert_eq!(g.verdict, Verdict::NoData);
    }

    #[test]
    fn only_blocked_is_failed_with_display_zero() {
        let g = gate(counts(0, 0, 3, 0, 0), 90);
        assert_eq!(g.verdict, Verdict::Failed);
        assert_eq!(g.pass_rate.unwrap().floor_percent(), 0);
    }

    #[test]
    fn skipped_and_untested_are_outside_the_denominator() {
        // 9 passed, 1 failed, plus skipped and untested: still 90 percent.
        let g = gate(counts(9, 1, 0, 5, 5), 90);
        assert_eq!(g.pass_rate.unwrap().whole, 10);
        assert_eq!(g.verdict, Verdict::Passed);
    }

    #[test]
    fn gate_threshold_is_a_parameter() {
        assert_eq!(gate(counts(8, 2, 0, 0, 0), 80).verdict, Verdict::Passed);
        assert_eq!(gate(counts(8, 2, 0, 0, 0), 81).verdict, Verdict::Failed);
    }

    #[test]
    fn round_percent_is_half_up() {
        assert_eq!(Ratio::new(1, 8).unwrap().round_percent(), 13); // 12.5 -> 13
        assert_eq!(Ratio::new(2, 3).unwrap().round_percent(), 67);
        assert_eq!(Ratio::new(1, 3).unwrap().round_percent(), 33);
    }

    #[test]
    fn delta_first_run_is_none() {
        assert_eq!(delta(Ratio::new(1, 1), None), None);
    }

    #[test]
    fn delta_with_no_data_on_either_side_is_none() {
        assert_eq!(delta(None, Ratio::new(1, 1)), None);
        assert_eq!(delta(Ratio::new(1, 1), None), None);
        assert_eq!(delta(None, None), None);
    }

    #[test]
    fn delta_positive_negative_and_zero() {
        // 2/3 against 1/1: -33.33 points, rounded to -33.
        assert_eq!(delta(Ratio::new(2, 3), Ratio::new(1, 1)), Some(-33));
        // 1/1 against 2/3: +33.33 points, rounded to +33.
        assert_eq!(delta(Ratio::new(1, 1), Ratio::new(2, 3)), Some(33));
        // Equal rates: zero.
        assert_eq!(delta(Ratio::new(1, 2), Ratio::new(2, 4)), Some(0));
    }

    #[test]
    fn delta_halves_round_up_both_signs() {
        // 1/2 against 0/1 is +50 exactly; 0/1 against 1/2 is -50 exactly.
        assert_eq!(delta(Ratio::new(1, 2), Ratio::new(0, 1)), Some(50));
        assert_eq!(delta(Ratio::new(0, 1), Ratio::new(1, 2)), Some(-50));
        // 1/8 (12.5 percent) against 1/4 (25 percent): -12.5 rounds half up to -12 (Math.round).
        assert_eq!(delta(Ratio::new(1, 8), Ratio::new(1, 4)), Some(-12));
    }

    #[test]
    fn stability_flip_boundaries_at_default_threshold() {
        use Status::*;
        // 3 flips: below the threshold of 4, last kept result Passed -> Stable.
        let below = [Failed, Passed, Failed, Passed];
        let r = stability(&below, 4);
        assert_eq!((r.flips, r.stability), (3, Stability::Stable));
        // 4 flips: at the threshold -> Flaky.
        let at = [Passed, Failed, Passed, Failed, Passed];
        let r = stability(&at, 4);
        assert_eq!((r.flips, r.stability), (4, Stability::Flaky));
        // 5 flips: above the threshold -> Flaky.
        let above = [Passed, Failed, Passed, Failed, Passed, Failed];
        let r = stability(&above, 4);
        assert_eq!((r.flips, r.stability), (5, Stability::Flaky));
    }

    #[test]
    fn stability_threshold_is_a_parameter() {
        use Status::*;
        let two_flips = [Passed, Failed, Passed];
        assert_eq!(stability(&two_flips, 2).stability, Stability::Flaky);
        assert_eq!(stability(&two_flips, 3).stability, Stability::Stable);
    }

    #[test]
    fn stability_ignores_blocked_skipped_and_untested() {
        use Status::*;
        // Kept results are Passed, Failed, Passed: two flips, last kept result Passed -> Stable.
        let statuses = [Passed, Blocked, Skipped, Untested, Failed, Untested, Passed];
        let r = stability(&statuses, 4);
        assert_eq!((r.flips, r.stability), (2, Stability::Stable));
    }

    #[test]
    fn stability_failing_when_last_kept_result_is_failed() {
        use Status::*;
        let r = stability(&[Passed, Passed, Failed], 4);
        assert_eq!((r.flips, r.stability), (1, Stability::Failing));
        // A later Blocked does not hide the last kept Failed.
        let r = stability(&[Passed, Failed, Blocked], 4);
        assert_eq!(r.stability, Stability::Failing);
    }

    #[test]
    fn stability_no_results_is_stable() {
        let r = stability(&[], 4);
        assert_eq!((r.flips, r.stability), (0, Stability::Stable));
    }

    #[test]
    fn flaky_outranks_failing() {
        use Status::*;
        // Four flips and a last kept result of Failed: Flaky, not Failing.
        let r = stability(&[Passed, Failed, Passed, Failed, Passed, Failed], 4);
        assert_eq!(r.stability, Stability::Flaky);
    }

    #[test]
    fn coverage_classes() {
        use Status::*;
        assert_eq!(coverage(&[]).coverage, Coverage::NoCases);
        assert_eq!(
            coverage(&[Some(Passed), Some(Passed)]).coverage,
            Coverage::Covered
        );
        assert_eq!(
            coverage(&[Some(Passed), Some(Failed)]).coverage,
            Coverage::Failing
        );
        assert_eq!(coverage(&[Some(Passed), None]).coverage, Coverage::NotRun);
        assert_eq!(
            coverage(&[Some(Passed), Some(Untested)]).coverage,
            Coverage::NotRun
        );
        assert_eq!(coverage(&[Some(Blocked)]).coverage, Coverage::NotRun);
        assert_eq!(coverage(&[None]).coverage, Coverage::NotRun);
    }

    #[test]
    fn coverage_failing_outranks_not_run_and_counts_executed() {
        use Status::*;
        let r = coverage(&[Some(Failed), None, Some(Untested), Some(Skipped)]);
        assert_eq!(r.coverage, Coverage::Failing);
        assert_eq!(r.total, 4);
        // Failed and Skipped have results; None and Untested do not.
        assert_eq!(r.executed, 2);
    }

    #[test]
    fn status_mapping_from_verdicts() {
        assert_eq!(Status::from_verdict("pass"), Some(Status::Passed));
        assert_eq!(Status::from_verdict("fail"), Some(Status::Failed));
        assert_eq!(Status::from_verdict("error"), Some(Status::Blocked));
        assert_eq!(Status::from_verdict("skipped"), Some(Status::Skipped));
        assert_eq!(Status::from_verdict("not_executed"), Some(Status::Untested));
        assert_eq!(Status::from_verdict("PASS"), None);
    }
}
