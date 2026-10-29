//! `tcms jira sync` run as the real binary against the jira-client mock (Cloud layout, plain
//! Jira backend). Verified against the mock only: nothing here touches a real Jira.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-sync-8b2f4d";
const PRODUCT: &str = "PROD-001";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(rel: &str) -> String {
    let p = root()
        .join("test-acceptance/glossary_entities/fixtures")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The product workspace: integration config, two test cases tracing to feature PAY-1 (one
/// normal range, one robustness), a run record, one open problem report.
fn workspace(base_url: &str, comments: bool) -> tempfile::TempDir {
    workspace_for(base_url, comments, "none")
}

/// The same product for any test backend (`none`, `xray`, `zephyr_scale`, `zephyr_squad`).
fn workspace_for(base_url: &str, comments: bool, backend: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    std::fs::write(
        d.join("integration-config.yaml"),
        format!(
            r#"product_id: {PRODUCT}
requirement_source: jira
systems:
  - name: jira
    type: jira_cloud
    base_url: {base_url}
    auth:
      method: service_account_token
      secret_ref: {ENV}
    id_pattern: '^[A-Z]+-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [PAY]
    feature_issue_types: [Story]
    defect_issue_type: Bug
    severity_to_priority:
      critical: Highest
    test_backend: {backend}
    tcms_issue_types:
      test: Test
      link_type: Relates
    fields:
      tcms_id: customfield_10001
      test_lifecycle_status: customfield_10002
      latest_verdict: customfield_10003
      baseline: customfield_10004
      verification_status: customfield_10005
    write_policy:
      feature_fields: true
      dedicated_issues: true
      defects: true
      comments: {comments}
sync:
  identity: tcms-bot
  record_actor: true
"#
        ),
    )
    .unwrap();
    std::fs::create_dir_all(d.join("testcases")).unwrap();
    for (name, rel) in [
        ("TC_TRACE_001.yml", "traceability/complete/TC_TRACE_001.yml"),
        ("TC_TRACE_002.yml", "traceability/complete/TC_TRACE_002.yml"),
    ] {
        let text = fixture(rel).replace("HLR-1", "PAY-1");
        std::fs::write(d.join("testcases").join(name), text).unwrap();
    }
    std::fs::create_dir_all(d.join("test-runs")).unwrap();
    std::fs::write(
        d.join("test-runs/RUN-1.yaml"),
        r#"type: test_run
schema: tcms/test-run.schema.json
run_id: RUN-1
build_id: build-42
environment: lab-1
tester: tester1
started_at: "2026-10-27T10:00:00Z"
finished_at: "2026-10-27T10:05:00Z"
results:
  - test_case_id: TC_TRACE_001
    verdict: pass
  - test_case_id: TC_TRACE_002
    verdict: fail
    problem_report_id: PR-007
"#,
    )
    .unwrap();
    std::fs::create_dir_all(d.join("problem-reports")).unwrap();
    std::fs::write(
        d.join("problem-reports/PR-007.yaml"),
        r#"type: problem_report
schema: tcms/problem-report.schema.json
id: PR-007
title: Robustness case fails
description: Robustness input is accepted.
severity: critical
category: code
status: open
raised_by: tester1
date: "2026-10-27"
test_case_id: TC_TRACE_002
affected_items:
  - entity_type: requirement
    entity_id: PAY-1
"#,
    )
    .unwrap();
    dir
}

fn mock() -> MockJira {
    let m = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    m.add_project("PAY", "Payments");
    m.add_issue(
        "PAY-1",
        "Story",
        "Login",
        "feature text",
        "To Do",
        "2026-10-01T00:00:00.000+0000",
        &[],
    );
    m
}

fn sync(dir: &Path, extra: &[&str]) -> Output {
    sync_env(dir, extra, &[])
}

/// `tcms jira sync` with extra environment (e.g. `TCMS_ZEPHYR_API_URL` for Zephyr Scale).
fn sync_env(dir: &Path, extra: &[&str], env: &[(&str, &str)]) -> Output {
    let mut args = vec![
        "--config".to_string(),
        "integration-config.yaml".into(),
        "--schemas-root".into(),
        format!("{}/schemas/", root().display()),
        "jira".into(),
        "sync".into(),
        "--product".into(),
        PRODUCT.into(),
        "--as".into(),
        "alice".into(),
        "--tests-dir".into(),
        "testcases".into(),
        "--runs-dir".into(),
        "test-runs".into(),
        "--problem-reports".into(),
        "problem-reports".into(),
        "--requirements-dir".into(),
        "requirements".into(),
        "--state-dir".into(),
        "state".into(),
    ];
    args.extend(extra.iter().map(|s| s.to_string()));
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .current_dir(dir)
        .args(&args)
        .env(ENV, TOKEN)
        .env_remove("TCMS_JIRA_USER")
        .envs(env.iter().copied())
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

/// Every file under `dir`, read as text (audit files, state).
fn all_text(dir: &Path) -> String {
    let mut out = String::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.push_str(&all_text(&p));
            } else if let Ok(t) = std::fs::read_to_string(&p) {
                out.push_str(&t);
            }
        }
    }
    out
}

