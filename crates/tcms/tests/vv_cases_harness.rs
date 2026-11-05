//! Drives every captured V&V case folder under `fixtures/vv-cases/` through the `tcms vv` binaries and compares
//! the output with the stored `expected/` output (exit code, stdout, stderr, files written).
//!
//! The expected output was checked against an independent reference model of docs/VV_METRICS.md
//! (scripts/vv/vv_cases/capture_expected.py). Cases listed as `KNOWN_DIVERGENCE.txt` are skipped here and
//! reported in DISCREPANCES_TEST_CASES.md.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn cases_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vv-cases")
}

fn schemas_root() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/")
        .to_string_lossy()
        .into_owned()
}

/// The schema root as the binary received it. Messages that name it are compared with a placeholder, so the
/// expected output does not depend on where the repository is checked out.
fn normalize_paths(text: &str) -> String {
    let raw = schemas_root().trim_end_matches('/').to_string();
    let mut out = text.replace(&format!("{raw}/"), "<SCHEMAS>/");
    out = out.replace(&raw, "<SCHEMAS>");
    if let Ok(canon) = fs::canonicalize(&raw) {
        out = out.replace(&canon.to_string_lossy().into_owned(), "<SCHEMAS>");
    }
    out
}

/// Copy the case inputs (not README, case.json, expected/) into a fresh temp dir.
fn copy_inputs(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let p = entry.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if matches!(name.as_str(), "README.md" | "case.json" | "expected") {
            continue;
        }
        let target = to.join(&name);
        if p.is_dir() {
            copy_inputs(&p, &target);
        } else {
            fs::copy(&p, &target).unwrap();
        }
    }
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// The argument vector the runner uses for each binary (see scripts/vv/vv_cases/run_vv_cases.py).
fn argv(binary: &str, args: &[String]) -> Vec<String> {
    let schemas = schemas_root();
    let mut v: Vec<String> = match binary {
        "metrics" => vec![
            "--schemas-root",
            schemas.as_str(),
            "vv",
            "metrics",
            "--root",
            ".",
            "--now",
            "2026-01-01T00:00:00Z",
        ],
        "combine" | "history" | "trends" => vec![
            "--schemas-root",
            schemas.as_str(),
            "vv",
            binary,
            "--root",
            ".",
        ],
        other => panic!("unknown binary {other}"),
    }
    .into_iter()
    .map(String::from)
    .collect();
    v.extend(args.iter().cloned());
    v
}

fn run_suite(binary: &str) -> (usize, usize) {
    let dir = cases_root().join(binary);
    let mut ran = 0;
    let mut skipped = 0;
    let mut failures = Vec::new();
    let mut names: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    for case in names {
        let case_json = case.join("case.json");
        if !case_json.exists() {
            continue;
        }
        let expected = case.join("expected");
        if expected.join("KNOWN_DIVERGENCE.txt").exists() {
            skipped += 1;
            continue;
        }
        let spec: Value = serde_json::from_str(&fs::read_to_string(&case_json).unwrap()).unwrap();
        let args: Vec<String> = spec["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap().to_string())
            .collect();
        let tmp = TempDir::new().unwrap();
        copy_inputs(&case, tmp.path());
        let out = Command::new(env!("CARGO_BIN_EXE_tcms"))
            .args(argv(binary, &args))
            .current_dir(tmp.path())
            .output()
            .unwrap();
        ran += 1;
        let stdout = normalize_paths(&String::from_utf8_lossy(&out.stdout));
        let stderr = normalize_paths(&String::from_utf8_lossy(&out.stderr));
        let name = case.file_name().unwrap().to_string_lossy().into_owned();

        let want_exit: i32 = fs::read_to_string(expected.join("exit.txt"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        if out.status.code() != Some(want_exit) {
            failures.push(format!(
                "{name}: exit {:?}, expected {want_exit}",
                out.status.code()
            ));
            continue;
        }
        if let Ok(text) = fs::read_to_string(expected.join("stdout.json")) {
            let want: Value = serde_json::from_str(&text).unwrap();
            match serde_json::from_str::<Value>(&stdout) {
                Ok(got) if got == want => {}
                other => failures.push(format!(
                    "{name}: stdout JSON differs from expected\n  got: {:?}\n  want: {want}",
                    other
                        .ok()
                        .map(|v| v.to_string().chars().collect::<String>())
                )),
            }
        }
        if let Ok(text) = fs::read_to_string(expected.join("stdout.txt")) {
            if stdout != text {
                failures.push(format!("{name}: stdout text differs from expected"));
            }
        }
        let want_stderr = fs::read_to_string(expected.join("stderr.txt")).unwrap_or_default();
        if !want_stderr.is_empty() && stderr.trim() != want_stderr.trim() {
            failures.push(format!("{name}: stderr differs from expected"));
        }
        let files_dir = expected.join("files");
        if files_dir.exists() {
            let mut want_files = Vec::new();
            collect_files(&files_dir, &mut want_files);
            for f in want_files {
                let rel = f.strip_prefix(&files_dir).unwrap();
                let got = fs::read(tmp.path().join(rel));
                if got.ok().as_deref() != Some(fs::read(&f).unwrap().as_slice()) {
                    failures.push(format!(
                        "{name}: file {} differs from expected",
                        rel.display()
                    ));
                }
            }
        }
    }
    assert!(ran > 0, "no cases ran in {binary}");
    assert!(
        failures.is_empty(),
        "{binary}: {} failing cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
    (ran, skipped)
}

#[test]
fn metrics_cases_match_expected_output() {
    run_suite("metrics");
}

#[test]
fn combine_cases_match_expected_output() {
    run_suite("combine");
}

#[test]
fn history_cases_match_expected_output() {
    run_suite("history");
}

#[test]
fn trends_cases_match_expected_output() {
    run_suite("trends");
}
