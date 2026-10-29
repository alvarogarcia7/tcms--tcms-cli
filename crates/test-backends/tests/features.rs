//! Feature roll-up writes (TCMS-42.6): validation follows the requirement, transitions only when
//! allowed and present, untraced features reported, dry run sends nothing, summary never written.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use id_resolver::IntegrationConfig;
use jira_client::{check_write, WriteKind, WriteRequest};
use serde_json::Value;
use test_backends::fake::FakeApi;
use test_backends::rollup::{FeatureWriter, PlannedWrite, ReqState, RollupInput, ValidationState};
use test_backends::{Api, StatusValue};

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

fn system(
    feature_fields: bool,
    feature_transitions: bool,
    transitions: &str,
) -> id_resolver::SystemConfig {
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
      verification_status: customfield_10101
      validation_status: customfield_10102
      coverage: customfield_10103
    status_values:
      passed: Verified
      failed: Failing
    write_policy:
      feature_fields: {feature_fields}
      feature_transitions: {feature_transitions}
{transitions}
"#
    );
    IntegrationConfig::from_yaml_str(&y, SCHEMAS, "features")
        .expect("config validates")
        .systems[0]
        .clone()
}

const TRANSITIONS: &str = "    transitions:\n      passed: Done\n      failed: Reopen\n";

fn traced(validation: ValidationState) -> RollupInput {
    RollupInput {
        requirements: vec![ReqState {
            covered: true,
            validation,
        }],
        test_statuses: vec![StatusValue::Passed],
        open_problem_reports: false,
    }
}

fn with_features(keys: &[&str]) -> Rc<RefCell<FakeApi>> {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    for k in keys {
        fake.borrow_mut().issues.insert(
            (*k).into(),
            serde_json::json!({"id": "1", "key": k, "fields": {"summary": "Feature", "description": "text",
                "project": {"key": "PAY"}, "issuetype": {"name": "Story"}}}),
        );
    }
    fake
}

fn feature_fields_of(fake: &FakeApi, key: &str) -> (Option<Value>, Option<Value>, Option<Value>) {
    (
        fake.issue_field(key, "customfield_10101"),
        fake.issue_field(key, "customfield_10102"),
        fake.issue_field(key, "customfield_10103"),
    )
}

#[test]
fn validation_status_follows_the_requirement_and_a_review_changes_it() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    fake.borrow_mut().issues.insert(
        "PAY-1".into(),
        serde_json::json!({"id": "1", "key": "PAY-1", "fields": {"summary": "Login", "description": "text",
            "project": {"key": "PAY"}, "issuetype": {"name": "Story"}}}),
    );
    let mut w = FeatureWriter::new(&system(true, false, ""), Box::new(Shared(fake.clone())));
    w.write("PAY-1", &traced(ValidationState::NotValidated), false)
        .unwrap();
    assert_eq!(
        feature_fields_of(&fake.borrow(), "PAY-1").1,
        Some(serde_json::json!("in_progress"))
    );
    // a validation review marks the requirement validated: the next roll-up follows
    w.write("PAY-1", &traced(ValidationState::Validated), false)
        .unwrap();
    assert_eq!(
        feature_fields_of(&fake.borrow(), "PAY-1").1,
        Some(serde_json::json!("Verified"))
    );
    // verification status uses status_values; coverage is a percent
    let (v, _, c) = feature_fields_of(&fake.borrow(), "PAY-1");
    assert_eq!(v, Some(serde_json::json!("Verified")));
    assert_eq!(c, Some(serde_json::json!(100)));
    // the feature's summary and description were never written
    let f = fake.borrow();
    assert_eq!(f.issues["PAY-1"]["fields"]["summary"], "Login");
    assert_eq!(f.issues["PAY-1"]["fields"]["description"], "text");
}

