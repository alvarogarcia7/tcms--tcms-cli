//! End-to-end checks for the Phase 2 Markdown traceability report: the roll-up grid, `--shape a|b`,
//! the "not recorded" header rule and determinism. Every test runs the real binary
//! (`CARGO_BIN_EXE_test-executor`) against fixtures shaped like the ones in `report_cli.rs`, and
//! asserts exact output lines and exit codes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Fixed generation time used by every render that does not test the timestamp.
const GENERATED: &str = "2026-10-31T09:00:00Z";

/// Two generation times used to check that only the `| Generated |` row varies (D-03-G3).
const EARLY: &str = "2026-11-01T09:00:00Z";
const LATE: &str = "2026-11-02T09:00:00Z";

// Cells of a requirement row of the grid, split on '|'; index 0 is the empty text before the
// first pipe.
const STATUS: usize = 3;
const OTHER_COL: usize = 8;

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

fn write(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

/// One link of the trace matrix, written as a YAML list item (same shape as `link()` in report_cli.rs).
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
fn project(links: &[String]) -> (tempfile::TempDir, PathBuf, PathBuf) {
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

/// A run file with (test case, verdict) results (same shape as `run_yaml()` in report_cli.rs).
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

/// Renders the report of a fixture with the given generation time and extra arguments.
fn render(matrix: &Path, runs: &Path, generated_at: &str, extra: &[&str]) -> Output {
    let matrix = path_arg(matrix);
    let runs = path_arg(runs);
    let mut args = vec![
        "--matrix",
        matrix.as_str(),
        "--runs",
        runs.as_str(),
        "--generated-at",
        generated_at,
    ];
    args.extend_from_slice(extra);
    report(&args)
}

/// Stdout of a run that must exit 0 with nothing on stderr.
fn ok(output: &Output) -> String {
    assert_eq!(output.status.code(), Some(0), "{}", stderr(output));
    assert!(stderr(output).is_empty(), "{}", stderr(output));
    stdout(output)
}

/// The cells of the grid row starting with `| <id> |`, trimmed; index 0 is empty.
fn grid_row<'a>(out: &'a str, id: &str) -> Vec<&'a str> {
    let prefix = format!("| {id} |");
    let line = out
        .lines()
        .find(|l| l.starts_with(prefix.as_str()))
        .unwrap_or_else(|| panic!("no row for {id} in:\n{out}"));
    line.split('|').map(str::trim).collect()
}

/// Asserts that a whole line of the output is exactly `line`.
fn has_line(out: &str, line: &str) {
    assert!(
        out.lines().any(|l| l == line),
        "missing line {line:?} in:\n{out}"
    );
}

/// Every requirement row (`| HLR-n |` or `| LLR-n |`) of the output, in order.
fn requirement_rows(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.starts_with("| HLR-") || l.starts_with("| LLR-"))
        .collect()
}

/// The three renders of one fixture: the grid, shape A and shape B.
fn all_shapes() -> [&'static [&'static str]; 3] {
    [
        &["--kind", "traceability"],
        &["--kind", "traceability", "--shape", "a"],
        &["--kind", "traceability", "--shape", "b"],
    ]
}

/// Six requirements, one per roll-up status, and RUN-1 (b1) results that produce each one:
/// HLR-V (TV pass) VERIFIED, HLR-F (TF fail) FAILED, HLR-E (TE error) ERRORED,
/// HLR-N (TN, no result) NOT RUN, HLR-P (TP1 pass, TP2 no result) PARTIAL, HLR-Z (no cases) NO TESTS.
fn status_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (dir, matrix, runs) = project(&[
        link("HLR-V", "HLR", &["TV"], true, false, &["RUN-1"]),
        link("HLR-F", "HLR", &["TF"], true, false, &["RUN-1"]),
        link("HLR-E", "HLR", &["TE"], true, false, &["RUN-1"]),
        link("HLR-N", "HLR", &["TN"], true, false, &["RUN-1"]),
        link("HLR-P", "HLR", &["TP1", "TP2"], true, false, &["RUN-1"]),
        link("HLR-Z", "HLR", &[], false, false, &[]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[
                ("TV", "pass"),
                ("TF", "fail"),
                ("TE", "error"),
                ("TP1", "pass"),
            ],
        ),
    );
    (dir, matrix, runs)
}

