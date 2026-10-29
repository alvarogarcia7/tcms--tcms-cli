//! The same scenario on all four backends (TCMS-42.4 AC #4): only `test_backend` changes.
//! Runs against the in-process fake; the configs are loaded through the schema-validated loader.

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use id_resolver::IntegrationConfig;
use serde_json::{json, Value};
use test_backends::fake::FakeApi;
use test_backends::{
    build_backend, external_ref, known_entity, sync_case, sync_run, Api, BackendKind, CaseSync,
    EntityKey, LifecycleStatus, RunCase, RunSync, RunVerdict, TestBackend,
};

const SCHEMAS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas/");

/// Shares one fake between the backend and the test that inspects it.
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

fn config_yaml(backend: &str) -> String {
    format!(
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
    feature_issue_types: [Story]
    defect_issue_type: Bug
    test_backend: {backend}
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
      dedicated_issues: true
      feature_fields: false
      feature_transitions: false
      comments: false
      defects: false
"#
    )
}

fn system_for(kind: BackendKind) -> id_resolver::SystemConfig {
    let cfg =
        IntegrationConfig::from_yaml_str(&config_yaml(kind.config_name()), SCHEMAS, "scenario")
            .expect("scenario config validates against the integration-config schema");
    cfg.systems[0].clone()
}

fn cases(known: Option<EntityKey>) -> Vec<CaseSync> {
    vec![
        CaseSync {
            test_case_id: "TC_LOGIN_001".into(),
            title: "Login with a valid account".into(),
            lifecycle: LifecycleStatus::Approved,
            feature_keys: vec!["PAY-1".into()],
            known,
        },
        CaseSync {
            test_case_id: "TC_PAY_002".into(),
            title: "Payment is refused over the limit".into(),
            lifecycle: LifecycleStatus::InReview,
            feature_keys: vec!["PAY-2".into()],
            known: None,
        },
    ]
}

struct Outcome {
    fake: Rc<RefCell<FakeApi>>,
    entities: Vec<EntityKey>,
    execution: Option<EntityKey>,
    kind: BackendKind,
}

fn run_on(kind: BackendKind) -> Outcome {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let system = system_for(kind);
    let mut backend: Box<dyn TestBackend> =
        build_backend(&system, Box::new(Shared(fake.clone()))).expect("backend builds");
    assert_eq!(backend.kind(), kind);

    // 1. create or find each test case's entity and write its lifecycle
    let mut entities = Vec::new();
    for c in cases(None) {
        entities.push(sync_case(backend.as_mut(), &c).expect("sync case"));
    }
    // 2. record a run: pass for the first case, fail for the second; verdicts written
    let run = RunSync {
        run_id: "RUN-2026-10-28-1".into(),
        baseline_id: "BL-7".into(),
        finished: true,
        cases: vec![
            RunCase {
                test_case_id: "TC_LOGIN_001".into(),
                entity: entities[0].clone(),
                verdict: RunVerdict::Pass,
                last_written: None,
            },
            RunCase {
                test_case_id: "TC_PAY_002".into(),
                entity: entities[1].clone(),
                verdict: RunVerdict::Fail,
                last_written: None,
            },
        ],
    };
    let execution = sync_run(backend.as_mut(), &run).expect("sync run");
    // 3. a test case deleted in TCMS: its entity is marked obsolete
    backend.mark_obsolete(&entities[1]).expect("mark obsolete");
    // 4. the same case synced again with no known key is found, not duplicated
    let again = sync_case(backend.as_mut(), &cases(None)[0]).expect("re-sync");
    assert_eq!(
        again.key, entities[0].key,
        "{kind:?}: the entity is found again"
    );

    Outcome {
        fake,
        entities,
        execution,
        kind,
    }
}

fn check_common(o: &Outcome) {
    let fake = o.fake.borrow();
    assert!(
        fake.calls.iter().all(|c| c.method != "DELETE"),
        "{:?}: a DELETE was sent",
        o.kind
    );
    assert_ne!(
        o.entities[0].key, o.entities[1].key,
        "{:?}: distinct entities",
        o.kind
    );
    let r = external_ref(o.kind, &o.entities[0], "2026-10-28T10:00:00Z");
    assert_eq!(r.system, o.kind.external_system());
    assert_eq!(r.key, o.entities[0].key);
    let known = known_entity(std::slice::from_ref(&r), o.kind).expect("known entity");
    assert_eq!(known.key, o.entities[0].key);
}

