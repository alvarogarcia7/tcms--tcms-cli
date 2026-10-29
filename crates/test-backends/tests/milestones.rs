//! Milestones (TCMS-42.7): Fix Version created once, description kept current, released on
//! completion, reopened plans reported, epic and issue targets refused clearly.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use id_resolver::IntegrationConfig;
use jira_client::{check_write, WriteKind, WriteRequest};
use serde_json::{json, Value};
use test_backends::fake::FakeApi;
use test_backends::milestone::{MilestoneSync, PlanState};
use test_backends::Api;

const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/");

struct Shared(Rc<RefCell<FakeApi>>);

impl Api for Shared {
    fn rest_prefix(&self) -> String {
        self.0.borrow().rest_prefix()
    }
    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.0.borrow_mut().get(path, query)
    }
    fn send(&mut self, m: &str, p: &str, b: Option<&Value>, k: Option<&str>) -> Result<Value> {
        self.0.borrow_mut().send(m, p, b, k)
    }
    fn search_jql(&mut self, jql: &str, f: &[&str]) -> Result<Vec<Value>> {
        self.0.borrow_mut().search_jql(jql, f)
    }
}

fn system(target: &str, feature_fields: bool) -> id_resolver::SystemConfig {
    let y = format!(
        r#"product_id: PRODUCT-PAY
requirement_source: jira
systems:
  - name: jira
    type: jira_cloud
    base_url: https://example.atlassian.net
    auth:
      method: service_account_token
      secret_ref: TCMS_JIRA_TOKEN
    id_pattern: "^(PAY|AUTH)-[0-9]+$"
    url_template: "{{base_url}}/browse/{{id}}"
    project_keys: [PAY]
    test_backend: none
    tcms_issue_types:
      test: Test
      link_type: Tests
    fields:
      tcms_id: customfield_10001
    milestone:
      target: {target}
      release_on_plan_complete: true
    write_policy:
      feature_fields: {feature_fields}
"#
    );
    IntegrationConfig::from_yaml_str(&y, SCHEMAS, "milestones")
        .expect("config validates")
        .systems[0]
        .clone()
}

fn plan(executed: u32, passed: u32, complete: bool) -> PlanState {
    PlanState {
        name: "SVP release 2.1".into(),
        baseline_id: "BL-7".into(),
        executed,
        total: 4,
        passed,
        complete,
        tcms_link: "tcms://test-plan/SVP-2.1".into(),
    }
}

#[test]
fn description_carries_the_plan_state_and_progress() {
    let p = plan(3, 2, false);
    let d = p.description();
    assert!(d.contains("Plan state: in progress"), "{d}");
    assert!(d.contains("Executed: 3/4"), "{d}");
    assert!(d.contains("Pass rate: 67%"), "{d}");
    assert!(d.contains("Baseline: BL-7"), "{d}");
    assert_eq!(plan(0, 0, false).state(), "not started");
    assert_eq!(plan(4, 4, true).state(), "complete");
}

#[test]
fn version_is_created_once_and_its_description_follows_every_sync() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    fake.borrow_mut().issues.insert(
        "PAY-1".into(),
        json!({"id": "1", "key": "PAY-1", "fields": {"summary": "Login", "project": {"key": "PAY"}, "issuetype": {"name": "Story"}}}),
    );
    let mut m =
        MilestoneSync::new(&system("fix_version", true), Box::new(Shared(fake.clone()))).unwrap();
    let first = m
        .sync(
            &plan(1, 1, false),
            None,
            &["PAY-1".into()],
            "2026-10-28",
            false,
        )
        .unwrap();
    assert!(first.created);
    assert!(!first.version_id.is_empty());
    // the next sync reuses the stored id and rewrites the description with the new progress
    let second = m
        .sync(
            &plan(3, 2, false),
            Some(&first.version_id),
            &[],
            "2026-10-28",
            false,
        )
        .unwrap();
    assert!(!second.created);
    assert_eq!(second.version_id, first.version_id);
    let stored = fake
        .borrow()
        .object(&format!("version-{}", first.version_id))
        .unwrap()["body"]
        .clone();
    assert!(
        stored["description"]
            .as_str()
            .unwrap()
            .contains("Executed: 3/4"),
        "{stored}"
    );
    assert_eq!(
        fake.borrow()
            .calls
            .iter()
            .filter(|c| c.method == "POST" && c.path.ends_with("/version"))
            .count(),
        1
    );
}

