//! Property-based fuzzing of `tcms vv metrics` (proptest). Each property generates random V&V run records,
//! runs the binary, and checks an invariant or an oracle computed from the generated input. Failing inputs
//! are shrunk by proptest; set PROPTEST_CASES to run more cases.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use proptest::prelude::*;
use tempfile::TempDir;

const NOW: &str = "2026-01-01T00:00:00Z";

fn tcms(root: &Path, extra: &[&str]) -> Output {
    let schemas = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas");
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", schemas, "vv", "metrics", "--root"])
        .arg(root)
        .args(["--now", NOW])
        .args(extra)
        .output()
        .unwrap()
}

fn json(o: &Output) -> serde_json::Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}

/// One generated run: an id, a start day, and (case index, verdict) pairs with distinct cases.
#[derive(Debug, Clone)]
struct RunSpec {
    day: u32,
    results: Vec<(usize, &'static str)>,
}

fn verdict() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        3 => Just("pass"),
        2 => Just("fail"),
        1 => Just("error"),
        1 => Just("blocked"),
        1 => Just("not_executed"),
        1 => Just("skipped"),
        1 => Just("untested"),
    ]
}

fn run_spec() -> impl Strategy<Value = RunSpec> {
    (
        1u32..=28,
        prop::collection::vec((0usize..6, verdict()), 1..6),
    )
        .prop_map(|(day, pairs)| {
            let mut seen = std::collections::BTreeSet::new();
            let results = pairs.into_iter().filter(|(c, _)| seen.insert(*c)).collect();
            RunSpec { day, results }
        })
}

/// Index of the run the binary selects by default: the latest `started_at`, then the larger folder name.
fn selected_index(runs: &[RunSpec]) -> usize {
    (0..runs.len()).max_by_key(|&i| (runs[i].day, i)).unwrap()
}

fn write_runs(root: &Path, runs: &[RunSpec]) {
    for (i, run) in runs.iter().enumerate() {
        let id = format!("RUN-{i:02}");
        let dir = root.join("runs").join(&id);
        fs::create_dir_all(&dir).unwrap();
        let mut y = format!(
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {id}\nbuild_id: b-1\n\
             environment: Bench A\ntester: t\nstarted_at: \"2026-10-{:02}T10:00:00Z\"\n\
             finished_at: \"2026-10-{:02}T11:00:00Z\"\n",
            run.day, run.day
        );
        if run.results.is_empty() {
            y.push_str("results: []\n");
        } else {
            y.push_str("results:\n");
            for (c, v) in &run.results {
                y.push_str(&format!("- test_case_id: TC_{c}\n  verdict: {v}\n"));
            }
        }
        fs::write(dir.join("test-run.yaml"), y).unwrap();
    }
}

/// Figures computed from the spec alone (no binary involved).
struct Expect {
    passed: u64,
    failed: u64,
    not_executed: u64,
    skipped: u64,
    total: u64,
}

