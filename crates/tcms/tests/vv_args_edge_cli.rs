//! Argument and clock handling of `tcms vv metrics` (`--now`, `TCMS_NOW`, unknown and malformed flags, paths).
//! The rule space of gates, deltas, stability and requirements is covered by the case folders
//! (fixtures/vv-cases) and vv_metrics_cli.rs; the tests here are the ones those do not reach.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn schemas() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas").to_string()
}

fn one_run(root: &Path, run: &str) {
    let dir = root.join("runs").join(run);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("test-run.yaml"),
        format!(
            "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run}\nbuild_id: b\n\
             environment: e\ntester: t\nstarted_at: \"2026-10-01T10:00:00Z\"\n\
             finished_at: \"2026-10-01T11:00:00Z\"\nresults:\n- test_case_id: TC_1\n  verdict: pass\n"
        ),
    )
    .unwrap();
}

fn metrics(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &schemas(), "vv", "metrics", "--root"])
        .arg(root)
        .args(extra)
        .output()
        .unwrap()
}

fn generated_at(o: &Output) -> String {
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    v["generatedAt"].as_str().unwrap().to_string()
}

#[test]
fn now_with_a_positive_offset_is_recorded_in_utc() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--now", "2026-01-01T02:00:00+02:00"]);
    assert!(o.status.success());
    assert_eq!(generated_at(&o), "2026-01-01T00:00:00Z");
}

#[test]
fn now_with_fractional_seconds_is_recorded_in_whole_seconds() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--now", "2026-01-01T00:00:00.999Z"]);
    assert!(o.status.success());
    assert_eq!(generated_at(&o), "2026-01-01T00:00:00Z");
}

#[test]
fn now_without_an_offset_is_refused_naming_the_value() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--now", "2026-01-01T00:00:00"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("must be RFC 3339"));
}

#[test]
fn tcms_now_is_used_when_the_flag_is_absent() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .env("TCMS_NOW", "2026-02-02T02:02:02Z")
        .args(["--schemas-root", &schemas(), "vv", "metrics", "--root"])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(generated_at(&o), "2026-02-02T02:02:02Z");
}

#[test]
fn the_now_flag_overrides_tcms_now() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .env("TCMS_NOW", "2026-02-02T02:02:02Z")
        .args(["--schemas-root", &schemas(), "vv", "metrics", "--root"])
        .arg(tmp.path())
        .args(["--now", "2026-03-03T03:03:03Z"])
        .output()
        .unwrap();
    assert_eq!(generated_at(&o), "2026-03-03T03:03:03Z");
}

#[test]
fn an_unknown_flag_is_a_usage_error_with_nothing_on_stdout() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--bogus"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).contains("--bogus"));
}

#[test]
fn a_gate_that_is_not_a_number_is_a_usage_error() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--gate", ""]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
}

#[test]
fn a_window_larger_than_u64_is_a_usage_error_not_a_panic() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--stability-window", "18446744073709551616"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(!String::from_utf8_lossy(&o.stderr).contains("panicked"));
}

#[test]
fn a_run_argument_with_a_trailing_slash_is_not_a_folder_name() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path(), "RUN-1");
    let o = metrics(tmp.path(), &["--run", "RUN-1/"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("RUN-1/"));
}

#[test]
fn a_root_path_with_spaces_and_unicode_is_read() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("données v&v run");
    one_run(&root, "RUN-U");
    let o = metrics(&root, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["run"]["runId"], "RUN-U");
}

#[test]
fn a_missing_root_is_refused_naming_the_path() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");
    let o = metrics(&missing, &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("does-not-exist"));
}

/// DEFECT D-11 (DISCREPANCES_TEST_CASES.md): docs/VV_METRICS.md says `--plan ""` selects nothing. The binary
/// selects the runs whose plan_id is the empty string (which `combine` writes when no plan is given).
/// Kept ignored until the defect is fixed.
#[test]
fn an_empty_plan_argument_selects_no_runs() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("runs/RUN-NOPLAN");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("test-run.yaml"),
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-NOPLAN\nbuild_id: b\n\
         environment: e\ntester: t\nstarted_at: \"2026-10-01T10:00:00Z\"\nplan_id: \"\"\n\
         results:\n- test_case_id: TC_1\n  verdict: pass\n",
    )
    .unwrap();
    let o = metrics(tmp.path(), &["--plan", ""]);
    assert_eq!(
        o.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&o.stdout)
    );
}
