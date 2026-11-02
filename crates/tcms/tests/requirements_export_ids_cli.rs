//! `tcms requirements export-ids` run as the real binary: Jira against the mock (Cloud and Data
//! Center), StrictDoc against the assumed-layout fixture. Verified against the mock only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use jira_client::mock::{Layout, MockJira};
use serde_json::json;

const ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const TOKEN: &str = "tok-exp-9d37";

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

/// Run the binary with `dir` as the current directory (the default requirements path is relative
/// to it). The configuration file and the requirements file live in different directories.
fn tcms_in(dir: &Path, cfg: &Path, args: &[&str], token: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_tcms"));
    cmd.current_dir(dir)
        .args(["--config", &cfg.to_string_lossy()])
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

/// A requirements file in `dir` with the given product and IDs, in the D-11 shape.
fn req_file(dir: &Path, name: &str, product: &str, ids: &[&str]) -> PathBuf {
    let requirements: Vec<serde_json::Value> = ids.iter().map(|id| json!({ "id": id })).collect();
    let body = json!({ "product_id": product, "requirements": requirements });
    raw(dir, name, &serde_json::to_string_pretty(&body).unwrap())
}

/// A file in `dir` with exactly this text.
fn raw(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

/// Product_id and ID list of a written export-ids document.
fn ids_of(json_text: &str) -> (String, Vec<String>) {
    let v: serde_json::Value =
        serde_json::from_str(json_text).unwrap_or_else(|e| panic!("{e}: {json_text}"));
    let product = v["product_id"].as_str().unwrap().to_string();
    let ids = v["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect();
    (product, ids)
}

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

// ---------------------------------------------------------------- Jira: happy path

fn export_all_valid(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(layout, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-1", "PAY-2", "PAY-3"],
    );
    let before = snapshot(cwd.path());

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--as", "alice"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{layout:?}: {stderr}");
    assert_eq!(
        ids_of(&stdout),
        ("PROD-001".into(), strings(&["PAY-1", "PAY-2", "PAY-3"])),
        "{layout:?}: {stdout}"
    );
    assert!(!stderr.contains("invalid:"), "{layout:?}: {stderr}");

    // D-12: one batch search for the whole file
    let searches = search_requests(&mock);
    assert_eq!(searches.len(), 1, "{layout:?}");
    assert!(
        searches[0].contains(r#"key in ("PAY-1", "PAY-2", "PAY-3")"#),
        "{}",
        searches[0]
    );

    assert!(!stdout.contains(TOKEN) && !stderr.contains(TOKEN));
    assert_eq!(
        snapshot(cwd.path()),
        before,
        "{layout:?}: export-ids wrote into the working directory"
    );
}

#[test]
fn export_ids_default_path_all_valid_cloud() {
    export_all_valid(Layout::Cloud);
}

#[test]
fn export_ids_default_path_all_valid_datacenter() {
    export_all_valid(Layout::DataCenter);
}

#[test]
fn export_ids_requirements_flag_reads_the_named_file() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap(); // no requirements.json here
    let other = tempfile::tempdir().unwrap();
    let file = req_file(other.path(), "product.json", "PROD-001", &["PAY-2"]);
    let file_arg = file.to_string_lossy().into_owned();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &[
            "requirements",
            "export-ids",
            "--requirements",
            &file_arg,
            "--as",
            "alice",
        ],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    assert_eq!(ids_of(&stdout).1, strings(&["PAY-2"]), "{stdout}");
}

// ---------------------------------------------------------------- exit 2: refused before any lookup

#[test]
fn export_ids_missing_default_file_exits_2() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap(); // empty

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("read requirements file"), "{stderr}");
    assert!(stderr.contains("requirements.json"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_product_mismatch_exits_2() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(cwd.path(), "requirements.json", "PROD-999", &["PAY-2"]);

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.contains(
            "product_id 'PROD-999' does not match product_id 'PROD-001' of the integration config"
        ),
        "{stderr}"
    );
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_unparseable_file_exits_2() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));

    for body in ["{not json", r#"{"product_id":"PROD-001"}"#] {
        let cwd = tempfile::tempdir().unwrap();
        raw(cwd.path(), "requirements.json", body);
        let o = tcms_in(
            cwd.path(),
            &cfg,
            &["requirements", "export-ids"],
            Some(TOKEN),
        );
        let (stdout, stderr) = text(&o);
        assert_eq!(o.status.code(), Some(2), "{body}: {stderr}");
        assert!(
            stderr.contains("parse requirements file"),
            "{body}: {stderr}"
        );
        assert!(stdout.is_empty(), "{body}: {stdout}");
    }
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_empty_list_exits_2() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(cwd.path(), "requirements.json", "PROD-001", &[]);

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("'requirements' is empty"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_needs_the_token() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(cwd.path(), "requirements.json", "PROD-001", &["PAY-2"]);

    let o = tcms_in(cwd.path(), &cfg, &["requirements", "export-ids"], None);
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains(ENV), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_unknown_id_format_exits_2() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-2", "abc"],
    );

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("'abc'"), "{stderr}");
    assert!(stderr.contains("matches no system id_pattern"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(mock.request_count(), 0);
}

#[test]
fn export_ids_extra_fields_are_an_error() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    raw(
        cwd.path(),
        "requirements.json",
        r#"{"product_id":"PROD-001","requirements":[{"id":"PAY-2","title":"x"}]}"#,
    );
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("valid.json");
    let out_arg = out.to_string_lossy().into_owned();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--output", &out_arg],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert!(!out.exists(), "a file was written on error");
    assert_eq!(mock.request_count(), 0);
}