/// The Phase 3 scenario from report_cli.rs without problem reports. HLR-1 (T1, T2) has T1 pass
/// and T2 fail in RUN-1, LLR-2 (T3) has T3 error, HLR-3 has no cases, LLR-4 (T4) has no result.
/// RUN-2 (b2) only holds T2 not_executed and T5 pass (T5 is not in the matrix).
fn scenario_fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let (dir, matrix, runs) = project(&[
        link(
            "HLR-1",
            "HLR",
            &["T1", "T2"],
            true,
            true,
            &["RUN-1", "RUN-2"],
        ),
        link("LLR-2", "LLR", &["T3"], true, false, &["RUN-1"]),
        link("HLR-3", "HLR", &[], false, false, &[]),
        link("LLR-4", "LLR", &["T4"], false, true, &[]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[("T1", "pass"), ("T2", "fail"), ("T3", "error")],
        ),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml(
            "RUN-2",
            "b2",
            "2026-10-02T10:00:00Z",
            &[("T2", "not_executed"), ("T5", "pass")],
        ),
    );
    (dir, matrix, runs)
}

// ---- Roll-up grid ----

#[test]
fn grid_rolls_up_each_status_on_its_own_requirement() {
    let (_dir, matrix, runs) = status_fixture();
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability"],
    ));
    assert_eq!(grid_row(&out, "HLR-V")[STATUS], "VERIFIED", "{out}");
    assert_eq!(grid_row(&out, "HLR-F")[STATUS], "FAILED", "{out}");
    assert_eq!(grid_row(&out, "HLR-E")[STATUS], "ERRORED", "{out}");
    assert_eq!(grid_row(&out, "HLR-N")[STATUS], "NOT RUN", "{out}");
    assert_eq!(grid_row(&out, "HLR-P")[STATUS], "PARTIAL", "{out}");
    assert_eq!(grid_row(&out, "HLR-Z")[STATUS], "NO TESTS", "{out}");
    has_line(
        &out,
        "| HLR-V | HLR | VERIFIED | yes | no | - | - | TV (pass, RUN-1) |",
    );
    has_line(
        &out,
        "| HLR-N | HLR | NOT RUN | yes | no | - | - | TN (NOT RUN, no result) |",
    );
    has_line(
        &out,
        "| HLR-P | HLR | PARTIAL | yes | no | - | - | TP1 (pass, RUN-1)<br>TP2 (NOT RUN, no result) |",
    );
    has_line(&out, "| HLR-Z | HLR | NO TESTS | no | no | - | - | - |");
    assert_eq!(requirement_rows(&out).len(), 6, "{out}");
}

#[test]
fn grid_scenario_rolls_up_failed_errored_no_tests_and_not_run() {
    let (_dir, matrix, runs) = scenario_fixture();
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability"],
    ));
    assert_eq!(grid_row(&out, "HLR-1")[STATUS], "FAILED", "{out}");
    assert_eq!(grid_row(&out, "LLR-2")[STATUS], "ERRORED", "{out}");
    assert_eq!(grid_row(&out, "HLR-3")[STATUS], "NO TESTS", "{out}");
    assert_eq!(grid_row(&out, "LLR-4")[STATUS], "NOT RUN", "{out}");
    // A RUN-2 not_executed result does not override the RUN-1 fail for T2.
    assert_eq!(
        grid_row(&out, "HLR-1")[OTHER_COL],
        "T1 (pass, RUN-1)<br>T2 (fail, RUN-1)",
        "{out}"
    );
}

#[test]
fn grid_error_outranks_fail_and_fail_outranks_a_missing_result() {
    let (_dir, matrix, runs) = project(&[
        link("HLR-1", "HLR", &["T1", "T2"], true, false, &["RUN-1"]),
        link("HLR-2", "HLR", &["T3", "T4"], true, false, &["RUN-1"]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[("T1", "fail"), ("T2", "error"), ("T3", "fail")],
        ),
    );
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability"],
    ));
    assert_eq!(grid_row(&out, "HLR-1")[STATUS], "ERRORED", "{out}");
    assert_eq!(grid_row(&out, "HLR-2")[STATUS], "FAILED", "{out}");
}

