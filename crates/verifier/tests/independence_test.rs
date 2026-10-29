//! TCMS-40.8: `verifier --test-run` refuses a run whose reviewer is missing or is the author of a
//! test case that requires independence.

use std::path::Path;
use std::process::{Command, Output};

const TEST_CASE: &str = "\
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: R
item: 1
tc: 1
id: TC_INDEP_001
description: needs an independent reviewer
author: alice
independence_required: AUTHOR_REQUIRED
general_initial_conditions: {}
initial_conditions: {}
test_sequences:
- id: 1
  name: seq
  description: d
  initial_conditions: {}
  steps:
  - step: 1
    description: echo
    command: echo hi
    expected:
      success: true
      result: '0'
      output: hi
    verification:
      result: '[[ $? -eq 0 ]]'
      output: grep -q hi <<< \"$COMMAND_OUTPUT\"
";

const LOG: &str = r#"[
  {"test_sequence": 1, "step": 1, "command": "echo hi", "exit_code": 0, "output": "hi",
   "timestamp": "2026-10-28T10:00:00", "result_verification_pass": true,
   "output_verification_pass": true}
]"#;

fn verify(dir: &Path, test_case: &str, extra: &[&str]) -> Output {
    std::fs::create_dir_all(dir.join("cases")).unwrap();
    std::fs::write(dir.join("cases/TC_INDEP_001.yaml"), test_case).unwrap();
    std::fs::write(dir.join("TC_INDEP_001_execution_log.json"), LOG).unwrap();
    Command::new(env!("CARGO_BIN_EXE_verifier"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env_remove("TCMS_RUN_ID")
        .args(["--test-case", "TC_INDEP_001", "--log"])
        .arg(dir.join("TC_INDEP_001_execution_log.json"))
        .arg("--test-case-dir")
        .arg(dir.join("cases"))
        .args([
            "--run-id",
            "RUN-1",
            "--build-id",
            "b1",
            "--environment",
            "Staging",
            "--tester",
            "alice",
            "--output",
        ])
        .arg(dir.join("results.yaml"))
        .arg("--test-run")
        .arg(dir.join("run.yaml"))
        .args(extra)
        .output()
        .unwrap()
}

fn case(independence_required: bool) -> String {
    TEST_CASE.replace("AUTHOR_REQUIRED", &independence_required.to_string())
}

#[test]
fn a_reviewer_who_is_the_author_is_refused_and_no_record_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &case(true), &["--reviewer", "Alice"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("independence check failed")
            && stderr.contains("TC_INDEP_001")
            && stderr.contains("is its author"),
        "{stderr}"
    );
    assert!(!dir.path().join("run.yaml").exists());
}

#[test]
fn a_missing_reviewer_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &case(true), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("has no reviewer"));
    assert!(!dir.path().join("run.yaml").exists());
}

#[test]
fn an_independent_reviewer_is_accepted_and_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &case(true), &["--reviewer", "bob"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let run = std::fs::read_to_string(dir.path().join("run.yaml")).unwrap();
    assert!(
        run.contains("reviewer: bob") && run.contains("verdict: pass"),
        "{run}"
    );
}

#[test]
fn test_cases_without_the_requirement_are_unaffected() {
    let dir = tempfile::tempdir().unwrap();
    // no reviewer at all, and the tester is the author
    let output = verify(dir.path(), &case(false), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("run.yaml").exists());
}
