//! End-to-end checks for the Phase 3 `test-executor report` kinds: `summary`, `gaps` and
//! `problems`. Every test runs the real binary (`CARGO_BIN_EXE_test-executor`) against the
//! Phase 3 scenario, which is the same fixture as `scenario()` in `report_cli.rs`, and asserts
//! exact output lines and exit codes.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Fixed generation time used by every render that does not test the timestamp.
const GENERATED: &str = "2026-10-31T09:00:00Z";

/// Two generation times used to check that only the `| Generated |` row varies (D-03-G3).
const EARLY: &str = "2026-11-01T09:00:00Z";
const LATE: &str = "2026-11-02T09:00:00Z";

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

/// Same shape as `link()` in report_cli.rs.
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

/// Same shape as `run_yaml_pr()` in report_cli.rs; `problem` adds a problem_report_id.
fn run_yaml(
    run_id: &str,
    build_id: &str,
    started_at: &str,
    results: &[(&str, &str, Option<&str>)],
) -> String {
    let mut yaml = format!(
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\nbuild_id: {build_id}\n\
         environment: e\ntester: t\nstarted_at: '{started_at}'\nresults:\n"
    );
    if results.is_empty() {
        yaml = yaml.replace("results:\n", "results: []\n");
    }
    for (tc, verdict, problem) in results {
        match problem {
            Some(pr) => yaml.push_str(&format!(
                "  - {{test_case_id: {tc}, verdict: {verdict}, problem_report_id: {pr}}}\n"
            )),
            None => yaml.push_str(&format!("  - {{test_case_id: {tc}, verdict: {verdict}}}\n")),
        }
    }
    yaml
}

/// The Phase 3 scenario from report_cli.rs. Matrix: HLR-1 (T1, T2; both coverage flags),
/// LLR-2 (T3; normal range only), HLR-3 (no cases), LLR-4 (T4; robustness only, no result).
/// RUN-1 (b1) holds T1 pass, T2 fail with PR-7 and T3 error; RUN-2 (b2) holds T2 not_executed
/// and T5 pass (T5 is not in the matrix). `t3_problem` is the problem report of T3's error.
fn scenario(t3_problem: Option<&str>) -> (tempfile::TempDir, PathBuf, PathBuf) {
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
            &[
                ("T1", "pass", None),
                ("T2", "fail", Some("PR-7")),
                ("T3", "error", t3_problem),
            ],
        ),
    );
    write(
        &runs,
        "run2.yaml",
        &run_yaml(
            "RUN-2",
            "b2",
            "2026-10-02T10:00:00Z",
            &[("T2", "not_executed", None), ("T5", "pass", None)],
        ),
    );
    (dir, matrix, runs)
}

/// Renders one kind of the scenario with the fixed generation time plus extra arguments.
fn render(matrix: &Path, runs: &Path, kind: &str, generated_at: &str) -> Output {
    let matrix = path_arg(matrix);
    let runs = path_arg(runs);
    report(&[
        "--matrix",
        matrix.as_str(),
        "--runs",
        runs.as_str(),
        "--kind",
        kind,
        "--generated-at",
        generated_at,
    ])
}

/// Renders one kind and asserts success, returning stdout.
fn render_ok(matrix: &Path, runs: &Path, kind: &str) -> String {
    let output = render(matrix, runs, kind, GENERATED);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    stdout(&output)
}

/// The text from `heading` to the end of the output, for the last document of a report.
fn tail<'a>(out: &'a str, heading: &str) -> &'a str {
    let start = out
        .find(heading)
        .unwrap_or_else(|| panic!("no `{heading}` in:\n{out}"));
    &out[start..]
}

/// The text of one section: from `heading` up to the next `## ` heading (or the end).
fn section<'a>(out: &'a str, heading: &str) -> &'a str {
    let start = out
        .find(heading)
        .unwrap_or_else(|| panic!("no `{heading}` in:\n{out}"));
    let body = &out[start + heading.len()..];
    let end = body.find("\n## ").map(|i| i + 1).unwrap_or(body.len());
    &out[start..start + heading.len() + end]
}