// ---- Shape A ----

#[test]
fn shape_a_exact_rows_for_each_roll_up_status() {
    let (_dir, matrix, runs) = status_fixture();
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability", "--shape", "a"],
    ));
    has_line(
        &out,
        "| Requirement | Test cases (ID, status) | Final verdict |",
    );
    has_line(&out, "| HLR-V | TV (pass) | VERIFIED |");
    has_line(&out, "| HLR-F | TF (fail) | FAILED |");
    has_line(&out, "| HLR-E | TE (error) | ERRORED |");
    has_line(&out, "| HLR-N | TN (NOT RUN) | NOT RUN |");
    has_line(&out, "| HLR-P | TP1 (pass)<br>TP2 (NOT RUN) | PARTIAL |");
    has_line(&out, "| HLR-Z | - | NO TESTS |");
    assert_eq!(requirement_rows(&out).len(), 6, "{out}");
    assert!(!out.contains("| normal_range |"), "{out}");
}

#[test]
fn shape_a_scenario_rows_show_each_case_with_its_status() {
    let (_dir, matrix, runs) = scenario_fixture();
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability", "--shape", "a"],
    ));
    has_line(&out, "| HLR-1 | T1 (pass)<br>T2 (fail) | FAILED |");
    has_line(&out, "| LLR-2 | T3 (error) | ERRORED |");
    has_line(&out, "| HLR-3 | - | NO TESTS |");
    has_line(&out, "| LLR-4 | T4 (NOT RUN) | NOT RUN |");
}

// ---- Shape B ----

#[test]
fn shape_b_exact_case_status_and_verdict_rows() {
    let (_dir, matrix, runs) = status_fixture();
    let out = ok(&render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability", "--shape", "b"],
    ));
    has_line(&out, "| TESTCASE ID | Status |");
    has_line(&out, "| TV | pass |");
    has_line(&out, "| TF | fail |");
    has_line(&out, "| TE | error |");
    has_line(&out, "| TN | NOT RUN |");
    has_line(&out, "| TP1 | pass |");
    has_line(&out, "| TP2 | NOT RUN |");
    has_line(&out, "| HLR-V | VERIFIED |");
    has_line(&out, "| HLR-F | FAILED |");
    has_line(&out, "| HLR-E | ERRORED |");
    has_line(&out, "| HLR-N | NOT RUN |");
    has_line(&out, "| HLR-P | PARTIAL |");
    has_line(&out, "| HLR-Z | NO TESTS |");
    // One verdict table per requirement, and no grid columns.
    assert_eq!(
        out.matches("| Requirement | Final verdict |").count(),
        6,
        "{out}"
    );
    assert!(!out.contains("| Requirement | Level | Status |"), "{out}");
}

// ---- Usage errors ----

/// A usage error: exit code 2, nothing on stdout, and stderr names every option in `needles`.
fn assert_usage_error(output: &Output, needles: &[&str]) {
    assert_eq!(output.status.code(), Some(2), "{}", stderr(output));
    assert!(stdout(output).is_empty(), "{}", stdout(output));
    let err = stderr(output);
    for needle in needles {
        assert!(err.contains(needle), "stderr lacks {needle}: {err}");
    }
}

#[test]
fn shape_with_a_conflicting_kind_exits_2_naming_both_options() {
    let (_dir, matrix, runs) = status_fixture();
    let cases: [&[&str]; 5] = [
        &["--shape", "a"],
        &["--shape", "b", "--kind", "summary"],
        &["--shape", "a", "--kind", "gaps"],
        &["--shape", "b", "--kind", "problems"],
        &[
            "--shape",
            "a",
            "--kind",
            "traceability",
            "--kind",
            "problems",
        ],
    ];
    for extra in cases {
        let output = render(&matrix, &runs, GENERATED, extra);
        assert_usage_error(&output, &["--shape", "--kind", "traceability"]);
    }
}

