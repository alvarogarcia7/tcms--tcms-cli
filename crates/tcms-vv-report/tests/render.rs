//! Tests for tcms-vv-report (TCMS-47.1). The report reads the metrics JSON written by `tcms vv metrics`
//! and must reproduce its figures. Goldens live in tests/golden/ and were generated from that JSON.
//!
//! A missing golden or fixture file is a test failure that names the path. No test returns early
//! because a file is absent.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tcms_vv_report::{render, Format};

/// The derived fixtures that produce a metrics document (zero-test-cases is refused by `tcms vv metrics`).
const FIXTURES: &[&str] = &[
    "sut-1",
    "failing-run",
    "no-data-run",
    "blocked-only-run",
    "skipped-outside-denominator",
    "uncovered-requirement",
    "missing-case-result",
];

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(name)
}

/// Reads a file that must exist; a missing file fails the test with its path.
fn must_read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "required file missing or unreadable: {}: {e}",
            path.display()
        )
    })
}

fn metrics_json(name: &str) -> String {
    must_read(&golden_path(&format!("{name}.metrics.json")))
}

fn metrics_value(name: &str) -> Value {
    serde_json::from_str(&metrics_json(name)).expect("golden metrics JSON parses")
}

fn report_path(name: &str, format: Format) -> PathBuf {
    let ext = match format {
        Format::Markdown => "md",
        Format::AsciiDoc => "adoc",
    };
    golden_path(&format!("{name}.{ext}"))
}

fn render_fixture(name: &str, format: Format) -> String {
    render(&metrics_json(name), format).expect("golden metrics JSON renders")
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tcms-vv-report"))
}

fn run_bin(args: &[&str]) -> Output {
    bin().args(args).output().expect("binary runs")
}

fn tmp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The first line that starts with `| <label> |`. Both formats render rows that way.
fn row(text: &str, label: &str) -> String {
    let prefix = format!("| {label} |");
    text.lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no row for '{label}' in report:\n{text}"))
        .to_string()
}

/// The cells of a table row in either format (Markdown rows end with ` |`, AsciiDoc rows do not).
fn cells(line: &str) -> Vec<String> {
    let body = line.strip_prefix("| ").unwrap_or(line);
    let body = body.strip_suffix(" |").unwrap_or(body);
    body.split(" | ").map(str::to_string).collect()
}

fn exact_text(v: &Value) -> String {
    v.to_string()
}

fn signed(d: i64) -> String {
    if d > 0 {
        format!("+{d}")
    } else {
        d.to_string()
    }
}

