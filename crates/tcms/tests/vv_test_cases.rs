//! Meta tests for the `tcms vv metrics` verification cases in `testcases/tcms-vv/` (TCMS-47).
//!
//! For each case file: it must validate against its schema, and then, for every step:
//!   - the command run twice gives the same exit status and the same stdout (repeatability);
//!   - the first run exits 0;
//!   - the step's verification (result and output) passes on that output.
//!
//! The commands use `$TCMS_BIN` for the binary; it is set here to the `tcms` built for this test.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::Value;
use testcase_common::load_and_validate_yaml;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn case_path(id: &str) -> PathBuf {
    repo_root()
        .join("testcases/tcms-vv")
        .join(format!("{id}.yml"))
}

fn repo_schemas() -> String {
    repo_root().join("schemas/").to_string_lossy().into_owned()
}

/// Run `script` with bash from the repository root; `env` adds variables.
fn bash(script: &str, env: &[(&str, String)]) -> (Option<i32>, String) {
    let mut cmd = Command::new("bash");
    cmd.arg("-c")
        .arg(script)
        .current_dir(repo_root())
        .env("TCMS_BIN", env!("CARGO_BIN_EXE_tcms"));
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("bash must be available");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn text(v: &Value, path: &[&str]) -> Option<String> {
    let mut cur = v;
    for key in path {
        cur = cur.get(*key)?;
    }
    cur.as_str().map(str::to_string)
}

/// Every step as (step description, command, verification result, verification output).
fn steps(doc: &Value) -> Vec<(String, String, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    for seq in doc["test_sequences"].as_sequence().into_iter().flatten() {
        for step in seq["steps"].as_sequence().into_iter().flatten() {
            out.push((
                text(step, &["description"]).unwrap_or_default(),
                text(step, &["command"]).expect("step command"),
                text(step, &["verification", "result"]),
                text(step, &["verification", "output"]),
            ));
        }
    }
    out
}

/// Check one case; returns every problem found (empty when the case passes).
fn check_case(id: &str) -> Vec<String> {
    let path = case_path(id);
    let doc: Value = load_and_validate_yaml(&path, repo_schemas())
        .unwrap_or_else(|e| panic!("{} does not validate: {e:#}", path.display()));
    let mut problems = Vec::new();
    let steps = steps(&doc);
    assert!(!steps.is_empty(), "{id} has no steps");
    for (description, command, ver_result, ver_output) in steps {
        let first = bash(&command, &[]);
        let second = bash(&command, &[]);
        if first != second {
            problems.push(format!(
                "{id}: '{description}' is not repeatable: {first:?} then {second:?}"
            ));
            continue;
        }
        let (status, stdout) = first;
        if status != Some(0) {
            problems.push(format!("{id}: '{description}' exited {status:?}"));
            continue;
        }
        let env = [
            ("EXIT_CODE", status.unwrap_or(-1).to_string()),
            ("COMMAND_OUTPUT", stdout.trim_end().to_string()),
        ];
        for check in [ver_result, ver_output].into_iter().flatten() {
            let (vstatus, vout) = bash(&check, &env);
            if vstatus != Some(0) {
                problems.push(format!(
                    "{id}: '{description}' verification failed ({check}) on output {stdout:?}: {vout}"
                ));
            }
        }
    }
    problems
}

fn assert_case(id: &str) {
    let problems = check_case(id);
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn tc_tcms_vv_c1_requirement_covered_and_passing() {
    assert_case("TC_TCMS_VV_C1_001");
}

#[test]
fn tc_tcms_vv_c2_requirement_not_covered() {
    assert_case("TC_TCMS_VV_C2_001");
}

#[test]
fn tc_tcms_vv_c3_requirement_covered_but_failing() {
    assert_case("TC_TCMS_VV_C3_001");
}

#[test]
fn tc_tcms_vv_c5_test_case_missing_from_run() {
    assert_case("TC_TCMS_VV_C5_001");
}

#[test]
fn every_case_file_in_the_folder_has_a_meta_test() {
    // Keep this list in step with the case files: a new case needs a test above.
    let mut found: Vec<String> = fs::read_dir(repo_root().join("testcases/tcms-vv"))
        .expect("testcases/tcms-vv")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yml"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    found.sort();
    assert_eq!(
        found,
        vec![
            "TC_TCMS_VV_C1_001",
            "TC_TCMS_VV_C2_001",
            "TC_TCMS_VV_C3_001",
            "TC_TCMS_VV_C5_001",
        ],
        "case files under {} changed",
        Path::new("testcases/tcms-vv").display()
    );
}
