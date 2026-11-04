//! End-to-end tests of the `find-files-by-schema` binary on a fixture repository.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const BIN: &str = env!("CARGO_BIN_EXE_find-files-by-schema");

/// A schemas root with three vendored types, and a repo holding documents of each kind.
fn fixture() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let schemas = dir.path().join("schemas");
    let vendored = schemas.join("tcms-schemas").join("tcms");
    fs::create_dir_all(&vendored).unwrap();
    for name in ["test-run", "test-case-main", "trace-matrix"] {
        fs::write(vendored.join(format!("{name}.schema.json")), "{}").unwrap();
    }
    let repo = dir.path().join("repo");
    fs::create_dir_all(repo.join("runs")).unwrap();
    fs::create_dir_all(repo.join("cases")).unwrap();
    fs::create_dir_all(repo.join("ignored")).unwrap();
    fs::write(repo.join(".gitignore"), "ignored/\n").unwrap();
    fs::write(
        repo.join("runs/test-run.yaml"),
        "type: test_run\nschema: tcms/test-run.schema.json\nrun_id: R1\n",
    )
    .unwrap();
    fs::write(
        repo.join("cases/tc.yaml"),
        "type: test-case\nschema: tcms/test-case.schema.v1.json\ntest_case_id: TC_1\n",
    )
    .unwrap();
    fs::write(
        repo.join("matrix.json"),
        r#"{"schema": "tcms/trace-matrix.schema.json"}"#,
    )
    .unwrap();
    fs::write(
        repo.join("cases/unknown.yaml"),
        "schema: tcms/not-a-type.schema.json\ntype: x\n",
    )
    .unwrap();
    fs::write(repo.join("cases/plain.yaml"), "title: no schema here\n").unwrap();
    fs::write(repo.join("cases/broken.yaml"), "schema: [unclosed\n").unwrap();
    fs::write(
        repo.join("ignored/hidden.yaml"),
        "type: test_run\nschema: tcms/test-run.schema.json\n",
    )
    .unwrap();
    (dir, repo, schemas)
}

fn run(repo: &Path, schemas: &Path, extra: &[&str]) -> Output {
    Command::new(BIN)
        .arg("--root")
        .arg(repo)
        .arg("--schemas-root")
        .arg(schemas)
        .args(extra)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

#[test]
fn plaintext_is_the_default_and_lists_matches_sorted() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout(&out),
        "cases/tc.yaml  type: test-case  schema: tcms/test-case-main.schema.json\n\
         matrix.json  type: -  schema: tcms/trace-matrix.schema.json\n\
         runs/test-run.yaml  type: test_run  schema: tcms/test-run.schema.json\n"
    );
}

#[test]
fn jsonl_prints_one_object_per_match_and_nothing_else_on_stdout() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--output=jsonl"]);
    assert!(out.status.success());
    let lines: Vec<serde_json::Value> = stdout(&out)
        .lines()
        .map(|l| serde_json::from_str(l).expect("each stdout line is JSON"))
        .collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0]["path"], "cases/tc.yaml");
    assert_eq!(lines[0]["schema"], "tcms/test-case-main.schema.json");
    assert_eq!(lines[0]["type"], "test-case");
    assert_eq!(lines[1]["path"], "matrix.json");
    assert!(lines[1]["type"].is_null());
    assert_eq!(lines[2]["path"], "runs/test-run.yaml");
}

#[test]
fn only_restricts_matches_to_the_named_types() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--output=jsonl", "--only", "test-run"]);
    assert!(out.status.success());
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 1);
    assert!(text.contains("runs/test-run.yaml"));

    let out = run(
        &repo,
        &schemas,
        &["--only", "test-run", "--only", "trace-matrix"],
    );
    assert_eq!(stdout(&out).lines().count(), 2);
}

#[test]
fn unknown_only_type_is_a_usage_error_listing_valid_types() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--only", "bogus"]);
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown type 'bogus'"), "{err}");
    assert!(err.contains("test-run"), "{err}");
    assert!(stdout(&out).is_empty());
}

#[test]
fn malformed_files_are_skipped_with_a_warning() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &[]);
    assert!(out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("warning: skipping"), "{err}");
    assert!(err.contains("broken.yaml"), "{err}");
}

#[test]
fn gitignored_files_are_not_scanned() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &[]);
    assert!(!stdout(&out).contains("hidden.yaml"));
}

#[test]
fn zero_matches_is_a_successful_scan() {
    let dir = TempDir::new().unwrap();
    let (_fixture, _repo, schemas) = fixture();
    fs::write(dir.path().join("readme.yaml"), "title: nothing\n").unwrap();
    let out = run(dir.path(), &schemas, &["--output=jsonl"]);
    assert!(out.status.success());
    assert!(stdout(&out).is_empty());
}

#[test]
fn a_missing_schemas_root_is_an_error() {
    let (_dir, repo, _schemas) = fixture();
    let out = run(&repo, Path::new("/nonexistent/schemas"), &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read tcms-schemas"));
}

#[test]
fn jsonl_output_validates_against_the_published_schema() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--output=jsonl"]);
    assert!(out.status.success());

    let schema_text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("schema")
            .join("find-files-by-schema-match.schema.json"),
    )
    .unwrap();
    let schema: serde_json::Value = serde_json::from_str(&schema_text).unwrap();
    let compiled = jsonschema::JSONSchema::compile(&schema).expect("schema compiles");

    let mut checked = 0;
    for line in stdout(&out).lines() {
        let instance: serde_json::Value = serde_json::from_str(line).unwrap();
        assert!(compiled.is_valid(&instance), "invalid line: {line}");
        checked += 1;
    }
    assert_eq!(checked, 3);
}
