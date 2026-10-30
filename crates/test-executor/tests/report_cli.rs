//! `test-executor report` (Phase 1): strict input loading, reconciliation and run selection.

use std::path::Path;
use std::process::{Command, Output};

fn matrix_yaml(run_ids: &[&str]) -> String {
    let runs: Vec<String> = run_ids.iter().map(|r| format!("\"{r}\"")).collect();
    format!(
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\n\
         links:\n  - requirement_id: HLR-1\n    level: HLR\n    test_case_ids: [T1]\n    \
         normal_range_covered: true\n    robustness_covered: false\n    run_ids: [{}]\n",
        runs.join(", ")
    )
}

fn run_yaml(run_id: &str, build_id: &str, started_at: &str, results: &[(&str, &str)]) -> String {
    let mut yaml = format!(
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\nbuild_id: {build_id}\n\
         environment: e\ntester: t\nstarted_at: '{started_at}'\nresults:\n"
    );
    if results.is_empty() {
        yaml = yaml.replace("results:\n", "results: []\n");
    }
    for (tc, verdict) in results {
        yaml.push_str(&format!("  - {{test_case_id: {tc}, verdict: {verdict}}}\n"));
    }
    yaml
}

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// Runs `test-executor --no-audit report` with the given extra arguments.
/// The schemas root is relative to the workspace, so the command runs from there.
fn report(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-audit", "report"])
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_arg(p: &Path) -> String {
    p.display().to_string()
}

/// A matrix plus a runs directory; returns (tempdir, matrix path, runs dir path)
fn fixture(run_ids: &[&str]) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let matrix = dir.path().join("matrix.yaml");
    std::fs::write(&matrix, matrix_yaml(run_ids)).unwrap();
    let runs = dir.path().join("runs");
    std::fs::create_dir(&runs).unwrap();
    (dir, matrix, runs)
}

#[test]
fn reconciliation_lists_discovered_loaded_and_skipped_inputs() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    write(
        &runs,
        "run.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    write(&runs, "notes.yaml", "just: some other yaml\n");
    write(&runs, "readme.txt", "not yaml\n");

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(
        out.contains("Inputs discovered: 4 (loaded: 2, skipped: 2)"),
        "{out}"
    );
    assert!(out.contains("no `type` field"), "{out}");
    assert!(out.contains("not a YAML file"), "{out}");
    assert!(out.contains(&path_arg(&matrix)), "{out}");
    assert!(out.contains("(RUN-1)"), "{out}");
}

#[test]
fn an_invalid_matrix_fails_naming_the_file() {
    let (dir, _matrix, runs) = fixture(&["RUN-1"]);
    let bad = dir.path().join("bad-matrix.yaml");
    std::fs::write(
        &bad,
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\n",
    )
    .unwrap();
    write(
        &runs,
        "run.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = report(&["--matrix", &path_arg(&bad), "--runs", &path_arg(&runs)]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&path_arg(&bad)),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_invalid_run_fails_naming_the_file() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    write(
        &runs,
        "run.yaml",
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-1\nbuild_id: b1\n\
         environment: e\nstarted_at: '2026-10-28T10:00:00Z'\nresults: []\n",
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&path_arg(&runs.join("run.yaml"))),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_yaml_syntax_error_in_a_runs_directory_fails() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    write(&runs, "broken.yaml", "a: [unclosed\n");

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("broken.yaml"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn duplicate_run_ids_fail_naming_both_files() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    let content = run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]);
    write(&runs, "a.yaml", &content);
    write(&runs, "b.yaml", &content);

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let err = stderr(&output);
    assert!(err.contains("duplicate run_id RUN-1"), "{err}");
    assert!(err.contains("a.yaml"), "{err}");
    assert!(err.contains("b.yaml"), "{err}");
}

#[test]
fn a_duplicate_test_case_inside_one_run_fails() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    write(
        &runs,
        "run.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", "pass"), ("T1", "fail")],
        ),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stderr(&output).contains("T1"), "{}", stderr(&output));
}