#[test]
fn completing_the_plan_releases_with_the_date_and_reopening_is_reported_not_undone() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut m =
        MilestoneSync::new(&system("fix_version", true), Box::new(Shared(fake.clone()))).unwrap();
    let created = m
        .sync(&plan(3, 3, false), None, &[], "2026-10-28", false)
        .unwrap();
    let done = m
        .sync(
            &plan(4, 4, true),
            Some(&created.version_id),
            &[],
            "2026-10-30",
            false,
        )
        .unwrap();
    assert!(done.released_now);
    let stored = fake
        .borrow()
        .object(&format!("version-{}", created.version_id))
        .unwrap()["body"]
        .clone();
    assert_eq!(stored["released"], json!(true));
    assert_eq!(stored["releaseDate"], json!("2026-10-30"));

    // reopened: nothing is un-released, the sync says so
    let reopened = m
        .sync(
            &plan(4, 3, false),
            Some(&created.version_id),
            &[],
            "2026-10-31",
            false,
        )
        .unwrap();
    assert!(!reopened.released_now);
    assert!(
        reopened.notes.iter().any(|n| n.contains("stays released")),
        "{:?}",
        reopened.notes
    );
    let still = fake
        .borrow()
        .object(&format!("version-{}", created.version_id))
        .unwrap()["body"]
        .clone();
    assert_eq!(still["released"], json!(true));
}

#[test]
fn features_get_the_fix_version_only_when_feature_fields_allows_it() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    fake.borrow_mut().issues.insert(
        "PAY-1".into(),
        json!({"id": "1", "key": "PAY-1", "fields": {"summary": "Login", "project": {"key": "PAY"}, "issuetype": {"name": "Story"}}}),
    );
    let mut m =
        MilestoneSync::new(&system("fix_version", true), Box::new(Shared(fake.clone()))).unwrap();
    let r = m
        .sync(
            &plan(1, 1, false),
            None,
            &["PAY-1".into()],
            "2026-10-28",
            false,
        )
        .unwrap();
    assert_eq!(r.features_assigned, vec!["PAY-1".to_string()]);
    let fv = &fake.borrow().issues["PAY-1"]["fields"]["fixVersions"];
    assert_eq!(fv[0]["id"], json!(r.version_id));
    assert_eq!(
        fake.borrow().issues["PAY-1"]["fields"]["summary"],
        "Login",
        "summary untouched"
    );

    // feature_fields off: no feature write, reported
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut m = MilestoneSync::new(
        &system("fix_version", false),
        Box::new(Shared(fake.clone())),
    )
    .unwrap();
    let r = m
        .sync(
            &plan(1, 1, false),
            None,
            &["PAY-1".into()],
            "2026-10-28",
            false,
        )
        .unwrap();
    assert!(r.features_assigned.is_empty());
    assert!(r.notes.iter().any(|n| n.contains("feature_fields")));
}

#[test]
fn dry_run_sends_nothing() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut m =
        MilestoneSync::new(&system("fix_version", true), Box::new(Shared(fake.clone()))).unwrap();
    let r = m
        .sync(
            &plan(4, 4, true),
            None,
            &["PAY-1".into()],
            "2026-10-28",
            true,
        )
        .unwrap();
    assert!(r.created && r.released_now);
    assert!(fake.borrow().writes().is_empty());
}

#[test]
fn epic_and_issue_targets_are_accepted_by_the_config_and_refused_clearly() {
    for target in ["epic", "issue"] {
        let sys = system(target, true);
        let err = MilestoneSync::new(
            &sys,
            Box::new(Shared(Rc::new(RefCell::new(FakeApi::new())))),
        )
        .err()
        .expect("refused");
        assert!(err.to_string().contains("not implemented yet"), "{err}");
    }
}

#[test]
fn the_guard_allows_version_writes_only_for_fix_version_and_fix_versions_on_features() {
    let fix = system("fix_version", true);
    assert!(check_write(
        &fix,
        &WriteRequest {
            kind: WriteKind::Milestone,
            issue_key: String::new(),
            field_ids: vec![]
        }
    )
    .is_ok());
    let epic = system("epic", true);
    assert!(check_write(
        &epic,
        &WriteRequest {
            kind: WriteKind::Milestone,
            issue_key: String::new(),
            field_ids: vec![]
        }
    )
    .is_err());
    let fv = WriteRequest {
        kind: WriteKind::FeatureFields,
        issue_key: "PAY-1".into(),
        field_ids: vec!["fixVersions".into()],
    };
    assert!(check_write(&fix, &fv).is_ok());
    assert!(
        check_write(&epic, &fv).is_err(),
        "fixVersions is only for the fix_version target"
    );
    let summary = WriteRequest {
        kind: WriteKind::FeatureFields,
        issue_key: "PAY-1".into(),
        field_ids: vec!["summary".into()],
    };
    assert!(check_write(&fix, &summary).is_err());
}
