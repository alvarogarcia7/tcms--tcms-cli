//! `tcms requirements locate` run as the real binary: Jira against the mock (Cloud and Data
//! Center), StrictDoc against the assumed-layout fixture. Verified against the mock only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-loc-7c20";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture() -> PathBuf {
    root().join("crates/requirement-import/tests/fixtures/strictdoc_export.json")
}

fn fixture_arg() -> String {
    fixture().to_string_lossy().into_owned()
}

fn write(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("integration-config.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn jira_yaml(layout: Layout, base_url: &str) -> String {
    let ty = match layout {
        Layout::Cloud => "jira_cloud",
        Layout::DataCenter => "jira_datacenter",
    };
    format!(
        r#"product_id: PROD-001
requirement_source: jira
systems:
  - name: jira
    type: {ty}
    base_url: {base_url}
    auth:
      method: service_account_token
      secret_ref: {ENV}
    id_pattern: '^[A-Z]+-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [PAY]
    feature_issue_types: [Epic, Story, Feature]
sync:
  identity: tcms-bot
  record_actor: true
"#
    )
}

/// Jira and StrictDoc in one configuration. The two ID patterns are disjoint.
fn mixed_yaml(layout: Layout, base_url: &str) -> String {
    let ty = match layout {
        Layout::Cloud => "jira_cloud",
        Layout::DataCenter => "jira_datacenter",
    };
    format!(
        r#"product_id: PROD-003
requirement_source: jira
systems:
  - name: jira
    type: {ty}
    base_url: {base_url}
    auth:
      method: service_account_token
      secret_ref: {ENV}
    id_pattern: '^(PAY|OTH)-[0-9]+$'
    url_template: '{{base_url}}/browse/{{id}}'
    project_keys: [PAY]
    feature_issue_types: [Epic, Story, Feature]
  - name: strictdoc
    type: strictdoc_html
    base_url: https://pages.example.com/srs
    id_pattern: '^(SYS|HLR|LLR)-[0-9.]+$'
    url_template: '{{base_url}}/{{doc}}.html#{{id}}'
    document_map:
      SYS: system-requirements
      HLR: software-requirements
      LLR: low-level-requirements
sync:
  identity: tcms-bot
  record_actor: true
"#
    )
}

const STRICTDOC_YAML: &str = r#"product_id: PROD-002
requirement_source: strictdoc
systems:
  - name: strictdoc
    type: strictdoc_html
    base_url: https://pages.example.com/srs
    id_pattern: '^(SYS|HLR|LLR)-[0-9.]+$'
    url_template: '{base_url}/{doc}.html#{id}'
    document_map:
      SYS: system-requirements
      HLR: software-requirements
      LLR: low-level-requirements
"#;

fn tcms(cfg: &Path, args: &[&str], token: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    cmd.args(["--config", &cfg.to_string_lossy()])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(args)
        .env_remove(ENV);
    if let Some(t) = token {
        cmd.env(ENV, t);
    }
    cmd.output().unwrap()
}

fn text(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn snapshot(dir: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// Decode `+` and `%XX` (hand-written: no new dependency).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => {
                let hex = std::str::from_utf8(&b[i + 1..i + 3])
                    .ok()
                    .and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => {
                        out.push(v);
                        i += 3;
                    }
                    None => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Decoded paths of every request that is a Jira search (Cloud `/search/jql`, DC `/search`).
fn search_requests(mock: &MockJira) -> Vec<String> {
    mock.requests()
        .iter()
        .filter(|r| r.path.contains("/search"))
        .map(|r| percent_decode(&r.path))
        .collect()
}

/// Five issues. The mock returns every stored issue for any search, so the tests rely on the
/// client matching keys.
fn seed(mock: &MockJira) {
    let upd = "2026-10-20T09:00:00.000+0000";
    mock.add_issue(
        "PAY-1",
        "Epic",
        "Payments",
        "Epic text",
        "In progress",
        upd,
        &[],
    );
    mock.add_issue(
        "PAY-2",
        "Story",
        "Pay by card",
        "Stored card",
        "In progress",
        upd,
        &[],
    );
    mock.add_issue(
        "PAY-3",
        "Bug",
        "Crash on refund",
        "Refund crash",
        "Open",
        upd,
        &[],
    );
    mock.add_issue(
        "PAY-4",
        "Story",
        "Refund to card",
        "Refund",
        "To Do",
        upd,
        &[],
    );
    mock.add_issue(
        "OTH-1",
        "Story",
        "Other team story",
        "Other",
        "Done",
        upd,
        &[],
    );
}

fn strictdoc_fixture_with(dir: &Path, body: &str) -> PathBuf {
    let p = dir.join("export.json");
    std::fs::write(&p, body).unwrap();
    p
}

// ---------------------------------------------------------------- Jira

fn jira_batch_finds_any_issue_type(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(layout, &mock.base_url()));
    let before = snapshot(dir.path());

    let o = tcms(
        &cfg,
        &[
            "requirements",
            "locate",
            "PAY-2",
            "PAY-3",
            "OTH-1",
            "PAY-1",
            "--as",
            "alice",
        ],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{layout:?}: {stderr}");
    let base = mock.base_url();
    // command-line order; the text format is one line per ID plus the summary
    let expected = format!(
        "✓ PAY-2: found in jira (jira) \"Pay by card\" [In progress] {base}/browse/PAY-2\n\
         ✓ PAY-3: found in jira (jira) \"Crash on refund\" [Open] {base}/browse/PAY-3\n\
         ✓ OTH-1: found in jira (jira) \"Other team story\" [Done] {base}/browse/OTH-1\n\
         ✓ PAY-1: found in jira (jira) \"Payments\" [In progress] {base}/browse/PAY-1\n\
         4 of 4 requirement IDs found\n"
    );
    assert_eq!(stdout, expected, "{layout:?}");

    // D-03: one batch search for all four IDs
    assert_eq!(search_requests(&mock).len(), 1, "{layout:?}");
    let searches = search_requests(&mock);
    let q = &searches[0];
    assert!(
        q.contains(r#"key in ("PAY-2", "PAY-3", "OTH-1", "PAY-1")"#),
        "{q}"
    );
    assert!(q.contains("fields=summary,status"), "{q}");
    assert!(!q.contains("project in"), "{q}");
    assert!(!q.contains("issuetype in"), "{q}");

    // the token reaches the request header only: no recorded URL carries it
    for r in mock.requests() {
        assert!(
            !percent_decode(&r.path).contains(TOKEN),
            "{layout:?}: token in request path {}",
            r.path
        );
    }
    assert!(!stdout.contains(TOKEN) && !stderr.contains(TOKEN));
    assert_eq!(
        snapshot(dir.path()),
        before,
        "{layout:?}: locate wrote a file"
    );
}

#[test]
fn locate_jira_batch_finds_any_issue_type_cloud() {
    jira_batch_finds_any_issue_type(Layout::Cloud);
}

#[test]
fn locate_jira_batch_finds_any_issue_type_datacenter() {
    jira_batch_finds_any_issue_type(Layout::DataCenter);
}

fn jira_matches_the_exact_key(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(layout, &mock.base_url()));

    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "PAY-99"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{layout:?}: {stderr}");
    let base = mock.base_url();
    let expected = format!(
        "✓ PAY-2: found in jira (jira) \"Pay by card\" [In progress] {base}/browse/PAY-2\n\
         ✗ PAY-99: not found in jira (jira)\n\
         1 of 2 requirement IDs found\n"
    );
    assert_eq!(stdout, expected, "{layout:?}");
    // the mock returned these too; only asked keys count
    for other in ["PAY-4", "PAY-1", "PAY-3", "OTH-1"] {
        assert!(!stdout.contains(other), "{other} leaked into {stdout}");
    }
    assert!(
        stderr.contains("not found: 1 of 2 requirement IDs"),
        "{stderr}"
    );
    assert_eq!(search_requests(&mock).len(), 1, "{layout:?}");
}

#[test]
fn locate_jira_matches_the_exact_key_cloud() {
    jira_matches_the_exact_key(Layout::Cloud);
}

#[test]
fn locate_jira_matches_the_exact_key_datacenter() {
    jira_matches_the_exact_key(Layout::DataCenter);
}

#[test]
fn locate_jira_needs_the_token() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let args = ["requirements", "locate", "PAY-2"];

    // variable unset
    let o = tcms(&cfg, &args, None);
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains(ENV), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);

    // variable set but empty
    let o = tcms(&cfg, &args, Some(""));
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains(ENV), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn locate_unknown_id_format_exits_2_naming_the_id() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));

    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "abc"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("'abc'"), "{stderr}");
    assert!(stderr.contains("matches no system id_pattern"), "{stderr}");
    // D-02: the valid ID is not printed and no lookup is sent
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn locate_repeated_id_is_reported_once() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));

    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "PAY-2"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    assert_eq!(stdout.matches("PAY-2: found").count(), 1, "{stdout}");
    assert!(
        stdout.ends_with("1 of 1 requirement IDs found\n"),
        "{stdout}"
    );
}

