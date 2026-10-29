//! `test-executor coverage-import` (TCMS-40.10)

use std::path::Path;
use std::process::{Command, Output};

const LCOV: &str = "SF:src/es6.c\nDA:1,1\nDA:2,0\nDA:3,0\nDA:4,1\nBRDA:4,0,0,1\nBRDA:4,0,1,0\n\
                    end_of_record\n";

fn write(dir: &Path, name: &str, content: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path.display().to_string()
}

fn import(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-audit", "coverage-import", "--baseline-id", "BL-1"])
        .args(args)
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[test]
fn annotated_lcov_becomes_a_coverage_report() {
    let dir = tempfile::tempdir().unwrap();
    let lcov = write(dir.path(), "c.info", LCOV);
    let ann = write(
        dir.path(),
        "ann.yaml",
        "- {location: 'src/es6.c:2-3', classification: dead_code, rationale: 'Unreachable after refactoring', problem_report_id: PR-1}\n\
         - {location: 'src/es6.c:4', coverage_type: decision, classification: justified, rationale: 'Defensive branch', approver: qa.lead}\n",
    );
    let out = import(&[
        "--lcov",
        &lcov,
        "--annotations",
        &ann,
        "--id",
        "COV-T",
        "--tool",
        "gcov",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let report: serde_yaml::Value =
        serde_yaml::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap();
    assert_eq!(report["type"], "coverage_report");
    assert_eq!(report["structural"]["statement_percent"], 50.0);
    assert_eq!(report["structural"]["decision_percent"], 50.0);
    assert_eq!(report["gaps"][0]["classification"], "dead_code");
    assert_eq!(report["gaps"][0]["problem_report_id"], "PR-1");
    assert_eq!(report["gaps"][1]["approver"], "qa.lead");
}

#[test]
fn unclassified_gaps_are_reported_and_fail_unless_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let lcov = write(dir.path(), "c.info", LCOV);
    let out = import(&["--lcov", &lcov]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("2 unclassified gap(s)"), "{err}");
    assert!(err.contains("src/es6.c:2-3 (statement)"), "{err}");
    assert!(err.contains("src/es6.c:4 (decision)"), "{err}");
    // the report is still written
    assert!(String::from_utf8_lossy(&out.stdout).contains("statement_percent"));

    let out = import(&["--lcov", &lcov, "--allow-unclassified"]);
    assert!(out.status.success());
}

#[test]
fn bad_inputs_give_clear_errors() {
    let dir = tempfile::tempdir().unwrap();
    let out = import(&[]);
    assert!(stderr(&out).contains("Nothing to import"));

    let out = import(&["--lcov", "/nonexistent.info"]);
    assert!(stderr(&out).contains("Cannot read LCOV file"));

    let lcov = write(dir.path(), "c.info", LCOV);
    let ann = write(
        dir.path(),
        "ann.yaml",
        "- {location: a, classification: justified, rationale: r}\n",
    );
    let out = import(&["--lcov", &lcov, "--annotations", &ann]);
    assert!(stderr(&out).contains("needs an approver"));
}
