//! `test-executor report` exit status (plan 05-02, CLI-02, D-01 to D-03).
//!
//! Every test runs the real binary. Exit 0 is success; exit 1 is any input, validation, usage or
//! write error; exit 2 is only `--fail-on-gaps` with gaps or failed results, and only after the
//! report was written and flushed. Other subcommands keep clap's code 2 for usage errors.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GENERATED: &str = "2026-10-31T09:00:00Z";

/// The workspace root, so `schemas/` resolves for the binary.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs the binary with `args` from `cwd`.
fn bin(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(cwd)
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

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status.code(),
        stdout(output),
        stderr(output)
    )
}

/// `test-executor --no-audit report` over the given inputs, with an explicit generation time.
fn report_in(cwd: &Path, matrix: &Path, runs: &Path, generated: &str, extra: &[&str]) -> Output {
    let matrix = matrix.display().to_string();
    let runs = runs.display().to_string();
    let mut args = vec![
        "--no-audit",
        "report",
        "--matrix",
        matrix.as_str(),
        "--runs",
        runs.as_str(),
        "--generated-at",
        generated,
    ];
    args.extend_from_slice(extra);
    bin(cwd, &args)
}

fn report(scenario: &Scenario, extra: &[&str]) -> Output {
    report_in(
        &workspace_root(),
        &scenario.matrix,
        &scenario.runs,
        GENERATED,
        extra,
    )
}

/// Matrix and runs directory in a tempdir. The tempdir lives as long as the scenario.
struct Scenario {
    dir: tempfile::TempDir,
    matrix: PathBuf,
    runs: PathBuf,
}

impl Scenario {
    fn write_run(&self, name: &str, content: &str) {
        std::fs::write(self.runs.join(name), content).unwrap();
    }
}