/// Checks every figure of one metrics document against the rows of a rendered report.
fn assert_figures(name: &str, text: &str, m: &Value) {
    let gate = &m["gate"];
    let gate_row = row(text, "Gate pass rate");
    if gate["passRate"].is_null() {
        assert!(gate_row.contains("NO_DATA"), "{name}: {gate_row}");
        assert!(!gate_row.contains('%'), "{name}: {gate_row}");
    } else {
        let pr = &gate["passRate"];
        assert!(
            gate_row.contains(&format!("{}%", pr["display"])),
            "{name}: {gate_row}"
        );
        assert!(
            gate_row.contains(&format!("exact {}%", exact_text(&pr["exact"]))),
            "{name}: {gate_row}"
        );
    }
    assert!(
        row(text, "Verdict").contains(gate["verdict"].as_str().unwrap()),
        "{name}: verdict"
    );
    assert!(
        row(text, "Denominator (passed + failed + blocked)")
            .contains(&gate["denominator"].to_string()),
        "{name}: denominator"
    );
    let c = &gate["counts"];
    let counts = format!(
        "{} / {} / {} / {} / {}",
        c["passed"], c["failed"], c["blocked"], c["skipped"], c["untested"]
    );
    assert!(
        row(
            text,
            "Counts: passed / failed / blocked / skipped / untested"
        )
        .contains(&counts),
        "{name}: counts"
    );
    let delta_row = row(text, "Delta from previous run");
    if gate["deltaFromPrevious"].is_null() {
        assert!(delta_row.contains("n/a"), "{name}: {delta_row}");
    } else {
        let d = gate["deltaFromPrevious"].as_i64().unwrap();
        assert!(delta_row.contains(&signed(d)), "{name}: {delta_row}");
    }

    let v = &m["vvRun"];
    let vv_row = row(text, "V&V test-case pass rate");
    if v["vvPassRate"].is_null() {
        assert!(vv_row.contains("n/a"), "{name}: {vv_row}");
    } else {
        assert!(
            vv_row.contains(&format!("{}%", v["vvPassRate"]["display"])),
            "{name}: {vv_row}"
        );
    }
    assert!(
        row(text, "Test cases passed / total").contains(&format!(
            "{} / {}",
            v["passedTestCases"], v["totalTestCases"]
        )),
        "{name}: test cases"
    );
    let step_row = row(text, "Step coverage");
    if v["stepCoverage"].is_null() {
        assert!(step_row.contains("n/a"), "{name}: {step_row}");
    } else {
        assert!(
            step_row.contains(&format!("{}%", v["stepCoverage"]["display"])),
            "{name}: {step_row}"
        );
    }
    assert!(
        row(text, "Steps passed / total")
            .contains(&format!("{} / {}", v["passedSteps"], v["totalSteps"])),
        "{name}: steps"
    );

    let reqs = &m["requirements"];
    for (key, label) in [
        ("total", "Requirements"),
        ("covered", "Covered"),
        ("failing", "Failing"),
        ("notRun", "Not run"),
        ("noCases", "No cases"),
        ("withDefect", "With problem report"),
        ("openDefects", "Open problem reports"),
    ] {
        assert_eq!(
            cells(&row(text, label))[1],
            reqs["summary"][key].to_string(),
            "{name}: requirement summary {label}"
        );
    }
    for item in reqs["items"].as_array().unwrap() {
        let id = item["id"].as_str().unwrap();
        let expected = format!(
            "| {id} | {} | {} / {} |",
            item["coverage"].as_str().unwrap(),
            item["executed"],
            item["total"]
        );
        assert!(
            text.lines().any(|l| l.starts_with(&expected)),
            "{name}: no requirement row starting '{expected}'"
        );
    }

    let s = &m["stability"];
    assert_eq!(
        cells(&row(text, "Flaky cases"))[1],
        s["flakyCount"].to_string(),
        "{name}: flaky count"
    );
    for case in s["cases"].as_array().unwrap() {
        let stab = case["stability"].as_str().unwrap();
        if stab != "STABLE" {
            let expected = vec![
                case["caseId"].as_str().unwrap().to_string(),
                stab.to_string(),
                case["flips"].to_string(),
            ];
            assert!(
                text.lines().any(|l| cells(l) == expected),
                "{name}: no stability row {expected:?}"
            );
        }
    }

    let load = &m["loadSummary"];
    assert_eq!(
        cells(&row(text, "Distinct cases"))[1],
        load["distinctCases"].to_string(),
        "{name}: distinct cases"
    );
    assert_eq!(
        cells(&row(text, "Runs recorded"))[1],
        load["runsRecorded"].to_string(),
        "{name}: runs recorded"
    );
    assert_eq!(
        cells(&row(text, "Schema errors"))[1],
        load["schemaErrorCount"].to_string(),
        "{name}: schema errors"
    );
}

#[test]
fn every_figure_matches_metrics_json() {
    for name in FIXTURES {
        let m = metrics_value(name);
        for format in [Format::Markdown, Format::AsciiDoc] {
            let text = render_fixture(name, format);
            assert_figures(name, &text, &m);
        }
    }
}

#[test]
fn gate_and_vv_pass_rates_have_different_labels() {
    // skipped-outside-denominator: gate 1 of 1 = 100 (PASSED), V&V 1 of 2 = 50.
    let m = metrics_value("skipped-outside-denominator");
    assert_eq!(m["gate"]["passRate"]["display"], 100);
    assert_eq!(m["vvRun"]["vvPassRate"]["display"], 50);
    for format in [Format::Markdown, Format::AsciiDoc] {
        let text = render_fixture("skipped-outside-denominator", format);
        let gate = row(&text, "Gate pass rate");
        let vv = row(&text, "V&V test-case pass rate");
        assert!(gate.contains("100%"), "gate row: {gate}");
        assert!(!gate.contains("50%"), "gate row: {gate}");
        assert!(vv.contains("50%"), "vv row: {vv}");
        assert!(!vv.contains("100%"), "vv row: {vv}");
    }
}

#[test]
fn no_data_shows_no_percentage_and_delta_n_a() {
    let m = metrics_value("no-data-run");
    assert!(m["gate"]["passRate"].is_null());
    assert!(m["gate"]["deltaFromPrevious"].is_null());
    for format in [Format::Markdown, Format::AsciiDoc] {
        let text = render_fixture("no-data-run", format);
        let gate = row(&text, "Gate pass rate");
        assert!(gate.contains("NO_DATA"), "gate row: {gate}");
        assert!(!gate.contains('%'), "gate row: {gate}");
        assert!(row(&text, "Delta from previous run").contains("n/a"));
    }
}

