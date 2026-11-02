//! `test-executor report --format markdown|asciidoc|json|yaml` (Phase 4, plan 04-04).
//!
//! Every test runs the real binary from the workspace root, so `schemas/` resolves. Paths that
//! are compared with the Markdown golden are relative, exactly as the golden was generated.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use testcase_reporting::report_document::validate_document;

const MATRIX: &str = "crates/test-executor/tests/golden/report_fixture_matrix.yaml";
const RUN: &str = "crates/test-executor/tests/golden/report_fixture_run.yaml";
const GENERATED: &str = "2026-10-31T09:00:00Z";
const GOLDEN_MARKDOWN: &str = include_str!("golden/report_markdown_pre_04-04.md");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn abs(relative: &str) -> String {
    root().join(relative).display().to_string()
}

/// The matrix, run and generated-at arguments of the golden fixture (relative paths).
fn base_args<'a>() -> Vec<&'a str> {
    vec![
        "--matrix",
        MATRIX,
        "--runs",
        RUN,
        "--generated-at",
        GENERATED,
    ]
}

/// Runs `test-executor --no-audit report <args>` in `dir`.
fn run_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(dir)
        .args(["--no-audit", "report"])
        .args(args)
        .output()
        .unwrap()
}

/// Runs the report from the workspace root.
fn report(args: &[&str]) -> Output {
    run_in(&root(), args)
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

fn with_format(format: &str) -> Vec<&str> {
    let mut args = base_args();
    args.extend(["--format", format]);
    args
}

fn report_schema() -> Value {
    let path = root().join("schemas/tcms/report.schema.v1.json");
    let text = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The JSON array of the four documents, checked against the schema file.
fn json_documents(output: &Output) -> Vec<Value> {
    let out = stdout(output);
    assert!(out.starts_with('['), "{out}");
    let documents: Vec<Value> = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("stdout is not a JSON array: {e}\n{out}"));
    let schema = report_schema();
    for document in &documents {
        if let Err(messages) = validate_document(document, &schema) {
            panic!("document does not validate: {messages:?}");
        }
    }
    documents
}

fn kinds_of(documents: &[Value]) -> Vec<String> {
    documents
        .iter()
        .map(|d| d["kind"].as_str().unwrap_or("").to_string())
        .collect()
}

/// True when some string anywhere in the value equals `needle`.
fn contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(s) => s == needle,
        Value::Array(items) => items.iter().any(|v| contains_string(v, needle)),
        Value::Object(map) => map.values().any(|v| contains_string(v, needle)),
        _ => false,
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

#[test]
fn default_format_is_markdown_unchanged() {
    let default = report(&base_args());
    assert!(default.status.success(), "{}", stderr(&default));
    assert_eq!(stdout(&default), GOLDEN_MARKDOWN);

    let explicit = report(&with_format("markdown"));
    assert!(explicit.status.success(), "{}", stderr(&explicit));
    assert_eq!(stdout(&explicit), GOLDEN_MARKDOWN);
}

#[test]
fn asciidoc_all_kinds() {
    let output = report(&with_format("asciidoc"));
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);

    assert!(out.starts_with("= Traceability Report\n"), "{out}");
    let level_zero = out.lines().filter(|l| l.starts_with("= ")).count();
    assert_eq!(level_zero, 1, "{out}");
    for title in ["== Run Summary", "== Coverage Gaps", "== Problem Reports"] {
        assert!(out.lines().any(|l| l == title), "missing {title}\n{out}");
    }
    for word in ["VERIFIED", "FAILED", "NOT RUN", "NO TESTS"] {
        assert!(out.contains(word), "missing {word}\n{out}");
    }
    assert!(!out.lines().any(|l| l.starts_with("| ---")), "{out}");
    let fences = out.lines().filter(|l| *l == "|===").count();
    assert_eq!(fences % 2, 0, "unbalanced table fences\n{out}");
    assert!(
        !out.contains("Reconciliation"),
        "input summary on stdout\n{out}"
    );
    assert!(
        stderr(&output).contains("Reconciliation"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn json_all_kinds() {
    let output = report(&with_format("json"));
    assert!(output.status.success(), "{}", stderr(&output));
    let documents = json_documents(&output);
    assert_eq!(
        kinds_of(&documents),
        ["traceability", "summary", "gaps", "problems"]
    );
    let err = stderr(&output);
    assert!(err.contains("Reconciliation"), "{err}");
    assert!(!err.contains("renderer not yet available"), "{err}");
}

#[test]
fn json_single_kind() {
    let mut args = with_format("json");
    args.extend(["--kind", "summary"]);
    let output = report(&args);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with('{'), "{out}");
    let value: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(value["kind"], "summary");
    assert_eq!(value["type"], "report");
    if let Err(messages) = validate_document(&value, &report_schema()) {
        panic!("{messages:?}");
    }
}

#[test]
fn yaml_all_kinds() {
    let json = report(&with_format("json"));
    let json_values = json_documents(&json);

    let output = report(&with_format("yaml"));
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    assert!(out.starts_with("type: report\n"), "{out}");
    assert_eq!(out.lines().filter(|l| *l == "---").count(), 3, "{out}");

    let yaml_values: Vec<Value> = serde_yaml::Deserializer::from_str(&out)
        .map(|doc| {
            let value: serde_yaml::Value = serde::Deserialize::deserialize(doc).unwrap();
            serde_json::to_value(value).unwrap()
        })
        .collect();
    assert_eq!(
        yaml_values, json_values,
        "YAML and JSON carry different data"
    );
}

#[test]
fn output_file() {
    let dir = tempfile::tempdir().unwrap();
    for (format, ext) in [("asciidoc", "adoc"), ("json", "json"), ("yaml", "yaml")] {
        let direct = report(&with_format(format));
        assert!(direct.status.success(), "{}", stderr(&direct));

        let target = dir.path().join(format!("out.{ext}"));
        let target_arg = target.display().to_string();
        let mut args = with_format(format);
        args.extend(["-o", target_arg.as_str()]);
        let output = report(&args);
        assert!(output.status.success(), "{}", stderr(&output));
        assert!(stdout(&output).is_empty(), "{format}: {}", stdout(&output));
        let written = std::fs::read(&target).unwrap();
        assert_eq!(written, direct.stdout, "{format}: file differs from stdout");
    }
}

#[test]
fn deterministic_per_format() {
    let dir = tempfile::tempdir().unwrap();
    let runs_dir = dir.path();
    // A second run so that the order of `--runs` can be varied.
    std::fs::write(
        runs_dir.join("run2.yaml"),
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: RUN-2\nbuild_id: build-2\n\
         environment: lab\ntester: bob\nstarted_at: '2026-10-30T12:00:00Z'\n\
         results:\n  - {test_case_id: T3, verdict: pass}\n",
    )
    .unwrap();
    let matrix = runs_dir.join("matrix.yaml");
    std::fs::write(
        &matrix,
        "type: trace_matrix\nschema: tcms/trace-matrix.schema.json\nid: TM-1\nlinks:\n\
         \x20 - requirement_id: HLR-1\n    level: HLR\n    test_case_ids: [T1]\n    \
         normal_range_covered: true\n    robustness_covered: false\n    run_ids: [RUN-1]\n\
         \x20 - requirement_id: LLR-1\n    level: LLR\n    test_case_ids: [T2, T3]\n    \
         normal_range_covered: true\n    robustness_covered: true\n    run_ids: [RUN-1, RUN-2]\n",
    )
    .unwrap();
    let run1 = abs(RUN);
    let run2 = runs_dir.join("run2.yaml").display().to_string();
    let matrix_arg = matrix.display().to_string();
    let run_args_forward = [
        "--matrix",
        matrix_arg.as_str(),
        "--runs",
        run1.as_str(),
        "--runs",
        run2.as_str(),
    ];
    let run_args_reverse = [
        "--matrix",
        matrix_arg.as_str(),
        "--runs",
        run2.as_str(),
        "--runs",
        run1.as_str(),
    ];

    for format in ["asciidoc", "json", "yaml"] {
        let mut args = run_args_forward.to_vec();
        args.extend(["--generated-at", GENERATED, "--format", format]);
        let first = run_in(&root(), &args);
        assert!(first.status.success(), "{format}: {}", stderr(&first));
        let second = run_in(&root(), &args);
        assert_eq!(
            stdout(&first),
            stdout(&second),
            "{format}: same args differ"
        );

        let reversed_args: Vec<&str> = run_args_reverse
            .iter()
            .copied()
            .chain(["--generated-at", GENERATED, "--format", format])
            .collect();
        let reversed = run_in(&root(), &reversed_args);
        assert!(reversed.status.success(), "{format}: {}", stderr(&reversed));
        assert_eq!(
            stdout(&first),
            stdout(&reversed),
            "{format}: run order differs"
        );

        let later = "2026-11-02T08:00:00Z";
        let mut later_args = run_args_forward.to_vec();
        later_args.extend(["--generated-at", later, "--format", format]);
        let other = run_in(&root(), &later_args);
        assert!(other.status.success(), "{format}: {}", stderr(&other));
        let a = stdout(&first);
        let b = stdout(&other);
        assert_ne!(
            a, b,
            "{format}: a new --generated-at must change the output"
        );
        let strip = |s: &str, stamp: &str| -> Vec<String> {
            s.lines()
                .filter(|l| !l.contains(stamp))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(
            strip(&a, GENERATED),
            strip(&b, later),
            "{format}: output differs outside the generated-time lines"
        );
    }
}

/// The hostile fixture (D-06): pipes, quotes, markup, attribute references and colons in ids
/// and in run metadata.
fn hostile_fixture(dir: &Path) -> (PathBuf, PathBuf) {
    let matrix = dir.join("hostile-matrix.yaml");
    std::fs::write(
        &matrix,
        r#"type: trace_matrix
schema: tcms/trace-matrix.schema.json
id: TM-HOSTILE
links:
  - requirement_id: "HLR|1 *bold* [[x]] <<y>>"
    level: HLR
    test_case_ids: [T1]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: LLR-1
    level: LLR
    test_case_ids: [T2]
    normal_range_covered: true
    robustness_covered: true
    run_ids: [RUN-1]
"#,
    )
    .unwrap();
    let run = dir.join("hostile-run.yaml");
    std::fs::write(
        &run,
        r#"type: test_run
schema: tcms/test-run.schema.json
run_id: RUN-1
build_id: "build|1 \"q\" {attr}: colon"
environment: "env|2 \"q\" {attr}: colon"
tester: "tester|3 \"q\" {attr}: colon"
started_at: '2026-10-30T10:00:00Z'
results:
  - {test_case_id: T1, verdict: pass}
  - {test_case_id: T2, verdict: fail}
"#,
    )
    .unwrap();
    (matrix, run)
}

#[test]
fn hostile_strings_do_not_break_any_format() {
    let dir = tempfile::tempdir().unwrap();
    let (matrix, run) = hostile_fixture(dir.path());
    let matrix_arg = matrix.display().to_string();
    let run_arg = run.display().to_string();
    let common = [
        "--matrix",
        matrix_arg.as_str(),
        "--runs",
        run_arg.as_str(),
        "--generated-at",
        GENERATED,
    ];
    let hostile_id = "HLR|1 *bold* [[x]] <<y>>";

    // AsciiDoc
    let mut args = common.to_vec();
    args.extend(["--format", "asciidoc"]);
    let output = report(&args);
    assert!(output.status.success(), "{}", stderr(&output));
    let out = stdout(&output);
    for line in out.lines().filter(|l| l.starts_with('|') && *l != "|===") {
        assert_eq!(
            line.matches('|').count(),
            1,
            "cell line with extra pipe: {line}\n{out}"
        );
    }
    let level_zero: Vec<&str> = out.lines().filter(|l| l.starts_with("= ")).collect();
    assert_eq!(level_zero, ["= Traceability Report"], "{out}");
    for title in ["== Run Summary", "== Coverage Gaps", "== Problem Reports"] {
        assert_eq!(out.lines().filter(|l| *l == title).count(), 1, "{out}");
    }

    // JSON and YAML
    for format in ["json", "yaml"] {
        let mut args = common.to_vec();
        args.extend(["--format", format]);
        let output = report(&args);
        assert!(output.status.success(), "{format}: {}", stderr(&output));
        let documents: Vec<Value> = if format == "json" {
            json_documents(&output)
        } else {
            let schema = report_schema();
            serde_yaml::Deserializer::from_str(&stdout(&output))
                .map(|doc| {
                    let value: serde_yaml::Value = serde::Deserialize::deserialize(doc).unwrap();
                    let value = serde_json::to_value(value).unwrap();
                    validate_document(&value, &schema).unwrap_or_else(|m| panic!("{m:?}"));
                    value
                })
                .collect()
        };
        assert_eq!(documents.len(), 4, "{format}");
        assert!(
            contains_string(&documents[0], hostile_id),
            "{format}: traceability document lost the requirement id"
        );
    }
}

#[test]
fn validation_failure_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&root().join("schemas"), &dir.path().join("schemas"));
    let schema_path = dir.path().join("schemas/tcms/report.schema.v1.json");
    let mut schema: Value =
        serde_json::from_str(&std::fs::read_to_string(&schema_path).unwrap()).unwrap();
    schema["required"]
        .as_array_mut()
        .expect("report schema has a top-level required list")
        .push(Value::from("nonexistent_field"));
    std::fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap()).unwrap();

    let matrix = abs(MATRIX);
    let run = abs(RUN);
    for format in ["json", "yaml"] {
        let target = dir.path().join(format!("out.{format}"));
        let target_arg = target.display().to_string();
        let args = [
            "--matrix",
            matrix.as_str(),
            "--runs",
            run.as_str(),
            "--generated-at",
            GENERATED,
            "--format",
            format,
            "-o",
            target_arg.as_str(),
        ];
        let output = run_in(dir.path(), &args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{format}: {}",
            stdout(&output)
        );
        assert!(stdout(&output).is_empty(), "{format}: {}", stdout(&output));
        let err = stderr(&output);
        assert!(err.contains("report.schema.v1.json"), "{err}");
        assert!(err.contains("nonexistent_field"), "{err}");
        assert!(
            !target.exists(),
            "{format}: file written despite the failure"
        );
    }

    for format in ["asciidoc", "markdown"] {
        let args = [
            "--matrix",
            matrix.as_str(),
            "--runs",
            run.as_str(),
            "--generated-at",
            GENERATED,
            "--format",
            format,
        ];
        let output = run_in(dir.path(), &args);
        assert!(output.status.success(), "{format}: {}", stderr(&output));
    }
}

