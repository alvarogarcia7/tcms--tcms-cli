//! Shared helpers for the mock-backed tests. Everything here runs against the mock Jira
//! (`jira_client::mock`), never a real site.
#![allow(dead_code)]

use id_resolver::IntegrationConfig;
use jira_client::mock::{Layout, MockJira};
use jira_client::{JiraClient, RetryPolicy, Secret};

/// Token the tests use. Tests assert it never shows up in records, logs or errors.
pub const TOKEN: &str = "tcms-test-token-4b1d9e";
/// Service account of the client (Cloud uses Basic auth with it, Data Center a PAT).
pub const SERVICE_USER: &str = "tcms-bot@example.com";
/// Acting TCMS user the audit entries and comments must name.
pub const ACTOR: &str = "alice";

pub fn schemas_root() -> String {
    format!("{}/../../schemas/", env!("CARGO_MANIFEST_DIR"))
}

/// Authorization header the mock accepts for a layout.
pub fn expected_auth(layout: Layout) -> String {
    match layout {
        Layout::Cloud => format!("Basic {}", base64(&format!("{SERVICE_USER}:{TOKEN}"))),
        Layout::DataCenter => format!("Bearer {TOKEN}"),
    }
}

/// Service account user passed to the client (Cloud: Basic auth; Data Center: none, PAT).
pub fn service_user(layout: Layout) -> Option<String> {
    match layout {
        Layout::Cloud => Some(SERVICE_USER.to_string()),
        Layout::DataCenter => None,
    }
}

pub fn system_type(layout: Layout) -> &'static str {
    match layout {
        Layout::Cloud => "jira_cloud",
        Layout::DataCenter => "jira_datacenter",
    }
}

/// Write policy switches of a test config.
#[derive(Clone, Copy)]
pub struct Policy {
    pub feature_fields: bool,
    pub feature_transitions: bool,
    pub comments: bool,
    pub dedicated_issues: bool,
    pub defects: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            feature_fields: true,
            feature_transitions: true,
            comments: true,
            dedicated_issues: true,
            defects: true,
        }
    }
}

pub fn config_yaml(layout: Layout, base_url: &str, project_keys: &[&str], p: Policy) -> String {
    let keys = project_keys.join(", ");
    format!(
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
    project_keys: [{keys}]
    feature_issue_types: [Epic, Story]
    defect_issue_type: Bug
    severity_to_priority:
      critical: Highest
      major: High
      minor: Medium
      cosmetic: Low
    tcms_issue_types:
      test: Test
      link_type: tests
    fields:
      verification_status: customfield_10101
      validation_status: customfield_10102
      coverage: customfield_10103
      test_lifecycle_status: customfield_10104
      latest_verdict: customfield_10105
      baseline: customfield_10106
      tcms_id: customfield_10107
    status_values:
      not_started: Not started
      in_progress: In progress
      passed: Passed
      failed: Failed
      blocked: Blocked
    write_policy:
      feature_fields: {ff}
      feature_transitions: {ft}
      dedicated_issues: {di}
      comments: {co}
      defects: {de}
    transitions:
      passed: Verified
      failed: Reopen
sync:
  trigger: cli
  identity: tcms-bot
  record_actor: true
"#,
        ty = system_type(layout),
        ff = p.feature_fields,
        ft = p.feature_transitions,
        di = p.dedicated_issues,
        co = p.comments,
        de = p.defects,
    )
}

/// Load a config for the mock at `base_url`, through the same loader the CLI uses.
pub fn config(
    layout: Layout,
    base_url: &str,
    project_keys: &[&str],
    p: Policy,
) -> IntegrationConfig {
    let yaml = config_yaml(layout, base_url, project_keys, p);
    IntegrationConfig::from_yaml_str(&yaml, &schemas_root(), "test-config")
        .expect("test config loads")
}

/// Client for the first system of `cfg`, with token [`TOKEN`] and actor [`ACTOR`].
pub fn client(cfg: &IntegrationConfig, layout: Layout) -> JiraClient {
    JiraClient::with_token(
        &cfg.systems[0],
        &cfg.sync,
        ACTOR,
        Secret::new(TOKEN),
        service_user(layout),
    )
    .expect("client builds")
    .with_retry(fast_retry())
}

pub fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_retries: 2,
        base_delay: std::time::Duration::from_millis(1),
        max_delay: std::time::Duration::from_millis(5),
    }
}

/// Mock with the two standard projects and one feature issue.
pub fn mock_with_issue(layout: Layout) -> MockJira {
    let mock = MockJira::start(layout, &expected_auth(layout));
    mock.add_transition("31", "Verified");
    mock.add_transition("41", "Reopen");
    mock.add_issue(
        "PAY-1",
        "Story",
        "Pay by card",
        "Customer pays with a stored card",
        "In progress",
        "2026-10-20T09:00:00.000+0000",
        &["safety"],
    );
    mock
}

fn base64(input: &str) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let b = input.as_bytes();
    let mut out = String::new();
    for chunk in b.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}
