//! `tcms vv history`, `tcms vv trends`, `failingCases` and the run-folder contract of `--run`
//! (TCMS-50.4, TCMS-50.5, TCMS-50.6), and the not_executed bucket of TCMS-50.2, on the shared V&V
//! fixtures (crates/tcms/tests/fixtures/vv-metrics).

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

fn tcms(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv"])
        .args(args)
        .output()
        .unwrap()
}

fn json_ok(args: &[&str]) -> Value {
    let o = tcms(args);
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

fn metrics(root: &Path, extra: &[&str]) -> Value {
    let root = root.to_string_lossy().into_owned();
    let mut args = vec!["metrics", "--root", &root];
    args.extend_from_slice(extra);
    json_ok(&args)
}

fn derived(name: &str) -> PathBuf {
    fixtures().join("derived").join(name)
}

fn sut1() -> PathBuf {
    fixtures().join("sut-1")
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

fn temp_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tcms-vv-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

#[test]
fn not_executed_is_in_the_denominator_as_not_passed_and_never_failed() {
    let v = metrics(&derived("not-executed-run"), &[]);
    assert_eq!(v["gate"]["verdict"], "FAILED");
    assert_eq!(v["gate"]["denominator"], 1);
    assert_eq!(v["gate"]["passRate"]["display"], 0);
    assert_eq!(v["gate"]["counts"]["notExecuted"], 1);
    assert_eq!(v["gate"]["counts"]["failed"], 0);
    assert_eq!(v["gate"]["counts"]["skipped"], 1);
    // The previous run passed 100 percent, so the delta is -100 (TCMS-50.3).
    assert_eq!(v["gate"]["deltaFromPrevious"], -100);
}

#[test]
fn failing_cases_name_the_run_where_the_failure_started() {
    let v = metrics(&derived("failing-run"), &[]);
    let run_id = v["run"]["runId"].as_str().unwrap().to_string();
    let failing = v["failingCases"].as_array().unwrap();
    assert!(!failing.is_empty(), "failing-run has a failed case");
    // SUT1_PASS_001 passed in the previous run, so its failure starts in the selected run.
    let entry = failing
        .iter()
        .find(|c| c["caseId"] == "SUT1_PASS_001")
        .expect("SUT1_PASS_001 is listed");
    assert_eq!(entry["failureStartRunId"], run_id);
}

#[test]
fn failing_cases_are_empty_when_nothing_failed() {
    let v = metrics(&sut1(), &[]);
    assert_eq!(v["failingCases"], serde_json::json!([]));
}

#[test]
fn history_lists_one_row_per_run_oldest_first() {
    let root = sut1().to_string_lossy().into_owned();
    let rows = json_ok(&["history", "--root", &root, "--case", "SUT1_PASS_001"]);
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 14);
    assert!(rows.iter().all(|r| r["status"] == "PASSED"));
    let v = metrics(&sut1(), &[]);
    assert_eq!(rows.last().unwrap()["runId"], v["run"]["runId"]);
}

#[test]
fn history_of_an_unknown_case_is_absent_in_every_run() {
    let root = sut1().to_string_lossy().into_owned();
    let rows = json_ok(&["history", "--root", &root, "--case", "NO_SUCH_CASE"]);
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 14);
    assert!(rows.iter().all(|r| r["status"].is_null()));
}

