//! End-to-end checks for `test-executor report --shape a|b` in every format (04-05, D-04-6).
//!
//! Every test runs the real `test-executor` binary on the checked-in golden fixtures. Helpers are
//! copied from `report_formats_e2e.rs` on purpose: test files do not import from each other.
//! Shape documents must validate against `schemas/tcms/report.schema.v1.json` before any byte is
//! written, so the failure tests tamper with a copy of the schemas directory.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use testcase_reporting::report_document::validate_document;

const GENERATED: &str = "2026-10-31T09:00:00Z";
const GOLDEN_MATRIX: &str = "crates/test-executor/tests/golden/report_fixture_matrix.yaml";
const GOLDEN_RUN: &str = "crates/test-executor/tests/golden/report_fixture_run.yaml";

/// The schemas root is resolved from the current directory, so the command runs from here.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Runs `test-executor --no-audit report <args>` in `cwd`.
fn report_in(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(cwd)
        .args(["--no-audit", "report"])
        .args(args)
        .output()
        .expect("test-executor runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn stdout(output: &Output) -> String {
    text(&output.stdout)
}

fn stderr(output: &Output) -> String {
    text(&output.stderr)
}

/// The golden inputs and the fixed generation time, as arguments. The input paths are absolute,
/// because some tests run the command from a temporary directory.
fn golden_args() -> Vec<String> {
    vec![
        "--matrix".to_string(),
        root().join(GOLDEN_MATRIX).display().to_string(),
        "--runs".to_string(),
        root().join(GOLDEN_RUN).display().to_string(),
        "--generated-at".to_string(),
        GENERATED.to_string(),
    ]
}

fn report_schema() -> Value {
    let path = root().join("schemas/tcms/report.schema.v1.json");
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

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

/// A temporary directory holding a copy of `schemas/` whose report schema requires one more
/// top-level key, so every report document fails validation.
fn tampered_schemas() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&root().join("schemas"), &dir.path().join("schemas"));
    let schema_path = dir.path().join("schemas/tcms/report.schema.v1.json");
    let mut schema = report_schema();
    schema["required"]
        .as_array_mut()
        .expect("report schema has a top-level required list")
        .push(Value::from("nonexistent_field"));
    std::fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap()).unwrap();
    dir
}

/// The argument list for one report run with the golden inputs and extra arguments.
fn args_with(extra: &[&str]) -> Vec<String> {
    let mut args = golden_args();
    args.extend(extra.iter().map(|s| s.to_string()));
    args
}

fn run(cwd: &Path, args: &[String]) -> Output {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    report_in(cwd, &refs)
}

fn parse_json(out: &str) -> Value {
    serde_json::from_str(out).expect("JSON output parses")
}

fn parse_yaml(out: &str) -> Value {
    let value: serde_yaml::Value = serde_yaml::from_str(out).expect("YAML output parses");
    serde_json::to_value(value).expect("YAML converts to JSON")
}

/// Requirement ids of a traceability document, in document order.
fn requirement_ids(document: &Value) -> Vec<String> {
    document["content"]["documents"]
        .as_array()
        .expect("documents")
        .iter()
        .flat_map(|d| d["requirements"].as_array().expect("requirements").clone())
        .map(|r| r["requirement_id"].as_str().expect("id").to_string())
        .collect()
}

/// Test case ids of a traceability document, in document order.
fn case_ids(document: &Value) -> Vec<String> {
    document["content"]["documents"]
        .as_array()
        .expect("documents")
        .iter()
        .flat_map(|d| d["requirements"].as_array().expect("requirements").clone())
        .flat_map(|r| r["cases"].as_array().expect("cases").clone())
        .map(|c| c["test_case_id"].as_str().expect("id").to_string())
        .collect()
}

#[test]
fn shape_a_and_b_json_and_yaml_write_valid_documents() {
    let dir = tempfile::tempdir().unwrap();
    let grid = run(
        &root(),
        &args_with(&["--kind", "traceability", "--format", "json"]),
    );
    assert!(grid.status.success(), "{}", stderr(&grid));
    let grid_doc = parse_json(&stdout(&grid));

    for (shape, letter) in [("a", "a"), ("b", "b")] {
        for format in ["json", "yaml"] {
            let target = dir.path().join(format!("shape_{letter}.{format}"));
            let target_arg = target.display().to_string();
            let output = run(
                &root(),
                &args_with(&[
                    "--kind",
                    "traceability",
                    "--shape",
                    shape,
                    "--format",
                    format,
                    "-o",
                    target_arg.as_str(),
                ]),
            );
            assert_eq!(
                output.status.code(),
                Some(0),
                "{format} {shape}: {}",
                stderr(&output)
            );
            assert!(
                stdout(&output).is_empty(),
                "{format} {shape}: {}",
                stdout(&output)
            );
            let written = std::fs::read_to_string(&target).expect("file written");
            let doc = if format == "json" {
                parse_json(&written)
            } else {
                parse_yaml(&written)
            };
            let errors = validate_document(&doc, &report_schema());
            assert!(errors.is_ok(), "{format} {shape}: {errors:?}");
            assert_eq!(doc["kind"], "traceability");
            assert_eq!(doc["content"]["shape"], letter);
            assert_eq!(
                requirement_ids(&doc),
                requirement_ids(&grid_doc),
                "{format} {shape}"
            );
            assert_eq!(case_ids(&doc), case_ids(&grid_doc), "{format} {shape}");
        }
    }
}

