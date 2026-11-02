//! End-to-end checks for `test-executor report --format markdown|asciidoc|json|yaml` (Phase 4).
//!
//! Every test runs the real `test-executor` binary. Inputs are written to a temporary directory,
//! except the Markdown golden test, which uses the checked-in golden fixtures. The schemas root is
//! relative to the workspace, so the command runs from there (or from a copy of `schemas/`).

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use testcase_reporting::report_document::validate_document;

const GENERATED: &str = "2026-10-31T09:00:00Z";
const GOLDEN_MATRIX: &str = "crates/test-executor/tests/golden/report_fixture_matrix.yaml";
const GOLDEN_RUN: &str = "crates/test-executor/tests/golden/report_fixture_run.yaml";
const MARKDOWN_GOLDEN: &str = include_str!("golden/report_markdown_pre_04-04.md");

/// Requirement ids that each start with, or contain, a character AsciiDoc reads as markup. The
/// ids are listed in the byte order the report uses.
const HOSTILE_IDS: [&str; 8] = [
    "* star item",
    "- dash item",
    "1. ordered item",
    "NOTE: colon item",
    "[bracket] item",
    "`tick` item",
    "include::/etc/passwd[]",
    "| pipe item",
];

const HOSTILE_MATRIX: &str = r#"type: trace_matrix
schema: tcms/trace-matrix.schema.json
id: TM-H
links:
  - requirement_id: "* star item"
    level: HLR
    test_case_ids: [T2]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "- dash item"
    level: HLR
    test_case_ids: [T1]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "1. ordered item"
    level: LLR
    test_case_ids: [T7]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "NOTE: colon item"
    level: LLR
    test_case_ids: [T6]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "[bracket] item"
    level: HLR
    test_case_ids: [T4]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "`tick` item"
    level: LLR
    test_case_ids: [T5]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "include::/etc/passwd[]"
    level: LLR
    test_case_ids: [T8]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
  - requirement_id: "| pipe item"
    level: HLR
    test_case_ids: [T3]
    normal_range_covered: true
    robustness_covered: false
    run_ids: [RUN-1]
"#;

const HOSTILE_RUN: &str = r#"type: test_run
schema: tcms/test-run.schema.json
run_id: RUN-1
build_id: "b|1 *x*"
environment: "== Fake heading"
tester: "NOTE: x"
started_at: '2026-10-30T10:00:00Z'
results:
  - {test_case_id: T1, verdict: pass}
  - {test_case_id: T2, verdict: fail}
  - {test_case_id: T3, verdict: pass}
  - {test_case_id: T4, verdict: pass}
  - {test_case_id: T5, verdict: pass}
  - {test_case_id: T6, verdict: pass}
  - {test_case_id: T7, verdict: pass}
  - {test_case_id: T8, verdict: pass}
"#;

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

/// Runs the report from the workspace root.
fn report(args: &[&str]) -> Output {
    report_in(&root(), args)
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

/// A matrix file and a runs directory holding one run, all in a temporary directory.
struct Fixture {
    _dir: tempfile::TempDir,
    matrix: PathBuf,
    runs: PathBuf,
}

fn hostile_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let matrix = dir.path().join("matrix.yaml");
    std::fs::write(&matrix, HOSTILE_MATRIX).unwrap();
    let runs = dir.path().join("runs");
    std::fs::create_dir(&runs).unwrap();
    std::fs::write(runs.join("run1.yaml"), HOSTILE_RUN).unwrap();
    Fixture {
        _dir: dir,
        matrix,
        runs,
    }
}

/// Runs the report on a fixture with a fixed generation time and extra arguments.
fn render(fixture: &Fixture, extra: &[&str]) -> Output {
    let matrix = fixture.matrix.display().to_string();
    let runs = fixture.runs.display().to_string();
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

/// The golden Markdown fixture, with the same arguments the golden was generated with.
fn golden_args() -> Vec<&'static str> {
    vec![
        "--matrix",
        GOLDEN_MATRIX,
        "--runs",
        GOLDEN_RUN,
        "--generated-at",
        GENERATED,
    ]
}

/// Asserts a successful run and returns its stdout.
fn succeeded(output: &Output) -> String {
    assert!(output.status.success(), "{}", stderr(output));
    stdout(output)
}

fn assert_has_line(out: &str, line: &str) {
    assert!(
        out.lines().any(|l| l == line),
        "missing exact line {line:?}\n{out}"
    );
}