#[test]
fn trends_show_the_last_runs_and_agree_with_metrics() {
    let root = sut1().to_string_lossy().into_owned();
    let t = json_ok(&["trends", "--root", &root, "--runs", "3"]);
    let runs = t["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 3);
    assert!(runs.iter().all(|r| r["passRate"]["display"] == 100));
    let v = metrics(&sut1(), &[]);
    assert_eq!(
        runs.last().unwrap()["runId"],
        v["run"]["runId"],
        "the newest trend row is the selected run"
    );
    assert_eq!(
        runs.last().unwrap()["passRate"]["display"],
        v["gate"]["passRate"]["display"]
    );
    let cases = t["heatmap"]["cases"].as_array().unwrap();
    assert!(cases.iter().all(|c| c["stability"] == "STABLE"));
}

#[test]
fn trends_show_a_null_figure_for_a_run_without_data() {
    // no-data-run has only skipped results: NO_DATA, so the figure is null.
    let root = derived("no-data-run").to_string_lossy().into_owned();
    let t = json_ok(&["trends", "--root", &root, "--runs", "1"]);
    let runs = t["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert!(runs[0]["passRate"].is_null());
    assert_eq!(runs[0]["verdict"], "NO_DATA");
}

#[test]
fn run_is_addressed_by_its_folder_name_not_by_the_run_id_inside() {
    let root = temp_root("folder-name");
    copy_dir(&derived("failing-run"), &root);
    let runs_dir = root.join("runs");
    // The selected run (the newest) is the folder the metrics report names.
    let folder = metrics(&root, &[])["run"]["runId"]
        .as_str()
        .unwrap()
        .to_string();
    // Give the run a run_id inside its record that differs from the folder name.
    let record = runs_dir.join(&folder).join("test-run.yaml");
    let text = fs::read_to_string(&record).unwrap();
    let edited: String = text
        .lines()
        .map(|l| {
            if l.starts_with("run_id:") {
                "run_id: RUN-INSIDE-DIFFERENT".to_string()
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&record, edited + "\n").unwrap();

    let root_s = root.to_string_lossy().into_owned();
    let v = json_ok(&["metrics", "--root", &root_s, "--run", &folder]);
    assert_eq!(v["run"]["runId"], folder.as_str());

    let o = tcms(&[
        "metrics",
        "--root",
        &root_s,
        "--run",
        "RUN-INSIDE-DIFFERENT",
    ]);
    assert!(
        !o.status.success(),
        "the run_id inside the record is not a valid --run"
    );
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("valid run ids"), "stderr: {err}");
    assert!(
        err.contains(&folder),
        "the valid folder names are listed: {err}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Heatmap label of one case in a `trends` report.
fn heat_label(t: &Value, case: &str) -> (String, u64) {
    let c = t["heatmap"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["caseId"] == case)
        .unwrap_or_else(|| panic!("{case} is in the heatmap"));
    (
        c["stability"].as_str().unwrap().to_string(),
        c["flips"].as_u64().unwrap(),
    )
}

/// Set the verdict of `case` in the run folder `run` under `root`.
fn set_verdict(root: &Path, run: &str, case: &str, verdict: &str) {
    let path = root.join("runs").join(run).join("test-run.yaml");
    let text = fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let i = lines
        .iter()
        .position(|l| l.trim().trim_start_matches("- ") == format!("test_case_id: {case}"))
        .unwrap_or_else(|| panic!("{case} in {run}"));
    let j = (i + 1..lines.len())
        .find(|&j| lines[j].trim_start().starts_with("verdict:"))
        .unwrap();
    let indent: String = lines[j].chars().take_while(|c| c.is_whitespace()).collect();
    lines[j] = format!("{indent}verdict: {verdict}");
    fs::write(&path, lines.join("\n") + "\n").unwrap();
}

#[test]
fn heatmap_labels_failing_on_the_derived_failing_run() {
    let root = derived("failing-run").to_string_lossy().into_owned();
    let t = json_ok(&["trends", "--root", &root]);
    assert_eq!(heat_label(&t, "SUT1_PASS_001"), ("FAILING".into(), 1));
}

#[test]
fn heatmap_flaky_and_failing_labels_follow_the_flaky_threshold() {
    // Three flips for SUT1_PASS_001 in the newest five runs (inside the window of 12): the newest
    // kept result is FAILED, so the label is FAILING at the default threshold of 4, and FLAKY at 3.
    let root = temp_root("flaky-labels");
    copy_dir(&sut1(), &root);
    let root_s = root.to_string_lossy().into_owned();
    let history = json_ok(&["history", "--root", &root_s, "--case", "SUT1_PASS_001"]);
    let rows = history.as_array().unwrap();
    let newest_five: Vec<String> = rows[rows.len() - 5..]
        .iter()
        .map(|r| r["runId"].as_str().unwrap().to_string())
        .collect();
    for (run, verdict) in newest_five
        .iter()
        .zip(["pass", "pass", "fail", "pass", "fail"])
    {
        set_verdict(&root, run, "SUT1_PASS_001", verdict);
    }

    let default = json_ok(&["trends", "--root", &root_s]);
    assert_eq!(heat_label(&default, "SUT1_PASS_001"), ("FAILING".into(), 3));

    let flaky = json_ok(&["trends", "--root", &root_s, "--flaky-threshold", "3"]);
    assert_eq!(heat_label(&flaky, "SUT1_PASS_001"), ("FLAKY".into(), 3));

    // The metrics report of the same run agrees with the heatmap (one rule, two outputs).
    let m = metrics(&root, &[]);
    let c = m["stability"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["caseId"] == "SUT1_PASS_001")
        .unwrap();
    assert_eq!(
        (c["stability"].as_str(), c["flips"].as_u64()),
        (Some("FAILING"), Some(3))
    );
    let _ = fs::remove_dir_all(&root);
}
