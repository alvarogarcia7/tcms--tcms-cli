//! `tcms vv metrics` reads `<DATA_ROOT>/vv-config.yaml`, takes `--stability-window`, and reports where
//! each parameter came from (TCMS-47.2). Every test runs on a fresh copy of the SUT-1 data set.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

const NOW: &str = "2026-01-01T00:00:00Z";

/// The five earliest runs of SUT-1 by `started_at`; the selected (last) run is RUN-32921.
const FIRST_FIVE_RUNS: [&str; 5] = [
    "RUN-32377",
    "RUN-32386",
    "RUN-32389",
    "RUN-32393",
    "RUN-32396",
];

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

/// A fresh copy of SUT-1 as a data root. The guard keeps the temporary directory alive.
fn sut1_root() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("data");
    copy_dir(&fixtures().join("sut-1"), &root);
    (tmp, root)
}

/// Copy `config/<case>/vv-config.yaml` to the data root.
fn use_config(root: &Path, case: &str) {
    fs::copy(
        fixtures().join("config").join(case).join("vv-config.yaml"),
        root.join("vv-config.yaml"),
    )
    .unwrap();
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

fn report(root: &Path, extra: &[&str]) -> Value {
    let o = metrics(root, extra);
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    serde_json::from_slice(&o.stdout).unwrap()
}

/// Stderr of a run that must fail.
fn refused(root: &Path, extra: &[&str]) -> String {
    let o = metrics(root, extra);
    assert!(
        !o.status.success(),
        "expected a non-zero exit, stdout: {}",
        String::from_utf8_lossy(&o.stdout)
    );
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn stability_of<'a>(v: &'a Value, case: &str) -> &'a Value {
    v["stability"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["caseId"] == case)
        .unwrap_or_else(|| panic!("case {case} missing from stability"))
}

/// Set the verdict of one case in one recorded run file, keeping the line's indentation.
fn set_verdict(run_file: &Path, case_id: &str, verdict: &str) {
    let text = fs::read_to_string(run_file).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let id_line = lines
        .iter()
        .position(|l| l.trim_start_matches("- ").trim() == format!("test_case_id: {case_id}"))
        .unwrap_or_else(|| panic!("{case_id} not in {}", run_file.display()));
    let verdict_line = (id_line + 1..lines.len())
        .find(|&i| lines[i].trim_start().starts_with("verdict:"))
        .unwrap();
    let indent: String = lines[verdict_line]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    lines[verdict_line] = format!("{indent}verdict: {verdict}");
    fs::write(run_file, lines.join("\n") + "\n").unwrap();
}

#[test]
fn file_value_used_when_no_flag() {
    let (_tmp, root) = sut1_root();
    use_config(&root, "valid");
    let v = report(&root, &[]);
    assert_eq!(v["gate"]["gateThresholdPercent"], 95);
    assert_eq!(v["stability"]["flakyThreshold"], 3);
    assert_eq!(
        v["parameters"]["gateThresholdPercent"],
        json!({"value": 95, "source": "file"})
    );
    assert_eq!(
        v["parameters"]["flakyThreshold"],
        json!({"value": 3, "source": "file"})
    );
    assert_eq!(
        v["parameters"]["stability_window_runs"],
        json!({"value": 12, "source": "default"})
    );
}

#[test]
fn flag_overrides_file() {
    let (_tmp, root) = sut1_root();
    use_config(&root, "valid");
    let v = report(&root, &["--gate", "80", "--flaky-threshold", "6"]);
    assert_eq!(v["gate"]["gateThresholdPercent"], 80);
    assert_eq!(v["stability"]["flakyThreshold"], 6);
    assert_eq!(
        v["parameters"]["gateThresholdPercent"],
        json!({"value": 80, "source": "flag"})
    );
    assert_eq!(
        v["parameters"]["flakyThreshold"],
        json!({"value": 6, "source": "flag"})
    );
}