#[test]
fn default_run_is_a_dry_run_and_writes_nothing() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let out = sync(d.path(), &[]);
    assert!(out.status.success(), "{}", text(&out));
    let t = text(&out);
    assert!(t.contains("dry run"), "{t}");
    assert!(t.contains("would write"), "{t}");
    assert!(t.contains("dry run: nothing written"), "{t}");
    assert!(
        m.write_requests().is_empty(),
        "no write request in a dry run: {:?}",
        m.write_requests()
    );
    assert!(
        !d.path().join("state").exists(),
        "no state file in a dry run"
    );
    assert!(
        !d.path().join("requirements").exists(),
        "import writes no record in a dry run"
    );
}

#[test]
fn apply_writes_the_cycle_and_names_the_user_in_every_audit_entry() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let out = sync(d.path(), &["--apply"]);
    assert!(out.status.success(), "{}", text(&out));
    let t = text(&out);
    assert!(t.contains("write(s) applied"), "{t}");

    // the test entities exist on the mock, with their lifecycle and verdict
    let tests: Vec<_> = m
        .write_requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path.ends_with("/issue") && r.body.contains("\"Test\""))
        .collect();
    assert_eq!(tests.len(), 2, "one Test issue per test case: {tests:?}");

    // the state holds the entity keys, never a secret
    let state = std::fs::read_to_string(d.path().join("state/state.yaml")).unwrap();
    assert!(
        state.contains("TC_TRACE_001") && state.contains("TC_TRACE_002"),
        "{state}"
    );
    assert!(!state.contains(TOKEN));

    // every audit entry names the acting user
    let audit_root = d.path().join("state/audit");
    let entries: Vec<_> = std::fs::read_dir(
        std::fs::read_dir(&audit_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path(),
    )
    .unwrap()
    .flatten()
    .collect();
    assert!(!entries.is_empty());
    for e in &entries {
        let body = std::fs::read_to_string(e.path()).unwrap();
        assert!(body.contains("who: alice"), "{body}");
        assert!(body.contains("action: sync"), "{body}");
        assert!(
            body.contains("schema: tcms/audit-trail-entry.schema.json"),
            "{body}"
        );
        assert!(!body.contains(TOKEN));
    }
    // no secret in the output either
    assert!(!t.contains(TOKEN));
    // for schema validation by hand: TCMS_SYNC_TEST_KEEP=<dir> keeps the output
    if let Ok(keep) = std::env::var("TCMS_SYNC_TEST_KEEP") {
        let _ = std::fs::remove_dir_all(&keep);
        copy_dir(d.path(), Path::new(&keep));
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let dest = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &dest);
        } else {
            std::fs::copy(&p, &dest).unwrap();
        }
    }
}