#[test]
fn latest_executed_result_decides_and_not_executed_does_not_count() {
    let (_dir, matrix, runs) = fixture(&["RUN-1", "RUN-2", "RUN-3"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml("RUN-2", "b2", "2026-10-28T11:00:00Z", &[("T1", "fail")]),
    );
    write(
        &runs,
        "run3.yaml",
        &run_yaml(
            "RUN-3",
            "b3",
            "2026-10-28T12:00:00Z",
            &[("T1", "not_executed")],
        ),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("    T1  fail  RUN-2"), "{out}");
    assert!(out.contains("Runs excluded: 2"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.contains("RUN-3") && l.contains("only not_executed results")),
        "{out}"
    );
    assert!(
        out.lines().any(|l| l.contains("RUN-1")
            && l.contains("every executed result superseded by a later run")),
        "{out}"
    );
}

#[test]
fn a_later_pass_overrides_an_earlier_fail() {
    let (_dir, matrix, runs) = fixture(&["RUN-1", "RUN-2"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "fail")]),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml("RUN-2", "b2", "2026-10-28T11:00:00Z", &[("T1", "pass")]),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("    T1  pass  RUN-2"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn every_build_used_is_listed() {
    let (_dir, matrix, runs) = fixture(&["RUN-1", "RUN-2"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml("RUN-2", "b2", "2026-10-28T11:00:00Z", &[("T2", "fail")]),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("Builds used: b1, b2"), "{out}");
    assert!(
        out.contains("for test cases not in the matrix: T2"),
        "{out}"
    );
}

#[test]
fn a_run_named_by_the_matrix_but_not_loaded_is_a_note() {
    let (_dir, matrix, runs) = fixture(&["RUN-1", "RUN-9"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("matrix references run RUN-9, which was not loaded"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn report_kinds_default_to_all_and_can_be_selected() {
    let (_dir, matrix, runs) = fixture(&["RUN-1"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    let matrix_arg = path_arg(&matrix);
    let runs_arg = path_arg(&runs);

    let all = report(&["--matrix", &matrix_arg, "--runs", &runs_arg]);
    assert!(all.status.success(), "{}", stderr(&all));
    let out = stdout(&all);
    for kind in ["traceability", "summary", "gaps", "problems"] {
        assert!(
            out.contains(&format!("  {kind}: renderer not yet available")),
            "{out}"
        );
    }

    let some = report(&[
        "--matrix",
        &matrix_arg,
        "--runs",
        &runs_arg,
        "--kind",
        "summary",
        "--kind",
        "gaps",
    ]);
    assert!(some.status.success(), "{}", stderr(&some));
    let out = stdout(&some);
    assert!(out.contains("summary: renderer not yet available"), "{out}");
    assert!(out.contains("gaps: renderer not yet available"), "{out}");
    assert!(!out.contains("traceability:"), "{out}");
}

#[test]
fn output_flag_writes_the_block_to_a_file() {
    let (dir, matrix, runs) = fixture(&["RUN-1"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    let out_file = dir.path().join("out.txt");

    let output = report(&[
        "--matrix",
        &path_arg(&matrix),
        "--runs",
        &path_arg(&runs),
        "--output",
        &path_arg(&out_file),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let written = std::fs::read_to_string(&out_file).unwrap();
    assert!(written.contains("Reconciliation"), "{written}");
    assert!(written.contains("Run selection"), "{written}");
    assert!(
        !stdout(&output).contains("Reconciliation"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn runs_flag_is_required() {
    let (_dir, matrix, _runs) = fixture(&["RUN-1"]);
    let output = report(&["--matrix", &path_arg(&matrix)]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--runs"), "{}", stderr(&output));
}

#[test]
fn same_inputs_give_identical_output() {
    let (_dir, matrix, runs) = fixture(&["RUN-1", "RUN-2", "RUN-3"]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml("RUN-2", "b2", "2026-10-28T11:00:00Z", &[("T1", "fail")]),
    );
    write(
        &runs,
        "run3.yaml",
        &run_yaml(
            "RUN-3",
            "b3",
            "2026-10-28T12:00:00Z",
            &[("T1", "not_executed")],
        ),
    );
    let args = ["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)];

    let first = report(&args);
    let second = report(&args);
    assert!(first.status.success(), "{}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
}
