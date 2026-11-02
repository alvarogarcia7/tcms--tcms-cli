//! `tcms vv metrics` on the shared V&V fixtures (TCMS-47). Expected figures are derived by hand from
//! D-95 and D-96 and were checked against the tcms-web reference code run with the JDK
//! (see crates/tcms/tests/fixtures/vv-metrics/derived/README.md).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vv-metrics")
}

fn repo_schemas() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/")
        .to_string_lossy()
        .into_owned()
}

fn metrics(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(root)
        .args(extra)
        .output()
        .unwrap()
}

fn json(root: &Path, extra: &[&str]) -> Value {
    let o = metrics(root, extra);
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let p = entry.unwrap().path();
        let target = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &target);
        } else {
            fs::copy(&p, &target).unwrap();
        }
    }
}

fn derived(name: &str) -> PathBuf {
    fixtures().join("derived").join(name)
}

fn coverage_of(v: &Value, id: &str) -> String {
    v["requirements"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("requirement {id} missing"))["coverage"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn sut1_last_run_passes_with_no_delta_and_100_percent() {
    let v = json(&fixtures().join("sut-1"), &[]);
    assert_eq!(v["run"]["runId"], "RUN-32921");
    assert_eq!(v["gate"]["verdict"], "PASSED");
    assert_eq!(v["gate"]["passRate"]["display"], 100);
    assert_eq!(v["gate"]["denominator"], 2);
    assert_eq!(v["gate"]["deltaFromPrevious"], 0);
    assert_eq!(v["gate"]["provisional"], false);
    assert_eq!(v["vvRun"]["totalSteps"], 6);
    assert_eq!(v["vvRun"]["stepCoverage"]["display"], 100);
    assert_eq!(v["requirements"]["summary"]["covered"], 2);
    assert_eq!(v["requirements"]["summary"]["total"], 2);
    assert_eq!(v["stability"]["flakyCount"], 0);
    assert_eq!(v["loadSummary"]["runsRecorded"], 14);
    assert_eq!(v["loadSummary"]["distinctCases"], 2);
    assert_eq!(v["loadSummary"]["schemaErrorCount"], 0);
}

#[test]
fn failing_run_is_failed_with_negative_delta_and_failing_requirement() {
    let v = json(&derived("failing-run"), &[]);
    assert_eq!(v["run"]["runId"], "RUN-DERIVED-FAIL-01");
    assert_eq!(v["gate"]["verdict"], "FAILED");
    assert_eq!(v["gate"]["passRate"]["display"], 50);
    assert_eq!(v["gate"]["passRate"]["exact"], 50.0);
    assert_eq!(v["gate"]["deltaFromPrevious"], -50);
    assert_eq!(v["gate"]["previousRunId"], "RUN-32921");
    // V&V view: 1 of 2 cases; steps 5 of 6 = 83.33, rounded half up to 83.
    assert_eq!(v["vvRun"]["vvPassRate"]["display"], 50);
    assert_eq!(v["vvRun"]["passedSteps"], 5);
    assert_eq!(v["vvRun"]["stepCoverage"]["display"], 83);
    assert_eq!(coverage_of(&v, "SUT1-2"), "FAILING");
    assert_eq!(coverage_of(&v, "SUT1-1"), "COVERED");
    assert_eq!(v["requirements"]["summary"]["failing"], 1);
    // One pass then one fail: a single flip, so FAILING, not FLAKY.
    let pass_case = v["stability"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["caseId"] == "SUT1_PASS_001")
        .unwrap();
    assert_eq!(pass_case["stability"], "FAILING");
    assert_eq!(pass_case["flips"], 1);
}

#[test]
fn gate_is_exactly_reached_at_the_threshold_and_missed_one_above() {
    // 1 of 2 is 50 percent: passed at gate 50 (exact), failed at gate 51.
    let at = json(&derived("failing-run"), &["--gate", "50"]);
    assert_eq!(at["gate"]["verdict"], "PASSED");
    assert_eq!(at["gate"]["gateThresholdPercent"], 50);
    let above = json(&derived("failing-run"), &["--gate", "51"]);
    assert_eq!(above["gate"]["verdict"], "FAILED");
}

#[test]
fn no_data_run_has_null_pass_rate_delta_and_step_coverage() {
    let v = json(&derived("no-data-run"), &[]);
    assert_eq!(v["gate"]["verdict"], "NO_DATA");
    assert!(v["gate"]["passRate"].is_null());
    assert!(v["gate"]["deltaFromPrevious"].is_null());
    assert_eq!(v["gate"]["denominator"], 0);
    assert_eq!(v["gate"]["counts"]["skipped"], 1);
    assert_eq!(v["gate"]["counts"]["untested"], 1);
    // Zero steps: no step coverage, not 0 percent.
    assert_eq!(v["vvRun"]["totalSteps"], 0);
    assert!(v["vvRun"]["stepCoverage"].is_null());
    assert_eq!(coverage_of(&v, "SUT1-2"), "NOT_RUN");
    assert_eq!(coverage_of(&v, "SUT1-1"), "NOT_RUN");
}

#[test]
fn blocked_only_run_is_failed_with_display_zero() {
    let v = json(&derived("blocked-only-run"), &[]);
    assert_eq!(v["gate"]["verdict"], "FAILED");
    assert_eq!(v["gate"]["passRate"]["display"], 0);
    assert_eq!(v["gate"]["denominator"], 2);
    assert_eq!(v["gate"]["counts"]["blocked"], 2);
    assert_eq!(v["gate"]["deltaFromPrevious"], -100);
    assert_eq!(v["vvRun"]["vvPassRate"]["display"], 0);
}

#[test]
fn skipped_result_is_outside_the_denominator_but_not_the_vv_figure() {
    let v = json(&derived("skipped-outside-denominator"), &[]);
    assert_eq!(v["gate"]["verdict"], "PASSED");
    assert_eq!(v["gate"]["denominator"], 1);
    assert_eq!(v["gate"]["counts"]["skipped"], 1);
    // The same run: gate uses 1 of 1, the V&V test-case figure uses 1 of 2.
    assert_eq!(v["gate"]["passRate"]["display"], 100);
    assert_eq!(v["vvRun"]["vvPassRate"]["display"], 50);
    assert_eq!(v["vvRun"]["totalTestCases"], 2);
    assert_eq!(coverage_of(&v, "SUT1-1"), "NOT_RUN");
    assert_eq!(coverage_of(&v, "SUT1-2"), "COVERED");
}

#[test]
fn selecting_an_earlier_run_compares_it_with_its_own_predecessor() {
    // The first recorded run has no predecessor, so no delta.
    let v = json(&fixtures().join("sut-1"), &["--run", "RUN-32377"]);
    assert_eq!(v["run"]["runId"], "RUN-32377");
    assert!(v["gate"]["previousRunId"].is_null());
    assert!(v["gate"]["deltaFromPrevious"].is_null());
}

#[test]
fn flaky_threshold_is_a_parameter() {
    // Pass, fail, pass, fail, pass: four flips. Flaky at the default threshold of 4, stable at 5.
    let dir = tempfile::tempdir().unwrap();
    let verdicts = ["pass", "fail", "pass", "fail", "pass"];
    for (i, verdict) in verdicts.iter().enumerate() {
        let run = dir.path().join(format!("runs/R{i}"));
        fs::create_dir_all(&run).unwrap();
        fs::write(
            run.join("test-run.yaml"),
            format!(
                "run_id: R{i}\nstarted_at: \"2026-01-0{}T00:00:00Z\"\nfinished_at: \"2026-01-0{}T00:00:00Z\"\nresults:\n- test_case_id: T1\n  verdict: {verdict}\n",
                i + 1,
                i + 1
            ),
        )
        .unwrap();
    }
    let flaky_case = |v: &Value| {
        v["stability"]["cases"][0]["stability"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let default = json(dir.path(), &[]);
    assert_eq!(flaky_case(&default), "FLAKY");
    assert_eq!(default["stability"]["cases"][0]["flips"], 4);
    assert_eq!(default["stability"]["flakyCount"], 1);
    let strict = json(dir.path(), &["--flaky-threshold", "5"]);
    assert_eq!(flaky_case(&strict), "STABLE");
    assert_eq!(strict["stability"]["flakyCount"], 0);
}

#[test]
fn schema_error_case_files_are_counted_and_listed() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&fixtures().join("sut-1"), dir.path());
    // A test case without the required fields: it must be reported, not counted as a case.
    fs::write(
        dir.path().join("BROKEN.yml"),
        "type: test_case\nschema: tcms/test-case.schema.v1.json\nrequirement: SUT1-1\n",
    )
    .unwrap();
    let v = json(dir.path(), &[]);
    assert_eq!(v["loadSummary"]["schemaErrorCount"], 1);
    assert_eq!(v["loadSummary"]["schemaErrors"][0]["file"], "BROKEN.yml");
    assert!(!v["loadSummary"]["schemaErrors"][0]["error"]
        .as_str()
        .unwrap()
        .is_empty());
    assert_eq!(v["loadSummary"]["distinctCases"], 2);
}

#[test]
fn gate_outside_50_to_100_is_refused() {
    let o = metrics(&fixtures().join("sut-1"), &["--gate", "49"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("--gate must be between 50 and 100"));
}

#[test]
fn unknown_run_id_is_refused() {
    let o = metrics(&fixtures().join("sut-1"), &["--run", "RUN-NOPE"]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("RUN-NOPE"));
}

#[test]
fn root_without_runs_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let o = metrics(dir.path(), &[]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("no recorded V&V runs"));
}

fn metrics_with_clock(root: &Path, clock: Option<&str>, extra: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    cmd.args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(root)
        .args(extra)
        .env_remove("TCMS_NOW");
    if let Some(c) = clock {
        cmd.env("TCMS_NOW", c);
    }
    cmd.output().unwrap()
}

#[test]
fn out_file_is_byte_identical_to_stdout_and_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = fixtures().join("sut-1");
    let file = dir.path().join("a/b/report.json");
    let o = metrics_with_clock(
        &root,
        Some("2026-11-02T10:00:00+04:00"),
        &["--out", file.to_str().unwrap()],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(o.stdout.is_empty(), "with --out nothing goes to stdout");
    let stdout = metrics_with_clock(&root, Some("2026-11-02T10:00:00+04:00"), &[]).stdout;
    assert_eq!(fs::read(&file).unwrap(), stdout);
    // No temporary file is left behind.
    let names: Vec<_> = fs::read_dir(file.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "{names:?}");
}

#[test]
fn generated_at_follows_the_fixed_clock_in_utc_whole_seconds() {
    let root = fixtures().join("sut-1");
    let v = serde_json::from_slice::<Value>(
        &metrics_with_clock(&root, Some("2026-11-02T10:00:00.987+04:00"), &[]).stdout,
    )
    .unwrap();
    assert_eq!(v["generatedAt"], "2026-11-02T06:00:00Z");
}

#[test]
fn repeated_runs_with_a_fixed_clock_are_byte_identical() {
    let root = fixtures().join("derived/failing-run");
    let first = metrics_with_clock(
        &root,
        Some("2026-11-02T06:00:00Z"),
        &["--run", "RUN-DERIVED-FAIL-01"],
    )
    .stdout;
    assert!(!first.is_empty());
    for _ in 0..5 {
        let again = metrics_with_clock(
            &root,
            Some("2026-11-02T06:00:00Z"),
            &["--run", "RUN-DERIVED-FAIL-01"],
        )
        .stdout;
        assert_eq!(first, again);
    }
}

#[test]
fn the_clock_changes_only_generated_at() {
    let root = fixtures().join("sut-1");
    let a: Value = serde_json::from_slice(
        &metrics_with_clock(&root, Some("2026-11-02T06:00:00Z"), &[]).stdout,
    )
    .unwrap();
    let b: Value = serde_json::from_slice(
        &metrics_with_clock(&root, Some("2031-01-15T23:59:59Z"), &[]).stdout,
    )
    .unwrap();
    assert_ne!(a["generatedAt"], b["generatedAt"]);
    let (mut a, mut b) = (a, b);
    a.as_object_mut().unwrap().remove("generatedAt");
    b.as_object_mut().unwrap().remove("generatedAt");
    assert_eq!(a, b);
}

#[test]
fn without_a_clock_generated_at_is_a_current_utc_timestamp() {
    let root = fixtures().join("sut-1");
    let v: Value = serde_json::from_slice(&metrics_with_clock(&root, None, &[]).stdout).unwrap();
    let s = v["generatedAt"].as_str().unwrap();
    assert!(s.ends_with('Z') && s.len() == 20, "{s}");
    chrono_check(s);
}

fn chrono_check(s: &str) {
    chrono::DateTime::parse_from_rfc3339(s).expect("generatedAt is RFC 3339");
}

#[test]
fn an_invalid_clock_is_refused_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("r.json");
    let o = metrics_with_clock(
        &fixtures().join("sut-1"),
        Some("yesterday"),
        &["--out", file.to_str().unwrap()],
    );
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("RFC 3339"));
    assert!(!file.exists());
}

#[test]
fn now_flag_wins_over_environment() {
    let root = fixtures().join("sut-1");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    let o = cmd
        .args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(&root)
        .args(["--now", "2026-11-02T06:00:00Z"])
        .env("TCMS_NOW", "2000-01-01T00:00:00Z")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["generatedAt"], "2026-11-02T06:00:00Z");
}
