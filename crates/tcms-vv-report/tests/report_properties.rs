//! Property-based fuzzing of `tcms-vv-report` (proptest). Inputs are generated metrics documents and random bytes;
//! properties check that the renderer never panics, always names the gate verdict, and escapes pipes in tables.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use proptest::prelude::*;

fn scratch(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tcms-vv-report-prop-{tag}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn report(metrics: &PathBuf, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms-vv-report"))
        .args(["--metrics"])
        .arg(metrics)
        .args(extra)
        .output()
        .unwrap()
}

/// A metrics document with the fields the renderer reads, built from generated values.
fn document(passed: u64, failed: u64, gate: u64, req_id: &str, verdict: &str) -> serde_json::Value {
    let den = passed + failed;
    let display = match passed.checked_mul(100).and_then(|n| n.checked_div(den)) {
        None => serde_json::Value::Null,
        Some(display) => serde_json::json!({
            "display": display, "exact": passed as f64 * 100.0 / den as f64 }),
    };
    serde_json::json!({
        "generatedAt": "2026-01-01T00:00:00Z",
        "parameters": {
            "gateThresholdPercent": {"value": gate, "source": "flag"},
            "flakyThreshold": {"value": 4, "source": "default"},
            "stability_window_runs": {"value": 12, "source": "default"}
        },
        "run": {"runId": "RUN-P", "startedAt": "2026-10-01T10:00:00Z",
                "finishedAt": "2026-10-01T11:00:00Z", "provisional": false},
        "gate": {"gateThresholdPercent": gate, "denominator": den,
                 "counts": {"passed": passed, "failed": failed, "notExecuted": 0, "skipped": 0},
                 "passRate": display, "verdict": verdict, "provisional": false,
                 "deltaFromPrevious": null, "previousRunId": null},
        "vvRun": {"totalTestCases": den, "passedTestCases": passed,
                  "vvPassRate": null, "totalSteps": 0, "passedSteps": 0, "stepCoverage": null},
        "requirements": {"summary": {"total": 1, "covered": 0, "failing": 0, "notRun": 1, "noCases": 0,
                                     "withDefect": 0, "openDefects": 0},
                         "items": [{"id": req_id, "coverage": "NOT_RUN", "executed": 0, "total": 0,
                                    "problemReports": [], "openProblemReports": [], "cases": []}]},
        "stability": {"flakyThreshold": 4, "flakyCount": 0, "cases": []},
        "failingCases": [],
        "loadSummary": {"distinctCases": 0, "runsRecorded": 1, "schemaErrorCount": 0, "schemaErrors": []}
    })
}

/// Requirement ids: printable text including pipes. Backslashes are left out here: a backslash before a pipe is
/// not escaped by the renderer (see the ignored test `backslash_before_pipe_keeps_the_table_row_intact`, D-10).
fn req_id() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 |_-]{1,12}"
}

fn unescaped_pipes(line: &str) -> usize {
    let mut n = 0;
    let mut escaped = false;
    for c in line.chars() {
        match (escaped, c) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '|') => n += 1,
            _ => {}
        }
    }
    n
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Any well-formed document renders with exit 0, and the gate verdict appears in the output.
    #[test]
    fn every_well_formed_document_renders_with_its_verdict(
        passed in 0u64..50, failed in 0u64..50, gate in 50u64..=100, id in req_id(),
        verdict in prop_oneof![Just("PASSED"), Just("FAILED"), Just("NO_DATA")],
        fmt in prop_oneof![Just("markdown"), Just("asciidoc")],
    ) {
        let dir = scratch("render");
        let path = dir.join("m.json");
        fs::write(&path, document(passed, failed, gate, &id, verdict).to_string()).unwrap();
        let o = report(&path, &["--format", fmt]);
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let _ = fs::remove_dir_all(&dir);
        prop_assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
        prop_assert!(text.contains(verdict), "verdict {verdict} missing");
    }

    /// Every Markdown table row has the same number of unescaped pipes, whatever the requirement id contains.
    #[test]
    fn table_rows_keep_their_shape_for_any_requirement_id(id in req_id()) {
        let dir = scratch("pipes");
        let path = dir.join("m.json");
        fs::write(&path, document(1, 1, 90, &id, "FAILED").to_string()).unwrap();
        let o = report(&path, &["--format", "markdown"]);
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let _ = fs::remove_dir_all(&dir);
        prop_assert_eq!(o.status.code(), Some(0));
        // Each table is a run of consecutive lines starting with '|'; within a table every row has one shape.
        let mut tables: Vec<Vec<&str>> = Vec::new();
        let mut current: Vec<&str> = Vec::new();
        for line in text.lines() {
            if line.starts_with('|') {
                current.push(line);
            } else if !current.is_empty() {
                tables.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            tables.push(current);
        }
        prop_assert!(!tables.is_empty());
        for table in tables {
            let width = unescaped_pipes(table[0]);
            for row in table {
                prop_assert_eq!(unescaped_pipes(row), width, "row: {}", row);
            }
        }
    }

    /// Random bytes as the metrics file are refused with exit 1 (or accepted), never a panic or a signal.
    #[test]
    fn random_bytes_as_metrics_never_crash(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
        let dir = scratch("bytes");
        let path = dir.join("m.json");
        fs::write(&path, &bytes).unwrap();
        let o = report(&path, &[]);
        let err = String::from_utf8_lossy(&o.stderr).into_owned();
        let _ = fs::remove_dir_all(&dir);
        prop_assert!(o.status.code().is_some(), "killed by a signal");
        prop_assert!(!err.contains("panicked"), "{err}");
        prop_assert!(matches!(o.status.code(), Some(0..=2)), "exit {:?}", o.status.code());
    }

    /// --fail-on-gate exits 2 exactly when the verdict is FAILED (and the report is still written).
    #[test]
    fn fail_on_gate_exits_two_only_for_failed(
        passed in 0u64..20, failed in 0u64..20,
        verdict in prop_oneof![Just("PASSED"), Just("FAILED"), Just("NO_DATA")],
    ) {
        let dir = scratch("gate");
        let path = dir.join("m.json");
        fs::write(&path, document(passed, failed, 90, "SUT1-1", verdict).to_string()).unwrap();
        let o = report(&path, &["--fail-on-gate"]);
        let text = String::from_utf8_lossy(&o.stdout).into_owned();
        let _ = fs::remove_dir_all(&dir);
        let want = if verdict == "FAILED" { 2 } else { 0 };
        prop_assert_eq!(o.status.code(), Some(want));
        prop_assert!(!text.is_empty(), "the report must be written before the exit code");
    }
}

/// DEFECT D-10 (DISCREPANCES_TEST_CASES.md): a requirement id ending in a backslash, followed by a pipe in the
/// text, renders as `\\|`, which Markdown reads as an escaped backslash followed by a column separator. The
/// renderer must escape the backslash too. Kept ignored until the defect is fixed.
#[test]
fn backslash_before_pipe_keeps_the_table_row_intact() {
    let dir = scratch("backslash");
    let path = dir.join("m.json");
    fs::write(&path, document(1, 1, 90, "a\\|b", "FAILED").to_string()).unwrap();
    let o = report(&path, &["--format", "markdown"]);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let _ = fs::remove_dir_all(&dir);
    let row = text.lines().find(|l| l.contains("a\\")).unwrap();
    assert_eq!(unescaped_pipes(row), 6, "{row}");
}