#[test]
fn a_second_apply_finds_the_same_entities_and_creates_no_new_test_issue() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    assert!(sync(d.path(), &["--apply"]).status.success());
    let creates_before = m
        .write_requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.ends_with("/issue"))
        .count();
    let second = sync(d.path(), &["--apply"]);
    assert!(second.status.success(), "{}", text(&second));
    let creates_after = m
        .write_requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.ends_with("/issue"))
        .count();
    // the defect is created once (in the first run); the test issues are found, not created
    assert_eq!(
        creates_after, creates_before,
        "a second run creates nothing"
    );
    let defect_creates = m
        .write_requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.ends_with("/issue") && r.body.contains("\"Bug\""))
        .count();
    assert_eq!(defect_creates, 1, "one defect for PR-007");
}

#[test]
fn defect_step_creates_the_defect_and_records_its_key_on_the_report() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let out = sync(d.path(), &["--apply", "--only", "defects"]);
    assert!(out.status.success(), "{}", text(&out));
    let rec = std::fs::read_to_string(d.path().join("problem-reports/PR-007.yaml")).unwrap();
    assert!(rec.contains("jira") && rec.contains("PAY-"), "{rec}");
    let audit = all_text(&d.path().join("state/audit"));
    assert!(
        audit.contains("who: alice") && audit.contains("problem-report"),
        "{audit}"
    );
}

#[test]
fn comment_on_the_feature_names_the_acting_user() {
    let m = mock();
    let d = workspace(&m.base_url(), true);
    // the import step writes the requirement records the roll-up reads
    let out = sync(d.path(), &["--apply", "--only", "import,features"]);
    assert!(out.status.success(), "{}", text(&out));
    let comments: Vec<_> = m
        .write_requests()
        .into_iter()
        .filter(|r| r.path.contains("/issue/PAY-1/comment"))
        .collect();
    assert_eq!(comments.len(), 1, "{:?}", m.write_requests());
    assert!(
        comments[0].body.contains("by alice"),
        "{}",
        comments[0].body
    );
    assert!(!comments[0].body.contains(TOKEN));
}

#[test]
fn refuses_another_product_and_an_unknown_step() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let wrong = Command::new(env!("CARGO_BIN_EXE_tcms"))
        .current_dir(d.path())
        .args(["--config", "integration-config.yaml", "--schemas-root"])
        .arg(format!("{}/schemas/", root().display()))
        .args(["jira", "sync", "--product", "OTHER-9"])
        .env(ENV, TOKEN)
        .output()
        .unwrap();
    assert!(!wrong.status.success());
    assert!(
        text(&wrong).contains("does not match product_id"),
        "{}",
        text(&wrong)
    );
    let bad = sync(d.path(), &["--only", "nonsense"]);
    assert!(!bad.status.success());
    assert!(text(&bad).contains("unknown step"), "{}", text(&bad));
    assert!(m.write_requests().is_empty());
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn schemas_root() -> String {
    format!("{}/schemas/", root().display())
}

fn posts_to(m: &MockJira, suffix: &str) -> usize {
    m.write_requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.ends_with(suffix))
        .count()
}

