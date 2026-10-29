//! Requirement import against the mock Jira (both layouts) and the StrictDoc fixture.
//! Verified against the mock only; the StrictDoc layout is an assumption (see strictdoc.rs).

use std::path::{Path, PathBuf};

use id_resolver::IntegrationConfig;
use jira_client::mock::{Layout, MockJira};
use jira_client::{JiraClient, Secret};
use requirement_import::jira::{self, LabelFlags};
use requirement_import::{strictdoc, write_requirements, RequirementRecord};

const TOKEN: &str = "tok-import-7d2c";
const ACTOR: &str = "alice";

fn schemas_root() -> String {
    format!("{}/../../schemas/", env!("CARGO_MANIFEST_DIR"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn jira_config(layout: Layout, base_url: &str) -> IntegrationConfig {
    let ty = if layout == Layout::Cloud {
        "jira_cloud"
    } else {
        "jira_datacenter"
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
      secret_ref: TCMS_TEST_TOKEN
    id_pattern: '^[A-Z]+-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [PAY, AUTH]
    feature_issue_types: [Epic, Story, Feature]
    defect_issue_type: Bug
sync:
  identity: tcms-bot
  record_actor: true
"#
    );
    IntegrationConfig::from_yaml_str(&yaml, &schemas_root(), "import-test").expect("config loads")
}

fn strictdoc_config() -> IntegrationConfig {
    let yaml = r#"product_id: PROD-002
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
    IntegrationConfig::from_yaml_str(yaml, &schemas_root(), "strictdoc-test").expect("config loads")
}

fn mock_jira(layout: Layout) -> MockJira {
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
        "Customer pays with a stored card",
        "In progress",
        "2026-10-20T09:00:00.000+0000",
        &["safety"],
    );
    mock.add_issue(
        "PAY-3",
        "Feature",
        "Refunds",
        "",
        "Open",
        "2026-10-21T09:00:00.000+0000",
        &["security"],
    );
    // a Bug is not a feature issue: never imported
    mock.add_issue(
        "PAY-9",
        "Bug",
        "Crash",
        "crash",
        "Open",
        "2026-10-21T09:00:00.000+0000",
        &[],
    );
    mock
}

fn import_jira(cfg: &IntegrationConfig, flags: LabelFlags) -> Vec<RequirementRecord> {
    let client =
        JiraClient::with_token(&cfg.systems[0], &cfg.sync, ACTOR, Secret::new(TOKEN), None)
            .expect("client");
    jira::fetch(&client, &cfg.systems[0], flags).expect("jira import")
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

fn check_both_layouts(layout: Layout) {
    let mock = mock_jira(layout);
    let cfg = jira_config(layout, &mock.base_url());
    let flags = LabelFlags {
        safety: Some("safety"),
        security: Some("security"),
    };
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("requirements");

    // first import: three records, the Bug is excluded
    let recs = import_jira(&cfg, flags);
    let ids: Vec<&str> = recs.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["PAY-1", "PAY-2", "PAY-3"], "{layout:?}");
    let rep = write_requirements(&out, &recs, &schemas_root()).expect("write");
    assert_eq!(rep.created, vec!["PAY-1", "PAY-2", "PAY-3"], "{layout:?}");
    let before = snapshot(&out);

    // the mapping in the files
    let epic: serde_yaml::Value = serde_yaml::from_str(&before[0].1).unwrap();
    assert_eq!(epic["type"], "system");
    assert_eq!(epic["source_system"], "jira");
    assert_eq!(epic["source"], "PAY-1");
    assert_eq!(epic["version"], "2026-10-01T10:00:00.000+0000");
    let story: serde_yaml::Value = serde_yaml::from_str(&before[1].1).unwrap();
    assert_eq!(story["type"], "HLR");
    assert_eq!(story["title"], "Pay by card");
    assert_eq!(story["text"], "Customer pays with a stored card");
    assert_eq!(story["safety_related"], true);
    assert_eq!(story["security_related"], false);
    assert_eq!(story["external_refs"][0]["key"], "PAY-2");
    assert_eq!(story["external_refs"][0]["status"], "In progress");
    let feature: serde_yaml::Value = serde_yaml::from_str(&before[2].1).unwrap();
    assert_eq!(feature["security_related"], true);
    assert!(
        !before.iter().any(|(n, _)| n.contains("PAY-9")),
        "Bug imported"
    );

    // re-import with nothing changed: no file is touched
    let stamp_before: Vec<_> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().modified().unwrap())
        .collect();
    let recs2 = import_jira(&cfg, flags);
    let rep2 = write_requirements(&out, &recs2, &schemas_root()).expect("rewrite");
    assert!(
        rep2.created.is_empty() && rep2.updated.is_empty(),
        "{layout:?}: {rep2:?}"
    );
    assert_eq!(rep2.unchanged, vec!["PAY-1", "PAY-2", "PAY-3"]);
    assert_eq!(
        snapshot(&out),
        before,
        "{layout:?}: re-import changed content"
    );
    let stamp_after: Vec<_> = std::fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().metadata().unwrap().modified().unwrap())
        .collect();
    assert_eq!(stamp_before.len(), stamp_after.len());

    // one issue changes in Jira: exactly that file is updated
    mock.add_issue(
        "PAY-2",
        "Story",
        "Pay by card",
        "Customer pays with a stored or new card",
        "In review",
        "2026-10-27T08:00:00.000+0000",
        &["safety"],
    );
    let recs3 = import_jira(&cfg, flags);
    let rep3 = write_requirements(&out, &recs3, &schemas_root()).expect("update");
    assert_eq!(rep3.updated, vec!["PAY-2"], "{layout:?}");
    assert_eq!(rep3.unchanged, vec!["PAY-1", "PAY-3"], "{layout:?}");
    let after = snapshot(&out);
    let changed: Vec<&str> = before
        .iter()
        .zip(after.iter())
        .filter(|(a, b)| a != b)
        .map(|(a, _)| a.0.as_str())
        .collect();
    assert_eq!(changed, vec!["PAY-2.yaml"], "{layout:?}");
}

