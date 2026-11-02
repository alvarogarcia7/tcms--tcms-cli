//! `tcms requirements export-ids` run as the real binary on an export made by the real StrictDoc
//! tool (crates/tcms/tests/fixtures/strictdoc). No network and no Python at test time; StrictDoc is
//! never called here. Every expected answer is a literal in this file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TOKEN_ENV: &str = "TCMS_TEST_JIRA_TOKEN";
const CONFIG_NAME: &str = "integration-config.yaml";

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

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Checked-in real StrictDoc export for one case (locate, refused-level, no-level).
fn export(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/strictdoc")
        .join(case)
        .join("export.json")
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn fix_locate() -> String {
    path_str(&export("locate"))
}

fn fix_refused() -> String {
    path_str(&export("refused-level"))
}

fn fix_no_level() -> String {
    path_str(&export("no-level"))
}

/// A temporary directory holding the StrictDoc configuration. Kept apart from the working directory.
fn config_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(CONFIG_NAME), STRICTDOC_YAML).unwrap();
    dir
}

/// Run `tcms requirements export-ids <args>` with `cwd` as the current directory and the token
/// variable removed.
fn export_ids(cwd: &Path, cfg_dir: &Path, args: &[&str]) -> Output {
    let cfg = path_str(&cfg_dir.join(CONFIG_NAME));
    Command::new(env!("CARGO_BIN_EXE_tcms"))
        .current_dir(cwd)
        .args(["--config", &cfg])
        .args(["--schemas-root", &format!("{}/schemas/", root().display())])
        .args(["requirements", "export-ids"])
        .args(args)
        .env_remove(TOKEN_ENV)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// A requirements file in `dir` in the D-11 shape, named `name`.
fn req_file(dir: &Path, name: &str, product: &str, ids: &[&str]) -> PathBuf {
    let requirements: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "id": id }))
        .collect();
    let body = serde_json::json!({ "product_id": product, "requirements": requirements });
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

// ---------------------------------------------------------------- valid IDs (exit 0 / 1)

#[test]
fn export_ids_all_valid_writes_every_id_to_stdout() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(
        work.path(),
        "req.json",
        "PROD-002",
        &["HLR-1", "LLR-1", "SYS-1"],
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(
        ids_of(&stdout(&o)),
        (
            "PROD-002".to_string(),
            strings(&["HLR-1", "LLR-1", "SYS-1"])
        )
    );
    assert!(!stderr(&o).contains("invalid:"), "stderr: {}", stderr(&o));
}

#[test]
fn export_ids_reads_requirements_json_of_the_current_directory_by_default() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    req_file(
        work.path(),
        "requirements.json",
        "PROD-002",
        &["SYS-1", "HLR-1"],
    );
    let fix = fix_locate();
    let o = export_ids(work.path(), cfg.path(), &["--input", &fix]);
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(
        ids_of(&stdout(&o)),
        ("PROD-002".to_string(), strings(&["SYS-1", "HLR-1"]))
    );
}

#[test]
fn export_ids_one_invalid_id_exits_1_and_still_writes_the_valid_ones() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(
        work.path(),
        "req.json",
        "PROD-002",
        &["HLR-1", "HLR-99", "SYS-1"],
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(
        ids_of(&stdout(&o)),
        ("PROD-002".to_string(), strings(&["HLR-1", "SYS-1"]))
    );
    let err = stderr(&o);
    assert!(
        err.contains("invalid: HLR-99: not found in strictdoc (strictdoc)"),
        "stderr: {err}"
    );
    assert!(
        err.contains("invalid: 1 of 3 requirement IDs"),
        "stderr: {err}"
    );
    assert!(!stdout(&o).contains("HLR-99"), "stdout: {}", stdout(&o));
}

#[test]
fn export_ids_every_id_invalid_exits_1_with_an_empty_list() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-98", "LLR-99"]);
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(
        ids_of(&stdout(&o)),
        ("PROD-002".to_string(), Vec::<String>::new())
    );
    assert!(
        stderr(&o).contains("invalid: 2 of 2 requirement IDs"),
        "stderr: {}",
        stderr(&o)
    );
}

#[test]
fn export_ids_an_id_of_each_level_is_valid_and_ids_not_in_the_export_are_not() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(
        work.path(),
        "req.json",
        "PROD-002",
        &["SYS-1", "SYS-99", "HLR-1", "HLR-99", "LLR-1", "LLR-99"],
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(ids_of(&stdout(&o)).1, strings(&["SYS-1", "HLR-1", "LLR-1"]));
    assert!(
        stderr(&o).contains("invalid: 3 of 6 requirement IDs"),
        "stderr: {}",
        stderr(&o)
    );
}

