//! `tcms requirements locate` run as the real binary, so every exit path goes through
//! `locate::run` and `main`. Jira runs against the mock (Cloud layout); StrictDoc runs against
//! the existing export fixture. No new backend is needed for any case.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-locate-7c3e";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> PathBuf {
    root().join("crates/requirement-import/tests/fixtures/strictdoc_export.json")
}

fn write(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("integration-config.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn jira_yaml(base_url: &str) -> String {
    format!(
        r#"product_id: PROD-001
requirement_source: jira
systems:
  - name: jira
    type: jira_cloud
    base_url: {base_url}
    auth:
      method: service_account_token
      secret_ref: {ENV}
    id_pattern: '^[A-Z]+-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [PAY]
    feature_issue_types: [Epic, Story, Feature]
sync:
  identity: tcms-bot
  record_actor: true
"#
    )
}

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
"#;

fn tcms(cfg: &Path, args: &[&str], token: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    cmd.args(["--config", &cfg.to_string_lossy()])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(args)
        .env_remove(ENV);
    if let Some(t) = token {
        cmd.env(ENV, t);
    }
    cmd.output().unwrap()
}

fn code(o: &Output) -> i32 {
    o.status.code().expect("process ended by a signal")
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A mock with PAY-1 and PAY-2 present; PAY-9 is never added.
fn jira_mock() -> MockJira {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    mock.add_issue(
        "PAY-1",
        "Epic",
        "Payments",
        "Epic text",
        "In progress",
        "2026-10-01T10:00:00.000+0000",
        &[],
    );
    mock.add_issue(
        "PAY-2",
        "Story",
        "Pay by card",
        "Stored card",
        "Done",
        "2026-10-20T09:00:00.000+0000",
        &[],
    );
    mock
}

fn locate_jira(mock: &MockJira, ids: &[&str], format: Option<&str>) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(&mock.base_url()));
    let mut args = vec!["requirements", "locate"];
    args.extend_from_slice(ids);
    args.extend_from_slice(&["--as", "alice"]);
    if let Some(f) = format {
        args.extend_from_slice(&["--format", f]);
    }
    tcms(&cfg, &args, Some(TOKEN))
}

#[test]
fn exit_0_when_every_jira_id_is_found() {
    let mock = jira_mock();
    let o = locate_jira(&mock, &["PAY-1", "PAY-2"], None);
    assert_eq!(code(&o), 0, "stderr: {}", err(&o));
    let base = mock.base_url();
    assert_eq!(
        out(&o),
        format!(
            "✓ PAY-1: found in jira (jira) \"Payments\" [In progress] {base}/browse/PAY-1\n\
             ✓ PAY-2: found in jira (jira) \"Pay by card\" [Done] {base}/browse/PAY-2\n\
             2 of 2 requirement IDs found\n"
        )
    );
    assert!(
        !err(&o).contains("not found:"),
        "no summary on success: {}",
        err(&o)
    );
}

#[test]
fn exit_1_when_a_jira_id_is_missing_from_the_source() {
    let mock = jira_mock();
    let o = locate_jira(&mock, &["PAY-1", "PAY-9"], None);
    assert_eq!(code(&o), 1, "stderr: {}", err(&o));
    let base = mock.base_url();
    assert_eq!(
        out(&o),
        format!(
            "✓ PAY-1: found in jira (jira) \"Payments\" [In progress] {base}/browse/PAY-1\n\
             ✗ PAY-9: not found in jira (jira)\n\
             1 of 2 requirement IDs found\n"
        )
    );
    assert_eq!(err(&o), "not found: 1 of 2 requirement IDs\n");
}

#[test]
fn exit_1_summary_is_on_stderr_and_plaintext_has_no_summary_line() {
    let mock = jira_mock();
    let o = locate_jira(&mock, &["PAY-9"], Some("plaintext"));
    assert_eq!(code(&o), 1, "stderr: {}", err(&o));
    assert_eq!(out(&o), "PAY-9: not found in jira (jira)\n");
    assert_eq!(err(&o), "not found: 1 of 1 requirement IDs\n");
}

#[test]
fn exit_1_json_still_prints_the_array_on_stdout() {
    let mock = jira_mock();
    let o = locate_jira(&mock, &["PAY-1", "PAY-9"], Some("json"));
    assert_eq!(code(&o), 1, "stderr: {}", err(&o));
    let value: serde_json::Value = serde_json::from_str(&out(&o)).unwrap();
    assert_eq!(value[0]["found"], true);
    assert_eq!(value[1]["id"], "PAY-9");
    assert_eq!(value[1]["found"], false);
    assert_eq!(err(&o), "not found: 1 of 2 requirement IDs\n");
}

#[test]
fn exit_2_for_an_unresolvable_id_with_empty_stdout() {
    let mock = jira_mock();
    let o = locate_jira(&mock, &["PAY-1", "not-an-id"], None);
    assert_eq!(code(&o), 2, "stderr: {}", err(&o));
    assert!(out(&o).is_empty(), "stdout must be empty: {}", out(&o));
    assert!(err(&o).starts_with("error:"), "{}", err(&o));
    assert!(err(&o).contains("not-an-id"), "{}", err(&o));
}

#[test]
fn exit_2_when_the_jira_token_is_missing_with_empty_stdout() {
    let mock = jira_mock();
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(&mock.base_url()));
    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-1", "--as", "alice"],
        None,
    );
    assert_eq!(code(&o), 2, "stderr: {}", err(&o));
    assert!(out(&o).is_empty(), "stdout must be empty: {}", out(&o));
    assert!(err(&o).starts_with("error:"), "{}", err(&o));
    assert!(err(&o).contains(ENV), "{}", err(&o));
}

#[test]
fn exit_2_for_a_strictdoc_id_without_input_with_empty_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let o = tcms(&cfg, &["requirements", "locate", "SYS-4"], None);
    assert_eq!(code(&o), 2, "stderr: {}", err(&o));
    assert!(out(&o).is_empty(), "stdout must be empty: {}", out(&o));
    assert!(err(&o).contains("pass --input"), "{}", err(&o));
}

#[test]
fn exit_2_for_a_usage_error_with_empty_stdout() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let o = tcms(&cfg, &["requirements", "locate"], None);
    assert_eq!(code(&o), 2, "stderr: {}", err(&o));
    assert!(out(&o).is_empty(), "stdout must be empty: {}", out(&o));
    assert!(!err(&o).is_empty());
}

#[test]
fn strictdoc_exit_0_when_every_id_is_in_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture().to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-12", "SYS-4", "--input", &fx],
        None,
    );
    assert_eq!(code(&o), 0, "stderr: {}", err(&o));
    assert!(
        out(&o).contains("2 of 2 requirement IDs found"),
        "{}",
        out(&o)
    );
    assert!(!err(&o).contains("not found:"), "{}", err(&o));
}

#[test]
fn strictdoc_exit_1_when_an_id_is_missing_from_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture().to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-12", "HLR-99", "--input", &fx],
        None,
    );
    assert_eq!(code(&o), 1, "stderr: {}", err(&o));
    assert!(
        out(&o).contains("✗ HLR-99: not found in strictdoc (strictdoc)"),
        "{}",
        out(&o)
    );
    assert!(
        out(&o).contains("1 of 2 requirement IDs found"),
        "{}",
        out(&o)
    );
    assert_eq!(err(&o), "not found: 1 of 2 requirement IDs\n");
}