/// TCMS-42.9 AC 1 (test case): the entity key lands in the record's external_refs with a
/// targeted edit: the record keeps every byte before the block, a dry run changes nothing and a
/// second apply changes nothing.
#[test]
fn apply_stores_the_entity_key_in_the_test_case_record() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let case_path = d.path().join("testcases/TC_TRACE_001.yml");
    let before = read(&case_path);

    let dry = sync(d.path(), &["--only", "tests"]);
    assert!(dry.status.success(), "{}", text(&dry));
    assert_eq!(
        read(&case_path),
        before,
        "a dry run does not touch the record"
    );

    let out = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(out.status.success(), "{}", text(&out));
    let after = read(&case_path);
    assert!(
        after.starts_with(&before),
        "every byte before the external_refs block is kept:\n{after}"
    );
    assert!(
        after.contains("external_refs:\n  - system: jira"),
        "{after}"
    );

    let loaded: testcase_models::TestCase =
        testcase_common::load_and_validate_yaml(&case_path, schemas_root()).unwrap();
    assert_eq!(loaded.external_refs.len(), 1, "{after}");
    let r = &loaded.external_refs[0];
    assert_eq!(r.system, testcase_models::ExternalSystem::Jira);
    assert!(r.key.starts_with("PAY-"), "{}", r.key);
    assert!(
        r.synced_at.is_some(),
        "synced_at is set when the entry is written"
    );
    assert!(
        r.url.is_none(),
        "url is derived by the resolver, never stored"
    );
    let state = read(&d.path().join("state/state.yaml"));
    assert!(
        state.contains(&r.key),
        "the state and the record agree: {state}"
    );

    // the audit names the record change
    assert!(
        all_text(&d.path().join("state/audit")).contains("external ref stored in the test case")
    );

    // a second apply: the record is byte-identical and no new Test issue is created
    let creates = posts_to(&m, "/issue");
    let second = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(second.status.success(), "{}", text(&second));
    assert_eq!(read(&case_path), after, "re-sync is idempotent");
    assert_eq!(posts_to(&m, "/issue"), creates, "no new entity");
}

