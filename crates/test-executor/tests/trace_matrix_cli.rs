//! `test-executor trace-matrix` (TCMS-40.5, TCMS-40.6): the matrix, the coverage check and its
//! exit code.

use std::path::Path;
use std::process::{Command, Output};

fn test_case(id: &str, requirement: &str, test_type: &str, category: &str) -> String {
    format!(
        "type: test_case\nschema: tcms/test-case.schema.v1.json\n\
         requirement: LEGACY-NOT-USED\nitem: 1\ntc: 1\nid: {id}\ndescription: d\n\
         requirements: [{requirement}]\ntest_type: {test_type}\ntest_category: {category}\n\
         general_initial_conditions: {{}}\ninitial_conditions: {{}}\ntest_sequences: []\n"
    )
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn trace_matrix(dir: &Path, args: &[&str]) -> Output {
    // test cases are validated against the repository's schemas (relative to the workspace)
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-audit", "trace-matrix"])
        .arg(dir.join("cases"))
        .args(args)
        .output()
        .unwrap()
}

fn setup(requirements: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("cases")).unwrap();
    write(dir.path(), "reqs.txt", requirements);
    dir
}

fn args<'a>(dir: &'a Path, extra: &[&'a str]) -> Vec<String> {
    let mut all = vec![
        "--requirements".to_string(),
        dir.join("reqs.txt").display().to_string(),
        "--id".to_string(),
        "TM-TEST".to_string(),
    ];
    all.extend(extra.iter().map(|s| s.to_string()));
    all
}

fn run(dir: &Path, extra: &[&str]) -> Output {
    let all = args(dir, extra);
    let refs: Vec<&str> = all.iter().map(String::as_str).collect();
    trace_matrix(dir, &refs)
}

#[test]
fn matrix_is_written_and_a_requirement_with_both_test_types_passes_the_check() {
    let dir = setup("HLR-1\n");
    let cases = dir.path().join("cases");
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );
    write(
        &cases,
        "r.yaml",
        &test_case("R1", "HLR-1", "robustness", "overflow"),
    );

    let output = run(dir.path(), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matrix: serde_yaml::Value =
        serde_yaml::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(matrix["id"], "TM-TEST");
    assert_eq!(matrix["links"][0]["level"], "HLR");
    assert_eq!(matrix["links"][0]["normal_range_covered"], true);
    assert_eq!(matrix["links"][0]["robustness_covered"], true);
    assert_eq!(matrix["links"][0]["test_case_ids"][0], "N1");
}

#[test]
fn a_missing_test_type_fails_the_check_naming_requirement_and_kind_but_still_writes_the_matrix() {
    let dir = setup("HLR-1\nLLR-2:LLR\n");
    let cases = dir.path().join("cases");
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );

    let output = run(dir.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("HLR-1 (HLR): no robustness test"),
        "{stderr}"
    );
    assert!(
        stderr.contains("LLR-2 (LLR): no normal_range and no robustness test"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Requirements without test cases: LLR-2"),
        "{stderr}"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("requirement_id: LLR-2"));
}