fn jira_based(o: &Outcome) {
    let fake = o.fake.borrow();
    // test issues: TCMS-owned, found by tcms_id, typed Test
    for (e, id) in o.entities.iter().zip(["TC_LOGIN_001", "TC_PAY_002"]) {
        let issue = fake.issues.get(&e.key).expect("test issue exists");
        assert_eq!(issue["fields"]["issuetype"]["name"], "Test");
        assert_eq!(issue["fields"]["customfield_10001"], id);
    }
    // lifecycle: the second case was marked obsolete last
    assert_eq!(
        fake.issue_field(&o.entities[0].key, "customfield_10002"),
        Some(json!("approved"))
    );
    assert_eq!(
        fake.issue_field(&o.entities[1].key, "customfield_10002"),
        Some(json!("obsolete"))
    );
    // latest verdict and baseline written on the test issue
    assert_eq!(
        fake.issue_field(&o.entities[0].key, "customfield_10003"),
        Some(json!("passed"))
    );
    assert_eq!(
        fake.issue_field(&o.entities[1].key, "customfield_10003"),
        Some(json!("failed"))
    );
    assert_eq!(
        fake.issue_field(&o.entities[0].key, "customfield_10004"),
        Some(json!("BL-7"))
    );
    // features linked
    assert_eq!(fake.links.len(), 2, "one link per feature");
}

fn scenario(kind: BackendKind) {
    let o = run_on(kind);
    check_common(&o);
    match kind {
        BackendKind::PlainJira => {
            jira_based(&o);
            assert!(o.execution.is_none(), "plain Jira has no execution entity");
        }
        BackendKind::Xray => {
            jira_based(&o);
            let exec = o.execution.clone().expect("Xray execution");
            let fake = o.fake.borrow();
            assert_eq!(
                fake.issues[&exec.key]["fields"]["issuetype"]["name"],
                "Test Execution"
            );
            let statuses: Vec<&Value> = fake
                .objects
                .values()
                .filter(|o| o["kind"] == "xray_status")
                .map(|o| &o["body"]["status"])
                .collect();
            assert_eq!(statuses.len(), 2);
            assert!(statuses.contains(&&json!("PASS")) && statuses.contains(&&json!("FAIL")));
        }
        BackendKind::ZephyrSquad => {
            jira_based(&o);
            let cycle = o.execution.clone().expect("Squad cycle");
            let fake = o.fake.borrow();
            let execs: Vec<&Value> = fake
                .objects
                .values()
                .filter(|o| o["kind"] == "squad_execution")
                .map(|o| &o["body"])
                .collect();
            assert_eq!(execs.len(), 2);
            assert!(execs
                .iter()
                .all(|b| b["cycleId"] == json!(cycle.id.clone().unwrap())));
            let st: Vec<&Value> = execs.iter().map(|b| &b["status"]).collect();
            assert!(st.contains(&&json!("1")) && st.contains(&&json!("2")));
        }
        BackendKind::ZephyrScale => {
            let fake = o.fake.borrow();
            assert!(
                fake.links.is_empty(),
                "Zephyr Scale links are not written by this backend"
            );
            let case = fake.object(&o.entities[0].key).expect("zephyr case");
            assert_eq!(case["body"]["labels"], json!(["tcms:TC_LOGIN_001"]));
            assert_eq!(
                case["body"]["customFields"]["customfield_10003"],
                json!("passed")
            );
            assert_eq!(
                case["body"]["customFields"]["customfield_10004"],
                json!("BL-7")
            );
            let obsolete = fake.object(&o.entities[1].key).expect("zephyr case 2");
            assert_eq!(obsolete["body"]["statusName"], json!("Obsolete"));
            assert_eq!(
                obsolete["body"]["customFields"]["customfield_10002"],
                json!("obsolete")
            );
            let cycle = o.execution.clone().expect("Zephyr cycle");
            let execs: Vec<&Value> = fake
                .objects
                .values()
                .filter(|o| o["kind"] == "execution")
                .map(|o| &o["body"]["statusName"])
                .collect();
            assert!(execs.contains(&&json!("Pass")) && execs.contains(&&json!("Fail")));
            assert!(cycle.key.starts_with("PAY-C"), "cycle key {}", cycle.key);
        }
    }
}

#[test]
fn plain_jira_scenario() {
    scenario(BackendKind::PlainJira);
}

#[test]
fn xray_scenario() {
    scenario(BackendKind::Xray);
}

#[test]
fn zephyr_scale_scenario() {
    scenario(BackendKind::ZephyrScale);
}

#[test]
fn zephyr_squad_scenario() {
    scenario(BackendKind::ZephyrSquad);
}

#[test]
fn an_unknown_test_backend_is_refused() {
    let mut sys = system_for(BackendKind::PlainJira);
    sys.test_backend = Some("testrail".into());
    let err = build_backend(&sys, Box::new(FakeApi::new()))
        .err()
        .expect("refused");
    assert!(err.to_string().contains("testrail"), "{err}");
}

#[test]
fn a_refused_write_never_reaches_the_transport() {
    // dedicated_issues off: creating a test entity is refused before any request
    let mut sys = system_for(BackendKind::PlainJira);
    sys.write_policy.dedicated_issues = false;
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let mut b = build_backend(&sys, Box::new(Shared(fake.clone()))).unwrap();
    let err = b.ensure_test(&cases(None)[0]).unwrap_err();
    assert!(err.to_string().contains("dedicated_issues"), "{err}");
    assert!(fake.borrow().writes().is_empty(), "no write was sent");
}