/// TCMS-42.9 AC 1 (test plan) and TCMS-42.7 AC 1: a plan without external_refs gets one Fix
/// Version, its id lands in the plan record, and a later sync creates no version, also when the
/// sync state is gone and the record alone knows the id.
#[test]
fn plan_without_external_refs_gets_one_fix_version_and_its_id_in_the_plan() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let cfg = read(&d.path().join("integration-config.yaml")).replacen(
        "    test_backend: none\n",
        "    test_backend: none\n    milestone:\n      target: fix_version\n",
        1,
    );
    std::fs::write(d.path().join("integration-config.yaml"), cfg).unwrap();
    std::fs::create_dir_all(d.path().join("test-plans")).unwrap();
    let plan_path = d.path().join("test-plans/PLAN-1.yaml");
    std::fs::write(
        &plan_path,
        "# the release plan\ntype: test_plan\nschema: tcms/test-plan.schema.json\nid: PLAN-1\nname: Payments release 1\nproduct: Payments\nproduct_id: PROD-001\ndate: \"2026-10-27\"\ndescription: Plan for the payment features.\nbaseline_id: BASE-1\n",
    )
    .unwrap();
    let before = read(&plan_path);

    let out = sync(d.path(), &["--apply", "--only", "milestones"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(posts_to(&m, "/version"), 1, "{:?}", m.write_requests());
    let after = read(&plan_path);
    assert!(after.starts_with(&before), "{after}");
    let plan: serde_yaml::Value = serde_yaml::from_str(&after).unwrap();
    let refs = plan["external_refs"].as_sequence().expect("external_refs");
    assert_eq!(refs.len(), 1, "{after}");
    assert_eq!(refs[0]["system"].as_str(), Some("jira"));
    let id = refs[0]["key"].as_str().unwrap().to_string();
    assert_eq!(
        m.version(&id).expect("the Fix Version exists on the mock")["name"],
        "Payments release 1"
    );
    testcase_common::load_and_validate_yaml::<serde_yaml::Value, _, _>(&plan_path, schemas_root())
        .expect("the plan with external_refs validates against its schema");

    // second apply: no version, the plan record unchanged
    let second = sync(d.path(), &["--apply", "--only", "milestones"]);
    assert!(second.status.success(), "{}", text(&second));
    assert_eq!(posts_to(&m, "/version"), 1);
    assert_eq!(read(&plan_path), after);

    // the sync state is gone: the plan record alone still knows the Fix Version
    std::fs::remove_dir_all(d.path().join("state")).unwrap();
    let third = sync(d.path(), &["--apply", "--only", "milestones"]);
    assert!(third.status.success(), "{}", text(&third));
    assert_eq!(
        posts_to(&m, "/version"),
        1,
        "a sync without state creates nothing"
    );
    assert_eq!(read(&plan_path), after);
}

/// Rewrites the run record of `workspace()` so that both test cases pass, and the problem report
/// PR-007 as fixed; `defect` links PR-007 to a Jira issue (its status on the mock decides).
fn all_pass_and_fixed_report(d: &Path, defect: Option<&str>) {
    std::fs::write(
        d.join("test-runs/RUN-1.yaml"),
        r#"type: test_run
schema: tcms/test-run.schema.json
run_id: RUN-1
build_id: build-42
environment: lab-1
tester: tester1
started_at: "2026-10-27T10:00:00Z"
finished_at: "2026-10-27T10:05:00Z"
results:
  - test_case_id: TC_TRACE_001
    verdict: pass
  - test_case_id: TC_TRACE_002
    verdict: pass
"#,
    )
    .unwrap();
    let refs = match defect {
        Some(key) => format!("external_refs:\n  - system: jira\n    key: {key}\n"),
        None => String::new(),
    };
    std::fs::write(
        d.join("problem-reports/PR-007.yaml"),
        format!(
            r#"type: problem_report
schema: tcms/problem-report.schema.json
id: PR-007
title: Robustness case fails
description: Robustness input is accepted.
severity: critical
category: code
status: fixed
raised_by: tester1
date: "2026-10-27"
test_case_id: TC_TRACE_002
affected_items:
  - entity_type: requirement
    entity_id: PAY-1
{refs}"#
        ),
    )
    .unwrap();
}

fn verification_of(m: &MockJira) -> Option<String> {
    m.issue("PAY-1")?["fields"]["customfield_10005"]
        .as_str()
        .map(str::to_string)
}

/// TCMS-42.8 AC 3, through a sync run: an open linked defect blocks `passed` on --apply; the
/// same feature with no defect is passed; closing the defect on the mock and re-syncing makes
/// it passed.
#[test]
fn an_open_linked_defect_blocks_passed_through_a_sync_run() {
    let m = mock();
    m.add_issue(
        "PAY-500",
        "Bug",
        "Login defect",
        "defect text",
        "To Do",
        "2026-10-20T09:00:00.000+0000",
        &[],
    );

    // control: every case passes and nothing is open against PAY-1
    let control = workspace(&m.base_url(), false);
    all_pass_and_fixed_report(control.path(), None);
    let out = sync(control.path(), &["--apply", "--only", "import,features"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        verification_of(&m).as_deref(),
        Some("passed"),
        "the control feature is passed: {}",
        text(&out)
    );

    // the same feature with a linked open defect: not passed, and a dry run says nothing
    let blocked = workspace(&m.base_url(), false);
    all_pass_and_fixed_report(blocked.path(), Some("PAY-500"));
    let dry = sync(blocked.path(), &["--only", "features"]);
    assert!(dry.status.success(), "{}", text(&dry));
    let out = sync(blocked.path(), &["--apply", "--only", "import,features"]);
    assert!(out.status.success(), "{}", text(&out));
    let v = verification_of(&m);
    assert!(
        v.is_some() && v.as_deref() != Some("passed"),
        "an open linked defect blocks passed, got {v:?}: {}",
        text(&out)
    );

    // the defect is closed on the mock; the next sync makes the feature passed
    m.add_issue(
        "PAY-500",
        "Bug",
        "Login defect",
        "defect text",
        "Done",
        "2026-10-28T09:00:00.000+0000",
        &[],
    );
    let out = sync(blocked.path(), &["--apply", "--only", "import,features"]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(
        verification_of(&m).as_deref(),
        Some("passed"),
        "after the defect is closed: {}",
        text(&out)
    );
}

/// TCMS-42.4 AC 1 (Zephyr Scale, sync level; the API is an ASSUMPTION, the mock models it): the
/// sync creates the entities, the key lands in the record's external_refs (system zephyr_scale)
/// with a targeted edit, and a second apply creates nothing and leaves the record unchanged.
#[test]
fn zephyr_scale_apply_stores_the_entity_key_and_a_second_apply_creates_nothing() {
    let m = mock();
    let api = format!("{}/v2", m.base_url());
    let env = [("TCMS_ZEPHYR_API_URL", api.as_str())];
    let d = workspace_for(&m.base_url(), false, "zephyr_scale");
    let case_path = d.path().join("testcases/TC_TRACE_001.yml");
    let before = read(&case_path);

    let dry = sync_env(d.path(), &["--only", "tests"], &env);
    assert!(dry.status.success(), "{}", text(&dry));
    assert_eq!(
        read(&case_path),
        before,
        "a dry run does not touch the record"
    );
    assert!(m.write_requests().is_empty(), "a dry run sends no write");

    let out = sync_env(d.path(), &["--apply", "--only", "tests"], &env);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("write(s) applied"), "{}", text(&out));
    // the entities: one case per test case, one cycle for the run, one execution per case
    assert_eq!(
        posts_to(&m, "/v2/testcases"),
        2,
        "one Zephyr test case per test case"
    );
    assert_eq!(posts_to(&m, "/v2/testcycles"), 1, "one cycle for the run");
    assert_eq!(
        posts_to(&m, "/v2/testexecutions"),
        2,
        "one execution per case"
    );

    let after = read(&case_path);
    assert!(
        after.starts_with(&before),
        "every byte before the external_refs block is kept:\n{after}"
    );
    assert!(
        after.contains("external_refs:\n  - system: zephyr_scale"),
        "{after}"
    );
    let loaded: testcase_models::TestCase =
        testcase_common::load_and_validate_yaml(&case_path, schemas_root()).unwrap();
    let r = &loaded.external_refs[0];
    assert_eq!(r.system, testcase_models::ExternalSystem::ZephyrScale);
    assert!(r.key.starts_with("PAY-T"), "{}", r.key);
    let state = read(&d.path().join("state/state.yaml"));
    assert!(
        state.contains(&r.key),
        "the state and the record agree: {state}"
    );
    assert!(!state.contains(TOKEN));

    // a second apply: no entity, no cycle and no execution is created; the record is unchanged
    let before_second = (
        posts_to(&m, "/v2/testcases"),
        posts_to(&m, "/v2/testcycles"),
        posts_to(&m, "/v2/testexecutions"),
    );
    let second = sync_env(d.path(), &["--apply", "--only", "tests"], &env);
    assert!(second.status.success(), "{}", text(&second));
    assert_eq!(
        read(&case_path),
        after,
        "re-sync leaves the record byte-identical"
    );
    assert_eq!(
        (
            posts_to(&m, "/v2/testcases"),
            posts_to(&m, "/v2/testcycles"),
            posts_to(&m, "/v2/testexecutions"),
        ),
        before_second,
        "a second apply creates no Zephyr Scale object"
    );
    assert!(!text(&second).contains(TOKEN));
}

/// TCMS-42.4 AC 1 (Zephyr Squad, sync level; the zapi API is an ASSUMPTION, the mock models it):
/// the Test issues are created, the key lands in the record's external_refs (system
/// zephyr_squad), and a second apply creates no issue, no cycle and no execution.
#[test]
fn zephyr_squad_apply_stores_the_entity_key_and_a_second_apply_creates_nothing() {
    let m = mock();
    let d = workspace_for(&m.base_url(), false, "zephyr_squad");
    let case_path = d.path().join("testcases/TC_TRACE_001.yml");
    let before = read(&case_path);

    let dry = sync(d.path(), &["--only", "tests"]);
    assert!(dry.status.success(), "{}", text(&dry));
    assert_eq!(
        read(&case_path),
        before,
        "a dry run does not touch the record"
    );
    assert!(m.write_requests().is_empty(), "a dry run sends no write");

    let out = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("write(s) applied"), "{}", text(&out));
    assert_eq!(posts_to(&m, "/issue"), 2, "one Test issue per test case");
    assert_eq!(
        posts_to(&m, "/rest/zapi/latest/cycle"),
        1,
        "one cycle for the run"
    );
    assert_eq!(
        posts_to(&m, "/rest/zapi/latest/execution"),
        2,
        "one execution per case"
    );

    let after = read(&case_path);
    assert!(
        after.starts_with(&before),
        "every byte before the external_refs block is kept:\n{after}"
    );
    assert!(
        after.contains("external_refs:\n  - system: zephyr_squad"),
        "{after}"
    );
    let loaded: testcase_models::TestCase =
        testcase_common::load_and_validate_yaml(&case_path, schemas_root()).unwrap();
    let r = &loaded.external_refs[0];
    assert_eq!(r.system, testcase_models::ExternalSystem::ZephyrSquad);
    assert!(r.key.starts_with("PAY-"), "{}", r.key);
    let state = read(&d.path().join("state/state.yaml"));
    assert!(
        state.contains(&r.key),
        "the state and the record agree: {state}"
    );
    assert!(!state.contains(TOKEN));

    let before_second = (
        posts_to(&m, "/issue"),
        posts_to(&m, "/rest/zapi/latest/cycle"),
        posts_to(&m, "/rest/zapi/latest/execution"),
    );
    let second = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(second.status.success(), "{}", text(&second));
    assert_eq!(
        read(&case_path),
        after,
        "re-sync leaves the record byte-identical"
    );
    assert_eq!(
        (
            posts_to(&m, "/issue"),
            posts_to(&m, "/rest/zapi/latest/cycle"),
            posts_to(&m, "/rest/zapi/latest/execution"),
        ),
        before_second,
        "a second apply creates no Squad or Test object"
    );
    assert!(!text(&second).contains(TOKEN));
}

/// The defect the earlier run of the sync created for PR-007: it carries the report's label.
fn add_labelled_defect(m: &MockJira, key: &str, labels: &[&str]) {
    m.add_issue(
        key,
        "Bug",
        "[PR-007] Robustness case fails",
        "Robustness input is accepted.",
        "To Do",
        "2026-10-27T10:00:00.000+0000",
        labels,
    );
}

/// TCMS-42.9 defect fix (D-42.9a): a fresh checkout (no state.yaml, no key in the report) against
/// a mock that already holds the report's defect creates nothing and records the existing key.
#[test]
fn a_fresh_checkout_finds_the_existing_defect_by_label_and_creates_nothing() {
    let m = mock();
    add_labelled_defect(&m, "PAY-500", &["tcms", "tcms-pr-PR-007"]);
    let d = workspace(&m.base_url(), false);
    assert!(
        !d.path().join("state").exists(),
        "a fresh checkout has no state"
    );
    let out = sync(d.path(), &["--apply", "--only", "defects"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("existing PAY-500"),
        "the plan names the found defect: {}",
        text(&out)
    );
    assert!(
        m.write_requests().is_empty(),
        "no write is sent: {:?}",
        m.write_requests()
            .iter()
            .map(|r| format!("{} {}", r.method, r.path))
            .collect::<Vec<_>>()
    );
    let rec = read(&d.path().join("problem-reports/PR-007.yaml"));
    assert!(rec.contains("PAY-500"), "the found key is recorded: {rec}");
    assert!(
        all_text(&d.path().join("state/audit")).contains("defect found by label"),
        "the record change is audited"
    );
}

/// TCMS-42.9 reproduction (CI): two applies from fresh copies of the same committed fixtures
/// create one defect, not one per run.
#[test]
fn two_applies_from_fresh_checkouts_create_one_defect() {
    let m = mock();
    let first = workspace(&m.base_url(), false);
    let out = sync(first.path(), &["--apply", "--only", "defects"]);
    assert!(out.status.success(), "{}", text(&out));
    assert!(
        m.issue("PAY-101").is_some(),
        "the first run creates PAY-101"
    );

    let second = workspace(&m.base_url(), false);
    let out = sync(second.path(), &["--apply", "--only", "defects"]);
    assert!(out.status.success(), "{}", text(&out));

    assert_eq!(posts_to(&m, "/issue"), 1, "one defect issue for two runs");
    assert!(m.issue("PAY-102").is_none(), "no second defect");
    let labels = m.issue("PAY-101").unwrap()["fields"]["labels"].clone();
    assert!(
        labels.to_string().contains("tcms-pr-PR-007"),
        "the defect carries the report label: {labels}"
    );
    let rec = read(&second.path().join("problem-reports/PR-007.yaml"));
    assert!(
        rec.contains("PAY-101"),
        "the second run records the found key: {rec}"
    );
}

/// Two issues with the report's label are refused, not guessed between: nothing is written.
#[test]
fn two_labelled_defects_are_refused_and_nothing_is_written() {
    let m = mock();
    add_labelled_defect(&m, "PAY-500", &["tcms-pr-PR-007"]);
    add_labelled_defect(&m, "PAY-501", &["tcms-pr-PR-007"]);
    let d = workspace(&m.base_url(), false);
    let out = sync(d.path(), &["--apply", "--only", "defects"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("PAY-500, PAY-501"), "{}", text(&out));
    assert!(m.write_requests().is_empty());
}

/// The key of a test case in the sync state (`test_cases.<id>.key`).
fn state_key(dir: &Path, case: &str) -> String {
    let s = read(&dir.join("state/state.yaml"));
    s.lines()
        .skip_while(|l| !l.trim_start().starts_with(&format!("{case}:")))
        .find(|l| l.trim_start().starts_with("key:"))
        .expect("the case has a key in the sync state")
        .trim_start()
        .trim_start_matches("key:")
        .trim()
        .to_string()
}

/// TCMS-42.4 AC 5, through the sync: a test case removed from the product marks its Jira entity
/// obsolete on --apply, nothing is deleted, a dry run writes nothing and a later apply writes
/// nothing more for it.
#[test]
fn a_test_case_removed_from_the_product_marks_its_entity_obsolete_and_deletes_nothing() {
    let m = mock();
    let d = workspace(&m.base_url(), false);
    let first = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(first.status.success(), "{}", text(&first));
    let key = state_key(d.path(), "TC_TRACE_001");
    assert_eq!(
        m.issue(&key).expect("entity")["fields"]["customfield_10002"],
        "draft"
    );

    std::fs::remove_file(d.path().join("testcases/TC_TRACE_001.yml")).unwrap();
    let dry = sync(d.path(), &["--only", "tests"]);
    assert!(dry.status.success(), "{}", text(&dry));
    assert_eq!(
        m.issue(&key).expect("entity")["fields"]["customfield_10002"],
        "draft",
        "a dry run writes nothing"
    );

    let applied = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(applied.status.success(), "{}", text(&applied));
    let issue = m.issue(&key).expect("the entity is kept, never deleted");
    assert_eq!(issue["fields"]["customfield_10002"], "obsolete", "{issue}");
    assert_eq!(
        m.write_requests()
            .iter()
            .filter(|r| r.method == "DELETE")
            .count(),
        0,
        "no delete is sent"
    );

    // the lifecycle is written once: the later apply reports no obsolete write for the case
    assert!(
        text(&applied).contains("PAY-101 marked obsolete"),
        "{}",
        text(&applied)
    );
    let again = sync(d.path(), &["--apply", "--only", "tests"]);
    assert!(again.status.success(), "{}", text(&again));
    assert!(
        !text(&again).contains("marked obsolete"),
        "an entity already obsolete is not written again:\n{}",
        text(&again)
    );
}