#[test]
fn the_check_can_be_disabled() {
    let dir = setup("HLR-1\n");
    let cases = dir.path().join("cases");
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );

    let output = run(dir.path(), &["--no-coverage-check"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_legacy_requirement_string_does_not_trace() {
    let dir = setup("LEGACY-NOT-USED:HLR\n");
    let cases = dir.path().join("cases");
    // names HLR-9 in requirements[]; the legacy `requirement` says LEGACY-NOT-USED
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-9", "normal_range", "boundary_value"),
    );

    let output = run(dir.path(), &["--no-coverage-check"]);
    let matrix: serde_yaml::Value =
        serde_yaml::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(
        matrix["links"][0]["test_case_ids"]
            .as_sequence()
            .unwrap()
            .len(),
        0
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("HLR-9"));
}

#[test]
fn an_unreadable_test_case_is_an_error_not_a_silent_gap() {
    let dir = setup("HLR-1\n");
    let cases = dir.path().join("cases");
    write(
        &cases,
        "good.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );
    write(&cases, "broken.yaml", "id: not a test case\n");

    let output = run(dir.path(), &["--no-coverage-check"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1 of 2 YAML files") && stderr.contains("broken.yaml"),
        "{stderr}"
    );
}

#[test]
fn runs_link_through_run_ids() {
    let dir = setup("HLR-1\n");
    let cases = dir.path().join("cases");
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );
    let runs = dir.path().join("runs");
    std::fs::create_dir(&runs).unwrap();
    write(
        &runs,
        "run.yaml",
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-7\nbuild_id: b\n\
         environment: e\ntester: t\nstarted_at: '2026-10-28T10:00:00Z'\n\
         results:\n  - {test_case_id: N1, verdict: fail}\n",
    );
    write(&runs, "notes.yaml", "just: some other yaml\n");

    let runs_arg = runs.display().to_string();
    let output = run(dir.path(), &["--runs", &runs_arg, "--no-coverage-check"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matrix: serde_yaml::Value =
        serde_yaml::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert_eq!(matrix["links"][0]["run_ids"][0], "RUN-7");
}

/// The crafted StrictDoc export (assumed layout): SYS-1 (system), HLR-1, LLR-2, HLR-3.
fn strictdoc_fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/strictdoc_export.json")
}

fn run_with_requirements(dir: &Path, requirements: &Path, extra: &[&str]) -> Output {
    let req = requirements.display().to_string();
    let mut all = vec!["--requirements", req.as_str(), "--id", "TM-TEST"];
    all.extend_from_slice(extra);
    trace_matrix(dir, &all)
}

#[test]
fn a_strictdoc_export_is_the_requirement_set_and_the_coverage_check_runs_on_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("cases")).unwrap();
    let cases = dir.path().join("cases");
    // HLR-1 has both kinds, LLR-2 only normal-range, HLR-3 none
    write(
        &cases,
        "n.yaml",
        &test_case("N1", "HLR-1", "normal_range", "boundary_value"),
    );
    write(
        &cases,
        "r.yaml",
        &test_case("R1", "HLR-1", "robustness", "overflow"),
    );
    write(
        &cases,
        "n2.yaml",
        &test_case("N2", "LLR-2", "normal_range", "boundary_value"),
    );

    let output = run_with_requirements(dir.path(), &strictdoc_fixture(), &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LLR-2 (LLR): no robustness test"),
        "{stderr}"
    );
    assert!(
        stderr.contains("HLR-3 (HLR): no normal_range and no robustness test"),
        "{stderr}"
    );
    assert!(!stderr.contains("HLR-1 (HLR)"), "{stderr}");
    assert!(
        !stderr.contains("SYS-1"),
        "system items are not in the matrix: {stderr}"
    );

    let matrix: serde_yaml::Value =
        serde_yaml::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap();
    let links = matrix["links"].as_sequence().unwrap();
    let ids: Vec<&str> = links
        .iter()
        .map(|l| l["requirement_id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["HLR-1", "LLR-2", "HLR-3"],
        "HLR and LLR only, in document order"
    );
    assert_eq!(links[0]["level"], "HLR");
    assert_eq!(links[1]["level"], "LLR");
}

#[test]
fn a_strictdoc_export_with_every_requirement_covered_passes_the_check() {
    let dir = tempfile::tempdir().unwrap();
    let cases = dir.path().join("cases");
    std::fs::create_dir(&cases).unwrap();
    for (id, req, kind, category) in [
        ("N1", "HLR-1", "normal_range", "boundary_value"),
        ("R1", "HLR-1", "robustness", "overflow"),
        ("N2", "LLR-2", "normal_range", "boundary_value"),
        ("R2", "LLR-2", "robustness", "overflow"),
        ("N3", "HLR-3", "normal_range", "boundary_value"),
        ("R3", "HLR-3", "robustness", "overflow"),
    ] {
        write(
            &cases,
            &format!("{id}.yaml"),
            &test_case(id, req, kind, category),
        );
    }

    let output = run_with_requirements(dir.path(), &strictdoc_fixture(), &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn an_invalid_strictdoc_export_is_an_error_with_the_file_named() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("cases")).unwrap();
    write(dir.path(), "export.json", "{ \"documents\": [");

    let output = run_with_requirements(dir.path(), &dir.path().join("export.json"), &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("export.json is not a StrictDoc export"),
        "{stderr}"
    );
}

#[test]
fn a_strictdoc_export_with_an_unknown_node_type_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("cases")).unwrap();
    let text = std::fs::read_to_string(strictdoc_fixture())
        .unwrap()
        .replace(
            "\"LLR-2\", \"level\": \"LLR\"",
            "\"LLR-2\", \"level\": \"Epic\"",
        );
    write(dir.path(), "export.json", &text);

    let output = run_with_requirements(dir.path(), &dir.path().join("export.json"), &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LLR-2") && stderr.contains("'Epic'"),
        "{stderr}"
    );
}

#[test]
fn a_strictdoc_export_without_hlr_or_llr_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("cases")).unwrap();
    write(
        dir.path(),
        "export.json",
        r#"{"documents":[{"path":"a.sdoc","requirements":[{"uid":"SYS-1","level":"system"}]}]}"#,
    );

    let output = run_with_requirements(dir.path(), &dir.path().join("export.json"), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No HLR or LLR found"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
