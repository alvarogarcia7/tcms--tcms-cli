//! `test-executor independence-assignment create|check` (TCMS-40.8).

use std::path::Path;
use std::process::{Command, Output};

fn executor(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_test-executor"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(["--no-audit", "independence-assignment"])
        .args(args)
        .output()
        .unwrap()
}

fn create(author: &str, verifier: &str, out: &Path) -> Output {
    executor(&[
        "create",
        "--id",
        "IA-1",
        "--objective",
        "Results reviewed by someone else",
        "--artifact",
        "RUN-1",
        "--author",
        author,
        "--verifier",
        verifier,
        "--date",
        "2026-10-28",
        "--output",
        &out.display().to_string(),
    ])
}

#[test]
fn created_records_are_independent_only_for_another_verifier_and_pass_the_check() {
    let dir = tempfile::tempdir().unwrap();
    let independent = dir.path().join("ia1.yaml");
    assert!(create("alice", "bob", &independent).status.success());
    let text = std::fs::read_to_string(&independent).unwrap();
    assert!(text.contains("verifier_is_independent: true"), "{text}");
    assert!(
        text.contains("date: \"2026-10-28\""),
        "dates stay strings: {text}"
    );

    let own = dir.path().join("ia2.yaml");
    let output = create("alice", "ALICE", &own);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not independent"));
    assert!(std::fs::read_to_string(&own)
        .unwrap()
        .contains("verifier_is_independent: false"));

    // both are honest records and validate
    let check = executor(&[
        "check",
        &independent.display().to_string(),
        &own.display().to_string(),
    ]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn a_record_claiming_independence_for_the_author_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.yaml");
    std::fs::write(
        &file,
        "id: IA-9\nobjective: o\nartifact: RUN-1\nauthor: alice\nverifier: alice\n\
         verifier_is_independent: true\n",
    )
    .unwrap();
    let output = executor(&["check", &file.display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("IA-9") && stderr.contains("is the author"),
        "{stderr}"
    );
}

#[test]
fn a_record_that_breaks_the_schema_is_rejected_and_the_tcms_schemas_sample_passes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("incomplete.yaml");
    std::fs::write(&file, "id: IA-8\nauthor: alice\n").unwrap();
    let output = executor(&["check", &file.display().to_string()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("required property"));

    let sample = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/tcms-schemas/samples/independence-assignment/ia_001.yml");
    let output = executor(&["check", &sample.display().to_string()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
