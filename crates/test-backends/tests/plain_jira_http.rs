//! Plain Jira over real HTTP against the jira-client mock server (Cloud layout): the guarded
//! writes reach the wire only when allowed, and the mock records what was sent.

use id_resolver::IntegrationConfig;
use jira_client::mock::{Layout, MockJira};
use jira_client::{JiraClient, Secret};
use test_backends::{build_backend, sync_case, CaseSync, JiraApi, LifecycleStatus};

const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/");
const TOKEN: &str = "mock-token-do-not-leak";

fn yaml(base: &str) -> String {
    format!(
        r#"product_id: PRODUCT-PAY
requirement_source: jira
systems:
  - name: jira
    type: jira_cloud
    base_url: {base}
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
      test_lifecycle_status: customfield_10002
      latest_verdict: customfield_10003
      baseline: customfield_10004
    write_policy:
      dedicated_issues: true
"#
    )
}

#[test]
fn plain_jira_creates_links_and_writes_over_http() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    mock.add_issue(
        "PAY-1",
        "Story",
        "Login",
        "feature text",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    let cfg = IntegrationConfig::from_yaml_str(&yaml(&mock.base_url()), SCHEMAS, "http-test")
        .expect("config validates");
    let sys = cfg.systems[0].clone();
    let client = JiraClient::with_token(&sys, &cfg.sync, "alvaro", Secret::new(TOKEN), None)
        .expect("client");
    let mut backend = build_backend(&sys, Box::new(JiraApi::new(client))).expect("backend");

    let case = CaseSync {
        test_case_id: "TC_LOGIN_001".into(),
        title: "Login".into(),
        lifecycle: LifecycleStatus::Approved,
        feature_keys: vec!["PAY-1".into()],
        known: None,
    };
    let entity = sync_case(backend.as_mut(), &case).expect("sync over HTTP");
    // found again on the second sync, not duplicated
    let again = sync_case(backend.as_mut(), &case).expect("second sync");
    assert_eq!(again.key, entity.key);

    let issue = mock.issue(&entity.key).expect("test issue on the mock");
    assert_eq!(issue["fields"]["issuetype"]["name"], "Test");
    assert_eq!(issue["fields"]["customfield_10001"], "TC_LOGIN_001");
    assert_eq!(issue["fields"]["customfield_10002"], "approved");

    // the feature itself was never written: only the test issue and the link
    let writes = mock.write_requests();
    assert!(
        writes.iter().all(|r| r.path != "/rest/api/3/issue/PAY-1"),
        "feature touched: {writes:?}"
    );
    assert!(
        writes.iter().any(|r| r.path.ends_with("/issueLink")),
        "link created"
    );
    // no request carries the token in its body
    assert!(writes.iter().all(|r| !r.body.contains(TOKEN)));
}