fn report_schema() -> Value {
    let path = root().join("schemas/tcms/report.schema.v1.json");
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

fn assert_valid(document: &Value) {
    if let Err(messages) = validate_document(document, &report_schema()) {
        panic!("document does not validate: {messages:?}");
    }
}

/// The four JSON documents, each validated against the report schema.
fn json_documents(out: &str) -> Vec<Value> {
    let documents: Vec<Value> =
        serde_json::from_str(out).unwrap_or_else(|e| panic!("not a JSON array: {e}\n{out}"));
    documents.iter().for_each(assert_valid);
    documents
}

/// The four YAML documents as JSON data, each validated against the report schema.
fn yaml_documents(out: &str) -> Vec<Value> {
    serde_yaml::Deserializer::from_str(out)
        .map(|doc| {
            let value: serde_yaml::Value = serde::Deserialize::deserialize(doc).unwrap();
            let value = serde_json::to_value(value).unwrap();
            assert_valid(&value);
            value
        })
        .collect()
}

fn kinds(documents: &[Value]) -> Vec<String> {
    documents
        .iter()
        .map(|d| d["kind"].as_str().unwrap_or("").to_string())
        .collect()
}

/// The requirement ids of the traceability document, in output order.
fn traceability_ids(traceability: &Value) -> Vec<String> {
    traceability["content"]["documents"][0]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["requirement_id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn asciidoc_hostile_requirement_ids_are_escaped_one_cell_per_line() {
    let fixture = hostile_fixture();
    let out = succeeded(&render(&fixture, &["--format", "asciidoc"]));

    // Each escaped id is a whole table-cell line, so no raw markup starts a line or a block.
    for line in [
        "| &#42; star item",
        "| &#45; dash item",
        "| &#49;. ordered item",
        "| NOTE&#58; colon item",
        "| &#91;bracket&#93; item",
        "| &#96;tick&#96; item",
        "| include&#58;&#58;/etc/passwd&#91;&#93;",
        "| &#124; pipe item",
    ] {
        assert_has_line(&out, line);
    }
    for raw in [
        "* star",
        "- dash",
        "1. ordered",
        "NOTE: colon",
        "[bracket]",
        "`tick`",
        "include::",
        "| pipe",
    ] {
        assert!(
            !out.contains(raw),
            "raw value {raw:?} reached AsciiDoc\n{out}"
        );
    }
}

#[test]
fn asciidoc_hostile_header_and_input_values_are_escaped() {
    let fixture = hostile_fixture();
    let out = succeeded(&render(&fixture, &["--format", "asciidoc"]));

    assert_has_line(&out, "| b&#124;1 &#42;x&#42;");
    assert_has_line(&out, "| &#61;&#61; Fake heading");
    assert_has_line(&out, "| NOTE&#58; x");
    assert_has_line(&out, "Builds used: b&#124;1 &#42;x&#42; +");
    assert!(!out.contains("== Fake heading"), "{out}");
    assert!(!out.contains("NOTE: x"), "{out}");
}

#[test]
fn asciidoc_tables_contain_only_cell_lines_and_one_title() {
    let fixture = hostile_fixture();
    let out = succeeded(&render(&fixture, &["--format", "asciidoc"]));

    assert_eq!(out.lines().next(), Some("= Traceability Report"), "{out}");
    let titles: Vec<&str> = out.lines().filter(|l| l.starts_with("= ")).collect();
    assert_eq!(titles, ["= Traceability Report"], "{out}");

    // Inside a table, every non-blank line is a cell line. A value that started a row, a list
    // or a literal block would put a line here that does not begin with a cell marker.
    let mut inside = false;
    let mut fences = 0;
    for line in out.lines() {
        if line == "|===" {
            inside = !inside;
            fences += 1;
        } else if inside && !line.is_empty() {
            assert!(line.starts_with('|'), "table line {line:?}\n{out}");
        }
    }
    assert!(!inside, "unclosed table\n{out}");
    assert!(fences > 0 && fences % 2 == 0, "{out}");
}

#[test]
fn json_keeps_raw_values_in_order_and_validates_every_document() {
    let fixture = hostile_fixture();
    let output = render(&fixture, &["--format", "json"]);
    let out = succeeded(&output);
    assert!(
        out.starts_with("[\n  {\n    \"type\": \"report\",\n"),
        "{out}"
    );

    let documents = json_documents(&out);
    assert_eq!(
        kinds(&documents),
        ["traceability", "summary", "gaps", "problems"]
    );
    assert_eq!(traceability_ids(&documents[0]), HOSTILE_IDS);
    assert_eq!(documents[0]["header"]["build_ids"][0], "b|1 *x*");
    assert_eq!(documents[0]["header"]["environments"][0], "== Fake heading");
    assert_eq!(documents[0]["header"]["testers"][0], "NOTE: x");
    assert!(
        !out.contains("&#"),
        "AsciiDoc escaping leaked into JSON\n{out}"
    );
    assert!(
        stderr(&output).contains("Reconciliation"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn json_round_trips_through_serde_in_compact_and_pretty_form() {
    let fixture = hostile_fixture();
    let out = succeeded(&render(&fixture, &["--format", "json"]));
    let parsed: Value = serde_json::from_str(&out).unwrap();

    let compact: Value = serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
    let pretty: Value =
        serde_json::from_str(&serde_json::to_string_pretty(&parsed).unwrap()).unwrap();
    assert_eq!(compact, parsed);
    assert_eq!(pretty, parsed);
}

#[test]
fn yaml_quotes_hostile_scalars_and_parses_to_the_json_data() {
    let fixture = hostile_fixture();
    let json = succeeded(&render(&fixture, &["--format", "json"]));
    let yaml = succeeded(&render(&fixture, &["--format", "yaml"]));

    assert!(
        yaml.starts_with("type: report\nschema: tcms/report.schema.v1.json\n"),
        "{yaml}"
    );
    assert_eq!(yaml.lines().filter(|l| *l == "---").count(), 3, "{yaml}");
    // A hazardous value must never be emitted as a plain scalar that YAML reads as markup.
    for plain in [
        "requirement_id: - ",
        "requirement_id: * ",
        "requirement_id: | ",
        "requirement_id: [",
        "requirement_id: `",
    ] {
        assert!(
            !yaml.contains(plain),
            "plain hazardous scalar {plain:?}\n{yaml}"
        );
    }
    assert_has_line(&yaml, "    - requirement_id: '* star item'");

    assert_eq!(yaml_documents(&yaml), json_documents(&json));
}

#[test]
fn yaml_round_trips_through_serde_yaml() {
    let fixture = hostile_fixture();
    let out = succeeded(&render(&fixture, &["--format", "yaml"]));
    for doc in serde_yaml::Deserializer::from_str(&out) {
        let value: serde_yaml::Value = serde::Deserialize::deserialize(doc).unwrap();
        let emitted = serde_yaml::to_string(&value).unwrap();
        let again: serde_yaml::Value = serde_yaml::from_str(&emitted).unwrap();
        assert_eq!(again, value, "round trip changed the document:\n{emitted}");
    }
}

#[test]
fn json_and_yaml_expose_the_same_four_kinds_in_order() {
    let fixture = hostile_fixture();
    let json = succeeded(&render(&fixture, &["--format", "json"]));
    let yaml = succeeded(&render(&fixture, &["--format", "yaml"]));

    let expected = ["traceability", "summary", "gaps", "problems"];
    assert_eq!(kinds(&json_documents(&json)), expected);
    let yaml_kind_lines: Vec<&str> = yaml.lines().filter(|l| l.starts_with("kind: ")).collect();
    let expected_lines: Vec<String> = expected.iter().map(|k| format!("kind: {k}")).collect();
    assert_eq!(yaml_kind_lines, expected_lines);
}

#[test]
fn markdown_default_and_explicit_match_the_pre_format_golden() {
    let default = report(&golden_args());
    assert_eq!(stdout(&default), MARKDOWN_GOLDEN);
    assert!(default.status.success(), "{}", stderr(&default));

    let mut args = golden_args();
    args.extend(["--format", "markdown"]);
    let explicit = report(&args);
    assert_eq!(stdout(&explicit), MARKDOWN_GOLDEN);
    assert!(explicit.status.success(), "{}", stderr(&explicit));

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("out.md");
    let target_arg = target.display().to_string();
    let mut args = golden_args();
    args.extend(["--format", "markdown", "-o", target_arg.as_str()]);
    let written = report(&args);
    assert!(written.status.success(), "{}", stderr(&written));
    assert_eq!(std::fs::read(&target).unwrap(), MARKDOWN_GOLDEN.as_bytes());
}

#[test]
fn every_format_is_byte_identical_for_the_same_inputs() {
    let fixture = hostile_fixture();
    let dir = tempfile::tempdir().unwrap();
    for format in ["markdown", "asciidoc", "json", "yaml"] {
        let mut runs = Vec::new();
        for attempt in ["first", "second"] {
            let target = dir.path().join(format!("{format}-{attempt}.out"));
            let target_arg = target.display().to_string();
            let output = render(&fixture, &["--format", format, "-o", target_arg.as_str()]);
            assert!(output.status.success(), "{format}: {}", stderr(&output));
            runs.push((
                std::fs::read(&target).unwrap(),
                output.stderr.clone(),
                output.stdout.clone(),
            ));
        }
        assert!(!runs[0].0.is_empty(), "{format}: empty file");
        assert_eq!(runs[0].0, runs[1].0, "{format}: file bytes differ");
        assert_eq!(runs[0].1, runs[1].1, "{format}: stderr differs");
        assert!(
            runs[0].2.is_empty() && runs[1].2.is_empty(),
            "{format}: stdout with -o"
        );

        let stdout_first = render(&fixture, &["--format", format]);
        let stdout_second = render(&fixture, &["--format", format]);
        assert_eq!(
            stdout_first.stdout, stdout_second.stdout,
            "{format}: stdout differs"
        );
        assert_eq!(
            stdout_first.stdout, runs[0].0,
            "{format}: file differs from stdout"
        );
    }
}

#[test]
fn failed_validation_writes_no_file_and_keeps_an_existing_one() {
    let dir = tempfile::tempdir().unwrap();
    copy_dir(&root().join("schemas"), &dir.path().join("schemas"));
    let schema_path = dir.path().join("schemas/tcms/report.schema.v1.json");
    let mut schema = report_schema();
    schema["required"]
        .as_array_mut()
        .expect("report schema has a top-level required list")
        .push(Value::from("nonexistent_field"));
    std::fs::write(&schema_path, serde_json::to_string_pretty(&schema).unwrap()).unwrap();

    let fixture = hostile_fixture();
    let existing = dir.path().join("existing.json");
    std::fs::write(&existing, "SENTINEL\n").unwrap();
    let fresh = dir.path().join("fresh.yaml");
    let existing_arg = existing.display().to_string();
    let fresh_arg = fresh.display().to_string();

    for (format, target_arg, target) in [
        ("json", existing_arg.as_str(), &existing),
        ("yaml", fresh_arg.as_str(), &fresh),
    ] {
        let matrix = fixture.matrix.display().to_string();
        let runs = fixture.runs.display().to_string();
        let args = [
            "--matrix",
            matrix.as_str(),
            "--runs",
            runs.as_str(),
            "--generated-at",
            GENERATED,
            "--format",
            format,
            "-o",
            target_arg,
        ];
        let output = report_in(dir.path(), &args);
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
        if target == &existing {
            assert_eq!(std::fs::read_to_string(target).unwrap(), "SENTINEL\n");
        } else {
            assert!(
                !target.exists(),
                "{format}: file written despite the failure"
            );
        }
    }

    let matrix = fixture.matrix.display().to_string();
    let runs = fixture.runs.display().to_string();
    let to_stdout = report_in(
        dir.path(),
        &[
            "--matrix",
            matrix.as_str(),
            "--runs",
            runs.as_str(),
            "--generated-at",
            GENERATED,
            "--format",
            "json",
        ],
    );
    assert_eq!(to_stdout.status.code(), Some(1));
    assert!(stdout(&to_stdout).is_empty(), "{}", stdout(&to_stdout));
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

#[test]
fn exit_codes_for_usage_and_input_errors() {
    let fixture = hostile_fixture();
    let matrix = fixture.matrix.display().to_string();
    let runs = fixture.runs.display().to_string();
    let absent = fixture.runs.join("absent.yaml").display().to_string();

    // (arguments, exit code, a word the message must contain)
    let cases: Vec<(Vec<&str>, i32, &str)> = vec![
        (vec!["--runs", runs.as_str()], 1, "--matrix"),
        (vec!["--matrix", matrix.as_str()], 1, "--runs"),
        (
            vec![
                "--matrix",
                matrix.as_str(),
                "--runs",
                runs.as_str(),
                "--format",
                "xml",
            ],
            1,
            "asciidoc",
        ),
        (
            vec![
                "--matrix",
                matrix.as_str(),
                "--runs",
                runs.as_str(),
                "--kind",
                "bogus",
            ],
            1,
            "bogus",
        ),
        (
            vec!["--matrix", absent.as_str(), "--runs", runs.as_str()],
            1,
            "absent.yaml",
        ),
        (
            vec![
                "--matrix",
                matrix.as_str(),
                "--runs",
                runs.as_str(),
                "--generated-at",
                "not-a-time",
            ],
            1,
            "not-a-time",
        ),
    ];
    for (args, code, word) in cases {
        let output = report(&args);
        assert_eq!(
            output.status.code(),
            Some(code),
            "{args:?}: {}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty(), "{args:?}: {}", stdout(&output));
        assert!(
            stderr(&output).contains(word),
            "{args:?}: {}",
            stderr(&output)
        );
    }

    for format in ["markdown", "asciidoc", "json", "yaml"] {
        let mut args = vec!["--matrix", matrix.as_str(), "--runs", runs.as_str()];
        args.extend(["--format", format]);
        let output = report(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{format}: {}",
            stderr(&output)
        );
    }
}