fn expect_counts(results: &[(usize, &'static str)]) -> Expect {
    let mut e = Expect {
        passed: 0,
        failed: 0,
        not_executed: 0,
        skipped: 0,
        total: results.len() as u64,
    };
    for (_, v) in results {
        match *v {
            "pass" => e.passed += 1,
            "fail" | "error" | "blocked" => e.failed += 1,
            "not_executed" | "untested" => e.not_executed += 1,
            "skipped" => e.skipped += 1,
            other => panic!("generator produced {other}"),
        }
    }
    e
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// The gate figures of the last run equal the figures computed from its generated results.
    #[test]
    fn gate_figures_match_the_selected_run(runs in prop::collection::vec(run_spec(), 1..4)) {
        prop_assume!(runs.iter().all(|r| !r.results.is_empty()));
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), &runs);
        let last = &runs[selected_index(&runs)];
        let e = expect_counts(&last.results);
        let v = json(&tcms(tmp.path(), &[]));
        let den = e.passed + e.failed + e.not_executed;
        prop_assert_eq!(v["gate"]["counts"]["passed"].as_u64().unwrap(), e.passed);
        prop_assert_eq!(v["gate"]["counts"]["failed"].as_u64().unwrap(), e.failed);
        prop_assert_eq!(v["gate"]["counts"]["notExecuted"].as_u64().unwrap(), e.not_executed);
        prop_assert_eq!(v["gate"]["counts"]["skipped"].as_u64().unwrap(), e.skipped);
        prop_assert_eq!(v["vvRun"]["totalTestCases"].as_u64().unwrap(), e.total);
        prop_assert_eq!(v["gate"]["denominator"].as_u64().unwrap(), den);
        match e.passed.checked_mul(100).and_then(|n| n.checked_div(den)) {
            None => {
                prop_assert_eq!(&v["gate"]["verdict"], "NO_DATA");
                prop_assert!(v["gate"]["passRate"].is_null());
            }
            Some(display) => {
                prop_assert_eq!(v["gate"]["passRate"]["display"].as_u64().unwrap(), display);
                let passed = e.passed * 100 >= 90 * den;
                prop_assert_eq!(&v["gate"]["verdict"], if passed { "PASSED" } else { "FAILED" });
            }
        }
    }

    /// The order of the result lines in test-run.yaml does not change the figures.
    #[test]
    fn result_order_does_not_change_the_figures(run in run_spec()) {
        prop_assume!(!run.results.is_empty());
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        write_runs(a.path(), std::slice::from_ref(&run));
        let mut reversed = run.clone();
        reversed.results.reverse();
        write_runs(b.path(), std::slice::from_ref(&reversed));
        let (va, vb) = (json(&tcms(a.path(), &[])), json(&tcms(b.path(), &[])));
        prop_assert_eq!(&va["gate"], &vb["gate"]);
        prop_assert_eq!(&va["vvRun"], &vb["vvRun"]);
    }

    /// A skipped result for a case not yet in the run is outside the gate denominator.
    #[test]
    fn an_added_skipped_result_leaves_the_gate_unchanged(run in run_spec()) {
        prop_assume!(!run.results.is_empty());
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        write_runs(a.path(), std::slice::from_ref(&run));
        let mut more = run.clone();
        more.results.push((9, "skipped"));
        write_runs(b.path(), std::slice::from_ref(&more));
        let (va, vb) = (json(&tcms(a.path(), &[])), json(&tcms(b.path(), &[])));
        prop_assert_eq!(&va["gate"]["passRate"], &vb["gate"]["passRate"]);
        prop_assert_eq!(&va["gate"]["verdict"], &vb["gate"]["verdict"]);
        prop_assert_eq!(vb["gate"]["counts"]["skipped"].as_u64().unwrap(), va["gate"]["counts"]["skipped"].as_u64().unwrap() + 1);
    }

    /// Raising the gate never turns a FAILED verdict into PASSED.
    #[test]
    fn a_higher_gate_never_makes_a_verdict_easier(run in run_spec(), low in 50u64..=100, delta in 0u64..=20) {
        prop_assume!(!run.results.is_empty());
        let high = (low + delta).min(100);
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), std::slice::from_ref(&run));
        let at_low = json(&tcms(tmp.path(), &["--gate", &low.to_string()]));
        let at_high = json(&tcms(tmp.path(), &["--gate", &high.to_string()]));
        if at_high["gate"]["verdict"] == "PASSED" {
            prop_assert_eq!(&at_low["gate"]["verdict"], "PASSED");
        }
    }

    /// The printed delta equals the difference of the two printed rates.
    #[test]
    fn delta_is_the_difference_of_the_printed_rates(a in run_spec(), b in run_spec()) {
        prop_assume!(!a.results.is_empty() && !b.results.is_empty());
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), &[a.clone(), b.clone()]);
        let v = json(&tcms(tmp.path(), &[]));
        // Ordered by (started_at, folder name): run 0 is RUN-00, run 1 is RUN-01.
        let (first, second) = if (a.day, 0) <= (b.day, 1) { (&a, &b) } else { (&b, &a) };
        let (pa, pb) = (expect_counts(&first.results), expect_counts(&second.results));
        let (da, db) = (pa.passed + pa.failed + pa.not_executed, pb.passed + pb.failed + pb.not_executed);
        if da > 0 && db > 0 {
            let delta = (pb.passed * 100 / db) as i64 - (pa.passed * 100 / da) as i64;
            prop_assert_eq!(v["gate"]["deltaFromPrevious"].as_i64().unwrap(), delta);
        } else {
            prop_assert!(v["gate"]["deltaFromPrevious"].is_null());
        }
    }

    /// failingCases is sorted by caseId and contains exactly the FAILED cases of the selected run.
    #[test]
    fn failing_cases_are_sorted_and_complete(runs in prop::collection::vec(run_spec(), 1..4)) {
        prop_assume!(runs.iter().all(|r| !r.results.is_empty()));
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), &runs);
        let v = json(&tcms(tmp.path(), &[]));
        let ids: Vec<String> = v["failingCases"].as_array().unwrap()
            .iter().map(|c| c["caseId"].as_str().unwrap().to_string()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        prop_assert_eq!(&ids, &sorted);
        let want: std::collections::BTreeSet<String> = runs[selected_index(&runs)].results.iter()
            .filter(|(_, v)| matches!(*v, "fail" | "error" | "blocked"))
            .map(|(c, _)| format!("TC_{c}")).collect();
        let got: std::collections::BTreeSet<String> = ids.into_iter().collect();
        prop_assert_eq!(got, want);
    }

    /// Random bytes as test-run.yaml never crash the command (no panic, no signal).
    #[test]
    fn random_test_run_bytes_never_crash(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("runs/RUN-X")).unwrap();
        fs::write(tmp.path().join("runs/RUN-X/test-run.yaml"), &bytes).unwrap();
        let o = tcms(tmp.path(), &[]);
        prop_assert!(o.status.code().is_some(), "killed by a signal");
        prop_assert!(!String::from_utf8_lossy(&o.stderr).contains("panicked"));
    }

    /// Random bytes as verification-results.yaml never crash the command.
    #[test]
    fn random_verification_results_bytes_never_crash(bytes in prop::collection::vec(any::<u8>(), 0..400)) {
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), &[RunSpec { day: 1, results: vec![(0, "pass")] }]);
        fs::write(tmp.path().join("runs/RUN-00/verification-results.yaml"), &bytes).unwrap();
        let o = tcms(tmp.path(), &[]);
        prop_assert!(o.status.code().is_some());
        prop_assert!(!String::from_utf8_lossy(&o.stderr).contains("panicked"));
    }

    /// Any gate value outside 50..=100 is refused with exit 1 and nothing on stdout.
    #[test]
    fn out_of_range_gates_are_refused(g in prop_oneof![0u64..50, 101u64..1000]) {
        let tmp = TempDir::new().unwrap();
        write_runs(tmp.path(), &[RunSpec { day: 1, results: vec![(0, "pass")] }]);
        let o = tcms(tmp.path(), &["--gate", &g.to_string()]);
        prop_assert_eq!(o.status.code(), Some(1));
        prop_assert!(o.stdout.is_empty());
    }
}