#[test]
fn fail_verdict_exits_0_by_default() {
    let path = golden_path("failing-run.metrics.json");
    let out = run_bin(&["--metrics", path.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout
            .lines()
            .any(|line| line.trim() == "| Verdict | FAILED |"),
        "expected the exact verdict row in stdout: {stdout}"
    );
}

#[test]
fn fail_verdict_with_fail_on_gate_writes_report_and_exits_2() {
    let path = golden_path("failing-run.metrics.json");
    let dir = tmp_dir("fail-on-gate");
    let report = dir.join("nested").join("dir").join("report.md");
    let _ = fs::remove_file(&report);
    let out = run_bin(&[
        "--metrics",
        path.to_str().unwrap(),
        "--out",
        report.to_str().unwrap(),
        "--fail-on-gate",
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = must_read(&report);
    assert!(written.contains("FAILED"));
    assert_eq!(written, render_fixture("failing-run", Format::Markdown));
}

#[test]
fn no_data_with_fail_on_gate_exits_0() {
    let path = golden_path("no-data-run.metrics.json");
    let out = run_bin(&["--metrics", path.to_str().unwrap(), "--fail-on-gate"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        render_fixture("no-data-run", Format::Markdown),
        "stdout is not the no-data report"
    );
    assert!(
        row(&String::from_utf8_lossy(&out.stdout), "Verdict").contains("NO_DATA"),
        "verdict is not NO_DATA"
    );
}

#[test]
fn missing_field_exits_1_and_names_it() {
    let mut m = metrics_value("sut-1");
    m.as_object_mut().unwrap().remove("vvRun");
    let dir = tmp_dir("missing-field");
    let path = dir.join("no-vv-run.metrics.json");
    fs::write(&path, serde_json::to_string_pretty(&m).unwrap()).unwrap();
    let out = run_bin(&["--metrics", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("vvRun"), "stderr: {err}");
    assert!(out.stdout.is_empty());
}

#[test]
fn null_field_that_is_absent_is_a_missing_field() {
    // passRate is documented as an object or null: an absent key is missing, not null.
    let mut m = metrics_value("sut-1");
    m["gate"].as_object_mut().unwrap().remove("passRate");
    let err = render(&serde_json::to_string(&m).unwrap(), Format::Markdown).unwrap_err();
    assert!(err.to_string().contains("passRate"), "error: {err}");
}

#[test]
fn output_is_identical_on_second_run() {
    for name in FIXTURES {
        let json = metrics_json(name);
        for format in [Format::Markdown, Format::AsciiDoc] {
            let first = render(&json, format).unwrap();
            assert!(!first.is_empty(), "{name}: empty report");
            assert_eq!(
                first,
                must_read(&report_path(name, format)),
                "{name}: first run differs from golden"
            );
            let second = render(&json, format).unwrap();
            assert_eq!(first.as_bytes(), second.as_bytes(), "{name}");
        }
    }
}

#[test]
fn golden_reports_match() {
    for name in FIXTURES {
        for format in [Format::Markdown, Format::AsciiDoc] {
            let expected = must_read(&report_path(name, format));
            assert_eq!(render_fixture(name, format), expected, "{name}");
        }
    }
}

#[test]
fn pipes_and_newlines_are_escaped_in_tables() {
    let json = must_read(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pipe-and-newline.metrics.json"),
    );
    for format in [Format::Markdown, Format::AsciiDoc] {
        let text = render(&json, format).unwrap();
        assert!(text.contains(r"SUT1\|PASS 001"), "case id not escaped");
        assert!(
            text.contains(r"cases/bad\|name.yml"),
            "file name not escaped"
        );
        assert!(
            text.contains(r"field a \| b is wrong second line"),
            "error not escaped"
        );
        assert!(
            !text
                .lines()
                .any(|l| l.starts_with("second line") || l.starts_with("001")),
            "a line break split a table row"
        );
    }
}

#[test]
fn help_documents_flags_and_exit_codes() {
    let out = run_bin(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    for flag in ["--metrics", "--format", "--out", "--fail-on-gate"] {
        assert!(text.contains(flag), "help lacks {flag}");
    }
    assert!(
        text.to_lowercase().contains("exit"),
        "help lacks exit codes"
    );
    assert!(text.contains("NO_DATA"), "help lacks null rendering");
}