// ---------------------------------------------------------------- exit 1: mixed and duplicates

fn export_mixed_valid_and_invalid(layout: Layout) {
    let mock = MockJira::start(layout, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(layout, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-2", "PAY-99", "PAY-3", "PAY-98"],
    );

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{layout:?}: {stderr}");
    // valid IDs are still written, in file order
    assert_eq!(
        ids_of(&stdout),
        ("PROD-001".into(), strings(&["PAY-2", "PAY-3"])),
        "{layout:?}: {stdout}"
    );
    assert!(
        stderr.contains("invalid: PAY-99: not found in jira (jira)"),
        "{layout:?}: {stderr}"
    );
    assert!(
        stderr.contains("invalid: PAY-98: not found in jira (jira)"),
        "{layout:?}: {stderr}"
    );
    assert!(
        stderr.contains("invalid: 2 of 4 requirement IDs"),
        "{layout:?}: {stderr}"
    );
    // the mock returned these too; only asked keys count
    for other in ["PAY-1", "PAY-4", "OTH-1"] {
        assert!(!stdout.contains(other), "{other} leaked into {stdout}");
    }
    assert_eq!(search_requests(&mock).len(), 1, "{layout:?}");
}

#[test]
fn export_ids_mixed_valid_and_invalid_exits_1_cloud() {
    export_mixed_valid_and_invalid(Layout::Cloud);
}

#[test]
fn export_ids_mixed_valid_and_invalid_exits_1_datacenter() {
    export_mixed_valid_and_invalid(Layout::DataCenter);
}

#[test]
fn export_ids_duplicate_id_is_reported_once() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-2", "PAY-99", "PAY-2", "PAY-99"],
    );

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids"],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    assert_eq!(ids_of(&stdout).1, strings(&["PAY-2"]), "{stdout}");
    assert_eq!(stderr.matches("invalid: PAY-99").count(), 1, "{stderr}");
    assert!(
        stderr.contains("invalid: 1 of 2 requirement IDs"),
        "{stderr}"
    );
    // checked once: the one search names PAY-2 once
    let searches = search_requests(&mock);
    assert_eq!(searches.len(), 1);
    assert_eq!(
        searches[0].matches("\"PAY-2\"").count(),
        1,
        "{}",
        searches[0]
    );
}

// ---------------------------------------------------------------- --output

#[test]
fn export_ids_output_file_instead_of_stdout() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-1", "PAY-2"],
    );
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("valid.json");
    let out_arg = out.to_string_lossy().into_owned();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--output", &out_arg],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(0), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    let written = std::fs::read_to_string(&out).unwrap();
    assert_eq!(
        ids_of(&written),
        ("PROD-001".into(), strings(&["PAY-1", "PAY-2"])),
        "{written}"
    );
}

#[test]
fn export_ids_output_file_on_exit_1_has_the_valid_ids() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-001",
        &["PAY-2", "PAY-99"],
    );
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("valid.json");
    let out_arg = out.to_string_lossy().into_owned();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--output", &out_arg],
        Some(TOKEN),
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
    assert_eq!(
        ids_of(&std::fs::read_to_string(&out).unwrap()).1,
        strings(&["PAY-2"])
    );
    assert!(
        stderr.contains("invalid: PAY-99: not found in jira (jira)"),
        "{stderr}"
    );
}

#[test]
fn export_ids_output_file_not_created_on_error() {
    let mock = MockJira::start(Layout::Cloud, &format!("Bearer {TOKEN}"));
    seed(&mock);
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), &jira_yaml(Layout::Cloud, &mock.base_url()));
    let cwd = tempfile::tempdir().unwrap();
    req_file(cwd.path(), "requirements.json", "PROD-999", &["PAY-2"]);
    let out_dir = tempfile::tempdir().unwrap();
    let out = out_dir.path().join("valid.json");
    let out_arg = out.to_string_lossy().into_owned();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--output", &out_arg],
        Some(TOKEN),
    );
    let (_, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(!out.exists(), "output written on error");
}

// ---------------------------------------------------------------- StrictDoc

#[test]
fn export_ids_strictdoc_with_input() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), STRICTDOC_YAML);
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-002",
        &["HLR-12", "HLR-99", "SYS-4"],
    );
    let inp = fixture_arg();

    let o = tcms_in(
        cwd.path(),
        &cfg,
        &["requirements", "export-ids", "--input", &inp],
        None,
    );
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(1), "{stderr}");
    assert_eq!(
        ids_of(&stdout),
        ("PROD-002".into(), strings(&["HLR-12", "SYS-4"])),
        "{stdout}"
    );
    assert!(
        stderr.contains("invalid: HLR-99: not found in strictdoc (strictdoc)"),
        "{stderr}"
    );
    assert!(
        stderr.contains("invalid: 1 of 3 requirement IDs"),
        "{stderr}"
    );
}

#[test]
fn export_ids_strictdoc_without_input_exits_2() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write(cfg_dir.path(), STRICTDOC_YAML);
    let cwd = tempfile::tempdir().unwrap();
    req_file(
        cwd.path(),
        "requirements.json",
        "PROD-002",
        &["HLR-12", "SYS-4"],
    );

    let o = tcms_in(cwd.path(), &cfg, &["requirements", "export-ids"], None);
    let (stdout, stderr) = text(&o);
    assert_eq!(o.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("--input"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}
