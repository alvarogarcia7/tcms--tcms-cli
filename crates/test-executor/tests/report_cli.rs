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
    assert!(out.starts_with("# Traceability Report\n"), "{out}");
    assert!(out.contains("## Inputs"), "{out}");
    for kind in ["summary", "gaps", "problems"] {
        assert!(
            out.contains(&format!("Report kind {kind}: renderer not yet available")),
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
    let args = [
        "--matrix",
        &path_arg(&matrix),
        "--runs",
        &path_arg(&runs),
        "--generated-at",
        "2026-10-31T09:00:00Z",
    ];

    let first = report(&args);
    let second = report(&args);
    assert!(first.status.success(), "{}", stderr(&first));
    assert_eq!(stdout(&first), stdout(&second));
}

// ---- Phase 2: Markdown traceability report ----

const GENERATED: &str = "2026-10-31T09:00:00Z";

// Columns of a requirement row, counting the empty cell before the first pipe as index 0
const LEVEL: usize = 2;
const STATUS: usize = 3;
const NORMAL_FLAG: usize = 4;
const ROBUST_FLAG: usize = 5;
const NORMAL_COL: usize = 6;
const ROBUST_COL: usize = 7;
const OTHER_COL: usize = 8;

/// One link of the trace matrix, written as a YAML list item.
fn link(
    req: &str,
    level: &str,
    cases: &[&str],
    normal: bool,
    robust: bool,
    runs: &[&str],
) -> String {
    let list = |v: &[&str]| {
        v.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "  - requirement_id: {req}\n    level: {level}\n    test_case_ids: [{}]\n    \
         normal_range_covered: {normal}\n    robustness_covered: {robust}\n    run_ids: [{}]\n",
        list(cases),
        list(runs)
    )
}

/// A matrix file with the given links and an empty runs directory.
fn project(links: &[String]) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let matrix = dir.path().join("matrix.yaml");
    let yaml = format!(
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nlinks:\n{}",
        links.concat()
    );
    std::fs::write(&matrix, yaml).unwrap();
    let runs = dir.path().join("runs");
    std::fs::create_dir(&runs).unwrap();
    (dir, matrix, runs)
}

/// A test case file; `typed` gives (test_type, test_category), `None` leaves the case untyped.
fn case_yaml(id: &str, requirement: &str, typed: Option<(&str, &str)>, obsolete: bool) -> String {
    let mut yaml = format!(
        "type: test_case\nschema: tcms/test-case.schema.v1.json\n\
         requirement: LEGACY-NOT-USED\nitem: 1\ntc: 1\nid: {id}\ndescription: d\n\
         requirements: [{requirement}]\n"
    );
    if let Some((test_type, category)) = typed {
        yaml.push_str(&format!(
            "test_type: {test_type}\ntest_category: {category}\n"
        ));
    }
    if obsolete {
        yaml.push_str("status: obsolete\n");
    }
    yaml.push_str("general_initial_conditions: {}\ninitial_conditions: {}\ntest_sequences: []\n");
    yaml
}

/// Runs the report with the fixture's matrix and runs, a fixed generation time and extra args.
fn render(matrix: &Path, runs: &Path, extra: &[&str]) -> Output {
    let matrix = path_arg(matrix);
    let runs = path_arg(runs);
    let mut args = vec![
        "--matrix",
        matrix.as_str(),
        "--runs",
        runs.as_str(),
        "--generated-at",
        GENERATED,
    ];
    args.extend_from_slice(extra);
    report(&args)
}

/// The cells of the requirement row starting with `| <id> |`, trimmed; index 0 is empty.
fn row<'a>(out: &'a str, id: &str) -> Vec<&'a str> {
    let prefix = format!("| {id} |");
    let line = out
        .lines()
        .find(|l| l.starts_with(prefix.as_str()))
        .unwrap_or_else(|| panic!("no row for {id} in:\n{out}"));
    line.split('|').map(str::trim).collect()
}

/// The `Field | Value` line of the report header for `field`.
fn header_value<'a>(out: &'a str, field: &str) -> &'a str {
    let prefix = format!("| {field} |");
    out.lines()
        .find(|l| l.starts_with(prefix.as_str()))
        .unwrap_or_else(|| panic!("no header row {field} in:\n{out}"))
}

/// Overwrites the matrix with the given header lines (before `links:`) and one HLR-1 link.
fn matrix_with_header(matrix: &Path, header: &str) {
    let yaml = format!(
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\n{header}links:\n{}",
        link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])
    );
    std::fs::write(matrix, yaml).unwrap();
}