#[test]
fn transition_applies_only_when_allowed_and_present_otherwise_skipped() {
    // allowed and the workflow has "Done": the transition is applied
    let fake = with_features(&["PAY-1", "PAY-2"]);
    let mut w = FeatureWriter::new(
        &system(true, true, TRANSITIONS),
        Box::new(Shared(fake.clone())),
    );
    let planned = w
        .write("PAY-1", &traced(ValidationState::Validated), false)
        .unwrap();
    assert!(
        planned.contains(&PlannedWrite::Transition {
            feature: "PAY-1".into(),
            name: "Done".into()
        }),
        "{planned:?}"
    );
    let posts = fake
        .borrow()
        .writes()
        .iter()
        .filter(|c| c.path.ends_with("/transitions"))
        .count();
    assert_eq!(posts, 1);

    // the workflow has no "Reopen": logged and skipped
    let fake = with_features(&["PAY-1", "PAY-2"]);
    let mut w = FeatureWriter::new(
        &system(true, true, TRANSITIONS),
        Box::new(Shared(fake.clone())),
    );
    let failing = RollupInput {
        test_statuses: vec![StatusValue::Failed],
        ..traced(ValidationState::Validated)
    };
    let planned = w.write("PAY-2", &failing, false).unwrap();
    assert!(
        planned.iter().any(
            |p| matches!(p, PlannedWrite::Skipped { reason, .. } if reason.contains("Reopen"))
        ),
        "{planned:?}"
    );
    assert!(
        fake.borrow()
            .writes()
            .iter()
            .all(|c| !c.path.ends_with("/transitions")),
        "nothing posted"
    );

    // policy off: skipped without a workflow lookup for the write
    let fake = with_features(&["PAY-1"]);
    let mut w = FeatureWriter::new(
        &system(true, false, TRANSITIONS),
        Box::new(Shared(fake.clone())),
    );
    let planned = w
        .write("PAY-1", &traced(ValidationState::Validated), false)
        .unwrap();
    assert!(planned.iter().any(|p| matches!(p, PlannedWrite::Skipped { reason, .. } if reason.contains("feature_transitions"))));
    assert!(fake
        .borrow()
        .writes()
        .iter()
        .all(|c| !c.path.ends_with("/transitions")));
}

#[test]
fn feature_without_traced_requirement_is_reported_not_written() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut w = FeatureWriter::new(&system(true, false, ""), Box::new(Shared(fake.clone())));
    let planned = w.write("PAY-9", &RollupInput::default(), false).unwrap();
    assert!(
        matches!(planned.as_slice(), [PlannedWrite::Unreported { feature, .. }] if feature == "PAY-9")
    );
    assert!(fake.borrow().writes().is_empty());
}

#[test]
fn dry_run_prints_every_intended_write_and_sends_nothing() {
    let fake = with_features(&["PAY-1"]);
    let mut w = FeatureWriter::new(
        &system(true, true, TRANSITIONS),
        Box::new(Shared(fake.clone())),
    );
    let planned = w
        .write("PAY-1", &traced(ValidationState::Validated), true)
        .unwrap();
    assert_eq!(
        planned
            .iter()
            .filter(|p| matches!(p, PlannedWrite::Field { .. }))
            .count(),
        3,
        "{planned:?}"
    );
    assert!(planned
        .iter()
        .any(|p| matches!(p, PlannedWrite::Transition { .. })));
    assert!(
        fake.borrow().writes().is_empty(),
        "a dry run sends no write"
    );
}

#[test]
fn the_guard_refuses_summary_and_description_on_a_feature() {
    let sys = system(true, false, "");
    for f in ["summary", "description"] {
        let req = WriteRequest {
            kind: WriteKind::FeatureFields,
            issue_key: "PAY-1".into(),
            field_ids: vec![f.into()],
        };
        let err = check_write(&sys, &req).unwrap_err();
        assert!(err.to_string().contains("never written"), "{err}");
    }
    // a configured custom field passes; an unconfigured one is refused
    let ok = WriteRequest {
        kind: WriteKind::FeatureFields,
        issue_key: "PAY-1".into(),
        field_ids: vec!["customfield_10101".into()],
    };
    assert!(check_write(&sys, &ok).is_ok());
    let other = WriteRequest {
        kind: WriteKind::FeatureFields,
        issue_key: "PAY-1".into(),
        field_ids: vec!["customfield_99999".into()],
    };
    assert!(check_write(&sys, &other).is_err());
}

#[test]
fn the_guard_refuses_summary_and_description_on_an_update_of_a_dedicated_issue() {
    // a configured field id that maps to summary (a misconfiguration) must not overwrite the
    // summary of an existing TCMS-owned issue; creating one may still set summary and description
    let mut sys = system(false, false, "");
    sys.write_policy.dedicated_issues = true;
    for f in ["summary", "description"] {
        let update = WriteRequest {
            kind: WriteKind::DedicatedIssue,
            issue_key: "PAY-9".into(),
            field_ids: vec![f.into()],
        };
        let err = check_write(&sys, &update).unwrap_err();
        assert!(err.to_string().contains("never written"), "{err}");
    }
    let create = WriteRequest {
        kind: WriteKind::DedicatedIssue,
        issue_key: String::new(),
        field_ids: vec!["summary".into(), "description".into()],
    };
    assert!(check_write(&sys, &create).is_ok());
}
