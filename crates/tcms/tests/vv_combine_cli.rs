//! `tcms vv combine` on the 63-file fixture (TCMS-47.3). Layout and expected figures: see
//! crates/tcms/tests/fixtures/vv-combine/README.md.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value as Json;
use serde_yaml::Value as Yaml;
use tempfile::TempDir;

const RUN: &str = "RUN-FIX-001";

const RUN_FLAGS: &[&str] = &[
    "--run",
    RUN,
    "--plan",
    "PLAN-FIX",
    "--build-id",
    "build-fix-1",
    "--tester",
    "fixture-bot",
    "--environment",
    "Fixture bench",
];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vv-combine")
}

fn repo_schemas() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/")
        .to_string_lossy()
        .into_owned()
}

fn copy_files(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let p = entry.unwrap().path();
        if p.is_file() {
            fs::copy(&p, to.join(p.file_name().unwrap())).unwrap();
        }
    }
}

/// Lay the fixture out as one run folder under `root`: `runs/RUN-FIX-001/{per-case,execution_logs}`.
fn stage_fixture(root: &Path) -> PathBuf {
    let run_dir = root.join("runs").join(RUN);
    let per_case = fixture().join("per-case");
    copy_files(&per_case, &run_dir.join("per-case"));
    copy_files(
        &per_case.join("execution_logs"),
        &run_dir.join("execution_logs"),
    );
    run_dir
}

fn combine(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv", "combine", "--root"])
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

fn combine_ok(root: &Path) {
    let o = combine(root, RUN_FLAGS);
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn read_yaml(path: &Path) -> Yaml {
    serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn run_file(root: &Path, run: &str, name: &str) -> PathBuf {
    root.join("runs").join(run).join(name)
}

/// (test_case_id, verdict) of every result of the combined run record, in file order.
fn results_of(test_run: &Yaml) -> Vec<(String, String)> {
    test_run["results"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["test_case_id"].as_str().unwrap().to_string(),
                r["verdict"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn count_verdict(results: &[(String, String)], verdict: &str) -> usize {
    results.iter().filter(|(_, v)| v == verdict).count()
}

fn case_yaml(
    id: &str,
    total: u64,
    passed: u64,
    failed: u64,
    not_exec: u64,
    overall: bool,
) -> String {
    format!(
        "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: {id}\n\
         description: Inline case {id}\nsequences: []\ntotal_steps: {total}\npassed_steps: {passed}\n\
         failed_steps: {failed}\nnot_executed_steps: {not_exec}\noverall_pass: {overall}\n"
    )
}

fn log_json(timestamp: &str) -> String {
    format!(
        "[{{\"test_sequence\": 1, \"step\": 1, \"command\": \"true\", \"exit_code\": 0, \
         \"output\": \"\", \"result_verification_pass\": true, \"output_verification_pass\": true, \
         \"timestamp\": \"{timestamp}\"}}]\n"
    )
}

#[test]
fn combine_reads_63_case_files_into_one_record() {
    let tmp = TempDir::new().unwrap();
    let run_dir = stage_fixture(tmp.path());
    assert_eq!(fs::read_dir(run_dir.join("per-case")).unwrap().count(), 63);
    combine_ok(tmp.path());

    let test_run = read_yaml(&run_file(tmp.path(), RUN, "test-run.yaml"));
    let results = results_of(&test_run);
    assert_eq!(results.len(), 62, "one entry per distinct test_case_id");
    let ids: BTreeSet<_> = results.iter().map(|(id, _)| id.clone()).collect();
    assert_eq!(ids.len(), 62, "no duplicate test_case_id in results");
    assert_eq!(count_verdict(&results, "pass"), 49);
    assert_eq!(count_verdict(&results, "fail"), 10);
    assert_eq!(count_verdict(&results, "not_executed"), 3);
    assert_eq!(test_run["run_id"].as_str(), Some(RUN));
    assert_eq!(test_run["plan_id"].as_str(), Some("PLAN-FIX"));
}

#[test]
fn combine_writes_verification_results_keyed_by_case_id() {
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    combine_ok(tmp.path());

    let steps = read_yaml(&run_file(tmp.path(), RUN, "verification-results.yaml"));
    let rows = steps["test_results"].as_sequence().unwrap();
    assert_eq!(rows.len(), 62, "one test_results entry per test_case_id");
    let ids: Vec<&str> = rows
        .iter()
        .map(|r| r["test_case_id"].as_str().unwrap())
        .collect();
    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), 62);
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "sorted by test_case_id"
    );

    let dup = rows
        .iter()
        .find(|r| r["test_case_id"].as_str() == Some("TC_FIX_010"))
        .unwrap();
    assert_eq!(dup["total_steps"].as_u64(), Some(2));
    assert_eq!(
        dup["passed_steps"].as_u64(),
        Some(1),
        "steps of the winning attempt"
    );
    let not_run = rows
        .iter()
        .find(|r| r["test_case_id"].as_str() == Some("TC_FIX_060"))
        .unwrap();
    assert_eq!(not_run["passed_steps"].as_u64(), Some(0));
}

#[test]
fn duplicate_case_latest_started_at_wins() {
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    combine_ok(tmp.path());

    let test_run = read_yaml(&run_file(tmp.path(), RUN, "test-run.yaml"));
    let results = results_of(&test_run);
    let dup: Vec<_> = results
        .iter()
        .filter(|(id, _)| id == "TC_FIX_010")
        .collect();
    assert_eq!(dup.len(), 1, "one entry for the duplicated case");
    // a1 (started 06:00, pass) is earlier than a2 (started 09:00, fail): a2 wins.
    assert_eq!(dup[0].1, "fail");
}

#[test]
fn not_executed_never_overrides_executed() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("runs").join("RUN-NX");
    let per_case = run_dir.join("per-case");
    let logs = run_dir.join("execution_logs");
    fs::create_dir_all(&per_case).unwrap();
    fs::create_dir_all(&logs).unwrap();
    // Executed attempt, started earlier.
    fs::write(
        per_case.join("CASE_A_result.yaml"),
        case_yaml("CASE_A", 1, 1, 0, 0, true),
    )
    .unwrap();
    fs::write(
        logs.join("CASE_A_execution_log.json"),
        log_json("2026-11-02T06:00:00Z"),
    )
    .unwrap();
    // Not-executed attempt, logged later: newer, but must not replace the executed one.
    fs::write(
        per_case.join("CASE_A__late_result.yaml"),
        case_yaml("CASE_A", 1, 0, 0, 1, false),
    )
    .unwrap();
    fs::write(
        logs.join("CASE_A__late_execution_log.json"),
        log_json("2026-11-02T10:00:00Z"),
    )
    .unwrap();

    let o = combine(
        tmp.path(),
        &[
            "--run",
            "RUN-NX",
            "--plan",
            "PLAN-FIX",
            "--build-id",
            "build-fix-1",
            "--tester",
            "fixture-bot",
            "--environment",
            "Fixture bench",
        ],
    );
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let test_run = read_yaml(&run_file(tmp.path(), "RUN-NX", "test-run.yaml"));
    assert_eq!(
        results_of(&test_run),
        vec![("CASE_A".to_string(), "pass".to_string())]
    );
}

#[test]
fn combined_record_is_readable_by_vv_metrics() {
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    combine_ok(tmp.path());

    let o = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &repo_schemas(), "vv", "metrics", "--root"])
        .arg(tmp.path())
        .args(["--now", "2026-01-01T00:00:00Z"])
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let report: Json = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(report["gate"]["counts"]["passed"].as_u64(), Some(49));
}

