//! `tcms requirements import` run as the real binary: Jira against the mock (Cloud and Data
//! Center), StrictDoc against the assumed-format fixture. Verified against the mock only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-req-5a1f";

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

fn jira_yaml(layout: Layout, base_url: &str) -> String {
    let ty = match layout {
        Layout::Cloud => "jira_cloud",
        Layout::DataCenter => "jira_datacenter",
    };
    format!(
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

fn text(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

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

fn jira_import(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {TOKEN}"));
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
        "In progress",
        "2026-10-20T09:00:00.000+0000",
        &["safety"],
    );
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(layout, &mock.base_url()));
    let out = dir.path().join("requirements");
    let out_s = out.to_string_lossy().into_owned();

    let args = [
        "requirements",
        "import",
        "--out",
        &out_s,
        "--as",
        "alice",
        "--safety-label",
        "safety",
    ];
    let o = tcms(&cfg, &args, Some(TOKEN));
    let (stdout, stderr) = text(&o);
    assert!(o.status.success(), "{layout:?}: {stderr}");
    assert!(stdout.contains("2 created"), "{stdout}");
    assert!(!stdout.contains(TOKEN) && !stderr.contains(TOKEN));
    let first = snapshot(&out);
    assert_eq!(first.len(), 2);
    assert!(
        first[1].1.contains("safety_related: true"),
        "{}",
        first[1].1
    );

    // second run: nothing to change
    let o2 = tcms(&cfg, &args, Some(TOKEN));
    let (stdout2, _) = text(&o2);
    assert!(o2.status.success());
    assert!(
        stdout2.contains("2 unchanged")
            && stdout2.contains("0 created")
            && stdout2.contains("0 updated"),
        "{stdout2}"
    );
    assert_eq!(snapshot(&out), first, "{layout:?}: re-import changed files");
}

#[test]
fn jira_import_cloud() {
    jira_import(Layout::Cloud);
}

#[test]
fn jira_import_datacenter() {
    jira_import(Layout::DataCenter);
}

#[test]
fn strictdoc_import_records_path_uid_and_commit_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let out = dir.path().join("requirements");
    let out_s = out.to_string_lossy().into_owned();
    let fx = fixture().to_string_lossy().into_owned();
    let args = [
        "requirements",
        "import",
        "--out",
        &out_s,
        "--input",
        &fx,
        "--commit",
        "9f1c2ab",
    ];

    let o = tcms(&cfg, &args, None);
    let (stdout, stderr) = text(&o);
    assert!(o.status.success(), "{stderr}");
    assert!(stdout.contains("3 created"), "{stdout}");
    let hlr = std::fs::read_to_string(out.join("HLR-12.yaml")).unwrap();
    assert!(hlr.contains("source: docs/srs.sdoc#HLR-12"), "{hlr}");
    assert!(
        hlr.contains("version: '9f1c2ab'") || hlr.contains("version: 9f1c2ab"),
        "{hlr}"
    );
    let first = snapshot(&out);

    let o2 = tcms(&cfg, &args, None);
    assert!(o2.status.success());
    assert!(text(&o2).0.contains("3 unchanged"));
    assert_eq!(snapshot(&out), first);
}

#[test]
fn strictdoc_needs_the_commit() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture().to_string_lossy().into_owned();
    let o = tcms(&cfg, &["requirements", "import", "--input", &fx], None);
    assert!(!o.status.success());
    assert!(text(&o).1.contains("--commit"), "{:?}", text(&o));
}

#[test]
fn a_product_naming_both_sources_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = STRICTDOC_YAML.replace(
        "requirement_source: strictdoc",
        "requirement_source: [jira, strictdoc]",
    );
    let cfg = write(dir.path(), &yaml);
    let o = tcms(&cfg, &["requirements", "import"], None);
    assert!(!o.status.success());
    let err = text(&o).1;
    assert!(err.contains("names both jira and strictdoc"), "{err}");
    assert!(!dir.path().join("requirements").exists(), "nothing written");
}

#[test]
fn requirement_source_without_its_system_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = STRICTDOC_YAML.replace("requirement_source: strictdoc", "requirement_source: jira");
    let cfg = write(dir.path(), &yaml);
    let o = tcms(&cfg, &["requirements", "import"], None);
    assert!(!o.status.success());
    assert!(
        text(&o)
            .1
            .contains("no jira_cloud or jira_datacenter system"),
        "{:?}",
        text(&o)
    );
}

#[test]
fn jira_only_flags_are_refused_for_strictdoc() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture().to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &[
            "requirements",
            "import",
            "--input",
            &fx,
            "--commit",
            "abc",
            "--safety-label",
            "safety",
        ],
        None,
    );
    assert!(!o.status.success());
    assert!(
        text(&o).1.contains("apply to a Jira source"),
        "{:?}",
        text(&o)
    );
}