/// The cells of every data row of the Markdown table in `text`, header and rule excluded.
/// Index 0 is the empty cell before the first pipe.
fn table_rows(text: &str) -> Vec<Vec<String>> {
    text.lines()
        .filter(|l| l.starts_with('|') && !l.starts_with("| ---"))
        .skip(1)
        .map(|l| l.split('|').map(|c| c.trim().to_string()).collect())
        .collect()
}

/// Asserts that `out` contains a line equal to `line`.
fn assert_line(out: &str, line: &str) {
    assert!(
        out.lines().any(|l| l == line),
        "missing exact line `{line}` in:\n{out}"
    );
}

/// Lines of the output that differ between two renders, as (left, right) pairs.
fn differing_lines<'a>(a: &'a str, b: &'a str) -> Vec<(&'a str, &'a str)> {
    let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    assert_eq!(a.len(), b.len(), "line counts differ\n{a:?}\n{b:?}");
    a.into_iter().zip(b).filter(|(x, y)| x != y).collect()
}

// ---------------------------------------------------------------- summary

#[test]
fn summary_exact_rows_and_generated_line() {
    let (_dir, matrix, runs) = scenario(None);
    let out = render_ok(&matrix, &runs, "summary");

    assert!(
        out.starts_with("Trace matrix: TM-1 (baseline: not recorded)\n"),
        "{out}"
    );
    assert_line(&out, "# Run Summary");
    assert_line(&out, "| Generated | 2026-10-31T09:00:00Z |");
    assert_line(&out, "| Runs | RUN-1, RUN-2 |");
    assert_line(&out, "| Builds | b1, b2 |");
    assert_line(&out, "  Selected results: 4");
    assert_line(&out, "    T1  pass  RUN-1");
    assert_line(&out, "    T2  fail  RUN-1");
    assert_line(&out, "    T3  error  RUN-1");
    assert_line(&out, "    T5  pass  RUN-2");

    assert_line(&out, "| RUN-1 | b1 | yes | 1 | 1 | 0 | 1 | 3 |");
    assert_line(&out, "| RUN-2 | b2 | yes | 1 | 0 | 1 | 0 | 2 |");
    assert_line(&out, "| HLR-1 | HLR | FAILED | 1 | 1 | 0 | 0 | 2 |");
    assert_line(&out, "| HLR-3 | HLR | NO TESTS | 0 | 0 | 0 | 0 | 0 |");
    assert_line(&out, "| LLR-2 | LLR | ERRORED | 0 | 0 | 0 | 1 | 1 |");
    assert_line(&out, "| LLR-4 | LLR | NOT RUN | 0 | 0 | 1 | 0 | 1 |");
    assert_line(&out, "pass 1, fail 1, not_executed 1, error 1, total 4");
}