type Link<'a> = (&'a str, &'a [&'a str], bool, bool);
type Result3<'a> = (&'a str, &'a str, Option<&'a str>);
type Run<'a> = (&'a str, &'a str, &'a [Result3<'a>]);

/// One scenario: links are (requirement, test cases, normal range covered, robustness covered);
/// every link lists every run. Runs are (run_id, started_at, results).
fn scenario(links: &[Link<'_>], runs: &[Run<'_>]) -> Scenario {
    let dir = tempfile::tempdir().unwrap();
    let run_ids: Vec<String> = runs.iter().map(|r| format!("\"{}\"", r.0)).collect();
    let mut matrix = String::from(
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nlinks:\n",
    );
    for (req, cases, normal, robust) in links {
        let cases: Vec<&str> = cases.to_vec();
        matrix.push_str(&format!(
            "  - requirement_id: {req}\n    level: HLR\n    test_case_ids: [{}]\n    \
             normal_range_covered: {normal}\n    robustness_covered: {robust}\n    \
             run_ids: [{}]\n",
            cases.join(", "),
            run_ids.join(", ")
        ));
    }
    let matrix_path = dir.path().join("matrix.yaml");
    std::fs::write(&matrix_path, matrix).unwrap();
    let runs_path = dir.path().join("runs");
    std::fs::create_dir(&runs_path).unwrap();
    let scenario = Scenario {
        dir,
        matrix: matrix_path,
        runs: runs_path,
    };
    for (run_id, started, results) in runs {
        scenario.write_run(
            &format!("{run_id}.yaml"),
            &run_text(run_id, started, results),
        );
    }
    scenario
}

fn run_text(run_id: &str, started: &str, results: &[Result3<'_>]) -> String {
    let mut yaml = format!(
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: {run_id}\n\
         build_id: build-{run_id}\nenvironment: e\ntester: t\nstarted_at: '{started}'\n"
    );
    if results.is_empty() {
        yaml.push_str("results: []\n");
    } else {
        yaml.push_str("results:\n");
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

const EARLY: &str = "2026-10-28T10:00:00Z";
const LATE: &str = "2026-10-28T11:00:00Z";

/// HLR-1 with T1: both coverage flags, T1 passes. No gaps, no failed results.
fn clean() -> Scenario {
    scenario(
        &[("HLR-1", &["T1"], true, true)],
        &[("RUN-1", EARLY, &[("T1", "pass", None)])],
    )
}

/// HLR-1 with T1 and T2: no robustness test, and T2 has no result. Two gap entries.
fn gaps() -> Scenario {
    scenario(
        &[("HLR-1", &["T1", "T2"], true, false)],
        &[("RUN-1", EARLY, &[("T1", "pass", None)])],
    )
}

/// HLR-1 with T1 and both flags; T1 fails with a problem report. One failed result.
fn failed() -> Scenario {
    scenario(
        &[("HLR-1", &["T1"], true, true)],
        &[("RUN-1", EARLY, &[("T1", "fail", Some("PR-1"))])],
    )
}

/// As `failed`, but the latest result is an error.
fn errored() -> Scenario {
    scenario(
        &[("HLR-1", &["T1"], true, true)],
        &[("RUN-1", EARLY, &[("T1", "error", Some("PR-2"))])],
    )
}

/// T1 fails in the earlier run and passes in the later run: the failure is superseded.
fn superseded() -> Scenario {
    scenario(
        &[("HLR-1", &["T1"], true, true)],
        &[
            ("RUN-1", EARLY, &[("T1", "fail", Some("PR-1"))]),
            ("RUN-2", LATE, &[("T1", "pass", None)]),
        ],
    )
}

/// Gaps and a failed result together, for the error cases.
fn gaps_and_failed() -> Scenario {
    scenario(
        &[("HLR-1", &["T1", "T2"], true, false)],
        &[("RUN-1", EARLY, &[("T1", "fail", Some("PR-1"))])],
    )
}

const ALL_KINDS: [&str; 4] = ["traceability", "summary", "gaps", "problems"];

#[test]
fn clean_exits_0_with_and_without_flag() {
    let s = clean();
    let without = report(&s, &[]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(0), "{}", describe(&with));
    assert!(
        !stderr(&with).contains("--fail-on-gaps"),
        "{}",
        describe(&with)
    );
}

#[test]
fn gaps_exit_0_without_flag_and_2_with_flag() {
    let s = gaps();
    let without = report(&s, &[]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
    assert!(
        stderr(&with).contains("2 gap entries"),
        "{}",
        describe(&with)
    );
}

#[test]
fn failed_result_exits_2_with_flag() {
    let s = failed();
    let without = report(&s, &[]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
    assert!(
        stderr(&with).contains("1 failed results"),
        "{}",
        describe(&with)
    );
}

#[test]
fn error_result_exits_2_with_flag() {
    let s = errored();
    let without = report(&s, &[]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
    assert!(
        stderr(&with).contains("1 failed results"),
        "{}",
        describe(&with)
    );
}

#[test]
fn superseded_failure_is_not_a_failure() {
    let s = superseded();
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(0), "{}", describe(&with));
    assert!(
        !stderr(&with).contains("--fail-on-gaps"),
        "{}",
        describe(&with)
    );
}

#[test]
fn report_written_before_exit_2_stdout() {
    let s = failed();
    let without = report(&s, &[]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(&s, &["--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
    assert!(
        stdout(&with).starts_with("# Traceability Report"),
        "{}",
        describe(&with)
    );
    assert_eq!(
        with.stdout,
        without.stdout,
        "exit 2 must not change the report bytes\n{}",
        describe(&with)
    );
    assert!(
        stderr(&with).contains("--fail-on-gaps"),
        "{}",
        describe(&with)
    );
    assert!(
        stderr(&with).contains("1 failed results"),
        "{}",
        describe(&with)
    );
}

#[test]
fn report_written_before_exit_2_file() {
    let s = gaps_and_failed();
    let plain = s.dir.path().join("plain.md");
    let flagged = s.dir.path().join("flagged.md");
    let without = report(&s, &["-o", &plain.display().to_string()]);
    assert_eq!(without.status.code(), Some(0), "{}", describe(&without));
    let with = report(
        &s,
        &["--fail-on-gaps", "-o", &flagged.display().to_string()],
    );
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
    assert!(stdout(&with).is_empty(), "{}", describe(&with));
    let written = std::fs::read(&flagged).expect("the report file exists after exit 2");
    assert!(!written.is_empty());
    assert_eq!(
        written,
        std::fs::read(&plain).unwrap(),
        "the file written before exit 2 must equal the exit-0 file"
    );
}

#[test]
fn every_kind_and_format_writes_then_exits_2() {
    let s = failed();
    let mut cases: Vec<Vec<String>> = ALL_KINDS
        .iter()
        .map(|kind| vec!["--kind".to_string(), kind.to_string()])
        .collect();
    for format in ["asciidoc", "json", "yaml"] {
        cases.push(vec![
            "--kind".into(),
            "traceability".into(),
            "--format".into(),
            format.into(),
        ]);
    }
    for extra in cases {
        let extra: Vec<&str> = extra.iter().map(String::as_str).collect();
        let without = report(&s, &extra);
        assert_eq!(
            without.status.code(),
            Some(0),
            "{extra:?}: {}",
            describe(&without)
        );
        let mut flagged_args = extra.clone();
        flagged_args.push("--fail-on-gaps");
        let with = report(&s, &flagged_args);
        assert_eq!(
            with.status.code(),
            Some(2),
            "{extra:?}: {}",
            describe(&with)
        );
        assert_eq!(
            with.stdout,
            without.stdout,
            "{extra:?}: {}",
            describe(&with)
        );
        // yaml and asciidoc are checked by the byte equality above; json is also parsed
        if extra.contains(&"json") {
            let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&with.stdout);
            assert!(parsed.is_ok(), "{extra:?}: {}", describe(&with));
        }
    }
}

#[test]
fn kind_summary_exits_2_on_a_failed_result() {
    let s = failed();
    let with = report(&s, &["--kind", "summary", "--fail-on-gaps"]);
    assert_eq!(with.status.code(), Some(2), "{}", describe(&with));
}

/// A --shape conflict is a semantic usage error that keeps exit 2 with or without the flag
/// (D-05-1 (b), D-02-6-2), and nothing is written.
#[test]
fn shape_conflict_exits_2_with_or_without_the_flag() {
    let s = gaps_and_failed();
    let out_dir = s.dir.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let target = out_dir.join("report.md");
    let target_arg = target.display().to_string();
    for flag in [true, false] {
        let mut args = vec![
            "-o",
            target_arg.as_str(),
            "--shape",
            "a",
            "--kind",
            "summary",
        ];
        if flag {
            args.push("--fail-on-gaps");
        }
        let output = report_in(&workspace_root(), &s.matrix, &s.runs, GENERATED, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "flag {flag}: {}",
            describe(&output)
        );
        assert!(
            stdout(&output).is_empty(),
            "flag {flag}: {}",
            describe(&output)
        );
        assert!(!target.exists(), "flag {flag}: file created");
    }
}

/// Each error with the flag: exit 1, no output file. Each runs with and without the flag too.
#[test]
fn errors_win_over_the_flag() {
    type Setup = fn(&Scenario);
    let cases: [(&str, Setup, Vec<&str>); 4] = [
        (
            "invalid matrix",
            |s| {
                std::fs::write(&s.matrix, "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nlinks: 5\n").unwrap();
            },
            vec![],
        ),
        (
            "duplicate run_id",
            |s| {
                s.write_run(
                    "copy.yaml",
                    &run_text("RUN-1", EARLY, &[("T1", "pass", None)]),
                );
            },
            vec![],
        ),
        (
            "yaml syntax error in runs",
            |s| {
                s.write_run("broken.yaml", "a: [unclosed\n");
            },
            vec![],
        ),
        ("invalid generated-at", |_| {}, vec!["GENERATED_BAD"]),
    ];
    for (name, setup, extra) in cases {
        let s = gaps_and_failed();
        setup(&s);
        let out_dir = s.dir.path().join("out");
        std::fs::create_dir(&out_dir).unwrap();
        let target = out_dir.join("report.md");
        let target_arg = target.display().to_string();
        let run_extra: Vec<&str> = vec!["-o", target_arg.as_str()];
        let generated = if extra.contains(&"GENERATED_BAD") {
            "not-a-time"
        } else {
            GENERATED
        };
        for flag in [true, false] {
            let mut args = run_extra.clone();
            if flag {
                args.push("--fail-on-gaps");
            }
            let output = report_in(&workspace_root(), &s.matrix, &s.runs, generated, &args);
            assert_eq!(
                output.status.code(),
                Some(1),
                "{name} (flag {flag}): {}",
                describe(&output)
            );
            assert!(
                stdout(&output).is_empty(),
                "{name} (flag {flag}): {}",
                describe(&output)
            );
            assert!(!target.exists(), "{name} (flag {flag}): file created");
        }
    }
}

/// A JSON document that fails the report schema exits 1 with the flag, and writes nothing.
#[test]
fn schema_violation_exits_1_with_flag_and_writes_nothing() {
    let s = gaps_and_failed();
    let root = tempfile::tempdir().unwrap();
    copy_dir(
        &workspace_root().join("schemas"),
        &root.path().join("schemas"),
    );
    std::fs::write(
        root.path().join("schemas/tcms/report.schema.v1.json"),
        r#"{"type":"object","required":["nonexistent_field"]}"#,
    )
    .unwrap();
    let target = s.dir.path().join("report.json");
    let target_arg = target.display().to_string();
    for flag in [true, false] {
        let mut args = vec![
            "--no-audit",
            "report",
            "--matrix",
            s.matrix.to_str().unwrap(),
            "--runs",
            s.runs.to_str().unwrap(),
            "--generated-at",
            GENERATED,
            "--format",
            "json",
            "--kind",
            "traceability",
            "-o",
            target_arg.as_str(),
        ];
        if flag {
            args.push("--fail-on-gaps");
        }
        let output = bin(root.path(), &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "flag {flag}: {}",
            describe(&output)
        );
        assert!(!target.exists(), "flag {flag}: file created");
    }
}

#[test]
fn usage_errors_exit_1() {
    let s = clean();
    let m = s.matrix.display().to_string();
    let r = s.runs.display().to_string();
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["--no-audit", "report", "--matrix", &m], "required"),
        (vec!["--no-audit", "report", "--runs", &r], "required"),
        (
            vec![
                "--no-audit",
                "report",
                "--matrix",
                &m,
                "--runs",
                &r,
                "--bogus",
            ],
            "unexpected argument",
        ),
        (
            vec![
                "--no-audit",
                "report",
                "--matrix",
                &m,
                "--runs",
                &r,
                "--kind",
                "nosuchkind",
            ],
            "invalid value",
        ),
        (
            vec![
                "--no-audit",
                "report",
                "--matrix",
                &m,
                "--runs",
                &r,
                "--format",
                "xml",
            ],
            "invalid value",
        ),
    ];
    for (args, expected) in cases {
        let output = bin(&workspace_root(), &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{args:?}: {}",
            describe(&output)
        );
        assert!(
            stderr(&output).contains(expected),
            "{args:?}: {}",
            describe(&output)
        );
    }
    let help = bin(&workspace_root(), &["--no-audit", "report", "--help"]);
    assert_eq!(help.status.code(), Some(0), "{}", describe(&help));
    assert!(
        stdout(&help).contains("--fail-on-gaps"),
        "{}",
        describe(&help)
    );
}

/// A usage error of report after a global option still exits 1 (the subcommand is found).
#[test]
fn report_usage_error_after_global_option_exits_1() {
    let output = bin(
        &workspace_root(),
        &["--log-level", "warn", "report", "--runs", "x"],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
}

#[test]
fn other_commands_keep_clap_code_2() {
    let generate = bin(&workspace_root(), &["generate"]);
    assert_eq!(generate.status.code(), Some(2), "{}", describe(&generate));
    let unknown = bin(&workspace_root(), &["nosuchsubcommand"]);
    assert_eq!(unknown.status.code(), Some(2), "{}", describe(&unknown));
    let help = bin(&workspace_root(), &["--help"]);
    assert_eq!(help.status.code(), Some(0), "{}", describe(&help));
}

/// Across a mix of invocations, the status is in {0, 1, 2}, and 2 occurs only with the flag.
#[test]
fn only_0_1_2() {
    let s_clean = clean();
    let s_gaps = gaps();
    let s_failed = failed();
    let s_mixed = gaps_and_failed();
    let m = s_clean.matrix.display().to_string();
    let r = s_clean.runs.display().to_string();
    let bad_matrix = s_clean
        .dir
        .path()
        .join("missing.yaml")
        .display()
        .to_string();
    let invocations: Vec<(Output, bool)> = vec![
        (report(&s_clean, &["--fail-on-gaps"]), true),
        (report(&s_gaps, &["--fail-on-gaps"]), true),
        (report(&s_failed, &["--fail-on-gaps"]), true),
        (report(&s_failed, &[]), false),
        (report(&s_gaps, &[]), false),
        (report(&s_mixed, &["--fail-on-gaps"]), true),
        (
            bin(&workspace_root(), &["--no-audit", "report", "--matrix", &m]),
            false,
        ),
        (
            bin(
                &workspace_root(),
                &[
                    "--no-audit",
                    "report",
                    "--matrix",
                    &bad_matrix,
                    "--runs",
                    &r,
                    "--fail-on-gaps",
                ],
            ),
            true,
        ),
    ];
    assert_eq!(invocations.len(), 8);
    for (output, flagged) in invocations {
        let code = output.status.code();
        assert!(matches!(code, Some(0..=2)), "{}", describe(&output));
        if code == Some(2) {
            assert!(
                flagged,
                "exit 2 without --fail-on-gaps: {}",
                describe(&output)
            );
        }
    }
}

/// Copies a directory tree with plain std (no new dependency).
fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