#[test]
fn locate_without_ids_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), &jira_yaml(Layout::Cloud, "http://127.0.0.1:9"));
    let o = tcms(&cfg, &["requirements", "locate"], Some(TOKEN));
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    // clap's missing-IDs error, not "unrecognized subcommand"
    assert!(
        stderr.contains("required arguments were not provided"),
        "{stderr}"
    );
    assert!(stderr.contains("<IDS>"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}

// ---------------------------------------------------------------- StrictDoc

#[test]
fn locate_strictdoc_reports_every_level() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let before = snapshot(dir.path());
    let fx = fixture_arg();

    let o = tcms(
        &cfg,
        &[
            "requirements",
            "locate",
            "HLR-12",
            "SYS-4",
            "HLR-3",
            "--input",
            &fx,
        ],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    // D-05: level system is found, alongside the HLR levels
    let expected = "\
✓ HLR-12: found in strictdoc (strictdoc) at level HLR \"Update profile metadata over SMS\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-12
✓ SYS-4: found in strictdoc (strictdoc) at level system \"Remote profile management\" [approved] https://pages.example.com/srs/system-requirements.html#SYS-4
✓ HLR-3: found in strictdoc (strictdoc) at level HLR \"Reject unsigned profiles\" [in_review] https://pages.example.com/srs/software-requirements.html#HLR-3
3 of 3 requirement IDs found
";
    assert_eq!(stdout, expected);
    assert_eq!(snapshot(dir.path()), before, "locate wrote a file");
}

#[test]
fn locate_strictdoc_missing_uid_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture_arg();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-12", "HLR-99", "--input", &fx],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    let expected = "\
✓ HLR-12: found in strictdoc (strictdoc) at level HLR \"Update profile metadata over SMS\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-12
✗ HLR-99: not found in strictdoc (strictdoc)
1 of 2 requirement IDs found
";
    assert_eq!(stdout, expected);
    assert!(
        stderr.contains("not found: 1 of 2 requirement IDs"),
        "{stderr}"
    );
}