#[test]
fn summary_buckets_sum_to_totals_and_match_linked_results() {
    let (_dir, matrix, runs) = scenario(None);
    let out = render_ok(&matrix, &runs, "summary");

    // Every per-run and per-requirement row: pass + fail + not_executed + error == Total.
    let run_rows = table_rows(section(&out, "## Results per run"));
    let req_rows = table_rows(section(&out, "## Results per requirement"));
    assert_eq!(run_rows.len(), 2, "{out}");
    assert_eq!(req_rows.len(), 4, "{out}");
    for cells in run_rows.iter().chain(req_rows.iter()) {
        let n: Vec<usize> = cells[4..=8].iter().map(|c| c.parse().unwrap()).collect();
        assert_eq!(n[0] + n[1] + n[2] + n[3], n[4], "row {cells:?}\n{out}");
    }

    // The per-requirement totals cover each matrix-linked test case exactly once, so they
    // sum to the linked total (T1 to T4). T5 is not linked and must not appear.
    let linked_total: usize = section(&out, "## Linked test cases")
        .lines()
        .find(|l| l.starts_with("pass "))
        .and_then(|l| l.rsplit("total ").next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no linked total\n{out}"));
    assert_eq!(linked_total, 4, "{out}");
    let req_total: usize = req_rows
        .iter()
        .map(|c| c[8].parse::<usize>().unwrap())
        .sum();
    assert_eq!(req_total, linked_total, "{out}");
    assert!(!out.contains("| T5 |"), "{out}");
}

#[test]
fn summary_exit_zero_and_no_stderr() {
    let (_dir, matrix, runs) = scenario(None);
    let output = render(&matrix, &runs, "summary", GENERATED);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

// ------------------------------------------------------------------- gaps

#[test]
fn gaps_exact_rows_for_each_category() {
    let (_dir, matrix, runs) = scenario(None);
    let out = render_ok(&matrix, &runs, "gaps");

    assert_line(&out, "# Coverage Gaps");
    assert_line(&out, "## Requirements with no test cases (1)");
    assert_line(&out, "## Requirements missing a normal-range test (2)");
    assert_line(&out, "## Requirements missing a robustness test (2)");
    assert_line(&out, "## Test cases with no recorded result (1)");

    let rows = |heading: &str| -> Vec<Vec<String>> {
        table_rows(section(&out, heading))
            .into_iter()
            .map(|cells| cells[1..cells.len() - 1].to_vec())
            .collect()
    };
    assert_eq!(
        rows("## Requirements with no test cases"),
        vec![vec![
            "HLR-3".to_string(),
            "HLR".into(),
            "not recorded".into()
        ]],
        "{out}"
    );
    assert_eq!(
        rows("## Requirements missing a normal-range test").len(),
        2,
        "{out}"
    );
    assert_line(&out, "| HLR-3 | HLR | not recorded |");
    assert_line(&out, "| LLR-4 | LLR | not recorded |");
    assert_line(&out, "| LLR-2 | LLR | not recorded |");
    assert_line(&out, "| T4 | LLR-4 |");
    assert!(!out.contains("| T1 |") && !out.contains("| T3 |"), "{out}");
}

#[test]
fn gaps_prints_none_for_each_empty_category() {
    let (_dir, matrix, runs) = project(&[
        link("HLR-1", "HLR", &["T1"], true, true, &["RUN-1"]),
        link("LLR-2", "LLR", &["T2"], true, true, &["RUN-1"]),
    ]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[("T1", "pass", None), ("T2", "pass", None)],
        ),
    );
    let out = render_ok(&matrix, &runs, "gaps");

    for heading in [
        "## Requirements with no test cases (0)",
        "## Requirements missing a normal-range test (0)",
        "## Requirements missing a robustness test (0)",
        "## Test cases with no recorded result (0)",
    ] {
        assert_line(&out, heading);
        let body = section(&out, heading);
        let after: Vec<&str> = body.lines().skip(1).filter(|l| !l.is_empty()).collect();
        assert_eq!(after, vec!["None"], "{heading}\n{out}");
    }
}

// --------------------------------------------------------------- problems

#[test]
fn problems_exact_rows_for_fail_and_error() {
    let (_dir, matrix, runs) = scenario(None);
    let out = render_ok(&matrix, &runs, "problems");

    assert_line(&out, "Failed or errored results: 2");
    assert_line(&out, "Without problem report: 1");
    assert_line(
        &out,
        "| Test case | Verdict | Run | Build | Problem report | Requirements |",
    );
    assert_line(&out, "| T2 | fail | RUN-1 | b1 | PR-7 | HLR-1 |");
    assert_line(
        &out,
        "| T3 | error | RUN-1 | b1 | **MISSING problem_report_id** | LLR-2 |",
    );

    // Only fail and error rows appear: no pass or not_executed case in the problem list.
    let problems = tail(&out, "# Problem Reports");
    assert!(!problems.contains("| T1 |"), "{out}");
    assert!(!problems.contains("| T4 |"), "{out}");
    assert!(!problems.contains("| T5 |"), "{out}");
}

#[test]
fn problems_missing_flag_is_on_the_error_row_only() {
    let (_dir, matrix, runs) = scenario(None);
    let out = render_ok(&matrix, &runs, "problems");
    let flagged: Vec<&str> = tail(&out, "# Problem Reports")
        .lines()
        .filter(|l| l.contains("MISSING problem_report_id"))
        .collect();
    assert_eq!(
        flagged,
        vec!["| T3 | error | RUN-1 | b1 | **MISSING problem_report_id** | LLR-2 |"],
        "{out}"
    );
}

#[test]
fn problems_missing_flag_clears_when_problem_report_is_recorded() {
    let (_dir, matrix, runs) = scenario(Some("PR-8"));
    let out = render_ok(&matrix, &runs, "problems");

    assert_line(&out, "Without problem report: 0");
    assert_line(&out, "| T3 | error | RUN-1 | b1 | PR-8 | LLR-2 |");
    assert!(
        !tail(&out, "# Problem Reports").contains("MISSING problem_report_id"),
        "{out}"
    );
}

#[test]
fn problems_with_no_failures_says_so() {
    let (_dir, matrix, runs) = project(&[link("HLR-1", "HLR", &["T1"], true, true, &["RUN-1"])]);
    write(
        &runs,
        "run1.yaml",
        &run_yaml(
            "RUN-1",
            "b1",
            "2026-10-01T10:00:00Z",
            &[("T1", "pass", None)],
        ),
    );
    let out = render_ok(&matrix, &runs, "problems");
    assert_line(&out, "Failed or errored results: 0");
    assert_line(&out, "No failed or errored results");
    assert_line(&out, "Without problem report: 0");
    assert!(!out.contains("| Test case | Verdict"), "{out}");
}

// ----------------------------------------------------------- determinism

#[test]
fn same_inputs_and_generated_at_give_byte_identical_output() {
    let (_dir, matrix, runs) = scenario(None);
    for kind in ["summary", "gaps", "problems"] {
        let first = render(&matrix, &runs, kind, GENERATED);
        let second = render(&matrix, &runs, kind, GENERATED);
        assert_eq!(first.status.code(), Some(0), "{kind}: {}", stderr(&first));
        assert_eq!(first.stdout, second.stdout, "{kind} is not byte-identical");
        assert_eq!(first.stderr, second.stderr, "{kind} stderr differs");
    }
}

#[test]
fn different_generated_at_changes_only_the_generated_row() {
    let (_dir, matrix, runs) = scenario(None);
    for kind in ["summary", "gaps", "problems"] {
        let early = stdout(&render(&matrix, &runs, kind, EARLY));
        let late = stdout(&render(&matrix, &runs, kind, LATE));
        assert_ne!(early, late, "{kind} ignores --generated-at");

        let diff = differing_lines(&early, &late);
        assert_eq!(
            diff.len(),
            1,
            "{kind}: more than the Generated row differs\n{diff:?}"
        );
        assert_eq!(
            diff[0],
            (
                "| Generated | 2026-11-01T09:00:00Z |",
                "| Generated | 2026-11-02T09:00:00Z |"
            ),
            "{kind}"
        );
    }
}

// ----------------------------------------------------------- exit codes

#[test]
fn invalid_generated_at_exits_one_naming_the_value() {
    let (_dir, matrix, runs) = scenario(None);
    let output = render(&matrix, &runs, "gaps", "yesterday");
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("'yesterday'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn missing_runs_flag_exits_two() {
    let (_dir, matrix, _runs) = project(&[]);
    let matrix = path_arg(&matrix);
    let output = report(&["--matrix", matrix.as_str(), "--kind", "summary"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(stderr(&output).contains("--runs"), "{}", stderr(&output));
}

#[test]
fn unknown_kind_exits_two() {
    let (_dir, matrix, runs) = scenario(None);
    let output = render(&matrix, &runs, "bogus", GENERATED);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(stderr(&output).contains("bogus"), "{}", stderr(&output));
}
