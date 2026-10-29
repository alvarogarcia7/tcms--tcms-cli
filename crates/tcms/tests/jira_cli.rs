//! `tcms jira test-connection` run as the real binary against the mock Jira (Cloud and Data
//! Center layouts). Verified against the mock only.

use std::path::PathBuf;
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const RIGHT: &str = "tok-right-3c7e1a";
const WRONG: &str = "tok-wrong-9f8e2d";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write_config(
    dir: &tempfile::TempDir,
    layout: Layout,
    base_url: &str,
    projects: &str,
) -> PathBuf {
    let ty = match layout {
        Layout::Cloud => "jira_cloud",
        Layout::DataCenter => "jira_datacenter",
    };
    let yaml = format!(
        r#"product_id: PROD-001
requirement_source: jira
systems:
  - name: jira
    type: {ty}
    base_url: {base_url}
    auth:
      method: service_account_token
      secret_ref: {ENV}
    id_pattern: '^[A-Z]+-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [{projects}]
sync:
  identity: tcms-bot
  record_actor: true
"#
    );
    let path = dir.path().join("integration-config.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

/// Run `tcms --config <cfg> jira test-connection`. `token` is the value of the env variable,
/// or None to leave it unset.
fn run_test_connection(cfg: &std::path::Path, token: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    cmd.args(["--config", &cfg.to_string_lossy()])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(["jira", "test-connection", "--as", "alice"])
        .env_remove(ENV)
        .env_remove("TCMS_JIRA_USER");
    if let Some(t) = token {
        cmd.env(ENV, t);
    }
    cmd.output().unwrap()
}

fn text(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn connection_ok(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {RIGHT}"));
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(&dir, layout, &mock.base_url(), "PAY, AUTH");
    let o = run_test_connection(&cfg, Some(RIGHT));
    let (out, err) = text(&o);
    assert!(o.status.success(), "{layout:?}: stdout={out} stderr={err}");
    assert!(
        out.contains(&format!("site:       {}", mock.base_url())),
        "{out}"
    );
    assert!(out.contains("user:       TCMS Bot"), "{out}");
    assert!(
        out.contains("PAY") && out.contains("Payments") && out.contains("accessible"),
        "{out}"
    );
    assert!(
        out.contains("AUTH") && out.contains("Authentication"),
        "{out}"
    );
    assert!(!out.contains("NOT ACCESSIBLE"), "{out}");
    assert!(
        !out.contains(RIGHT) && !err.contains(RIGHT),
        "token printed"
    );
    // the command only reads
    assert!(mock.write_requests().is_empty());
}

#[test]
fn test_connection_reports_site_user_and_projects_cloud() {
    connection_ok(Layout::Cloud);
}

#[test]
fn test_connection_reports_site_user_and_projects_datacenter() {
    connection_ok(Layout::DataCenter);
}

#[test]
fn inaccessible_project_is_reported_and_fails() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {RIGHT}"));
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(&dir, Layout::Cloud, &mock.base_url(), "PAY, NOPE");
    let o = run_test_connection(&cfg, Some(RIGHT));
    let (out, err) = text(&o);
    assert!(!o.status.success());
    assert!(
        out.contains("NOPE") && out.contains("NOT ACCESSIBLE"),
        "{out}"
    );
    assert!(
        err.contains("not accessible") && err.contains("NOPE"),
        "{err}"
    );
}

#[test]
fn wrong_token_fails_without_echoing_it() {
    for layout in [Layout::Cloud, Layout::DataCenter] {
        let mock = MockJira::start(layout, &format!("Bearer {RIGHT}"));
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_config(&dir, layout, &mock.base_url(), "PAY");
        let o = run_test_connection(&cfg, Some(WRONG));
        let (out, err) = text(&o);
        assert!(!o.status.success(), "{layout:?}");
        assert!(err.contains("HTTP 401"), "{layout:?}: {err}");
        assert!(
            !out.contains(WRONG) && !err.contains(WRONG),
            "{layout:?}: token echoed"
        );
    }
}

#[test]
fn unset_token_variable_is_named_in_the_error() {
    let mock = MockJira::start(Layout::DataCenter, &format!("Bearer {RIGHT}"));
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(&dir, Layout::DataCenter, &mock.base_url(), "PAY");
    let o = run_test_connection(&cfg, None);
    let (_, err) = text(&o);
    assert!(!o.status.success());
    assert!(err.contains(ENV), "{err}");
    assert_eq!(mock.request_count(), 0, "no request without a token");
}
