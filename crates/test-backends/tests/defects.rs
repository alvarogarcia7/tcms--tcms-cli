//! Defects (TCMS-42.8) over real HTTP against the jira-client mock (Cloud layout): creation,
//! linking, dry run, refusal by the write policy, reconciliation and secret hygiene.

use id_resolver::IntegrationConfig;
use jira_client::mock::{Layout, MockJira};
use jira_client::{JiraClient, Secret};
use test_backends::defects::{self, ProblemReport, PushOutcome};
use test_backends::{Ctx, JiraApi};

const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/");
const TOKEN: &str = "mock-token-do-not-leak";

fn yaml(base: &str, defects: bool) -> String {
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
    defect_issue_type: Bug
    severity_to_priority:
      critical: Highest
      major: High
      minor: Low
    test_backend: none
    tcms_issue_types:
      test: Test
      link_type: Relates
    fields:
      tcms_id: customfield_10001
    write_policy:
      dedicated_issues: true
      defects: {defects}
"#
    )
}

fn ctx(mock: &MockJira, defects: bool) -> Ctx {
    let cfg =
        IntegrationConfig::from_yaml_str(&yaml(&mock.base_url(), defects), SCHEMAS, "defects")
            .expect("config validates");
    let sys = cfg.systems[0].clone();
    let client = JiraClient::with_token(&sys, &cfg.sync, "alvaro", Secret::new(TOKEN), None)
        .expect("client");
    Ctx {
        api: Box::new(JiraApi::new(client)),
        system: sys,
        overwrites: Vec::new(),
    }
}

