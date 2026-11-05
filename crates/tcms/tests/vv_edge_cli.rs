//! Error and edge paths of `tcms vv` (business logic in src/vvcmd.rs, vvcombine.rs, vvconfig.rs), driven through
//! the binary. Production code is not changed; each test names the behaviour it pins.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn schemas() -> String {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/").to_string()
}

fn tcms(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .args(["--schemas-root", &schemas(), "vv"])
        .args(args)
        .output()
        .unwrap()
}

fn run_yaml(run_id: &str, started: &str, verdict: &str) -> String {
    format!(
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\nbuild_id: b-1\n\
         environment: Bench A\ntester: t\nstarted_at: \"{started}\"\nfinished_at: \"{started}\"\n\
         results:\n- test_case_id: TC_001\n  verdict: {verdict}\n"
    )
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn one_run(root: &Path) {
    write(
        &root.join("runs/RUN-1/test-run.yaml"),
        &run_yaml("RUN-1", "2026-10-01T10:00:00Z", "pass"),
    );
}

/// Permission-based tests are meaningless as root, which ignores file modes (CI runs as root).
fn running_as_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false)
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn a_run_folder_without_test_run_yaml_is_skipped() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    fs::create_dir_all(tmp.path().join("runs/RUN-EMPTY-FOLDER")).unwrap();
    let o = tcms(&[
        "metrics",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now",
        "2026-01-01T00:00:00Z",
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["run"]["runId"], "RUN-1");
    assert_eq!(v["loadSummary"]["runsRecorded"], 1);
}

