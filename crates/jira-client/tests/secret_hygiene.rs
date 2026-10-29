//! The token never appears in a record, a log line, an error message or a Debug print.
//! One test function, because the logger is process-global.

mod common;

use common::*;
use jira_client::guard::WriteKind;
use jira_client::mock::Layout;
use jira_client::{write_audit_entries, JiraClient, WriteTarget};
use serde_json::{json, Map, Value};
use std::sync::Mutex;

static LOG_LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct CaptureLogger;
static LOGGER: CaptureLogger = CaptureLogger;

impl log::Log for CaptureLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, record: &log::Record) {
        LOG_LINES
            .lock()
            .unwrap()
            .push(format!("{} {}", record.level(), record.args()));
    }
    fn flush(&self) {}
}

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect_files(&p, out);
        } else {
            out.push(p);
        }
    }
}

#[test]
fn token_never_reaches_records_logs_or_errors() {
    let secret_env = "TCMS_TEST_TOKEN";
    // from_config reads the token from the environment variable named by secret_ref
    std::env::set_var(secret_env, TOKEN);
    std::env::set_var("TCMS_JIRA_USER", SERVICE_USER);
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    let layout = Layout::Cloud;
    let mock = mock_with_issue(layout);
    let cfg = config(
        layout,
        &mock.base_url(),
        &["PAY", "NOPE"],
        Policy::default(),
    );
    let mut texts: Vec<String> = Vec::new();

    let mut c = JiraClient::from_config(&cfg.systems[0], &cfg.sync, ACTOR).expect("token from env");
    c = c.with_retry(fast_retry());
    texts.push(format!("{c:?}"));

    let report = c.test_connection().unwrap();
    texts.push(format!("{report:?}"));

    let mut fields = Map::new();
    fields.insert("customfield_10101".into(), json!("passed"));
    let entry = c
        .update_fields(
            "PAY-1",
            WriteKind::FeatureFields,
            &fields,
            &WriteTarget::new("requirement", "HLR-1"),
            "sync",
        )
        .unwrap();
    texts.push(serde_json::to_string(&entry).unwrap());
    c.add_comment(
        "PAY-1",
        "verification passed",
        &WriteTarget::new("requirement", "HLR-1"),
        "sync",
    )
    .unwrap();

    // a failing write and a refused write: their messages must not carry the token either
    let err = c
        .update_fields(
            "PAY-999",
            WriteKind::FeatureFields,
            &fields,
            &WriteTarget::new("requirement", "HLR-9"),
            "sync",
        )
        .unwrap_err();
    texts.push(format!("{err:#}"));
    let mut summary = Map::new();
    summary.insert("summary".into(), json!("renamed"));
    let err = c
        .update_fields(
            "PAY-1",
            WriteKind::FeatureFields,
            &summary,
            &WriteTarget::new("requirement", "HLR-1"),
            "sync",
        )
        .unwrap_err();
    texts.push(format!("{err:#}"));

    // a wrong token: the mock answers 401 and the error text is checked
    let bad = JiraClient::with_token(
        &cfg.systems[0],
        &cfg.sync,
        ACTOR,
        jira_client::Secret::new("wrong-token-value"),
        Some(SERVICE_USER.into()),
    )
    .unwrap();
    texts.push(format!("{:#}", bad.test_connection().unwrap_err()));

    // the audit entries written to disk (the records of the sync)
    let dir = tempfile::tempdir().unwrap();
    let entries: Vec<_> = c.audit_entries().to_vec();
    let paths = write_audit_entries(dir.path(), &entries).unwrap();
    assert_eq!(
        paths.len(),
        2,
        "one audit entry for the field write, one for the comment"
    );
    // and they validate against the audit-trail-entry schema
    for p in &paths {
        let text = std::fs::read_to_string(p).unwrap();
        let _: Value =
            testcase_common::parse_and_validate_yaml_string(&text, schemas_root(), "audit")
                .unwrap();
    }
    let mut files = Vec::new();
    collect_files(dir.path(), &mut files);
    for f in &files {
        texts.push(std::fs::read_to_string(f).unwrap());
    }
    texts.push(format!("{:?}", jira_client::Secret::new(TOKEN)));

    // the secret is in the environment and the request header, but not in any text checked here
    for (i, t) in texts.iter().enumerate() {
        assert!(!t.contains(TOKEN), "text #{i} contains the token: {t}");
    }
    let logs = LOG_LINES.lock().unwrap();
    assert!(
        !logs.is_empty(),
        "the client should have logged something at debug level"
    );
    for line in logs.iter() {
        assert!(!line.contains(TOKEN), "log line contains the token: {line}");
    }
    // the token did reach the wire (proof the check above is not vacuous)
    assert!(mock.requests().iter().any(|r| r
        .header("authorization")
        .is_some_and(|h| h.contains("Basic"))));
    assert!(!mock.write_requests().is_empty());
    std::env::remove_var(secret_env);
}
