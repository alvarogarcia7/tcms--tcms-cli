//! `tcms vv metrics` reading `<DATA_ROOT>/vv-config.yaml` and `--stability-window` (TCMS-47.2).
//! Fixtures: crates/tcms/tests/fixtures/vv-metrics/config/README.md.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const NOW: &str = "2026-01-01T00:00:00Z";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vv-metrics")
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

/// A temp copy of sut-1, with the config fixture `name` copied in as vv-config.yaml when given.
fn data_root(config: Option<&str>) -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    copy_dir(&fixtures().join("sut-1"), tmp.path());
    if let Some(name) = config {
        fs::copy(
            fixtures().join("config").join(name).join("vv-config.yaml"),
            tmp.path().join("vv-config.yaml"),
        )
        .unwrap();
    }
    tmp
}

fn metrics(root: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(root)
        .args(["--now", NOW])
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

fn failure(root: &Path, extra: &[&str]) -> String {
    let o = metrics(root, extra);
    assert!(!o.status.success(), "expected failure");
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn param(v: &Value, key: &str) -> (Value, Value) {
    let p = &v["parameters"][key];
    (p["value"].clone(), p["source"].clone())
}

#[test]
fn file_value_used_when_no_flag() {
    let root = data_root(Some("valid"));
    let v = json(root.path(), &[]);
    assert_eq!(v["gate"]["gateThresholdPercent"], 95);
    assert_eq!(v["stability"]["flakyThreshold"], 3);
    assert_eq!(
        param(&v, "gateThresholdPercent"),
        (95.into(), "file".into())
    );
    assert_eq!(param(&v, "flakyThreshold"), (3.into(), "file".into()));
}

#[test]
fn flag_overrides_file() {
    let root = data_root(Some("valid"));
    let v = json(root.path(), &["--gate", "80", "--flaky-threshold", "6"]);
    assert_eq!(v["gate"]["gateThresholdPercent"], 80);
    assert_eq!(v["stability"]["flakyThreshold"], 6);
    assert_eq!(
        param(&v, "gateThresholdPercent"),
        (80.into(), "flag".into())
    );
    assert_eq!(param(&v, "flakyThreshold"), (6.into(), "flag".into()));
}

#[test]
fn default_when_no_file() {
    let root = data_root(None);
    let v = json(root.path(), &[]);
    assert_eq!(
        param(&v, "gateThresholdPercent"),
        (90.into(), "default".into())
    );
    assert_eq!(param(&v, "flakyThreshold"), (4.into(), "default".into()));
    assert_eq!(
        param(&v, "stability_window_runs"),
        (12.into(), "default".into())
    );
    let golden: Value = serde_json::from_slice(
        &fs::read(fixtures().join("config/sut-1-default-output.json")).unwrap(),
    )
    .unwrap();
    let (got, want) = (v.as_object().unwrap(), golden.as_object().unwrap());
    for (k, expected) in want {
        assert_eq!(got.get(k), Some(expected), "top-level key {k} differs");
    }
    for k in got.keys() {
        assert!(
            k == "parameters" || want.contains_key(k),
            "unexpected key {k}"
        );
    }
}

/// sut-1 has 14 runs. The earliest run gets a failing SUT1_PASS_001, so one flip sits between the
/// first and the second run: outside a window of 12, inside a window of 14.
fn root_with_early_flip() -> TempDir {
    let root = data_root(None);
    let mut earliest: Option<(String, PathBuf)> = None;
    for e in fs::read_dir(root.path().join("runs")).unwrap() {
        let file = e.unwrap().path().join("test-run.yaml");
        let text = fs::read_to_string(&file).unwrap();
        let started = text
            .lines()
            .find_map(|l| l.strip_prefix("started_at: "))
            .unwrap()
            .to_string();
        if earliest.as_ref().is_none_or(|(s, _)| started < *s) {
            earliest = Some((started, file));
        }
    }
    let file = earliest.unwrap().1;
    let text = fs::read_to_string(&file).unwrap();
    let from = "- test_case_id: SUT1_PASS_001\n  verdict: pass";
    assert!(text.contains(from), "unexpected run record layout");
    fs::write(
        &file,
        text.replace(from, "- test_case_id: SUT1_PASS_001\n  verdict: fail"),
    )
    .unwrap();
    root
}

fn pass_case(v: &Value) -> Value {
    v["stability"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["caseId"] == "SUT1_PASS_001")
        .unwrap()
        .clone()
}

#[test]
fn stability_window_limits_history() {
    let root = root_with_early_flip();
    let inside = json(
        root.path(),
        &["--flaky-threshold", "1", "--stability-window", "12"],
    );
    assert_eq!(pass_case(&inside)["stability"], "STABLE");
    assert_eq!(pass_case(&inside)["flips"], 0);
    assert_eq!(param(&inside, "stability_window_runs").0, 12);

    let all = json(
        root.path(),
        &["--flaky-threshold", "1", "--stability-window", "14"],
    );
    assert_ne!(pass_case(&all)["stability"], "STABLE");
    assert_eq!(pass_case(&all)["flips"], 1);
    assert_eq!(param(&all, "stability_window_runs").0, 14);
}

#[test]
fn stability_window_zero_is_refused() {
    let root = data_root(None);
    let err = failure(root.path(), &["--stability-window", "0"]);
    assert!(err.contains("--stability-window"), "{err}");
    assert!(err.contains("at least 1"), "{err}");
}

#[test]
fn wrong_type_names_file_and_key() {
    let root = data_root(Some("wrong-type"));
    let err = failure(root.path(), &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("gate_threshold_percent"), "{err}");
}

#[test]
fn unknown_key_names_file_and_key() {
    let root = data_root(Some("unknown-key"));
    let err = failure(root.path(), &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("stability_window"), "{err}");
}

#[test]
fn out_of_range_names_file_and_key() {
    let root = data_root(Some("out-of-range"));
    let err = failure(root.path(), &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("gate_threshold_percent"), "{err}");
}

#[test]
fn parameters_block_reports_window_source() {
    let root = data_root(None);
    let flagged = json(root.path(), &["--stability-window", "5"]);
    assert_eq!(
        param(&flagged, "stability_window_runs"),
        (5.into(), "flag".into())
    );
    let default = json(root.path(), &[]);
    assert_eq!(
        param(&default, "stability_window_runs").1,
        Value::from("default")
    );
}
