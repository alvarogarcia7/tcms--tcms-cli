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
    // Real constraints, so that --verify has something to check.
    let schema_bodies = [
        (
            "test-run",
            r#"{"type":"object","required":["run_id"],"properties":{"run_id":{"type":"string"}}}"#,
        ),
        (
            "test-case-main",
            r#"{"type":"object","required":["test_case_id"],"properties":{"test_case_id":{"type":"string"}}}"#,
        ),
        ("trace-matrix", r#"{"type":"object","required":["id"]}"#),
    ];
    for (name, body) in schema_bodies {
        fs::write(vendored.join(format!("{name}.schema.json")), body).unwrap();
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

fn verify_lines(out: &Output) -> Vec<serde_json::Value> {
    stdout(out)
        .lines()
        .map(|l| serde_json::from_str(l).expect("each stdout line is JSON"))
        .collect()
}

#[test]
fn without_verify_the_output_has_no_verification_fields() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--output=jsonl"]);
    for line in verify_lines(&out) {
        assert!(line.get("verified").is_none(), "{line}");
        assert!(line.get("errors").is_none(), "{line}");
    }
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn verify_reports_a_valid_document_as_verified_with_no_errors() {
    let (_dir, repo, schemas) = fixture();
    fs::write(
        repo.join("runs/only-valid.yaml"),
        "schema: tcms/test-run.schema.json\nrun_id: R2\n",
    )
    .unwrap();
    let out = run(
        &repo,
        &schemas,
        &["--verify", "--output=jsonl", "--only", "test-run"],
    );
    let lines = verify_lines(&out);
    let valid = lines
        .iter()
        .find(|l| l["path"] == "runs/only-valid.yaml")
        .unwrap();
    assert_eq!(valid["verified"], true);
    assert_eq!(valid["errors"], serde_json::json!([]));
}

#[test]
fn verify_reports_error_pointers_and_root_violations() {
    let (_dir, repo, schemas) = fixture();
    fs::write(
        repo.join("runs/bad-run.yaml"),
        "schema: tcms/test-run.schema.json\nrun_id: 42\n",
    )
    .unwrap();
    let out = run(&repo, &schemas, &["--verify", "--output=jsonl"]);
    let lines = verify_lines(&out);

    let bad = lines
        .iter()
        .find(|l| l["path"] == "runs/bad-run.yaml")
        .unwrap();
    assert_eq!(bad["verified"], false);
    let errors = bad["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].as_str().unwrap().starts_with("/run_id: "),
        "{errors:?}"
    );

    // trace-matrix.json has no `id`: the violation is at the document root.
    let matrix = lines.iter().find(|l| l["path"] == "matrix.json").unwrap();
    assert_eq!(matrix["verified"], false);
    assert!(matrix["errors"][0]
        .as_str()
        .unwrap()
        .starts_with("(root): "));
}

#[test]
fn verify_failure_exits_3_and_still_lists_every_match() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--verify", "--output=jsonl"]);
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(verify_lines(&out).len(), 3);
}

#[test]
fn verify_with_only_valid_matches_exits_0() {
    let (_dir, repo, schemas) = fixture();
    let out = run(
        &repo,
        &schemas,
        &["--verify", "--output=jsonl", "--only", "test-run"],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn verify_plaintext_shows_status_and_indented_errors() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--verify"]);
    let text = stdout(&out);
    assert!(
        text.contains(
            "runs/test-run.yaml  type: test_run  schema: tcms/test-run.schema.json  verified: ok\n"
        ),
        "{text}"
    );
    assert!(text.contains("matrix.json  type: -  schema: tcms/trace-matrix.schema.json  verified: FAILED (1 errors)\n    (root): "), "{text}");
}

#[test]
fn a_schema_that_fails_to_compile_is_an_error_naming_the_file() {
    let (_dir, repo, schemas) = fixture();
    let vendored = schemas.join("tcms-schemas").join("tcms");
    fs::write(vendored.join("bad.schema.json"), r#"{"type": 12}"#).unwrap();
    fs::write(repo.join("bad.yaml"), "schema: tcms/bad.schema.json\n").unwrap();
    let out = run(&repo, &schemas, &["--verify"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cannot compile schema") && err.contains("bad.schema.json"),
        "{err}"
    );
}

#[test]
fn verified_jsonl_validates_against_the_published_schema() {
    let (_dir, repo, schemas) = fixture();
    let out = run(&repo, &schemas, &["--verify", "--output=jsonl"]);
    let schema_text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("schema")
            .join("find-files-by-schema-match.schema.json"),
    )
    .unwrap();
    let schema: serde_json::Value = serde_json::from_str(&schema_text).unwrap();
    let compiled = jsonschema::JSONSchema::compile(&schema).expect("schema compiles");
    let lines = verify_lines(&out);
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(compiled.is_valid(line), "invalid line: {line}");
    }
}