#[test]
fn shape_with_new_format_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let cases: [&[&str]; 4] = [
        &["--shape", "a", "--format", "json"],
        &["--shape", "b", "--format", "yaml"],
        &["--shape", "a", "--format", "asciidoc"],
        &["--shape", "a", "--kind", "summary"],
    ];
    for extra in cases {
        let target = dir.path().join("out.txt");
        let target_arg = target.display().to_string();
        let mut args = base_args();
        args.extend_from_slice(extra);
        args.extend(["-o", target_arg.as_str()]);
        let output = report(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{extra:?}: {}",
            stdout(&output)
        );
        assert!(stdout(&output).is_empty(), "{extra:?}: {}", stdout(&output));
        let err = stderr(&output);
        assert!(err.contains("--shape"), "{extra:?}: {err}");
        assert!(
            err.contains("requires --format markdown")
                || err.contains("requires --kind traceability"),
            "{extra:?}: the reason is not named: {err}"
        );
        assert!(!target.exists(), "{extra:?}: file created");
    }
}

#[test]
fn invalid_format_lists_the_valid_values() {
    let output = report(&["--matrix", MATRIX, "--runs", RUN, "--format", "xml"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("asciidoc"), "{}", stderr(&output));
}

#[test]
fn missing_schema_fails_naming_the_path_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&root().join("schemas"), &dir.path().join("schemas"));
    std::fs::remove_file(dir.path().join("schemas/tcms/report.schema.v1.json")).unwrap();

    let matrix = abs(MATRIX);
    let run = abs(RUN);
    let target = dir.path().join("out.json");
    let target_arg = target.display().to_string();
    let args = [
        "--matrix",
        matrix.as_str(),
        "--runs",
        run.as_str(),
        "--format",
        "json",
        "-o",
        target_arg.as_str(),
    ];
    let output = run_in(dir.path(), &args);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stdout(&output).is_empty());
    assert!(
        stderr(&output).contains("report.schema.v1.json"),
        "{}",
        stderr(&output)
    );
    assert!(!target.exists());
}