/// TCMS-42.4 AC 2 (re-sync): a second sync of the same run records it in the same Xray Test
/// Execution; it does not create one per sync (D-42.4m).
#[test]
fn xray_a_second_sync_of_the_same_run_reuses_its_test_execution() {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let system = system_for(BackendKind::Xray);
    let mut backend: Box<dyn TestBackend> =
        build_backend(&system, Box::new(Shared(fake.clone()))).expect("backend builds");
    let entities: Vec<EntityKey> = cases(None)
        .iter()
        .map(|c| sync_case(backend.as_mut(), c).expect("sync case"))
        .collect();
    let run = RunSync {
        run_id: "RUN-REUSE-1".into(),
        baseline_id: "BL-7".into(),
        finished: true,
        cases: vec![RunCase {
            test_case_id: "TC_LOGIN_001".into(),
            entity: entities[0].clone(),
            verdict: RunVerdict::Pass,
            last_written: None,
        }],
    };
    let first = sync_run(backend.as_mut(), &run)
        .expect("first sync")
        .expect("exec");
    let second = sync_run(backend.as_mut(), &run)
        .expect("second sync")
        .expect("exec");
    assert_eq!(first.key, second.key, "the same execution is reused");
    let execs_created = fake
        .borrow()
        .writes()
        .iter()
        .filter(|c| c.method == "POST" && c.body.to_string().contains("\"Test Execution\""))
        .count();
    assert_eq!(
        execs_created, 1,
        "one Test Execution for the run, not one per sync"
    );
}

/// Shared shape of the Zephyr re-sync regressions: one run synced twice through one backend.
/// Returns the fake after both syncs.
fn synced_twice(kind: BackendKind, run_id: &str) -> Rc<RefCell<FakeApi>> {
    let fake = Rc::new(RefCell::new(FakeApi::new()));
    let system = system_for(kind);
    let mut backend: Box<dyn TestBackend> =
        build_backend(&system, Box::new(Shared(fake.clone()))).expect("backend builds");
    let entities: Vec<EntityKey> = cases(None)
        .iter()
        .map(|c| sync_case(backend.as_mut(), c).expect("sync case"))
        .collect();
    let run = RunSync {
        run_id: run_id.into(),
        baseline_id: "BL-7".into(),
        finished: true,
        cases: vec![RunCase {
            test_case_id: "TC_LOGIN_001".into(),
            entity: entities[0].clone(),
            verdict: RunVerdict::Pass,
            last_written: None,
        }],
    };
    let first = sync_run(backend.as_mut(), &run).expect("first sync");
    let second = sync_run(backend.as_mut(), &run).expect("second sync");
    assert_eq!(
        first.map(|e| e.key),
        second.map(|e| e.key),
        "the same cycle is reused"
    );
    fake
}

fn count_writes(fake: &Rc<RefCell<FakeApi>>, method: &str, path: &str) -> usize {
    fake.borrow()
        .writes()
        .iter()
        .filter(|c| c.method == method && c.path == path)
        .count()
}

/// TCMS-42.4 AC 2 (re-sync, Zephyr Scale; D-42.4o): a second sync of the same run reuses its
/// cycle and updates the executions of the first sync; it creates neither again.
#[test]
fn zephyr_scale_a_second_sync_of_the_same_run_reuses_its_cycle_and_executions() {
    let fake = synced_twice(BackendKind::ZephyrScale, "RUN-REUSE-ZS");
    assert_eq!(
        count_writes(&fake, "POST", "/testcycles"),
        1,
        "one cycle per run"
    );
    assert_eq!(
        count_writes(&fake, "POST", "/testexecutions"),
        1,
        "one execution per case, created by the first sync only"
    );
    let updates = fake
        .borrow()
        .writes()
        .iter()
        .filter(|c| c.method == "PUT" && c.path.starts_with("/testexecutions/"))
        .count();
    assert_eq!(updates, 1, "the second sync updates the existing execution");
}

/// TCMS-42.4 AC 2 (re-sync, Zephyr Squad; D-42.4o): a second sync of the same run reuses its
/// cycle and updates the executions of the first sync.
#[test]
fn zephyr_squad_a_second_sync_of_the_same_run_reuses_its_cycle_and_executions() {
    let fake = synced_twice(BackendKind::ZephyrSquad, "RUN-REUSE-ZQ");
    assert_eq!(
        count_writes(&fake, "POST", "/rest/zapi/latest/cycle"),
        1,
        "one cycle per run"
    );
    assert_eq!(
        count_writes(&fake, "POST", "/rest/zapi/latest/execution"),
        1,
        "one execution per case, created by the first sync only"
    );
    let updates = fake
        .borrow()
        .writes()
        .iter()
        .filter(|c| c.method == "PUT" && c.path.starts_with("/rest/zapi/latest/execution/"))
        .count();
    assert_eq!(updates, 1, "the second sync updates the existing execution");
}