#[test]
fn default_when_no_file_matches_previous_output() {
    let (_tmp, root) = sut1_root();
    let mut v = report(&root, &[]);
    let parameters = v.as_object_mut().unwrap().remove("parameters");
    let golden: Value = serde_json::from_str(
        &fs::read_to_string(fixtures().join("config/golden-sut1-default.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        v, golden,
        "every key except parameters must match the previous output"
    );
    let parameters = parameters.expect("parameters block");
    assert_eq!(
        parameters["gateThresholdPercent"],
        json!({"value": 90, "source": "default"})
    );
    assert_eq!(
        parameters["flakyThreshold"],
        json!({"value": 4, "source": "default"})
    );
    assert_eq!(
        parameters["stability_window_runs"],
        json!({"value": 12, "source": "default"})
    );
}

#[test]
fn stability_window_limits_the_history() {
    let (_tmp, root) = sut1_root();
    // SUT1_PASS_001 in the five earliest runs: pass, fail, pass, fail, pass; later runs pass.
    // Four flips over all fourteen runs, two flips over the last twelve.
    for (run, verdict) in FIRST_FIVE_RUNS
        .iter()
        .zip(["pass", "fail", "pass", "fail", "pass"])
    {
        set_verdict(
            &root.join("runs").join(run).join("test-run.yaml"),
            "SUT1_PASS_001",
            verdict,
        );
    }

    let default = report(&root, &[]);
    let c = stability_of(&default, "SUT1_PASS_001");
    assert_eq!(
        (c["stability"].as_str(), c["flips"].as_u64()),
        (Some("STABLE"), Some(2))
    );

    let w12 = report(&root, &["--stability-window", "12"]);
    let c = stability_of(&w12, "SUT1_PASS_001");
    assert_eq!(
        (c["stability"].as_str(), c["flips"].as_u64()),
        (Some("STABLE"), Some(2))
    );
    assert_eq!(w12["parameters"]["stability_window_runs"]["value"], 12);

    let w14 = report(&root, &["--stability-window", "14"]);
    let c = stability_of(&w14, "SUT1_PASS_001");
    assert_eq!(
        (c["stability"].as_str(), c["flips"].as_u64()),
        (Some("FLAKY"), Some(4))
    );
    assert_eq!(w14["stability"]["flakyCount"], 1);
    assert_eq!(w14["parameters"]["stability_window_runs"]["value"], 14);
}

#[test]
fn stability_window_zero_is_refused() {
    let (_tmp, root) = sut1_root();
    let err = refused(&root, &["--stability-window", "0"]);
    assert!(
        err.contains("--stability-window must be at least 1"),
        "{err}"
    );
}

#[test]
fn wrong_type_names_file_and_key() {
    let (_tmp, root) = sut1_root();
    use_config(&root, "wrong-type");
    let err = refused(&root, &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("gate_threshold_percent"), "{err}");
}

#[test]
fn unknown_key_names_file_and_key() {
    let (_tmp, root) = sut1_root();
    use_config(&root, "unknown-key");
    let err = refused(&root, &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("gate_threshold_pct"), "{err}");
}

#[test]
fn out_of_range_names_file_and_key() {
    let (_tmp, root) = sut1_root();
    use_config(&root, "out-of-range");
    let err = refused(&root, &[]);
    assert!(err.contains("vv-config.yaml"), "{err}");
    assert!(err.contains("gate_threshold_percent"), "{err}");
}

#[test]
fn parameters_block_reports_window_source() {
    let (_tmp, root) = sut1_root();
    let flagged = report(&root, &["--stability-window", "5"]);
    assert_eq!(
        flagged["parameters"]["stability_window_runs"],
        json!({"value": 5, "source": "flag"})
    );
    let default = report(&root, &[]);
    assert_eq!(
        default["parameters"]["stability_window_runs"],
        json!({"value": 12, "source": "default"})
    );
}

/// TCMS-47.2 AC 7: validation against `tcms/vv-config.schema.json` passes for a valid file and fails for an
/// invalid one. Status: BLOCKED. The schema is not in this repository (CONTEXT Q2), so no schema is invented
/// here. Remove the `ignore` once the schema is vendored under `schemas/`.
#[test]
#[ignore = "TCMS-47.2 AC 7 blocked: no vv-config schema in the repository (CONTEXT Q2)"]
fn vv_config_fixtures_validate_against_the_schema() {
    let valid = fs::read_to_string(fixtures().join("config/valid/vv-config.yaml")).unwrap();
    let wrong = fs::read_to_string(fixtures().join("config/wrong-type/vv-config.yaml")).unwrap();
    let ok: serde_yaml::Value = testcase_common::parse_and_validate_yaml_string(
        &valid,
        repo_schemas(),
        "valid vv-config.yaml",
    )
    .expect("the valid fixture passes the schema");
    assert!(ok.is_mapping());
    let bad: Result<serde_yaml::Value, _> = testcase_common::parse_and_validate_yaml_string(
        &wrong,
        repo_schemas(),
        "wrong-type vv-config.yaml",
    );
    assert!(bad.is_err(), "the wrong-type fixture must fail the schema");
}
