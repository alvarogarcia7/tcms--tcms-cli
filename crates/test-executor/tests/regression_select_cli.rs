//! `test-executor regression-select` (TCMS-40.7)

use std::path::Path;
use std::process::{Command, Output};

fn test_case(id: &str, requirement: &str) -> String {
    format!(
        "type: test_case\nschema: tcms/test-case.schema.v1.json\n\
         requirement: LEGACY-NOT-USED\nitem: 1\ntc: 1\nid: {id}\ndescription: d\n\
         requirements: [{requirement}]\n\
         general_initial_conditions: {{}}\ninitial_conditions: {{}}\ntest_sequences: []\n"
    )
}

fn baseline(id: &str, previous: &str, listed: &str, requirements: &str) -> String {
    format!(
        "id: {id}\nbuild_id: b\ncreated: '2026-10-27T08:00:00Z'\nitems: [a@1]\n\
         previous_baseline_id: {previous}\nchange_impact:\n  impacted_requirements: [{requirements}]\n  \
         regression_test_cases: [{listed}]\n"
    )
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn setup() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases");
    std::fs::create_dir(&cases).unwrap();
    write(&cases, "t1.yaml", &test_case("T1", "HLR-1"));
    write(&cases, "t2.yaml", &test_case("T2", "HLR-2"));
    write(&cases, "t3.yaml", &test_case("T3", "HLR-3"));
    dir
}

fn select(dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-audit", "regression-select"])
        .arg(dir.join("cases"))
        .args(extra)
        .output()
        .unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[test]
fn only_listed_and_requirement_traced_test_cases_are_selected() {
    let dir = setup();
    write(
        dir.path(),
        "bl.yaml",
        &baseline("BL-1", "BL-0", "T3", "HLR-1"),
    );
    let bl = dir.path().join("bl.yaml");
    let out = select(dir.path(), &["--baseline", bl.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "T1\nT3\n");

    let out = select(
        dir.path(),
        &["--baseline", bl.to_str().unwrap(), "--format", "json"],
    );
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["selected"][0]["test_case_id"], "T1");
    assert_eq!(json["selected"][0]["reasons"][0], "HLR-1");
    assert_eq!(json["selected"][1]["reasons"][0], "listed");
}

#[test]
fn a_regression_plan_picks_its_baseline_and_checks_the_previous_one() {
    let dir = setup();
    write(
        dir.path(),
        "bl1.yaml",
        &baseline("BL-1", "BL-0", "T3", "HLR-1"),
    );
    write(
        dir.path(),
        "bl2.yaml",
        &baseline("BL-2", "BL-1", "", "HLR-2"),
    );
    write(
        dir.path(),
        "plan.yaml",
        "id: TP-1\nbaseline_id: BL-2\nregression:\n  previous_baseline_id: BL-1\n",
    );
    let (b1, b2, plan) = (
        dir.path().join("bl1.yaml"),
        dir.path().join("bl2.yaml"),
        dir.path().join("plan.yaml"),
    );
    let args = [
        "--baseline",
        b1.to_str().unwrap(),
        "--baseline",
        b2.to_str().unwrap(),
        "--plan",
        plan.to_str().unwrap(),
    ];
    let out = select(dir.path(), &args);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "T2\n");

    // the plan measures BL-2 from the wrong baseline
    write(
        dir.path(),
        "plan.yaml",
        "id: TP-1\nbaseline_id: BL-2\nregression:\n  previous_baseline_id: BL-0\n",
    );
    let out = select(dir.path(), &args);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("measures baseline 'BL-2' from 'BL-0'"));

    // without a plan two baselines are ambiguous
    let out = select(
        dir.path(),
        &[
            "--baseline",
            b1.to_str().unwrap(),
            "--baseline",
            b2.to_str().unwrap(),
        ],
    );
    assert!(stderr(&out).contains("no --plan"));
}

#[test]
fn missing_and_invalid_baselines_fail_with_a_clear_error() {
    let dir = setup();
    let out = select(dir.path(), &["--baseline", "/nonexistent/bl.yaml"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("Baseline not found: /nonexistent/bl.yaml"));

    write(dir.path(), "bad.yaml", "id: BL-1\nitems: []\n");
    let bad = dir.path().join("bad.yaml");
    let out = select(dir.path(), &["--baseline", bad.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("required property 'build_id' is missing"));

    write(
        dir.path(),
        "nochange.yaml",
        "id: BL-1\nbuild_id: b\ncreated: x\nitems: []\n",
    );
    let nochange = dir.path().join("nochange.yaml");
    let out = select(dir.path(), &["--baseline", nochange.to_str().unwrap()]);
    assert!(stderr(&out).contains("has no change_impact"));

    write(dir.path(), "plain-plan.yaml", "id: TP\nbaseline_id: BL-1\n");
    let good = dir.path().join("good.yaml");
    std::fs::write(&good, baseline("BL-1", "BL-0", "", "HLR-1")).unwrap();
    let plan = dir.path().join("plain-plan.yaml");
    let out = select(
        dir.path(),
        &[
            "--baseline",
            good.to_str().unwrap(),
            "--plan",
            plan.to_str().unwrap(),
        ],
    );
    assert!(stderr(&out).contains("not a regression plan"));
}

#[test]
fn listed_test_cases_that_do_not_exist_fail_unless_allowed() {
    let dir = setup();
    write(
        dir.path(),
        "bl.yaml",
        &baseline("BL-1", "BL-0", "GHOST", "HLR-1"),
    );
    let bl = dir.path().join("bl.yaml");
    let out = select(dir.path(), &["--baseline", bl.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("GHOST"));
    // the selection is still written
    assert_eq!(String::from_utf8_lossy(&out.stdout), "T1\n");

    let out = select(
        dir.path(),
        &[
            "--baseline",
            bl.to_str().unwrap(),
            "--allow-unknown-test-cases",
        ],
    );
    assert!(out.status.success());
}