#[test]
fn locate_strictdoc_needs_input() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let o = tcms(&cfg, &["requirements", "locate", "HLR-12"], None);
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("--input"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}

#[test]
fn locate_strictdoc_unreadable_input() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let missing = dir.path().join("missing.json");
    let missing_s = missing.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-12", "--input", &missing_s],
        None,
    );
    let (_, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("read "), "{stderr}");
    assert!(stderr.contains("missing.json"), "{stderr}");
}

#[test]
fn locate_strictdoc_wrong_layout() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let bad = strictdoc_fixture_with(dir.path(), "[1,2]");
    let bad_s = bad.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-12", "--input", &bad_s],
        None,
    );
    let (_, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("DOCUMENTS"), "{stderr}");
}

#[test]
fn locate_strictdoc_refused_level() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let bad = strictdoc_fixture_with(
        dir.path(),
        r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"chapter"}]}]}"#,
    );
    let bad_s = bad.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-1", "--input", &bad_s],
        None,
    );
    let (_, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("is not a requirement type"), "{stderr}");
}

#[test]
fn locate_strictdoc_duplicate_uid() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let bad = strictdoc_fixture_with(
        dir.path(),
        r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"HLR"},{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"HLR"}]}]}"#,
    );
    let bad_s = bad.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-1", "--input", &bad_s],
        None,
    );
    let (_, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("HLR-1"), "{stderr}");
    assert!(stderr.contains("more than once"), "{stderr}");
}

#[test]
fn locate_strictdoc_old_lowercase_layout_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let old = strictdoc_fixture_with(
        dir.path(),
        r#"{"documents":[{"path":"d.sdoc","requirements":[{"uid":"HLR-1","level":"HLR"}]}]}"#,
    );
    let old_s = old.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-1", "--input", &old_s],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("DOCUMENTS"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}

#[test]
fn locate_strictdoc_item_without_level_exits_2() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let bad = strictdoc_fixture_with(
        dir.path(),
        r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1"}]}]}"#,
    );
    let bad_s = bad.to_string_lossy().into_owned();
    let o = tcms(
        &cfg,
        &["requirements", "locate", "HLR-1", "--input", &bad_s],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains("HLR-1") && stderr.contains("has no LEVEL"),
        "{stderr}"
    );
    assert!(stdout.is_empty(), "{stdout}");
}

// ---------------------------------------------------------------- mixed sources

/// Cloud mock seeded, mixed configuration, and the export path.
fn mixed_setup(dir: &Path) -> (MockJira, PathBuf, String) {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg = write(dir, &mixed_yaml(Layout::Cloud, &mock.base_url()));
    (mock, cfg, fixture_arg())
}

#[test]
fn locate_mixed_sources_one_search() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, cfg, fx) = mixed_setup(dir.path());
    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "HLR-12", "--input", &fx],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    let expected = format!(
        "✓ PAY-2: found in jira (jira) \"Pay by card\" [In progress] {}/browse/PAY-2\n\
         ✓ HLR-12: found in strictdoc (strictdoc) at level HLR \"Update profile metadata over SMS\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-12\n\
         2 of 2 requirement IDs found\n",
        mock.base_url()
    );
    assert_eq!(stdout, expected);

    let searches = search_requests(&mock);
    assert_eq!(searches.len(), 1, "{searches:?}");
    assert!(
        searches[0].contains(r#"key in ("PAY-2")"#),
        "{}",
        searches[0]
    );
    assert!(!searches[0].contains("HLR-12"), "{}", searches[0]);
}