fn mock() -> MockJira {
    let m = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    m.add_issue(
        "PAY-1",
        "Story",
        "Login",
        "feature text",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m.add_issue(
        "PAY-9",
        "Bug",
        "Old defect",
        "x",
        "Done",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m.add_issue(
        "PAY-5",
        "Test",
        "Login test",
        "x",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m
}

fn pr(severity: &str, status: &str) -> ProblemReport {
    ProblemReport {
        id: "PR-007".into(),
        title: "Login fails on retry".into(),
        description: "Second login returns 500.".into(),
        severity: Some(severity.into()),
        status: status.into(),
        raised_by: "tester1".into(),
        feature_keys: vec!["PAY-1".into()],
        external_key: None,
    }
}

#[test]
fn push_creates_defect_with_priority_links_and_actor() {
    let m = mock();
    let mut c = ctx(&m, true);
    let out = defects::push(
        &mut c,
        &pr("critical", "open"),
        Some("PAY-5"),
        "alvaro",
        false,
    )
    .expect("push");
    let PushOutcome::Created { key, links, notes } = out else {
        panic!("expected a created defect, got {out:?}")
    };
    assert!(key.starts_with("PAY-"), "created in the project: {key}");
    assert_eq!(links, vec!["PAY-1".to_string(), "PAY-5".to_string()]);
    assert!(notes.is_empty(), "{notes:?}");

    let issue = m.issue(&key).expect("defect on the mock");
    assert_eq!(issue["fields"]["issuetype"]["name"], "Bug");
    assert_eq!(issue["fields"]["summary"], "[PR-007] Login fails on retry");
    assert_eq!(issue["fields"]["priority"]["name"], "Highest");
    assert_eq!(issue["fields"]["description"]["type"], "doc");
    let text = issue["fields"]["description"].to_string();
    assert!(
        text.contains("pushed by alvaro"),
        "actor named in the description: {text}"
    );

    let writes = m.write_requests();
    let links = writes
        .iter()
        .filter(|r| r.path.ends_with("/issueLink"))
        .count();
    assert_eq!(links, 2, "one link per feature and per test entity");
    // the feature itself was never written
    assert!(writes.iter().all(|r| r.path != "/rest/api/3/issue/PAY-1"));
    assert!(writes.iter().all(|r| !r.body.contains(TOKEN)));
}

#[test]
fn a_report_that_names_a_key_is_not_pushed_again() {
    let m = mock();
    let mut c = ctx(&m, true);
    let mut p = pr("major", "open");
    p.external_key = Some("PAY-9".into());
    let before = m.write_requests().len();
    let out = defects::push(&mut c, &p, None, "alvaro", false).expect("push");
    assert_eq!(
        out,
        PushOutcome::AlreadyLinked {
            key: "PAY-9".into()
        }
    );
    assert_eq!(m.write_requests().len(), before, "no request sent");
}

#[test]
fn dry_run_sends_nothing_and_names_the_plan() {
    let m = mock();
    let mut c = ctx(&m, true);
    let before = m.write_requests().len();
    let out = defects::push(&mut c, &pr("minor", "open"), None, "alvaro", true).expect("dry run");
    match out {
        PushOutcome::Planned { summary, links, .. } => {
            assert_eq!(summary, "[PR-007] Login fails on retry");
            assert_eq!(links, vec!["PAY-1".to_string()]);
        }
        other => panic!("expected a plan, got {other:?}"),
    }
    assert_eq!(m.write_requests().len(), before);
}

#[test]
fn refused_when_defects_policy_is_off() {
    let m = mock();
    let mut c = ctx(&m, false);
    let before = m.write_requests().len();
    let err =
        defects::push(&mut c, &pr("major", "open"), None, "alvaro", false).expect_err("refused");
    assert!(err.to_string().contains("defects is off"), "{err}");
    assert_eq!(
        m.write_requests().len(),
        before,
        "refused write never reaches the wire"
    );
}

#[test]
fn unmapped_severity_leaves_priority_unset_and_says_so() {
    let m = mock();
    let mut c = ctx(&m, true);
    let mut p = pr("cosmetic", "open");
    p.feature_keys.clear();
    let out = defects::push(&mut c, &p, None, "alvaro", false).expect("push");
    let PushOutcome::Created { key, notes, .. } = out else {
        panic!("created")
    };
    assert!(notes.iter().any(|n| n.contains("priority left unset")));
    assert!(m.issue(&key).unwrap()["fields"].get("priority").is_none());
}

#[test]
fn link_existing_reads_the_issue_and_refuses_other_projects() {
    let m = mock();
    let mut c = ctx(&m, true);
    let st = defects::link_existing(&mut c, "PAY-9").expect("link");
    assert!(st.done, "PAY-9 is Done");
    assert_eq!(st.status, "Done");
    let err = defects::link_existing(&mut c, "OTHER-3").expect_err("out of scope");
    assert!(err.to_string().contains("not in project_keys"), "{err}");
    assert!(defects::link_existing(&mut c, "PAY-404").is_err());
}

#[test]
fn closed_in_jira_is_reported_and_the_report_is_not_changed() {
    let m = mock();
    let mut c = ctx(&m, true);
    let st = defects::observe(&mut c, "PAY-9").expect("observe");
    let d = defects::reconcile("PR-003", "open", &st).expect("difference");
    assert!(d.message.contains("not changed"));
    // reconciliation sends no write
    assert!(m.write_requests().iter().all(|r| !r.path.contains("PAY-9")));
}

#[test]
fn open_linked_issue_or_open_report_blocks_the_feature_from_passed() {
    use test_backends::rollup::{verification, ReqState, RollupInput, ValidationState};
    use test_backends::StatusValue;
    let base = |open: bool| RollupInput {
        requirements: vec![ReqState {
            covered: true,
            validation: ValidationState::Validated,
        }],
        test_statuses: vec![StatusValue::Passed],
        open_problem_reports: open,
    };
    assert_eq!(verification(&base(false)), StatusValue::Passed);
    let m = mock();
    let mut c = ctx(&m, true);
    let open_issue = defects::observe(&mut c, "PAY-5").expect("PAY-5 is To Do");
    let open = defects::blocks_passed("fixed", Some(&open_issue));
    assert!(open);
    assert_eq!(verification(&base(open)), StatusValue::InProgress);
}

/// TCMS-42.9 defect fix: the label search is exact. Defects of `PR-1` are not `PR-10`'s, a
/// closed defect of the same report still counts (it is not created again), and a push finds the
/// existing one instead of creating (the search is the only write-free path).
#[test]
fn label_search_is_exact_and_a_found_defect_is_not_created_again() {
    let m = mock();
    m.add_issue(
        "PAY-20",
        "Bug",
        "other report",
        "x",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &["tcms", "tcms-pr-PR-10"],
    );
    m.add_issue(
        "PAY-21",
        "Bug",
        "same report, closed",
        "x",
        "Done",
        "2026-10-01T00:00:00.000+0000",
        &["tcms", "tcms-pr-PR-007"],
    );
    let mut c = ctx(&m, true);
    let found = defects::find_by_label(&mut c, "tcms-pr-PR-007").unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].key, "PAY-21");
    assert!(found[0].done);
    assert!(defects::find_by_label(&mut c, "tcms-pr-PR-1")
        .unwrap()
        .is_empty());

    let out = defects::push(&mut c, &pr("critical", "open"), None, "alvaro", false).unwrap();
    assert_eq!(
        out,
        PushOutcome::Found {
            key: "PAY-21".into()
        }
    );
    assert!(m.write_requests().is_empty(), "nothing created or linked");
}