#[test]
fn blank_baseline_renders_not_recorded() {
    let (_dir, matrix, runs) = project(&[]);
    matrix_with_header(&matrix, "id: TM-1\nbaseline_id: \"\"\n");
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("| Baseline | not recorded |"), "{out}");
    assert!(out.contains("(baseline: not recorded)"), "{out}");
    assert!(!out.contains("| Baseline |  |"), "{out}");
    assert!(!out.contains("(baseline: )"), "{out}");
}

#[test]
fn blank_matrix_id_renders_not_recorded() {
    let (_dir, matrix, runs) = project(&[]);
    matrix_with_header(&matrix, "id: \"   \"\n");
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(
        header_value(&out, "Matrix"),
        "| Matrix | not recorded |",
        "{out}"
    );
    assert!(out.contains("Trace matrix: not recorded"), "{out}");
}

/// A StrictDoc export: srs.sdoc holds HLR-1, LLR-2 and HLR-3; empty.sdoc holds one system item.
fn export_file(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("export.json");
    let json = r#"{"documents":[
      {"path":"docs/srs.sdoc","requirements":[
        {"uid":"HLR-1","level":"HLR","title":"t","statement":"s","status":"approved"},
        {"uid":"LLR-2","level":"LLR","title":"t","statement":"s","status":"draft"},
        {"uid":"HLR-3","level":"HLR","title":"t","statement":"s","status":"approved"}]},
      {"path":"docs/empty.sdoc","requirements":[
        {"uid":"SYS-9","level":"system","title":"t","statement":"s"}]}]}"#;
    std::fs::write(&path, json).unwrap();
    path
}

#[test]
fn traceability_report_is_one_markdown_document_with_inputs_first() {
    let (dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(out.lines().next(), Some("# Traceability Report"), "{out}");
    let inputs = out.find("## Inputs").unwrap();
    let header = out.find("## Report header").unwrap();
    assert!(inputs < header, "{out}");
    let section = &out[inputs..header];
    for needle in [
        "```",
        "Trace matrix: TM-1 (baseline: not recorded)",
        "Reconciliation",
        "Run selection",
    ] {
        assert!(section.contains(needle), "{section}");
    }

    let out_file = dir.path().join("report.md");
    let written = render(&matrix, &runs, &["--output", &path_arg(&out_file)]);
    assert!(written.status.success(), "{}", stderr(&written));
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert_eq!(text, out);
}

#[test]
fn verified_requirement_shows_each_passing_case() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let cells = row(&out, "HLR-1");
    assert_eq!(cells[STATUS], "VERIFIED", "{out}");
    assert_eq!(cells[OTHER_COL], "T1 (pass, RUN-1)", "{out}");
}

#[test]
fn failed_requirement_shows_failed() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "fail")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(row(&out, "HLR-1")[STATUS], "FAILED", "{out}");
}