#[test]
fn jira_import_cloud() {
    check_both_layouts(Layout::Cloud);
}

#[test]
fn jira_import_datacenter() {
    check_both_layouts(Layout::DataCenter);
}

#[test]
fn stale_records_are_reported_and_kept() {
    let mock = mock_jira(Layout::Cloud);
    let cfg = jira_config(Layout::Cloud, &mock.base_url());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path();
    let recs = import_jira(&cfg, LabelFlags::default());
    write_requirements(out, &recs, &schemas_root()).unwrap();
    // a record of the same source that the source no longer has
    let old = RequirementRecord::new(
        "PAY-77",
        "jira",
        "HLR",
        "2026-01-01T00:00:00.000+0000",
        "PAY-77",
    );
    write_requirements(out, &[old], &schemas_root()).unwrap();
    let rep = write_requirements(out, &recs, &schemas_root()).unwrap();
    assert_eq!(rep.stale, vec!["PAY-77"]);
    assert!(
        out.join("PAY-77.yaml").exists(),
        "stale record must not be deleted"
    );
}

#[test]
fn a_directory_of_the_other_source_is_refused_before_any_write() {
    let mock = mock_jira(Layout::DataCenter);
    let cfg = jira_config(Layout::DataCenter, &mock.base_url());
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("requirements");

    let sd = strictdoc::parse_export(
        &std::fs::read_to_string(fixture("strictdoc_export.json")).unwrap(),
        "9f1c2ab",
    )
    .unwrap();
    write_requirements(&out, &sd, &schemas_root()).unwrap();
    let before = snapshot(&out);

    let recs = import_jira(&cfg, LabelFlags::default());
    let err = format!(
        "{:#}",
        write_requirements(&out, &recs, &schemas_root()).unwrap_err()
    );
    assert!(
        err.contains("exactly one source") && err.contains("strictdoc"),
        "{err}"
    );
    assert_eq!(snapshot(&out), before, "nothing may be written");
}

#[test]
fn strictdoc_fixture_imports_with_path_uid_and_commit() {
    let text = std::fs::read_to_string(fixture("strictdoc_export.json")).unwrap();
    let cfg = strictdoc_config();
    assert_eq!(cfg.source_system().unwrap().name, "strictdoc");
    let recs = strictdoc::parse_export(&text, "9f1c2ab").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("requirements");

    let rep = write_requirements(&out, &recs, &schemas_root()).unwrap();
    assert_eq!(rep.created, vec!["HLR-12", "HLR-3", "SYS-4"]);
    let hlr12: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(out.join("HLR-12.yaml")).unwrap()).unwrap();
    assert_eq!(hlr12["source_system"], "strictdoc");
    assert_eq!(hlr12["type"], "HLR");
    assert_eq!(
        hlr12["source"], "docs/srs.sdoc#HLR-12",
        "document path and UID"
    );
    assert_eq!(hlr12["version"], "9f1c2ab", "commit");
    assert_eq!(hlr12["parent_ids"][0], "SYS-4");
    assert_eq!(hlr12["external_refs"][0]["system"], "strictdoc");
    assert_eq!(hlr12["external_refs"][0]["key"], "HLR-12");
    let sys: serde_yaml::Value =
        serde_yaml::from_str(&std::fs::read_to_string(out.join("SYS-4.yaml")).unwrap()).unwrap();
    assert_eq!(sys["type"], "system");
    assert_eq!(sys["source"], "docs/system.sdoc#SYS-4");

    // idempotent
    let before = snapshot(&out);
    let rep2 = write_requirements(&out, &recs, &schemas_root()).unwrap();
    assert_eq!(rep2.unchanged, vec!["HLR-12", "HLR-3", "SYS-4"]);
    assert_eq!(snapshot(&out), before);

    // the same export at another commit changes the version of every record, and only that
    let recs_new = strictdoc::parse_export(&text, "b7e0d41").unwrap();
    let rep3 = write_requirements(&out, &recs_new, &schemas_root()).unwrap();
    assert_eq!(rep3.updated.len(), 3);
}

#[test]
fn duplicate_ids_are_refused() {
    let text = std::fs::read_to_string(fixture("strictdoc_export.json")).unwrap();
    let mut recs = strictdoc::parse_export(&text, "abc").unwrap();
    recs.push(recs[0].clone());
    let dir = tempfile::tempdir().unwrap();
    let err = format!(
        "{:#}",
        write_requirements(dir.path(), &recs, &schemas_root()).unwrap_err()
    );
    assert!(err.contains("appears twice"), "{err}");
}