#[test]
fn invalid_record_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    // Without --tester the test-run record lacks the required `tester`: validation must fail.
    let without_tester: Vec<&str> = RUN_FLAGS
        .chunks(2)
        .filter(|pair| pair[0] != "--tester")
        .flat_map(|pair| pair.iter().copied())
        .collect();
    let o = combine(tmp.path(), &without_tester);
    assert!(!o.status.success());
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("tester"),
        "stderr names the field: {stderr}"
    );
    assert!(!run_file(tmp.path(), RUN, "test-run.yaml").exists());
    assert!(!run_file(tmp.path(), RUN, "verification-results.yaml").exists());
}

#[test]
fn steps_schema_failure_writes_nothing() {
    let tmp = TempDir::new().unwrap();
    let run_dir = stage_fixture(tmp.path());
    // An empty test_case_id passes the test-run schema but fails verification-results.schema.json
    // (minLength 1), so only the second validation can reject the run.
    let path = run_dir.join("per-case").join("TC_FIX_003_result.yaml");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("test_case_id: TC_FIX_003", "test_case_id: \"\""),
    )
    .unwrap();
    let o = combine(tmp.path(), RUN_FLAGS);
    assert!(!o.status.success());
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("verification-results.yaml"),
        "stderr names the failing document: {stderr}"
    );
    assert!(!run_file(tmp.path(), RUN, "test-run.yaml").exists());
    assert!(!run_file(tmp.path(), RUN, "verification-results.yaml").exists());
}

#[test]
fn valid_record_writes_both_files_with_the_schema_header() {
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    combine_ok(tmp.path());
    assert!(run_file(tmp.path(), RUN, "test-run.yaml").is_file());
    let steps = read_yaml(&run_file(tmp.path(), RUN, "verification-results.yaml"));
    assert_eq!(
        steps["schema"].as_str(),
        Some("tcms/verification-results.schema.json")
    );
    assert_eq!(steps["run_id"].as_str(), Some(RUN));
}

#[test]
fn unreadable_case_file_writes_nothing_and_names_the_file() {
    let tmp = TempDir::new().unwrap();
    let run_dir = stage_fixture(tmp.path());
    fs::write(
        run_dir.join("per-case").join("TC_FIX_003_result.yaml"),
        "overall_pass: [\n",
    )
    .unwrap();
    let o = combine(tmp.path(), RUN_FLAGS);
    assert!(!o.status.success());
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("TC_FIX_003_result.yaml"),
        "stderr: {stderr}"
    );
    assert!(!run_file(tmp.path(), RUN, "test-run.yaml").exists());
    assert!(!run_file(tmp.path(), RUN, "verification-results.yaml").exists());
}

#[test]
fn run_without_plan_records_an_empty_plan_id() {
    // TCMS-47.4 AC 1: no --plan writes plan_id as "" (the schema types it as a string, so null is not allowed).
    let tmp = TempDir::new().unwrap();
    stage_fixture(tmp.path());
    let flags: Vec<&str> = RUN_FLAGS
        .iter()
        .copied()
        .filter(|f| *f != "--plan" && *f != "PLAN-FIX")
        .collect();
    let o = combine(tmp.path(), &flags);
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let test_run = read_yaml(&run_file(tmp.path(), RUN, "test-run.yaml"));
    assert_eq!(test_run["plan_id"].as_str(), Some(""));
}