#[test]
fn unknown_shape_value_exits_2_naming_the_value() {
    let (_dir, matrix, runs) = status_fixture();
    let output = render(
        &matrix,
        &runs,
        GENERATED,
        &["--kind", "traceability", "--shape", "xyz"],
    );
    assert_usage_error(&output, &["--shape", "xyz"]);
}

// ---- Blank header values ----

#[test]
fn blank_baseline_renders_not_recorded_in_every_shape() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    std::fs::write(
        &matrix,
        format!(
            "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nbaseline_id: \"\"\nlinks:\n{}",
            link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])
        ),
    )
    .unwrap();
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    for extra in all_shapes() {
        let out = ok(&render(&matrix, &runs, GENERATED, extra));
        has_line(&out, "| Baseline | not recorded |");
        has_line(&out, "Trace matrix: TM-1 (baseline: not recorded)");
        assert!(!out.contains("| Baseline |  |"), "{out}");
        assert!(!out.contains("(baseline: )"), "{out}");
    }
}

#[test]
fn blank_matrix_id_renders_not_recorded_in_grid_and_shape_b() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])]);
    std::fs::write(
        &matrix,
        format!(
            "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: \"   \"\nlinks:\n{}",
            link("HLR-1", "HLR", &["T1"], true, false, &["RUN-1"])
        ),
    )
    .unwrap();
    write(
        &runs,
        "run1.yaml",
        &run_yaml("RUN-1", "b1", "2026-10-28T10:00:00Z", &[("T1", "pass")]),
    );
    for extra in [
        &["--kind", "traceability"][..],
        &["--kind", "traceability", "--shape", "b"][..],
    ] {
        let out = ok(&render(&matrix, &runs, GENERATED, extra));
        has_line(&out, "| Matrix | not recorded |");
        has_line(&out, "Trace matrix: not recorded (baseline: not recorded)");
        assert!(!out.contains("Trace matrix:  "), "{out}");
    }
}

// ---- Determinism ----

#[test]
fn same_inputs_and_generated_at_give_identical_bytes_in_every_shape() {
    let (dir, matrix, runs) = status_fixture();
    for extra in all_shapes() {
        let first = ok(&render(&matrix, &runs, GENERATED, extra));
        let second = ok(&render(&matrix, &runs, GENERATED, extra));
        assert_eq!(first.as_bytes(), second.as_bytes(), "{extra:?}");
    }
    // The file written with --output is the same document as stdout.
    let out_file = dir.path().join("report.md");
    let shape_b = all_shapes()[2];
    let mut args: Vec<&str> = shape_b.to_vec();
    let out_path = path_arg(&out_file);
    args.extend_from_slice(&["--output", out_path.as_str()]);
    let written = render(&matrix, &runs, GENERATED, &args);
    assert_eq!(written.status.code(), Some(0), "{}", stderr(&written));
    let text = std::fs::read_to_string(&out_file).unwrap();
    assert_eq!(text, ok(&render(&matrix, &runs, GENERATED, shape_b)));
}

#[test]
fn different_generated_at_differs_only_in_the_generated_row() {
    let (_dir, matrix, runs) = status_fixture();
    for extra in all_shapes() {
        let early = ok(&render(&matrix, &runs, EARLY, extra));
        let late = ok(&render(&matrix, &runs, LATE, extra));
        assert_ne!(early, late, "{extra:?}");
        let strip = |s: &str| {
            s.lines()
                .filter(|l| !l.starts_with("| Generated |"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(strip(&early), strip(&late), "{extra:?}");
    }
}

#[test]
fn generated_row_holds_each_generation_time_exactly_once() {
    let (_dir, matrix, runs) = status_fixture();
    for extra in all_shapes() {
        let early = ok(&render(&matrix, &runs, EARLY, extra));
        let late = ok(&render(&matrix, &runs, LATE, extra));
        has_line(&early, "| Generated | 2026-11-01T09:00:00Z |");
        has_line(&late, "| Generated | 2026-11-02T09:00:00Z |");
        assert_eq!(early.matches("| Generated |").count(), 1, "{early}");
        assert_eq!(late.matches("| Generated |").count(), 1, "{late}");
        assert!(!early.contains("2026-11-02T09:00:00Z"), "{early}");
        assert!(!late.contains("2026-11-01T09:00:00Z"), "{late}");
    }
}