#[test]
fn shape_markdown_writes_the_shape_text() {
    let dir = tempfile::tempdir().unwrap();
    let tampered = tampered_schemas();
    for shape in ["a", "b"] {
        let target = dir.path().join(format!("shape_{shape}.md"));
        let target_arg = target.display().to_string();
        let args = args_with(&[
            "--kind",
            "traceability",
            "--shape",
            shape,
            "--format",
            "markdown",
            "-o",
            target_arg.as_str(),
        ]);
        let output = run(tampered.path(), &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{shape}: {}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty(), "{shape}: {}", stdout(&output));
        assert!(
            !target.exists(),
            "{shape}: file created despite failed validation"
        );
    }

    for shape in ["a", "b"] {
        let target = dir.path().join(format!("real_{shape}.md"));
        let target_arg = target.display().to_string();
        let args = args_with(&[
            "--kind",
            "traceability",
            "--shape",
            shape,
            "--format",
            "markdown",
            "-o",
            target_arg.as_str(),
        ]);
        let output = run(&root(), &args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{shape}: {}",
            stderr(&output)
        );
        let written = std::fs::read_to_string(&target).expect("file written");
        assert!(
            written.starts_with("# Traceability Report"),
            "{shape}: {written}"
        );
    }
}

#[test]
fn shape_asciidoc_differs_from_grid() {
    let grid = run(
        &root(),
        &args_with(&["--kind", "traceability", "--format", "asciidoc"]),
    );
    assert!(grid.status.success(), "{}", stderr(&grid));
    let grid_text = stdout(&grid);
    for shape in ["a", "b"] {
        let output = run(
            &root(),
            &args_with(&[
                "--kind",
                "traceability",
                "--shape",
                shape,
                "--format",
                "asciidoc",
            ]),
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{shape}: {}",
            stderr(&output)
        );
        let shape_text = stdout(&output);
        assert!(
            shape_text.starts_with("= Traceability Report"),
            "{shape_text}"
        );
        assert_ne!(shape_text, grid_text, "shape {shape} rendered the grid");
    }
}

#[test]
fn shape_with_non_traceability_kind_exits_2_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cases: [&[&str]; 3] = [
        &["--shape", "a", "--kind", "summary"],
        &["--shape", "b", "--kind", "traceability", "--kind", "gaps"],
        &["--shape", "a"],
    ];
    for extra in cases {
        let target = dir.path().join("out.json");
        let target_arg = target.display().to_string();
        let mut args = args_with(extra);
        args.extend(["--format", "json", "-o", target_arg.as_str()].map(String::from));
        let output = run(&root(), &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{extra:?}: {}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty(), "{extra:?}: {}", stdout(&output));
        let err = stderr(&output);
        assert!(err.contains("--kind traceability"), "{extra:?}: {err}");
        assert!(
            !err.contains("requires --format markdown"),
            "{extra:?}: {err}"
        );
        assert!(!target.exists(), "{extra:?}: file created");
    }
}

#[test]
fn shape_failed_validation_writes_no_file_for_every_format() {
    let dir = tempfile::tempdir().unwrap();
    let tampered = tampered_schemas();
    let existing = dir.path().join("existing.txt");
    std::fs::write(&existing, "SENTINEL\n").unwrap();
    let existing_arg = existing.display().to_string();

    for shape in ["a", "b"] {
        for format in ["markdown", "json", "yaml", "asciidoc"] {
            let fresh = dir.path().join(format!("fresh_{shape}_{format}.out"));
            let fresh_arg = fresh.display().to_string();
            let args = args_with(&[
                "--kind",
                "traceability",
                "--shape",
                shape,
                "--format",
                format,
                "-o",
                fresh_arg.as_str(),
            ]);
            let output = run(tampered.path(), &args);
            let label = format!("{shape} {format}");
            assert_eq!(
                output.status.code(),
                Some(1),
                "{label}: {}",
                stderr(&output)
            );
            assert!(stdout(&output).is_empty(), "{label}: {}", stdout(&output));
            assert!(!fresh.exists(), "{label}: file created");

            let args = args_with(&[
                "--kind",
                "traceability",
                "--shape",
                shape,
                "--format",
                format,
                "-o",
                existing_arg.as_str(),
            ]);
            let output = run(tampered.path(), &args);
            assert_eq!(
                output.status.code(),
                Some(1),
                "{label}: {}",
                stderr(&output)
            );
            assert_eq!(
                std::fs::read_to_string(&existing).unwrap(),
                "SENTINEL\n",
                "{label}: existing file changed"
            );
        }
    }
}
