//! `tcms vv metrics --plan <id>` (TCMS-47): only the runs whose `plan_id` is that plan take part.
//!
//! The SUT-1 runs carry no `plan_id`. Each test copies the SUT-1 data to a temporary directory and
//! records a `plan_id` in the test-run.yaml of the runs it needs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn sut1() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vv-metrics/sut-1")
}

fn repo_schemas() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/")
        .to_string_lossy()
        .into_owned()
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

/// A copy of SUT-1 in which each listed run records the given `plan_id`.
fn data_with_plans(root: &Path, plans: &[(&str, &str)]) {
    copy_dir(&sut1(), root);
    for (run, plan) in plans {
        let file = root.join("runs").join(run).join("test-run.yaml");
        let text = fs::read_to_string(&file).unwrap();
        fs::write(&file, format!("plan_id: {plan}\n{text}")).unwrap();
    }
}

fn metrics(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(root)
        .args(["--now", "2026-11-02T10:00:00+04:00"])
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

#[test]
fn plan_counts_only_its_own_runs() {
    let d = tempfile::tempdir().unwrap();
    data_with_plans(
        d.path(),
        &[
            ("RUN-32921", "TP-1"),
            ("RUN-32389", "TP-1"),
            ("RUN-32466", "TP-2"),
        ],
    );
    let v = json(d.path(), &["--plan", "TP-1"]);
    assert_eq!(v["loadSummary"]["runsRecorded"], 2);
    let run = v["run"]["runId"].as_str().unwrap();
    assert!(run == "RUN-32921" || run == "RUN-32389", "run was {run}");
}

#[test]
fn plan_with_a_run_selects_that_run() {
    let d = tempfile::tempdir().unwrap();
    data_with_plans(d.path(), &[("RUN-32389", "TP-1"), ("RUN-32466", "TP-2")]);
    let v = json(d.path(), &["--plan", "TP-1", "--run", "RUN-32389"]);
    assert_eq!(v["run"]["runId"], "RUN-32389");
    assert_eq!(v["gate"]["verdict"], "PASSED");
}

#[test]
fn plan_without_any_run_is_refused() {
    let d = tempfile::tempdir().unwrap();
    data_with_plans(d.path(), &[("RUN-32921", "TP-1")]);
    let o = metrics(d.path(), &["--plan", "TP-9"]);
    assert!(!o.status.success());
    assert!(o.stdout.is_empty(), "nothing is written on refusal");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("no recorded run for plan TP-9"), "{err}");
}

#[test]
fn run_of_another_plan_is_refused() {
    let d = tempfile::tempdir().unwrap();
    data_with_plans(d.path(), &[("RUN-32921", "TP-1"), ("RUN-32466", "TP-2")]);
    let o = metrics(d.path(), &["--plan", "TP-1", "--run", "RUN-32466"]);
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("RUN-32466 is not a run of plan TP-1"), "{err}");
}

#[test]
fn without_plan_every_run_is_counted() {
    let d = tempfile::tempdir().unwrap();
    data_with_plans(d.path(), &[("RUN-32921", "TP-1")]);
    let v = json(d.path(), &[]);
    assert_eq!(v["loadSummary"]["runsRecorded"], 14);
}