#[test]
fn trends_with_no_run_folders_is_refused_by_name() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("runs")).unwrap();
    let o = tcms(&["trends", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("no recorded V&V runs"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn root_that_is_a_file_is_refused_for_every_reader() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("not-a-dir.txt");
    fs::write(&file, "x").unwrap();
    let f = file.to_str().unwrap();
    for args in [
        vec!["metrics", "--root", f],
        vec!["history", "--root", f, "--case", "TC_001"],
        vec!["trends", "--root", f],
    ] {
        let o = tcms(&args);
        assert_eq!(o.status.code(), Some(1), "{args:?}");
        assert!(
            stderr(&o).contains("is not a directory"),
            "{args:?}: {}",
            stderr(&o)
        );
    }
}

#[test]
fn trends_out_writes_the_report_file_and_nothing_to_stdout() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    let out = tmp.path().join("deep/trends.json");
    let o = tcms(&[
        "trends",
        "--root",
        tmp.path().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(o.stdout.is_empty());
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(v["runs"][0]["runId"], "RUN-1");
}

#[test]
fn combine_refuses_a_per_case_file_whose_type_is_not_test_result() {
    let tmp = TempDir::new().unwrap();
    let per = tmp.path().join("runs/RUN-X/per-case");
    write(
        &per.join("TC_001_result.yaml"),
        "type: test_run\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_001\n\
         total_steps: 1\npassed_steps: 1\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n",
    );
    let o = tcms(&[
        "combine",
        "--root",
        tmp.path().to_str().unwrap(),
        "--run",
        "RUN-X",
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("type must be test_result"),
        "{}",
        stderr(&o)
    );
    assert!(!tmp.path().join("runs/RUN-X/test-run.yaml").exists());
}

#[test]
fn combine_into_a_read_only_run_folder_fails_and_leaves_no_temporary_files() {
    if running_as_root() {
        return; // root ignores file permissions: nothing to check
    }
    let tmp = TempDir::new().unwrap();
    let run = tmp.path().join("runs/RUN-RO");
    write(
        &run.join("per-case/TC_001_result.yaml"),
        "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_001\n\
         total_steps: 1\npassed_steps: 1\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n",
    );
    write(
        &run.join("execution_logs/TC_001_execution_log.json"),
        "[{\"test_case_id\":\"TC_001\",\"step\":1,\"timestamp\":\"2026-11-02T06:00:00Z\"}]",
    );
    fs::set_permissions(&run, fs::Permissions::from_mode(0o555)).unwrap();
    let o = tcms(&[
        "combine",
        "--root",
        tmp.path().to_str().unwrap(),
        "--run",
        "RUN-RO",
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ]);
    fs::set_permissions(&run, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(!run.join("test-run.yaml").exists());
    assert!(!run.join("verification-results.yaml").exists());
    let leftovers: Vec<_> = fs::read_dir(&run)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(
        leftovers
            .iter()
            .all(|n| !n.to_string_lossy().contains(".tmp-")),
        "{leftovers:?}"
    );
}

#[test]
fn combine_rolls_back_the_first_target_when_the_second_cannot_be_replaced() {
    let tmp = TempDir::new().unwrap();
    let run = tmp.path().join("runs/RUN-RB");
    write(
        &run.join("per-case/TC_001_result.yaml"),
        "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_001\n\
         total_steps: 1\npassed_steps: 1\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n",
    );
    write(
        &run.join("execution_logs/TC_001_execution_log.json"),
        "[{\"test_case_id\":\"TC_001\",\"step\":1,\"timestamp\":\"2026-11-02T06:00:00Z\"}]",
    );
    // A directory where verification-results.yaml must go: the second rename fails after the first succeeded.
    fs::create_dir_all(run.join("verification-results.yaml")).unwrap();
    let o = tcms(&[
        "combine",
        "--root",
        tmp.path().to_str().unwrap(),
        "--run",
        "RUN-RB",
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ]);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("verification-results.yaml"),
        "{}",
        stderr(&o)
    );
    assert!(
        !run.join("test-run.yaml").exists(),
        "the first target must be rolled back"
    );
    assert!(run.join("verification-results.yaml").is_dir());
}

#[test]
fn vv_config_with_a_non_string_key_is_refused() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    write(
        &tmp.path().join("vv-config.yaml"),
        "type: vv-config\n1: 2\n",
    );
    let o = tcms(&["metrics", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("every key must be a string"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn history_with_an_unknown_plan_is_refused() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    let o = tcms(&[
        "history",
        "--root",
        tmp.path().to_str().unwrap(),
        "--case",
        "TC_001",
        "--plan",
        "NOPE",
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("no recorded run for plan NOPE"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn combine_refuses_a_root_that_is_a_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("root.txt");
    fs::write(&file, "x").unwrap();
    let o = tcms(&[
        "combine",
        "--root",
        file.to_str().unwrap(),
        "--run",
        "RUN-X",
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("is not a directory"), "{}", stderr(&o));
}

#[test]
fn out_under_a_path_whose_parent_is_a_file_names_the_creating_step() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    let blocker = tmp.path().join("blocker");
    fs::write(&blocker, "not a directory").unwrap();
    let out = blocker.join("sub/trends.json");
    let o = tcms(&[
        "trends",
        "--root",
        tmp.path().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("creating"), "{}", stderr(&o));
    assert!(!out.exists());
}

#[test]
fn combine_skips_execution_log_entries_without_a_timestamp() {
    let tmp = TempDir::new().unwrap();
    let run = tmp.path().join("runs/RUN-NT");
    write(
        &run.join("per-case/TC_001_result.yaml"),
        "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_001\n\
         total_steps: 1\npassed_steps: 1\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n",
    );
    write(
        &run.join("execution_logs/TC_001_execution_log.json"),
        "[{\"test_case_id\":\"TC_001\",\"step\":1},\
          {\"test_case_id\":\"TC_001\",\"step\":2,\"timestamp\":\"2026-11-02T06:00:05Z\"}]",
    );
    let o = tcms(&[
        "combine",
        "--root",
        tmp.path().to_str().unwrap(),
        "--run",
        "RUN-NT",
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    let text = fs::read_to_string(run.join("test-run.yaml")).unwrap();
    assert!(
        text.contains("started_at: \"2026-11-02T06:00:05Z\"")
            || text.contains("started_at: 2026-11-02T06:00:05Z"),
        "{text}"
    );
}

const PER_CASE_OK: &str =
    "type: test_result\nschema: tcms/test-result.schema.v1.json\ntest_case_id: TC_001\n\
    total_steps: 1\npassed_steps: 1\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n";

fn combine_run(root: &Path, run: &str) -> Output {
    tcms(&[
        "combine",
        "--root",
        root.to_str().unwrap(),
        "--run",
        run,
        "--build-id",
        "b",
        "--environment",
        "e",
        "--tester",
        "t",
    ])
}

#[test]
fn combine_refuses_run_ids_that_are_not_single_folder_names() {
    let tmp = TempDir::new().unwrap();
    for bad in ["..", ".", "a/b", "a\\b", ""] {
        let o = combine_run(tmp.path(), bad);
        assert_eq!(o.status.code(), Some(1), "--run {bad:?}");
        assert!(
            stderr(&o).contains("single folder name"),
            "--run {bad:?}: {}",
            stderr(&o)
        );
    }
}

#[test]
fn combine_ignores_non_yaml_entries_and_directories_in_the_per_case_folder() {
    let tmp = TempDir::new().unwrap();
    let per = tmp.path().join("runs/RUN-N/per-case");
    write(&per.join("TC_001_result.yaml"), PER_CASE_OK);
    write(
        &tmp.path()
            .join("runs/RUN-N/execution_logs/TC_001_execution_log.json"),
        "[{\"test_case_id\":\"TC_001\",\"step\":1,\"timestamp\":\"2026-11-02T06:00:00Z\"}]",
    );
    write(&per.join("notes.txt"), "not a result");
    fs::create_dir_all(per.join("folder.yaml")).unwrap();
    let o = combine_run(tmp.path(), "RUN-N");
    assert!(o.status.success(), "{}", stderr(&o));
    let text = fs::read_to_string(tmp.path().join("runs/RUN-N/test-run.yaml")).unwrap();
    assert!(text.contains("test_case_id: TC_001"), "{text}");
}

#[test]
fn combine_refuses_an_unreadable_per_case_file_naming_it() {
    if running_as_root() {
        return; // root ignores file permissions: nothing to check
    }
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("runs/RUN-U/per-case/TC_001_result.yaml");
    write(&file, PER_CASE_OK);
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    let o = combine_run(tmp.path(), "RUN-U");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("reading"), "{}", stderr(&o));
    assert!(stderr(&o).contains("TC_001_result.yaml"), "{}", stderr(&o));
}

fn run_with_log(log: &str) -> (TempDir, Output) {
    let tmp = TempDir::new().unwrap();
    write(
        &tmp.path().join("runs/RUN-L/per-case/TC_001_result.yaml"),
        PER_CASE_OK,
    );
    write(
        &tmp.path()
            .join("runs/RUN-L/execution_logs/TC_001_execution_log.json"),
        log,
    );
    let o = combine_run(tmp.path(), "RUN-L");
    (tmp, o)
}

#[test]
fn an_execution_log_that_is_not_json_is_refused_naming_the_log() {
    let (_tmp, o) = run_with_log("[{\"test_case_id\": ");
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("parsing"), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("TC_001_execution_log.json"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn an_execution_log_that_is_not_an_array_is_refused() {
    let (_tmp, o) = run_with_log("{\"test_case_id\": \"TC_001\"}");
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("must be a JSON array"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn a_timestamp_that_is_not_rfc3339_is_refused() {
    let (_tmp, o) =
        run_with_log("[{\"test_case_id\":\"TC_001\",\"step\":1,\"timestamp\":\"yesterday\"}]");
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("is not RFC 3339"), "{}", stderr(&o));
}

fn run_yaml_with_results(run_id: &str, results: &str) -> String {
    format!(
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\nbuild_id: b-1\n\
         environment: Bench A\ntester: t\nstarted_at: \"2026-10-01T10:00:00Z\"\n\
         finished_at: \"2026-10-01T11:00:00Z\"\nresults:\n{results}"
    )
}

#[test]
fn metrics_refuses_a_malformed_test_run_yaml_naming_the_file() {
    let tmp = TempDir::new().unwrap();
    write(
        &tmp.path().join("runs/RUN-BAD/test-run.yaml"),
        "type: [unclosed\n  : :\n",
    );
    let o = tcms(&["metrics", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("test-run.yaml"), "{}", stderr(&o));
}

#[test]
fn metrics_refuses_a_result_without_test_case_id_or_verdict() {
    for (label, results) in [
        ("no test_case_id", "- verdict: pass\n"),
        ("no verdict", "- test_case_id: TC_001\n"),
    ] {
        let tmp = TempDir::new().unwrap();
        write(
            &tmp.path().join("runs/RUN-R/test-run.yaml"),
            &run_yaml_with_results("RUN-R", results),
        );
        let o = tcms(&["metrics", "--root", tmp.path().to_str().unwrap()]);
        assert_eq!(o.status.code(), Some(1), "{label}");
        assert!(
            stderr(&o).contains("test-run.yaml"),
            "{label}: {}",
            stderr(&o)
        );
    }
}

#[test]
fn metrics_refuses_a_malformed_verification_results_yaml_naming_it() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    write(
        &tmp.path().join("runs/RUN-1/verification-results.yaml"),
        "test_results: [unclosed\n",
    );
    let o = tcms(&["metrics", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        stderr(&o).contains("verification-results.yaml"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn a_runs_entry_that_is_a_file_is_refused() {
    let tmp = TempDir::new().unwrap();
    write(&tmp.path().join("runs"), "not a folder");
    let o = tcms(&["metrics", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("runs"), "{}", stderr(&o));
}

#[test]
fn trends_refuses_a_malformed_vv_config() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    write(
        &tmp.path().join("vv-config.yaml"),
        "gate_threshold_percent: [unclosed\n",
    );
    let o = tcms(&["trends", "--root", tmp.path().to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("vv-config.yaml"), "{}", stderr(&o));
}

#[test]
fn metrics_out_writes_the_json_file_and_leaves_stdout_empty() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    let out = tmp.path().join("out/metrics.json");
    let o = tcms(&[
        "metrics",
        "--root",
        tmp.path().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(o.stdout.is_empty());
    let v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(v["run"]["runId"], "RUN-1");
}

#[test]
fn metrics_out_into_a_read_only_directory_fails_naming_the_temporary_file() {
    if running_as_root() {
        return; // root ignores file permissions: nothing to check
    }
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    let dir = tmp.path().join("locked");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let out = dir.join("metrics.json");
    let o = tcms(&[
        "metrics",
        "--root",
        tmp.path().to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("writing"), "{}", stderr(&o));
    assert!(!out.exists());
}

#[test]
fn combine_refuses_an_unreadable_existing_test_run_before_writing_anything() {
    if running_as_root() {
        return; // root ignores file permissions: nothing to check
    }
    let tmp = TempDir::new().unwrap();
    let run = tmp.path().join("runs/RUN-OLD");
    write(&run.join("per-case/TC_001_result.yaml"), PER_CASE_OK);
    write(
        &run.join("execution_logs/TC_001_execution_log.json"),
        "[{\"test_case_id\":\"TC_001\",\"step\":1,\"timestamp\":\"2026-11-02T06:00:00Z\"}]",
    );
    write(&run.join("test-run.yaml"), "previous record");
    fs::set_permissions(run.join("test-run.yaml"), fs::Permissions::from_mode(0o000)).unwrap();
    let o = combine_run(tmp.path(), "RUN-OLD");
    fs::set_permissions(run.join("test-run.yaml"), fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("reading"), "{}", stderr(&o));
    assert!(!run.join("verification-results.yaml").exists());
}

/// DEFECT D-09 (DISCREPANCES_TEST_CASES.md): docs/VV_METRICS.md says a failing file is listed under
/// `loadSummary.schemaErrors`. A case file that does not parse as YAML is silently skipped instead
/// (vvcmd.rs, `let Ok(doc) = load_yaml(&path) else { continue };`). Kept ignored until the defect is fixed.
#[test]
fn a_case_file_that_does_not_parse_is_listed_in_schema_errors() {
    let tmp = TempDir::new().unwrap();
    one_run(tmp.path());
    write(&tmp.path().join("BAD_001.yml"), "type: [unclosed\n");
    let o = tcms(&[
        "metrics",
        "--root",
        tmp.path().to_str().unwrap(),
        "--now",
        "2026-01-01T00:00:00Z",
    ]);
    assert!(o.status.success(), "{}", stderr(&o));
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["loadSummary"]["schemaErrorCount"], 1);
}
