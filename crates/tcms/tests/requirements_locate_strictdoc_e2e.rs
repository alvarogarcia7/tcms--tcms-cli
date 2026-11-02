//! `tcms requirements locate` for StrictDoc IDs, run as the real binary on exports made by the
//! real StrictDoc tool (crates/tcms/tests/fixtures/strictdoc). No network and no Python at test
//! time; StrictDoc is never called here. Every expected answer is a literal in this file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HLR1_LINE: &str = "HLR-1: found in strictdoc (strictdoc) at level HLR \"Update profile metadata\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-1";
const LLR1_LINE: &str = "LLR-1: found in strictdoc (strictdoc) at level LLR \"Parse the update frame\" [in_review] https://pages.example.com/srs/low-level-requirements.html#LLR-1";
const SYS1_LINE: &str = "SYS-1: found in strictdoc (strictdoc) at level system \"Remote profile management\" [approved] https://pages.example.com/srs/system-requirements.html#SYS-1";
const HLR99_LINE: &str = "HLR-99: not found in strictdoc (strictdoc)";

const TOKEN_ENV: &str = "TCMS_TEST_JIRA_TOKEN";

const STRICTDOC_YAML: &str = r#"product_id: PROD-002
requirement_source: strictdoc
systems:
  - name: strictdoc
    type: strictdoc_html
    base_url: https://pages.example.com/srs
    id_pattern: '^(SYS|HLR|LLR)-[0-9.]+$'
    url_template: '{base_url}/{doc}.html#{id}'
    document_map:
      SYS: system-requirements
      HLR: software-requirements
      LLR: low-level-requirements
"#;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Checked-in real StrictDoc export for one case (locate, refused-level, no-level).
fn export(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/strictdoc")
        .join(case)
        .join("export.json")
}

fn fixture_locate() -> String {
    export("locate").to_string_lossy().into_owned()
}

fn fixture_refused() -> String {
    export("refused-level").to_string_lossy().into_owned()
}

fn fixture_no_level() -> String {
    export("no-level").to_string_lossy().into_owned()
}

/// A fresh working directory holding the StrictDoc configuration.
fn fresh() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("integration-config.yaml"), STRICTDOC_YAML).unwrap();
    dir
}

/// Run `tcms requirements locate <args>` in `dir` with the token variable removed.
fn locate(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .current_dir(dir)
        .args([
            "--config",
            &dir.join("integration-config.yaml").to_string_lossy(),
        ])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(["requirements", "locate"])
        .args(args)
        .env_remove(TOKEN_ENV)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn code(o: &Output) -> Option<i32> {
    o.status.code()
}

/// Names and contents of every entry in `dir`, sorted.
fn snapshot(dir: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------- fixture

#[test]
fn fixture_is_the_output_of_the_real_strictdoc_tool() {
    let text = std::fs::read_to_string(export("locate")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value["_COMMENT"]
        .as_str()
        .unwrap()
        .starts_with("Fields with _ are metadata"));
    let docs = value["DOCUMENTS"].as_array().unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["TITLE"], "Locate Fixture Software");
    assert_eq!(docs[1]["TITLE"], "Locate Fixture System");
}

// ---------------------------------------------------------------- found, not found, batch (exit 0 / 1)

#[test]
fn locate_finds_an_hlr() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &["HLR-1", "--input", &input, "--format", "plaintext"],
    );
    assert_eq!(code(&o), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), format!("{HLR1_LINE}\n"));
    assert_eq!(stderr(&o), "");
}

#[test]
fn locate_finds_an_llr() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &["LLR-1", "--input", &input, "--format", "plaintext"],
    );
    assert_eq!(code(&o), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), format!("{LLR1_LINE}\n"));
    assert_eq!(stderr(&o), "");
}

#[test]
fn locate_finds_a_system_item() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &["SYS-1", "--input", &input, "--format", "plaintext"],
    );
    assert_eq!(code(&o), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), format!("{SYS1_LINE}\n"));
    assert_eq!(stderr(&o), "");
}

#[test]
fn locate_reports_an_id_that_is_not_in_the_export() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &["HLR-99", "--input", &input, "--format", "plaintext"],
    );
    assert_eq!(code(&o), Some(1));
    assert_eq!(stdout(&o), format!("{HLR99_LINE}\n"));
    assert!(
        stderr(&o).contains("not found: 1 of 1 requirement IDs"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn locate_a_uid_of_another_level_prefix_is_not_confused() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &[
            "LLR-99",
            "SYS-99",
            "--input",
            &input,
            "--format",
            "plaintext",
        ],
    );
    assert_eq!(code(&o), Some(1));
    assert_eq!(
        stdout(&o),
        "LLR-99: not found in strictdoc (strictdoc)\nSYS-99: not found in strictdoc (strictdoc)\n"
    );
    assert!(
        stderr(&o).contains("not found: 2 of 2 requirement IDs"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn locate_batch_gives_one_result_per_id_in_command_line_order() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &[
            "SYS-1",
            "HLR-99",
            "LLR-1",
            "HLR-1",
            "--input",
            &input,
            "--format",
            "plaintext",
        ],
    );
    assert_eq!(code(&o), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(
        stdout(&o),
        format!("{SYS1_LINE}\n{HLR99_LINE}\n{LLR1_LINE}\n{HLR1_LINE}\n")
    );
    assert!(
        stderr(&o).contains("not found: 1 of 4 requirement IDs"),
        "{}",
        stderr(&o)
    );
}

#[test]
fn locate_batch_all_found_exits_0_in_text_format() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(dir.path(), &["HLR-1", "LLR-1", "SYS-1", "--input", &input]);
    assert_eq!(code(&o), Some(0), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(
        out.contains("✓ HLR-1: found in strictdoc (strictdoc) at level HLR"),
        "{out}"
    );
    assert!(
        out.contains("✓ LLR-1: found in strictdoc (strictdoc) at level LLR"),
        "{out}"
    );
    assert!(
        out.contains("✓ SYS-1: found in strictdoc (strictdoc) at level system"),
        "{out}"
    );
    assert!(out.contains("3 of 3 requirement IDs found"), "{out}");
    assert!(!out.contains('✗'), "{out}");
}

#[test]
fn locate_text_format_marks_the_missing_id() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(dir.path(), &["HLR-1", "HLR-99", "--input", &input]);
    assert_eq!(code(&o), Some(1), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("✓ HLR-1: found in strictdoc"), "{out}");
    assert!(
        out.contains("✗ HLR-99: not found in strictdoc (strictdoc)"),
        "{out}"
    );
    assert!(out.contains("1 of 2 requirement IDs found"), "{out}");
}

#[test]
fn locate_a_repeated_id_is_reported_once() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &["HLR-1", "HLR-1", "--input", &input, "--format", "plaintext"],
    );
    assert_eq!(code(&o), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), format!("{HLR1_LINE}\n"));
}

