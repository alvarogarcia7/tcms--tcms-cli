//! `tcms pr push` / `tcms pr link` run as the real binary against the jira-client mock
//! (Cloud layout). Verified against the mock only.

use std::path::PathBuf;
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-pr-5d1c9e";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn setup(base_url: &str, defects: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let cfg = format!(
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
    defect_issue_type: Bug
    severity_to_priority:
      critical: Highest
      major: High
    tcms_issue_types:
      link_type: Relates
    write_policy:
      defects: {defects}
sync:
  identity: tcms-bot
  record_actor: true
"#
    );
    std::fs::write(dir.path().join("integration-config.yaml"), cfg).unwrap();
    std::fs::create_dir_all(dir.path().join("problem-reports")).unwrap();
    std::fs::write(
        dir.path().join("problem-reports/PR-007.yaml"),
        r#"type: problem_report
schema: tcms/problem-report.schema.json
id: PR-007
title: Login fails on retry
description: Second login returns 500.
severity: critical
category: code
status: open
raised_by: tester1
date: "2026-10-20"
affected_items:
  - entity_type: requirement
    entity_id: PAY-1
"#,
    )
    .unwrap();
    dir
}

fn tcms(dir: &tempfile::TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .current_dir(dir.path())
        .args(["--config", "integration-config.yaml"])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(args)
        .env(ENV, TOKEN)
        .env_remove("TCMS_JIRA_USER")
        .output()
        .unwrap()
}

fn mock() -> MockJira {
    let m = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    m.add_issue(
        "PAY-1",
        "Story",
        "Login",
        "f",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m.add_issue(
        "PAY-9",
        "Bug",
        "Old",
        "x",
        "Done",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m
}

#[test]
fn push_dry_run_writes_nothing() {
    let m = mock();
    let dir = setup(&m.base_url(), true);
    let before = std::fs::read_to_string(dir.path().join("problem-reports/PR-007.yaml")).unwrap();
    let out = tcms(
        &dir,
        &["pr", "push", "PR-007", "--as", "alvaro", "--dry-run"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("would create Bug '[PR-007] Login fails on retry'"),
        "{text}"
    );
    assert!(text.contains("would link PAY-1"), "{text}");
    assert_eq!(m.write_requests().len(), 0, "dry run sends nothing");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("problem-reports/PR-007.yaml")).unwrap(),
        before
    );
}

#[test]
fn push_creates_defect_and_records_its_key() {
    let m = mock();
    let dir = setup(&m.base_url(), true);
    let out = tcms(&dir, &["pr", "push", "PR-007", "--as", "alvaro"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rec = std::fs::read_to_string(dir.path().join("problem-reports/PR-007.yaml")).unwrap();
    let v: serde_yaml::Value = serde_yaml::from_str(&rec).unwrap();
    let key = v["external_refs"][0]["key"].as_str().unwrap().to_string();
    assert!(key.starts_with("PAY-"), "{rec}");
    assert_eq!(v["external_refs"][0]["system"], "jira");
    let issue = m.issue(&key).expect("defect on the mock");
    assert!(issue["fields"]["description"]
        .to_string()
        .contains("pushed by alvaro"));

    // a second push names the existing key and sends nothing
    let before = m.write_requests().len();
    let again = tcms(&dir, &["pr", "push", "PR-007", "--as", "alvaro"]);
    assert!(String::from_utf8_lossy(&again.stdout).contains("already linked"));
    assert_eq!(m.write_requests().len(), before);

    // the token never reaches the record or the output
    assert!(!rec.contains(TOKEN));
    assert!(!String::from_utf8_lossy(&out.stdout).contains(TOKEN));
}

#[test]
fn push_refused_by_policy_fails_and_writes_nothing() {
    let m = mock();
    let dir = setup(&m.base_url(), false);
    let out = tcms(&dir, &["pr", "push", "PR-007", "--as", "alvaro"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("defects is off"));
    assert_eq!(m.write_requests().len(), 0);
}

#[test]
fn link_records_an_existing_issue_and_refuses_a_second_one() {
    let m = mock();
    let dir = setup(&m.base_url(), true);
    let out = tcms(&dir, &["pr", "link", "PR-007", "PAY-9", "--as", "alvaro"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(m.write_requests().is_empty(), "linking reads only");
    let rec = std::fs::read_to_string(dir.path().join("problem-reports/PR-007.yaml")).unwrap();
    assert!(rec.contains("PAY-9"));
    let other = tcms(&dir, &["pr", "link", "PR-007", "PAY-1", "--as", "alvaro"]);
    assert!(!other.status.success());
    assert!(String::from_utf8_lossy(&other.stderr).contains("already linked to PAY-9"));
}

const TOKEN_CHECK: &str = TOKEN;
#[test]
fn token_is_not_in_the_mock_request_bodies() {
    let m = mock();
    let dir = setup(&m.base_url(), true);
    let _ = tcms(&dir, &["pr", "push", "PR-007", "--as", "alvaro"]);
    assert!(m
        .write_requests()
        .iter()
        .all(|r| !r.body.contains(TOKEN_CHECK)));
}