#[test]
fn errored_ranks_above_failed() {
    let (_dir, matrix, runs) =
        project(&[link("HLR-1", "HLR", &["T1", "T2"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", "fail"), ("T2", "error")],
        ),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(row(&out, "HLR-1")[STATUS], "ERRORED", "{out}");
}

#[test]
fn not_run_case_is_never_a_pass() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let cells = row(&out, "HLR-1");
    assert_eq!(cells[STATUS], "NOT RUN", "{out}");
    assert!(cells[OTHER_COL].contains("NOT RUN"), "{out}");
    assert!(!cells[OTHER_COL].contains("pass"), "{out}");
}

#[test]
fn partial_when_some_pass_and_the_rest_have_no_result() {
    let (_dir, matrix, runs) =
        project(&[link("HLR-1", "HLR", &["T1", "T2"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(row(&out, "HLR-1")[STATUS], "PARTIAL", "{out}");
}

#[test]
fn requirement_without_cases_shows_no_tests() {
    let (_dir, matrix, runs) = project(&[link("HLR-3", "HLR", &[], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let cells = row(&out, "HLR-3");
    assert_eq!(cells[STATUS], "NO TESTS", "{out}");
    assert_eq!(cells[NORMAL_COL], "-", "{out}");
    assert_eq!(cells[ROBUST_COL], "-", "{out}");
    assert_eq!(cells[OTHER_COL], "-", "{out}");
}

#[test]
fn coverage_is_shown_apart_from_status() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let cells = row(&out, "HLR-1");
    assert_eq!(cells[STATUS], "VERIFIED", "{out}");
    assert_eq!(cells[NORMAL_FLAG], "yes", "{out}");
    assert_eq!(cells[ROBUST_FLAG], "no", "{out}");
}

#[test]
fn obsolete_case_is_excluded_and_counted_with_test_cases() {
    let (dir, matrix, runs) =
        project(&[link("LLR-2", "LLR", &["T2", "T3"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T2", "pass"), ("T3", "fail")],
        ),
    );
    let cases = dir.path().join("cases");
    std::fs::create_dir(&cases).unwrap();
    write(&cases, "t2.yaml", &case_yaml("T2", "LLR-2", None, false));
    write(&cases, "t3.yaml", &case_yaml("T3", "LLR-2", None, true));

    let with = render(&matrix, &runs, &["--test-cases", &path_arg(&cases)]);
    assert!(with.status.success(), "{}", stderr(&with));
    let out = stdout(&with);
    let cells = row(&out, "LLR-2");
    assert_eq!(cells[STATUS], "VERIFIED", "{out}");
    assert!(cells[OTHER_COL].contains("T2 ("), "{out}");
    assert!(!out.contains("T3 ("), "{out}");
    assert_eq!(
        header_value(&out, "Obsolete test cases excluded"),
        "| Obsolete test cases excluded | 1 |",
        "{out}"
    );

    let without = render(&matrix, &runs, &[]);
    assert!(without.status.success(), "{}", stderr(&without));
    let out = stdout(&without);
    assert_eq!(row(&out, "LLR-2")[STATUS], "FAILED", "{out}");
    assert_eq!(
        header_value(&out, "Obsolete test cases excluded"),
        "| Obsolete test cases excluded | not recorded |",
        "{out}"
    );
}

#[test]
fn test_types_fill_their_columns() {
    let (dir, matrix, runs) = project(&[
        link("HLR-1", "HLR", &["T1"], true, true, &["RUN-1"]),
        link("LLR-2", "LLR", &["T2", "T3"], true, true, &["RUN-1"]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", "pass"), ("T2", "pass"), ("T3", "pass")],
        ),
    );
    let cases = dir.path().join("cases");
    std::fs::create_dir(&cases).unwrap();
    write(
        &cases,
        "t1.yaml",
        &case_yaml(
            "T1",
            "HLR-1",
            Some(("normal_range", "boundary_value")),
            false,
        ),
    );
    write(
        &cases,
        "t2.yaml",
        &case_yaml("T2", "LLR-2", Some(("robustness", "overflow")), false),
    );
    write(&cases, "t3.yaml", &case_yaml("T3", "LLR-2", None, false));

    let output = render(&matrix, &runs, &["--test-cases", &path_arg(&cases)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let hlr = row(&out, "HLR-1");
    assert!(hlr[NORMAL_COL].contains("T1 ("), "{out}");
    assert_eq!(hlr[ROBUST_COL], "-", "{out}");
    assert_eq!(hlr[OTHER_COL], "-", "{out}");
    let llr = row(&out, "LLR-2");
    assert_eq!(llr[NORMAL_COL], "-", "{out}");
    assert!(llr[ROBUST_COL].contains("T2 ("), "{out}");
    assert!(llr[OTHER_COL].contains("T3 ("), "{out}");
}

#[test]
fn hlr_and_llr_levels_are_shown_distinctly() {
    let (_dir, matrix, runs) = project(&[
        link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"]),
        link("LLR-2", "LLR", &["T2"], true, false, &["RUN-1"]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", "pass"), ("T2", "pass")],
        ),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(row(&out, "HLR-1")[LEVEL], "HLR", "{out}");
    assert_eq!(row(&out, "LLR-2")[LEVEL], "LLR", "{out}");
}

#[test]
fn documents_group_rows_and_mark_a_document_without_requirements() {
    let (dir, matrix, runs) = project(&[
        link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"]),
        link("LLR-2", "LLR", &["T2"], true, false, &["RUN-1"]),
        link("HLR-3", "HLR", &[], true, false, &["RUN-1"]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-28T10:00:00Z",
            &[("T1", "pass"), ("T2", "pass")],
        ),
    );
    let export = export_file(dir.path());

    let output = render(
        &matrix,
        &runs,
        &["--requirements-export", &path_arg(&export)],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let srs = out
        .find("## Document: docs/srs.sdoc")
        .unwrap_or_else(|| panic!("{out}"));
    let hlr1 = out.find("| HLR-1 |").unwrap_or_else(|| panic!("{out}"));
    let empty = out
        .find("## Document: docs/empty.sdoc")
        .unwrap_or_else(|| panic!("{out}"));
    // Sections are sorted by document name (D-12): docs/empty.sdoc sorts before docs/srs.sdoc
    assert!(empty < srs && srs < hlr1, "{out}");
    assert!(
        out[empty..].starts_with("## Document: docs/empty.sdoc\n\nNo traceable content"),
        "{out}"
    );
    assert!(!out.contains("## Document: not recorded"), "{out}");
}

#[test]
fn rows_without_an_export_sit_under_document_not_recorded() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("## Document: not recorded"), "{out}");
    assert!(row(&out, "HLR-1")[STATUS] == "VERIFIED", "{out}");
}

#[test]
fn header_records_inputs_and_generation_time() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = render(&matrix, &runs, &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert_eq!(header_value(&out, "Matrix"), "| Matrix | TM-1 |", "{out}");
    assert_eq!(
        header_value(&out, "Baseline"),
        "| Baseline | not recorded |",
        "{out}"
    );
    assert_eq!(header_value(&out, "Runs"), "| Runs | RUN-1 |", "{out}");
    assert_eq!(header_value(&out, "Builds"), "| Builds | b1 |", "{out}");
    assert_eq!(
        header_value(&out, "Environments"),
        "| Environments | e |",
        "{out}"
    );
    assert_eq!(header_value(&out, "Testers"), "| Testers | t |", "{out}");
    assert_eq!(
        header_value(&out, "Generated"),
        "| Generated | 2026-10-31T09:00:00Z |",
        "{out}"
    );
}

#[test]
fn same_inputs_and_generated_at_give_identical_bytes() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let first = render(&matrix, &runs, &[]);
    let second = render(&matrix, &runs, &[]);
    assert!(first.status.success(), "{}", stderr(&first));
    assert_eq!(first.stdout, second.stdout);

    let matrix_arg = path_arg(&matrix);
    let runs_arg = path_arg(&runs);
    let later = report(&[
        "--matrix",
        &matrix_arg,
        "--runs",
        &runs_arg,
        "--generated-at",
        "2026-11-01T09:00:00Z",
    ]);
    assert!(later.status.success(), "{}", stderr(&later));
    let (a, b) = (stdout(&first), stdout(&later));
    let (a_lines, b_lines): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    assert_eq!(a_lines.len(), b_lines.len(), "{a}\n---\n{b}");
    for (x, y) in a_lines.iter().zip(&b_lines) {
        if x != y {
            assert!(
                x.starts_with("| Generated |") && y.starts_with("| Generated |"),
                "{x}\n{y}"
            );
        }
    }
}

#[test]
fn generated_at_defaults_to_current_utc_time() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );

    let output = report(&["--matrix", &path_arg(&matrix), "--runs", &path_arg(&runs)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    let value = header_value(&out, "Generated")
        .trim_start_matches("| Generated | ")
        .trim_end_matches(" |");
    let parsed =
        chrono::DateTime::parse_from_rfc3339(value).unwrap_or_else(|e| panic!("{value}: {e}"));
    assert!(value.ends_with('Z'), "{value}");
    let age = chrono::Utc::now()
        .signed_duration_since(parsed)
        .num_seconds();
    assert!((0..300).contains(&age), "{value} is {age} seconds from now");
}

#[test]
fn invalid_generated_at_fails_naming_it_and_writes_nothing() {
    let (dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    let out_file = dir.path().join("report.md");

    let matrix_arg = path_arg(&matrix);
    let runs_arg = path_arg(&runs);
    let output = report(&[
        "--matrix",
        &matrix_arg,
        "--runs",
        &runs_arg,
        "--generated-at",
        "yesterday",
        "--output",
        &path_arg(&out_file),
    ]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("--generated-at"),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("yesterday"), "{}", stderr(&output));
    assert!(!out_file.exists());
}

#[test]
fn unreadable_test_case_directory_fails_naming_it_and_writes_nothing() {
    let (dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    let missing = dir.path().join("no-such-cases");
    let out_file = dir.path().join("report.md");

    let output = render(
        &matrix,
        &runs,
        &[
            "--test-cases",
            &path_arg(&missing),
            "--output",
            &path_arg(&out_file),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&path_arg(&missing)),
        "{}",
        stderr(&output)
    );
    assert!(!out_file.exists());
}

#[test]
fn invalid_requirements_export_fails_naming_it_and_writes_nothing() {
    let (dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    let bad = dir.path().join("bad-export.json");
    std::fs::write(&bad, "this is not json").unwrap();
    let out_file = dir.path().join("report.md");

    let output = render(
        &matrix,
        &runs,
        &[
            "--requirements-export",
            &path_arg(&bad),
            "--output",
            &path_arg(&out_file),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(
        stderr(&output).contains(&path_arg(&bad)),
        "{}",
        stderr(&output)
    );
    assert!(!out_file.exists());
}
