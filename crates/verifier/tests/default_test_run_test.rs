//! TCMS-40.4: the verifier writes the test run record by default, when the run, build,
//! environment and tester are known.

use std::path::Path;
use std::process::{Command, Output};

const TEST_CASE: &str = "\
type: test_case
schema: tcms/test-case.schema.v1.json
requirement: R
item: 1
tc: 1
id: TC_DEFAULT_RUN_001
description: default test run record
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

/// Run the verifier on one test case. `args` come after the common ones.
fn verify(dir: &Path, args: &[&str]) -> Output {
    std::fs::create_dir_all(dir.join("cases")).unwrap();
    std::fs::write(dir.join("cases/TC_DEFAULT_RUN_001.yaml"), TEST_CASE).unwrap();
    std::fs::write(dir.join("TC_DEFAULT_RUN_001_execution_log.json"), LOG).unwrap();
    Command::new(env!("CARGO_BIN_EXE_verifier"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .env_remove("TCMS_RUN_ID")
        .env_remove("TCMS_BUILD_ID")
        .env_remove("TCMS_TESTER")
        .env_remove("TCMS_REVIEWER")
        .args(["--test-case", "TC_DEFAULT_RUN_001", "--log"])
        .arg(dir.join("TC_DEFAULT_RUN_001_execution_log.json"))
        .arg("--test-case-dir")
        .arg(dir.join("cases"))
        .arg("--output")
        .arg(dir.join("results.yaml"))
        .args(args)
        .output()
        .unwrap()
}

const FULL_METADATA: [&str; 8] = [
    "--run-id",
    "RUN-1",
    "--build-id",
    "b1",
    "--environment",
    "Staging",
    "--tester",
    "alice",
];

#[test]
fn the_record_is_written_next_to_the_report_when_the_run_metadata_is_known() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &FULL_METADATA);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let record = std::fs::read_to_string(dir.path().join("results.test-run.yaml"))
        .expect("results.test-run.yaml next to results.yaml");
    assert!(
        record.contains("type: test_run") && record.contains("run_id: RUN-1"),
        "{record}"
    );
    assert!(record.contains("verdict: pass"), "{record}");
    assert!(
        record.contains("results.yaml#TC_DEFAULT_RUN_001"),
        "{record}"
    );
}

#[test]
fn a_run_without_the_metadata_has_no_record_and_says_why_but_still_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &["--run-id", "RUN-1"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("results.yaml").exists());
    assert!(!dir.path().join("results.test-run.yaml").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No test run record written")
            && stderr.contains("build_id, environment, tester"),
        "{stderr}"
    );
}

#[test]
fn no_test_run_turns_the_record_off() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = FULL_METADATA.to_vec();
    args.push("--no-test-run");
    let output = verify(dir.path(), &args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!dir.path().join("results.test-run.yaml").exists());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("No test run record"));
}

#[test]
fn asking_for_the_record_makes_missing_metadata_an_error() {
    // --test-run, --reviewer and --problem-report all ask for it
    for extra in [
        vec!["--test-run", "explicit.yaml"],
        vec!["--reviewer", "bob"],
        vec!["--problem-report", "TC_DEFAULT_RUN_001=PR-1"],
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut args = vec!["--run-id", "RUN-1"];
        let explicit;
        if extra[0] == "--test-run" {
            explicit = dir.path().join("explicit.yaml").display().to_string();
            args.extend(["--test-run", explicit.as_str()]);
        } else {
            args.extend(extra.iter().copied());
        }
        let output = verify(dir.path(), &args);
        assert!(!output.status.success(), "{extra:?} should fail");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("a test run needs"),
            "{extra:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_run_and_no_test_run_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let output = verify(dir.path(), &["--test-run", "x.yaml", "--no-test-run"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot be used with"));
}

#[test]
fn the_default_record_still_enforces_independence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("cases")).unwrap();
    let strict = TEST_CASE.replace(
        "description: default test run record",
        "description: needs a reviewer\nauthor: alice\nindependence_required: true",
    );
    std::fs::write(dir.path().join("cases/TC_DEFAULT_RUN_001.yaml"), strict).unwrap();
    std::fs::write(
        dir.path().join("TC_DEFAULT_RUN_001_execution_log.json"),
        LOG,
    )
    .unwrap();

    // complete metadata, no reviewer: the default record is refused for that test case
    let output = Command::new(env!("CARGO_BIN_EXE_verifier"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--test-case", "TC_DEFAULT_RUN_001", "--log"])
        .arg(dir.path().join("TC_DEFAULT_RUN_001_execution_log.json"))
        .arg("--test-case-dir")
        .arg(dir.path().join("cases"))
        .arg("--output")
        .arg(dir.path().join("results.yaml"))
        .args(FULL_METADATA)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("independence check failed"));
    assert!(!dir.path().join("results.test-run.yaml").exists());
}
