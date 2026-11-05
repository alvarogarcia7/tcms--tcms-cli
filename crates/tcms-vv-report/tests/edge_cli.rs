//! Error paths of `tcms-vv-report` (src/main.rs) driven through the binary. Production code is not changed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// A fresh directory under the system temp dir; removed by the caller.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tcms-vv-report-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn report(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tcms-vv-report"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn out_path_that_is_a_directory_is_an_error_not_a_panic() {
    let dir = scratch("out-is-dir");
    let metrics = dir.join("m.json");
    fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/sut-1.metrics.json"
        ),
        &metrics,
    )
    .unwrap();
    let target = dir.join("is-a-dir");
    fs::create_dir_all(&target).unwrap();
    let o = report(&[
        "--metrics",
        metrics.to_str().unwrap(),
        "--out",
        target.to_str().unwrap(),
    ]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(!err.contains("panicked"), "{err}");
    assert!(err.contains("writing"), "{err}");
}

#[test]
fn unreadable_metrics_path_names_the_file() {
    let dir = scratch("missing-metrics");
    let missing = dir.join("nope.json");
    let o = report(&["--metrics", missing.to_str().unwrap()]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(err.contains("nope.json"), "{err}");
}

#[test]
fn a_bare_out_file_name_is_written_in_the_current_directory() {
    let dir = scratch("bare-out");
    let metrics = dir.join("m.json");
    fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/sut-1.metrics.json"
        ),
        &metrics,
    )
    .unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_tcms-vv-report"))
        .current_dir(&dir)
        .args(["--metrics", "m.json", "--out", "report.md"])
        .output()
        .unwrap();
    let written = fs::read_to_string(dir.join("report.md"));
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(written.unwrap().contains("Requirement"));
}

#[test]
fn problem_report_ids_are_listed_in_the_requirement_table() {
    let dir = scratch("problem-reports");
    let mut v: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/sut-1.metrics.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let item = &mut v["requirements"]["items"][0];
    item["problemReports"] = serde_json::json!(["PR-101", "PR-202"]);
    item["openProblemReports"] = serde_json::json!(["PR-202"]);
    let metrics = dir.join("m.json");
    fs::write(&metrics, v.to_string()).unwrap();
    let o = report(&["--metrics", metrics.to_str().unwrap()]);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(text.contains("PR-101, PR-202"), "{text}");
    assert!(text.contains("PR-202"), "{text}");
}