#[test]
fn locate_json_has_the_expected_objects() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(
        dir.path(),
        &[
            "HLR-1", "SYS-1", "HLR-99", "--input", &input, "--format", "json",
        ],
    );
    assert_eq!(code(&o), Some(1), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(!out.contains('✓'), "{out}");
    assert!(!out.contains("requirement IDs found"), "{out}");
    let value: serde_json::Value = serde_json::from_str(&out).unwrap();
    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(
        array[0],
        serde_json::json!({
            "id": "HLR-1",
            "found": true,
            "source": "strictdoc",
            "system": "strictdoc",
            "level": "HLR",
            "title": "Update profile metadata",
            "status": "approved",
            "url": "https://pages.example.com/srs/software-requirements.html#HLR-1"
        })
    );
    assert_eq!(
        array[1],
        serde_json::json!({
            "id": "SYS-1",
            "found": true,
            "source": "strictdoc",
            "system": "strictdoc",
            "level": "system",
            "title": "Remote profile management",
            "status": "approved",
            "url": "https://pages.example.com/srs/system-requirements.html#SYS-1"
        })
    );
    assert_eq!(array[2]["id"], "HLR-99");
    assert_eq!(array[2]["found"], false);
    assert_eq!(array[2]["level"], serde_json::Value::Null);
    assert_eq!(array[2]["title"], serde_json::Value::Null);
    assert_eq!(array[2]["status"], serde_json::Value::Null);
}

#[test]
fn locate_writes_nothing() {
    let dir = fresh();
    let before = snapshot(dir.path());
    let input = fixture_locate();
    let bytes_before = std::fs::read(export("locate")).unwrap();
    let o = locate(dir.path(), &["HLR-1", "HLR-99", "--input", &input]);
    assert_eq!(code(&o), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(snapshot(dir.path()), before);
    assert_eq!(std::fs::read(export("locate")).unwrap(), bytes_before);
}

// ---------------------------------------------------------------- exit 2 (D-03)

#[test]
fn locate_an_id_matching_no_system_format_exits_2() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(dir.path(), &["PAY-1", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("PAY-1"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_unknown_format_in_a_batch_prints_nothing() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(dir.path(), &["HLR-1", "PAY-1", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("PAY-1"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_without_input_exits_2() {
    let dir = fresh();
    let o = locate(dir.path(), &["HLR-1"]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("--input"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_with_a_missing_input_file_exits_2() {
    let dir = fresh();
    let missing = dir.path().join("missing.json");
    let missing_arg = missing.to_string_lossy().into_owned();
    let o = locate(dir.path(), &["HLR-1", "--input", &missing_arg]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("missing.json"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_with_a_refused_level_exits_2() {
    let dir = fresh();
    let input = fixture_refused();
    let o = locate(dir.path(), &["HLR-8", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    let err = stderr(&o);
    assert!(err.contains("HLR-8"), "{err}");
    assert!(err.contains("chapter"), "{err}");
    assert!(err.contains("is not a requirement type"), "{err}");
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_with_an_item_without_level_exits_2() {
    let dir = fresh();
    let input = fixture_no_level();
    let o = locate(dir.path(), &["HLR-9", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    let err = stderr(&o);
    assert!(err.contains("HLR-9"), "{err}");
    assert!(err.contains("has no LEVEL"), "{err}");
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_with_the_old_lowercase_layout_exits_2() {
    let dir = fresh();
    let old = dir.path().join("old-layout.json");
    std::fs::write(
        &old,
        r#"{"documents":[{"path":"d.sdoc","requirements":[{"uid":"HLR-1","level":"HLR"}]}]}"#,
    )
    .unwrap();
    let input = old.to_string_lossy().into_owned();
    let o = locate(dir.path(), &["HLR-1", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("DOCUMENTS"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_with_a_non_json_input_exits_2() {
    let dir = fresh();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "not json").unwrap();
    let input = bad.to_string_lossy().into_owned();
    let o = locate(dir.path(), &["HLR-1", "--input", &input]);
    assert_eq!(code(&o), Some(2));
    assert!(stderr(&o).contains("StrictDoc export"), "{}", stderr(&o));
    assert_eq!(stdout(&o), "");
}

#[test]
fn locate_unknown_output_format_exits_2() {
    let dir = fresh();
    let input = fixture_locate();
    let o = locate(dir.path(), &["HLR-1", "--input", &input, "--format", "xml"]);
    assert_eq!(code(&o), Some(2));
    assert_eq!(stdout(&o), "");
}
