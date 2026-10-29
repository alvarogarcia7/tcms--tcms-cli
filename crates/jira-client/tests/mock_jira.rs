//! Client behaviour against the mock Jira, for both the Cloud (REST v3) and the Data Center
//! (REST v2) layouts. Verified against the mock only; no real Jira is involved.

mod common;

use common::*;
use jira_client::guard::WriteKind;
use jira_client::mock::Layout;
use jira_client::{JiraClient, WriteTarget};
use serde_json::{json, Map, Value};

fn fields(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn target() -> WriteTarget {
    WriteTarget::new("requirement", "PAY-1")
}

// AC1: connection test reports site, user and the projects in project_keys
#[test]
fn connection_reports_site_user_and_projects_cloud() {
    connection(Layout::Cloud);
}

#[test]
fn connection_reports_site_user_and_projects_datacenter() {
    connection(Layout::DataCenter);
}

fn connection(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(
        layout,
        &mock.base_url(),
        &["PAY", "AUTH", "NOPE"],
        Policy::default(),
    );
    let c = client(&cfg, layout);
    let report = c.test_connection().expect("connection test runs");
    assert_eq!(report.site, mock.base_url(), "{layout:?}");
    assert_eq!(report.user, "TCMS Bot", "{layout:?}");
    assert!(report.deployment.contains(if layout == Layout::Cloud {
        "Cloud"
    } else {
        "Data Center"
    }));
    assert_eq!(report.version.as_deref(), Some("1000.0.0"));
    let names: Vec<(&str, bool)> = report
        .projects
        .iter()
        .map(|p| (p.key.as_str(), p.accessible))
        .collect();
    assert_eq!(
        names,
        vec![("PAY", true), ("AUTH", true), ("NOPE", false)],
        "{layout:?}"
    );
    assert_eq!(report.projects[0].name.as_deref(), Some("Payments"));
    assert!(!report.all_projects_accessible());
    // the test only reads
    assert!(mock.write_requests().is_empty(), "{layout:?}");
    // the request carried the layout's REST version
    let prefix = if layout == Layout::Cloud {
        "/rest/api/3/"
    } else {
        "/rest/api/2/"
    };
    assert!(
        mock.requests().iter().all(|r| r.path.starts_with(prefix)),
        "{layout:?}"
    );
}

#[test]
fn wrong_token_is_an_error_without_the_token_cloud() {
    wrong_token(Layout::Cloud);
}

#[test]
fn wrong_token_is_an_error_without_the_token_datacenter() {
    wrong_token(Layout::DataCenter);
}

fn wrong_token(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let c = JiraClient::with_token(
        &cfg.systems[0],
        &cfg.sync,
        ACTOR,
        jira_client::Secret::new("not-the-token"),
        service_user(layout),
    )
    .unwrap();
    let err = format!("{:#}", c.test_connection().unwrap_err());
    assert!(err.contains("HTTP 401"), "{err}");
    assert!(
        !err.contains("not-the-token") && !err.contains(TOKEN),
        "{err}"
    );
}

// AC5 (search side): token and startAt paging both yield every issue
#[test]
fn search_follows_pagination_cloud() {
    search_paging(Layout::Cloud);
}

#[test]
fn search_follows_pagination_datacenter() {
    search_paging(Layout::DataCenter);
}

fn search_paging(layout: Layout) {
    let mock = mock_with_issue(layout);
    for i in 2..=54 {
        mock.add_issue(
            &format!("PAY-{i}"),
            "Story",
            "s",
            "d",
            "Open",
            "2026-10-20T09:00:00.000+0000",
            &[],
        );
    }
    mock.add_issue(
        "PAY-900",
        "Epic",
        "epic",
        "d",
        "Open",
        "2026-10-20T09:00:00.000+0000",
        &[],
    );
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let c = client(&cfg, layout);
    let issues = c
        .search_issues(
            "project in (PAY) AND issuetype in (Story)",
            &["summary", "issuetype"],
        )
        .expect("search");
    assert_eq!(issues.len(), 54, "{layout:?}");
    assert!(issues
        .iter()
        .all(|i| i["fields"]["issuetype"]["name"] == "Story"));
}

// AC2 (positive side): an allowed field write goes through and yields an audit entry
#[test]
fn allowed_field_write_is_sent_and_audited_cloud() {
    allowed_field_write(Layout::Cloud);
}

#[test]
fn allowed_field_write_is_sent_and_audited_datacenter() {
    allowed_field_write(Layout::DataCenter);
}

fn allowed_field_write(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(
        layout,
        &mock.base_url(),
        &["PAY", "AUTH"],
        Policy::default(),
    );
    let mut c = client(&cfg, layout);
    let entry = c
        .update_fields(
            "PAY-1",
            WriteKind::FeatureFields,
            &fields(&[("customfield_10101", json!("in_progress"))]),
            &target(),
            "status sync",
        )
        .expect("allowed write");
    assert_eq!(mock.write_requests().len(), 1, "{layout:?}");
    assert_eq!(
        mock.issue("PAY-1").unwrap()["fields"]["customfield_10101"],
        "in_progress"
    );
    // AC3: the audit entry names the acting user, the external ref written and the field
    assert_eq!(entry.who, ACTOR);
    assert_eq!(entry.action, "sync");
    let after = entry.after.clone().unwrap();
    assert_eq!(after["external_refs"][0]["key"], "PAY-1");
    assert_eq!(after["external_refs"][0]["system"], "jira");
    assert_eq!(after["written_fields"][0], "customfield_10101");
    assert_eq!(c.audit_entries().len(), 1);
}

// AC3: the comment body names the same acting user as the audit entry
#[test]
fn comment_names_the_acting_user_cloud() {
    comment_names_actor(Layout::Cloud);
}

#[test]
fn comment_names_the_acting_user_datacenter() {
    comment_names_actor(Layout::DataCenter);
}

fn comment_names_actor(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    let entry = c
        .add_comment(
            "PAY-1",
            "Verification status: in progress",
            &target(),
            "status sync",
        )
        .unwrap();
    let writes = mock.write_requests();
    assert_eq!(writes.len(), 1);
    assert!(
        writes[0].path.ends_with("/issue/PAY-1/comment"),
        "{}",
        writes[0].path
    );
    assert!(
        writes[0].body.contains(&format!("by {ACTOR}")),
        "{}",
        writes[0].body
    );
    assert!(writes[0].body.contains("Verification status: in progress"));
    assert_eq!(entry.who, ACTOR);
    assert_eq!(entry.after.unwrap()["written_fields"][0], "comment");
}

// AC2: refused before any request is sent. Each case runs against a fresh mock that must
// receive zero requests.
#[test]
fn refused_writes_send_no_request_cloud() {
    refused_writes(Layout::Cloud);
}

#[test]
fn refused_writes_send_no_request_datacenter() {
    refused_writes(Layout::DataCenter);
}

type Act = Box<dyn Fn(&mut jira_client::JiraClient) -> anyhow::Result<()>>;

fn refused_writes(layout: Layout) {
    let refused: Vec<(&str, Policy, Act, &str)> = vec![
        (
            "field not in fields",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.update_fields(
                    "PAY-1",
                    WriteKind::FeatureFields,
                    &fields(&[("customfield_99999", json!("x"))]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "not listed in fields",
        ),
        (
            "summary of a feature",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.update_fields(
                    "PAY-1",
                    WriteKind::FeatureFields,
                    &fields(&[("summary", json!("x"))]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "never written by TCMS",
        ),
        (
            "feature_fields off",
            Policy {
                feature_fields: false,
                ..Policy::default()
            },
            Box::new(|c: &mut jira_client::JiraClient| {
                c.update_fields(
                    "PAY-1",
                    WriteKind::FeatureFields,
                    &fields(&[("customfield_10101", json!("x"))]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "feature_fields is off",
        ),
        (
            "transition while feature_transitions off",
            Policy {
                feature_transitions: false,
                ..Policy::default()
            },
            Box::new(|c: &mut jira_client::JiraClient| {
                c.transition("PAY-1", "Verified", &target(), "r")
                    .map(|_| ())
            }),
            "feature_transitions is off",
        ),
        (
            "transition not in config",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.transition("PAY-1", "Delete project", &target(), "r")
                    .map(|_| ())
            }),
            "not listed in transitions",
        ),
        (
            "comment while comments off",
            Policy {
                comments: false,
                ..Policy::default()
            },
            Box::new(|c: &mut jira_client::JiraClient| {
                c.add_comment("PAY-1", "hello", &target(), "r").map(|_| ())
            }),
            "comments is off",
        ),
        (
            "issue outside project_keys",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.add_comment("OTHER-7", "hello", &target(), "r")
                    .map(|_| ())
            }),
            "not in project_keys",
        ),
        (
            "create while dedicated_issues off",
            Policy {
                dedicated_issues: false,
                ..Policy::default()
            },
            Box::new(|c: &mut jira_client::JiraClient| {
                c.create_issue(
                    WriteKind::DedicatedIssue,
                    &fields(&[("project", json!({"key": "PAY"})), ("summary", json!("t"))]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "dedicated_issues is off",
        ),
        (
            "create in a project outside project_keys",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.create_issue(
                    WriteKind::Defect,
                    &fields(&[
                        ("project", json!({"key": "OTHER"})),
                        ("summary", json!("bug")),
                    ]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "not in project_keys",
        ),
        (
            "defect with an unlisted field",
            Policy::default(),
            Box::new(|c: &mut jira_client::JiraClient| {
                c.create_issue(
                    WriteKind::Defect,
                    &fields(&[
                        ("project", json!({"key": "PAY"})),
                        ("customfield_99999", json!("x")),
                    ]),
                    &target(),
                    "r",
                )
                .map(|_| ())
            }),
            "not listed in fields",
        ),
    ];
    for (case, policy, act, want) in refused {
        let mock = mock_with_issue(layout);
        let cfg = config(layout, &mock.base_url(), &["PAY", "AUTH"], policy);
        let mut c = client(&cfg, layout);
        let err = act(&mut c).expect_err(case);
        let msg = format!("{err:#}");
        assert!(msg.contains(want), "{layout:?} / {case}: {msg}");
        assert_eq!(
            mock.request_count(),
            0,
            "{layout:?} / {case}: a request was sent"
        );
        assert!(
            c.audit_entries().is_empty(),
            "{case}: refused write was audited"
        );
    }
}

// AC4: 429 is retried with back-off, then succeeds
#[test]
fn rate_limit_is_retried_cloud() {
    rate_limit_retry(Layout::Cloud);
}

#[test]
fn rate_limit_is_retried_datacenter() {
    rate_limit_retry(Layout::DataCenter);
}

fn rate_limit_retry(layout: Layout) {
    let mock = mock_with_issue(layout);
    mock.script(
        "PUT",
        "/issue/PAY-1",
        429,
        r#"{"errorMessages":["slow down"]}"#,
        &[("Retry-After", "0")],
    );
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    c.update_fields(
        "PAY-1",
        WriteKind::FeatureFields,
        &fields(&[("customfield_10101", json!("passed"))]),
        &target(),
        "r",
    )
    .expect("succeeds after one 429");
    let puts = mock.write_requests();
    assert_eq!(puts.len(), 2, "one 429 and one success expected");
    assert_eq!(
        mock.issue("PAY-1").unwrap()["fields"]["customfield_10101"],
        "passed"
    );
}

// AC4: when 429 persists, the error names the issue and the number of retries
#[test]
fn rate_limit_exhausted_fails_with_issue_key_cloud() {
    rate_limit_exhausted(Layout::Cloud);
}

#[test]
fn rate_limit_exhausted_fails_with_issue_key_datacenter() {
    rate_limit_exhausted(Layout::DataCenter);
}

fn rate_limit_exhausted(layout: Layout) {
    let mock = mock_with_issue(layout);
    for _ in 0..3 {
        mock.script("PUT", "/issue/PAY-1", 429, "{}", &[]);
    }
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    let err = format!(
        "{:#}",
        c.update_fields(
            "PAY-1",
            WriteKind::FeatureFields,
            &fields(&[("customfield_10101", json!("passed"))]),
            &target(),
            "r"
        )
        .unwrap_err()
    );
    assert!(err.contains("issue PAY-1") && err.contains("429"), "{err}");
    assert_eq!(
        mock.write_requests().len(),
        3,
        "max_retries 2 means 3 attempts"
    );
}

// AC4: any other error fails at once, naming the issue key; no retry
#[test]
fn other_errors_fail_with_issue_key_cloud() {
    other_errors(Layout::Cloud);
}

#[test]
fn other_errors_fail_with_issue_key_datacenter() {
    other_errors(Layout::DataCenter);
}

fn other_errors(layout: Layout) {
    let mock = mock_with_issue(layout);
    mock.script("PUT", "/issue/PAY-1", 500, "internal boom", &[]);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    let err = format!(
        "{:#}",
        c.update_fields(
            "PAY-1",
            WriteKind::FeatureFields,
            &fields(&[("customfield_10101", json!("passed"))]),
            &target(),
            "r"
        )
        .unwrap_err()
    );
    assert!(
        err.contains("issue PAY-1") && err.contains("HTTP 500"),
        "{err}"
    );
    assert_eq!(mock.write_requests().len(), 1, "no retry on HTTP 500");

    // a missing issue (404) names its key too
    let mut c2 = client(&cfg, layout);
    let err = format!(
        "{:#}",
        c2.update_fields(
            "PAY-999",
            WriteKind::FeatureFields,
            &fields(&[("customfield_10101", json!("passed"))]),
            &target(),
            "r"
        )
        .unwrap_err()
    );
    assert!(
        err.contains("issue PAY-999") && err.contains("HTTP 404"),
        "{err}"
    );
    assert!(c2.audit_entries().is_empty());
}

// The transition path reads the workflow, then posts the transition id
#[test]
fn transition_uses_the_workflow_id_cloud() {
    transition_by_name(Layout::Cloud);
}

#[test]
fn transition_uses_the_workflow_id_datacenter() {
    transition_by_name(Layout::DataCenter);
}

fn transition_by_name(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    c.transition("PAY-1", "Verified", &target(), "tests passed")
        .expect("transition");
    let posts = mock.write_requests();
    assert_eq!(posts.len(), 1);
    let body: Value = posts[0].body_json();
    assert_eq!(body["transition"]["id"], "31");
}

// Creating a dedicated issue returns the key the mock assigned, and audits it
#[test]
fn create_dedicated_issue_is_audited_cloud() {
    create_dedicated(Layout::Cloud);
}

#[test]
fn create_dedicated_issue_is_audited_datacenter() {
    create_dedicated(Layout::DataCenter);
}

fn create_dedicated(layout: Layout) {
    let mock = mock_with_issue(layout);
    let cfg = config(layout, &mock.base_url(), &["PAY"], Policy::default());
    let mut c = client(&cfg, layout);
    let (key, entry) = c
        .create_issue(
            WriteKind::DedicatedIssue,
            &fields(&[
                ("project", json!({"key": "PAY"})),
                ("summary", json!("TC_001")),
                ("issuetype", json!({"name": "Test"})),
            ]),
            &WriteTarget::new("test-case", "TC_001"),
            "create test issue",
        )
        .expect("create");
    assert!(key.starts_with("PAY-"), "{key}");
    assert_eq!(entry.who, ACTOR);
    assert_eq!(entry.entity.entity_id, "TC_001");
    assert!(mock.issue(&key).is_some());
}