#[test]
fn export_ids_a_repeated_id_is_checked_and_reported_once() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(
        work.path(),
        "req.json",
        "PROD-002",
        &["HLR-99", "HLR-1", "HLR-99", "HLR-1"],
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(ids_of(&stdout(&o)).1, strings(&["HLR-1"]));
    let err = stderr(&o);
    assert_eq!(err.matches("invalid: HLR-99").count(), 1, "stderr: {err}");
    assert!(
        err.contains("invalid: 1 of 2 requirement IDs"),
        "stderr: {err}"
    );
}

// ---------------------------------------------------------------- --output

#[test]
fn export_ids_output_option_writes_the_file_and_leaves_stdout_empty() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-1", "LLR-1"]);
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let written = std::fs::read_to_string(&out).unwrap();
    assert_eq!(
        ids_of(&written),
        ("PROD-002".to_string(), strings(&["HLR-1", "LLR-1"]))
    );
}

#[test]
fn export_ids_output_file_has_the_valid_ids_on_exit_1() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-1", "HLR-99"]);
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert_eq!(
        ids_of(&std::fs::read_to_string(&out).unwrap()).1,
        strings(&["HLR-1"])
    );
    assert!(
        stderr(&o).contains("invalid: HLR-99"),
        "stderr: {}",
        stderr(&o)
    );
}

#[test]
fn export_ids_does_not_change_the_input_file_or_the_export() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(
        work.path(),
        "req.json",
        "PROD-002",
        &["HLR-1", "HLR-99", "SYS-1"],
    );
    let fix = fix_locate();
    let req_before = std::fs::read(&req).unwrap();
    let fix_before = std::fs::read(export("locate")).unwrap();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(1), "stderr: {}", stderr(&o));
    assert_eq!(std::fs::read(&req).unwrap(), req_before);
    assert_eq!(std::fs::read(export("locate")).unwrap(), fix_before);
}

// ---------------------------------------------------------------- exit 2: refusals, nothing written

#[test]
fn export_ids_product_mismatch_exits_2_and_writes_nothing() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-999", &["HLR-1"]);
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(
        stderr(&o).contains(
            "product_id 'PROD-999' does not match product_id 'PROD-002' of the integration config"
        ),
        "stderr: {}",
        stderr(&o)
    );
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_unknown_field_inside_a_requirement_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = raw(
        work.path(),
        "req.json",
        r#"{"product_id":"PROD-002","requirements":[{"id":"HLR-1","title":"x"}]}"#,
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("title"), "stderr: {}", stderr(&o));
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_unknown_top_level_field_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = raw(
        work.path(),
        "req.json",
        r#"{"product_id":"PROD-002","requirements":[{"id":"HLR-1"}],"extra":1}"#,
    );
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("extra"), "stderr: {}", stderr(&o));
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_unparseable_requirements_file_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = raw(work.path(), "req.json", "{not json");
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(
        stderr(&o).contains("parse requirements file"),
        "stderr: {}",
        stderr(&o)
    );
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_missing_requirements_json_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let fix = fix_locate();
    let o = export_ids(work.path(), cfg.path(), &["--input", &fix]);
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    let err = stderr(&o);
    assert!(err.contains("read requirements file"), "stderr: {err}");
    assert!(err.contains("requirements.json"), "stderr: {err}");
}

#[test]
fn export_ids_strictdoc_id_without_input_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-1"]);
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "");
    assert!(stderr(&o).contains("--input"), "stderr: {}", stderr(&o));
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_unknown_id_format_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-1", "PAY-1"]);
    let fix = fix_locate();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &["--requirements", &path_str(&req), "--input", &fix],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    assert!(stderr(&o).contains("PAY-1"), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), "", "HLR-1 is valid but nothing may be written");
}

#[test]
fn export_ids_with_a_refused_export_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-8"]);
    let fix = fix_refused();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("HLR-8"), "stderr: {err}");
    assert!(err.contains("is not a requirement type"), "stderr: {err}");
    assert!(!out.exists(), "a file was written on exit 2");
}

#[test]
fn export_ids_with_an_item_without_level_exits_2() {
    let cfg = config_dir();
    let work = tempfile::tempdir().unwrap();
    let outdir = tempfile::tempdir().unwrap();
    let out = outdir.path().join("valid.json");
    let req = req_file(work.path(), "req.json", "PROD-002", &["HLR-9"]);
    let fix = fix_no_level();
    let o = export_ids(
        work.path(),
        cfg.path(),
        &[
            "--requirements",
            &path_str(&req),
            "--input",
            &fix,
            "--output",
            &path_str(&out),
        ],
    );
    assert_eq!(o.status.code(), Some(2), "stderr: {}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("HLR-9"), "stderr: {err}");
    assert!(err.contains("has no LEVEL"), "stderr: {err}");
    assert!(!out.exists(), "a file was written on exit 2");
}