#[test]
fn locate_mixed_strictdoc_without_input_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, cfg, _fx) = mixed_setup(dir.path());
    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "HLR-12"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("HLR-12"), "{stderr}");
    assert!(stderr.contains("--input"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn locate_format_json() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, cfg, fx) = mixed_setup(dir.path());
    let o = tcms(
        &cfg,
        &[
            "requirements",
            "locate",
            "PAY-2",
            "HLR-12",
            "PAY-99",
            "--input",
            &fx,
            "--format=json",
        ],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    assert!(!stdout.contains('✓'), "{stdout}");
    assert!(!stdout.contains("requirement IDs found"), "{stdout}");

    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = v.as_array().expect("json array");
    assert_eq!(arr.len(), 3, "{stdout}");

    assert_eq!(arr[0]["id"].as_str(), Some("PAY-2"));
    assert_eq!(arr[0]["found"].as_bool(), Some(true));
    assert_eq!(arr[0]["source"].as_str(), Some("jira"));
    assert_eq!(arr[0]["system"].as_str(), Some("jira"));
    assert!(arr[0]["level"].is_null());
    assert_eq!(arr[0]["title"].as_str(), Some("Pay by card"));
    assert_eq!(arr[0]["status"].as_str(), Some("In progress"));
    assert_eq!(
        arr[0]["url"].as_str(),
        Some(format!("{}/browse/PAY-2", mock.base_url()).as_str())
    );

    assert_eq!(arr[1]["id"].as_str(), Some("HLR-12"));
    assert_eq!(arr[1]["found"].as_bool(), Some(true));
    assert_eq!(arr[1]["source"].as_str(), Some("strictdoc"));
    assert_eq!(arr[1]["system"].as_str(), Some("strictdoc"));
    assert_eq!(arr[1]["level"].as_str(), Some("HLR"));
    assert_eq!(arr[1]["status"].as_str(), Some("approved"));

    assert_eq!(arr[2]["id"].as_str(), Some("PAY-99"));
    assert_eq!(arr[2]["found"].as_bool(), Some(false));
    assert!(arr[2]["title"].is_null());
    assert!(arr[2]["status"].is_null());
    assert!(arr[2]["level"].is_null());
}

#[test]
fn locate_format_plaintext() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, cfg, fx) = mixed_setup(dir.path());
    let o = tcms(
        &cfg,
        &[
            "requirements",
            "locate",
            "PAY-2",
            "HLR-12",
            "PAY-99",
            "--input",
            &fx,
            "--format",
            "plaintext",
        ],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    let expected = format!(
        "PAY-2: found in jira (jira) \"Pay by card\" [In progress] {}/browse/PAY-2\n\
         HLR-12: found in strictdoc (strictdoc) at level HLR \"Update profile metadata over SMS\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-12\n\
         PAY-99: not found in jira (jira)\n",
        mock.base_url()
    );
    assert_eq!(stdout, expected);
    assert!(!stdout.contains('✓') && !stdout.contains('✗'));
    assert!(!stdout.contains("\x1b["));
    assert!(!stdout.contains("requirement IDs found"));
}

#[test]
fn locate_text_has_no_colour_when_not_a_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let (mock, cfg, fx) = mixed_setup(dir.path());
    let o = tcms(
        &cfg,
        &["requirements", "locate", "PAY-2", "HLR-12", "--input", &fx],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    let expected = format!(
        "✓ PAY-2: found in jira (jira) \"Pay by card\" [In progress] {}/browse/PAY-2\n\
         ✓ HLR-12: found in strictdoc (strictdoc) at level HLR \"Update profile metadata over SMS\" [approved] https://pages.example.com/srs/software-requirements.html#HLR-12\n\
         2 of 2 requirement IDs found\n",
        mock.base_url()
    );
    assert_eq!(stdout, expected);
    assert!(!stdout.contains("\x1b["), "{stdout:?}");
}

#[test]
fn locate_rejects_an_unknown_format() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write(dir.path(), STRICTDOC_YAML);
    let fx = fixture_arg();
    let o = tcms(
        &cfg,
        &[
            "requirements",
            "locate",
            "HLR-12",
            "--input",
            &fx,
            "--format",
            "xml",
        ],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    // clap's rejection of the value, naming the bad value and the flag
    assert!(stderr.contains("invalid value 'xml'"), "{stderr}");
    assert!(stderr.contains("--format"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}
