//! Test case statuses (TCMS-42.5): lifecycle at the next sync, hand edits overwritten and
//! logged, unmapped cases reported when creation is not allowed.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use id_resolver::IntegrationConfig;
use serde_json::{json, Map, Value};
use test_backends::fake::FakeApi;
use test_backends::{
    build_backend, overwrite_audit_entry, sync_case, sync_case_or_unmapped, Api, BackendKind,
    CaseOutcome, CaseStatus, CaseSync, EntityKey, LifecycleStatus, TestBackend,
};

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

fn system(kind: BackendKind, dedicated: bool) -> id_resolver::SystemConfig {
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
    test_backend: {}
    tcms_issue_types:
      test: Test
      test_run: Test Execution
      link_type: Tests
    fields:
      tcms_id: customfield_10001
      test_lifecycle_status: customfield_10002
      latest_verdict: customfield_10003
      baseline: customfield_10004
    write_policy:
      dedicated_issues: {}
"#,
        kind.config_name(),
        dedicated
    );
    IntegrationConfig::from_yaml_str(&y, SCHEMAS, "statuses")
        .expect("config validates")
        .systems[0]
        .clone()
}

fn case(lifecycle: LifecycleStatus, known: Option<EntityKey>) -> CaseSync {
    CaseSync {
        test_case_id: "TC_LOGIN_001".into(),
        title: "Login".into(),
        lifecycle,
        feature_keys: vec![],
        known,
    }
}

fn lifecycle_in(fake: &FakeApi, kind: BackendKind, e: &EntityKey) -> Option<Value> {
    match kind {
        BackendKind::ZephyrScale => fake
            .object(&e.key)
            .map(|o| o["body"]["customFields"]["customfield_10002"].clone()),
        _ => fake.issue_field(&e.key, "customfield_10002"),
    }
}

#[test]
fn lifecycle_change_is_written_at_the_next_sync() {
    for kind in [
        BackendKind::PlainJira,
        BackendKind::Xray,
        BackendKind::ZephyrSquad,
        BackendKind::ZephyrScale,
    ] {
        let fake = Rc::new(RefCell::new(FakeApi::new()));
        let mut b: Box<dyn TestBackend> =
            build_backend(&system(kind, true), Box::new(Shared(fake.clone()))).unwrap();
        let e = sync_case(b.as_mut(), &case(LifecycleStatus::InReview, None)).unwrap();
        assert_eq!(
            lifecycle_in(&fake.borrow(), kind, &e),
            Some(json!("in_review")),
            "{kind:?}"
        );
        // review approves the case; the next sync (with the known entity) writes it
        sync_case(
            b.as_mut(),
            &case(LifecycleStatus::Approved, Some(e.clone())),
        )
        .unwrap();
        assert_eq!(
            lifecycle_in(&fake.borrow(), kind, &e),
            Some(json!("approved")),
            "{kind:?}"
        );
    }
}

fn last(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn hand_edit_is_overwritten_and_the_overwrite_is_logged() {
    // plain Jira
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut b = build_backend(
        &system(BackendKind::PlainJira, true),
        Box::new(Shared(fake.clone())),
    )
    .unwrap();
    let e = sync_case(b.as_mut(), &case(LifecycleStatus::Approved, None)).unwrap();
    // someone edits the lifecycle field by hand in Jira
    fake.borrow_mut().issues.get_mut(&e.key).unwrap()["fields"]["customfield_10002"] =
        json!("draft");

    let status = CaseStatus {
        lifecycle: Some(LifecycleStatus::Approved),
        verdict: None,
        baseline: None,
        last_written: Some(last(&[("customfield_10002", json!("approved"))])),
    };
    b.write_case_status(&e, &status).unwrap();
    let logged = b.take_overwrites();
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0].before, json!("draft"));
    assert_eq!(logged[0].after, json!("approved"));
    assert_eq!(
        fake.borrow().issue_field(&e.key, "customfield_10002"),
        Some(json!("approved"))
    );
    assert!(b.take_overwrites().is_empty(), "taken once");

    // the audit entry for the overwrite validates as an audit-trail entry
    let entry = overwrite_audit_entry(
        &logged[0],
        "SYNC-1-001".into(),
        "alvaro",
        "TC_LOGIN_001",
        "2026-10-28T10:00:00Z",
    );
    assert_eq!(entry.action, "sync");
    assert_eq!(entry.before.as_ref().unwrap()["value"], json!("draft"));

    // no hand edit since: nothing logged
    b.write_case_status(&e, &status).unwrap();
    assert!(b.take_overwrites().is_empty());

    // no last-written value known (first sync): a difference is not called a hand edit
    fake.borrow_mut().issues.get_mut(&e.key).unwrap()["fields"]["customfield_10002"] =
        json!("draft");
    b.write_case_status(
        &e,
        &CaseStatus {
            last_written: None,
            ..status.clone()
        },
    )
    .unwrap();
    assert!(b.take_overwrites().is_empty());
}

#[test]
fn hand_edit_is_overwritten_on_zephyr_scale() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut b = build_backend(
        &system(BackendKind::ZephyrScale, true),
        Box::new(Shared(fake.clone())),
    )
    .unwrap();
    let e = sync_case(b.as_mut(), &case(LifecycleStatus::Approved, None)).unwrap();
    fake.borrow_mut().objects.get_mut(&e.key).unwrap()["body"]["customFields"]
        ["customfield_10002"] = json!("draft");
    let status = CaseStatus {
        lifecycle: None,
        verdict: None,
        baseline: None,
        last_written: Some(last(&[("customfield_10002", json!("approved"))])),
    };
    // a write must carry something to write: the lifecycle field is in the custom fields
    b.write_case_status(
        &e,
        &CaseStatus {
            lifecycle: Some(LifecycleStatus::Approved),
            ..status
        },
    )
    .unwrap();
    let logged = b.take_overwrites();
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0].before, json!("draft"));
    assert_eq!(
        lifecycle_in(&fake.borrow(), BackendKind::ZephyrScale, &e),
        Some(json!("approved"))
    );
}

#[test]
fn unmapped_when_creation_is_not_allowed_and_found_when_it_exists() {
    // no entity, creation off: reported as unmapped, nothing written
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut b = build_backend(
        &system(BackendKind::PlainJira, false),
        Box::new(Shared(fake.clone())),
    )
    .unwrap();
    let out = sync_case_or_unmapped(b.as_mut(), &case(LifecycleStatus::Approved, None)).unwrap();
    assert!(
        matches!(out, CaseOutcome::Unmapped(ref r) if r.contains("dedicated_issues")),
        "{out:?}"
    );
    assert!(
        fake.borrow().writes().is_empty(),
        "no write for an unmapped case"
    );

    // an existing Test issue with the tcms_id is found and mapped even with creation off
    fake.borrow_mut().issues.insert(
        "PAY-900".into(),
        json!({ "id": "900", "key": "PAY-900", "fields": {
            "project": {"key": "PAY"}, "issuetype": {"name": "Test"},
            "customfield_10001": "TC_LOGIN_001" } }),
    );
    let e = b
        .ensure_test(&case(LifecycleStatus::Approved, None))
        .unwrap();
    assert_eq!(e.key, "PAY-900", "found by tcms_id, not created");
    // writing its status is still a write to a TCMS-owned issue: refused while the policy is off
    let err = sync_case(b.as_mut(), &case(LifecycleStatus::Approved, None)).unwrap_err();
    assert!(err.to_string().contains("dedicated_issues"), "{err}");
}
