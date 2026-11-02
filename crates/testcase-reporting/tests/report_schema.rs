//! Schema tests for `schemas/tcms/report.schema.v1.json` (OUT-02, D-03).
//!
//! Hand-written documents only: the schema is checked on its own, before any Rust type is
//! involved. Each rejection is its own test so a failure names the violated rule.

use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};
use std::path::PathBuf;

const SCHEMA_URI: &str = "tcms/report.schema.v1.json";

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative)
}

fn read_json(path: &PathBuf) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn report_schema() -> Value {
    read_json(&repo_file("schemas/tcms/report.schema.v1.json"))
}

fn compiled() -> JSONSchema {
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&report_schema())
        .expect("report schema compiles as Draft-07")
}

fn assert_accepted(doc: &Value) {
    let schema = compiled();
    let messages: Vec<String> = match schema.validate(doc) {
        Ok(()) => return,
        Err(errors) => errors.map(|e| format!("{e}")).collect(),
    };
    panic!("expected the document to be accepted, got: {messages:#?}");
}

fn assert_rejected(doc: &Value) {
    assert!(
        !compiled().is_valid(doc),
        "expected the document to be rejected: {doc}"
    );
}

/// Navigate to the object that holds the last key of a JSON pointer.
fn parent_mut<'a>(doc: &'a mut Value, pointer: &str) -> (&'a mut Value, String) {
    let (parent, key) = pointer
        .rsplit_once('/')
        .unwrap_or_else(|| panic!("bad pointer {pointer}"));
    let parent_value = if parent.is_empty() {
        doc
    } else {
        doc.pointer_mut(parent)
            .unwrap_or_else(|| panic!("no value at {parent}"))
    };
    (parent_value, key.to_string())
}

fn set(doc: &mut Value, pointer: &str, value: Value) {
    let (parent, key) = parent_mut(doc, pointer);
    parent
        .as_object_mut()
        .unwrap_or_else(|| panic!("{pointer}: parent is not an object"))
        .insert(key, value);
}

fn remove(doc: &mut Value, pointer: &str) {
    let (parent, key) = parent_mut(doc, pointer);
    let removed = parent
        .as_object_mut()
        .unwrap_or_else(|| panic!("{pointer}: parent is not an object"))
        .remove(&key);
    assert!(removed.is_some(), "{pointer} was not present");
}

fn envelope(kind: &str, header: Value, content: Value) -> Value {
    json!({
        "type": "report",
        "schema": SCHEMA_URI,
        "kind": kind,
        "header": header,
        "content": content,
    })
}

fn empty_header() -> Value {
    json!({
        "matrix_id": "TM-1",
        "baseline_id": null,
        "run_ids": [],
        "build_ids": [],
        "environments": [],
        "testers": [],
        "generated_at": "2026-10-31T09:00:00Z",
        "obsolete_excluded": null,
    })
}

fn full_header() -> Value {
    json!({
        "matrix_id": "TM-1",
        "baseline_id": "BL-7",
        "run_ids": ["RUN-1", "RUN-2"],
        "build_ids": ["b1", "b2"],
        "environments": ["lab-1"],
        "testers": ["alice", "bob"],
        "generated_at": "2026-10-31T09:00:00Z",
        "obsolete_excluded": 2,
    })
}

fn minimal_traceability() -> Value {
    envelope(
        "traceability",
        empty_header(),
        json!({ "documents": [ { "name": null, "requirements": [] } ] }),
    )
}

const STATUS_LABELS: [&str; 6] = [
    "VERIFIED",
    "FAILED",
    "ERRORED",
    "NOT RUN",
    "PARTIAL",
    "NO TESTS",
];

fn requirement(id: &str, level: &str, status: &str, cases: Value) -> Value {
    json!({
        "requirement_id": id,
        "level": level,
        "status": status,
        "normal_range_covered": true,
        "robustness_covered": false,
        "cases": cases,
    })
}

fn full_traceability() -> Value {
    let mut requirements: Vec<Value> = STATUS_LABELS
        .iter()
        .enumerate()
        .map(|(i, status)| {
            requirement(
                &format!("REQ-{i}"),
                if i % 2 == 0 { "HLR" } else { "LLR" },
                status,
                json!([]),
            )
        })
        .collect();
    requirements[0]["cases"] = json!([
        { "test_case_id": "T1", "column": "normal_range", "outcome": "pass", "run_id": "RUN-1" },
        { "test_case_id": "T2", "column": "robustness", "outcome": "fail", "run_id": "RUN-2" },
        { "test_case_id": "T3", "column": "other", "outcome": "error", "run_id": null },
        { "test_case_id": "T4", "column": "other", "outcome": "NOT RUN", "run_id": null },
    ]);
    envelope(
        "traceability",
        full_header(),
        json!({
            "documents": [
                { "name": "docs/srs.sdoc", "requirements": requirements },
                { "name": "docs/empty.sdoc", "requirements": [] },
            ]
        }),
    )
}

fn empty_summary() -> Value {
    let zero = json!({ "pass": 0, "fail": 0, "not_executed": 0, "error": 0, "total": 0 });
    envelope(
        "summary",
        empty_header(),
        json!({ "runs": [], "requirements": [], "linked": zero }),
    )
}

fn full_summary() -> Value {
    envelope(
        "summary",
        full_header(),
        json!({
            "runs": [
                {
                    "run_id": "RUN-1", "build_id": "b1", "used": true, "excluded_reason": null,
                    "counts": { "pass": 1, "fail": 1, "not_executed": 0, "error": 0, "total": 2 }
                },
                {
                    "run_id": "RUN-2", "build_id": "b2", "used": false,
                    "excluded_reason": "superseded by RUN-1",
                    "counts": { "pass": 0, "fail": 0, "not_executed": 1, "error": 0, "total": 1 }
                },
            ],
            "requirements": [
                {
                    "requirement_id": "HLR-1", "level": "HLR", "status": "VERIFIED",
                    "counts": { "pass": 2, "fail": 0, "not_executed": 0, "error": 0, "total": 2 }
                },
            ],
            "linked": { "pass": 1, "fail": 1, "not_executed": 1, "error": 0, "total": 3 },
        }),
    )
}

fn empty_gaps() -> Value {
    envelope(
        "gaps",
        empty_header(),
        json!({
            "no_tests": [],
            "missing_normal_range": [],
            "missing_robustness": [],
            "no_result": [],
        }),
    )
}

fn full_gaps() -> Value {
    envelope(
        "gaps",
        full_header(),
        json!({
            "no_tests": [ { "requirement_id": "HLR-3", "level": "HLR", "document": null } ],
            "missing_normal_range": [
                { "requirement_id": "LLR-2", "level": "LLR", "document": "docs/srs.sdoc" }
            ],
            "missing_robustness": [],
            "no_result": [ { "test_case_id": "T4", "requirement_ids": ["HLR-1", "LLR-2"] } ],
        }),
    )
}

fn empty_problems() -> Value {
    envelope(
        "problems",
        empty_header(),
        json!({ "rows": [], "missing_count": 0 }),
    )
}

fn full_problems() -> Value {
    envelope(
        "problems",
        full_header(),
        json!({
            "rows": [
                {
                    "test_case_id": "T2", "verdict": "fail", "run_id": "RUN-1", "build_id": "b1",
                    "problem_report_id": "PR-1", "requirement_ids": ["HLR-1"]
                },
                {
                    "test_case_id": "T3", "verdict": "error", "run_id": "RUN-1", "build_id": "b1",
                    "problem_report_id": null, "requirement_ids": []
                },
            ],
            "missing_count": 1,
        }),
    )
}

// ---------------------------------------------------------------- compile and accept

#[test]
fn schema_compiles_as_draft07() {
    let schema = report_schema();
    assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
    let _ = compiled();
}

#[test]
fn accepts_minimal_traceability() {
    assert_accepted(&minimal_traceability());
}

#[test]
fn accepts_full_traceability() {
    assert_accepted(&full_traceability());
}

#[test]
fn accepts_empty_summary() {
    assert_accepted(&empty_summary());
}

#[test]
fn accepts_full_summary() {
    assert_accepted(&full_summary());
}

#[test]
fn accepts_empty_gaps() {
    assert_accepted(&empty_gaps());
}

#[test]
fn accepts_full_gaps() {
    assert_accepted(&full_gaps());
}

#[test]
fn accepts_empty_problems() {
    assert_accepted(&empty_problems());
}

#[test]
fn accepts_full_problems() {
    assert_accepted(&full_problems());
}

// ---------------------------------------------------------------- envelope rejections

#[test]
fn rejects_missing_type() {
    let mut doc = minimal_traceability();
    remove(&mut doc, "/type");
    assert_rejected(&doc);
}

#[test]
fn rejects_type_not_report() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/type", json!("test_results_container"));
    assert_rejected(&doc);
}

#[test]
fn rejects_missing_schema() {
    let mut doc = minimal_traceability();
    remove(&mut doc, "/schema");
    assert_rejected(&doc);
}

#[test]
fn rejects_schema_not_const() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/schema", json!("tcms/other.schema.v1.json"));
    assert_rejected(&doc);
}

#[test]
fn rejects_missing_kind() {
    let mut doc = minimal_traceability();
    remove(&mut doc, "/kind");
    assert_rejected(&doc);
}

#[test]
fn rejects_unknown_kind() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/kind", json!("matrix"));
    assert_rejected(&doc);
}

#[test]
fn rejects_traceability_without_content() {
    let mut doc = minimal_traceability();
    remove(&mut doc, "/content");
    assert_rejected(&doc);
}

// ---------------------------------------------------------------- header

macro_rules! header_missing_test {
    ($name:ident, $key:literal) => {
        #[test]
        fn $name() {
            let mut doc = minimal_traceability();
            remove(&mut doc, concat!("/header/", $key));
            assert_rejected(&doc);
        }
    };
}

header_missing_test!(rejects_header_missing_matrix_id, "matrix_id");
header_missing_test!(rejects_header_missing_baseline_id, "baseline_id");
header_missing_test!(rejects_header_missing_run_ids, "run_ids");
header_missing_test!(rejects_header_missing_build_ids, "build_ids");
header_missing_test!(rejects_header_missing_environments, "environments");
header_missing_test!(rejects_header_missing_testers, "testers");
header_missing_test!(rejects_header_missing_generated_at, "generated_at");
header_missing_test!(rejects_header_missing_obsolete_excluded, "obsolete_excluded");

#[test]
fn rejects_header_extra_key() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/header/extra", json!("x"));
    assert_rejected(&doc);
}

// ---------------------------------------------------------------- traceability rows

#[test]
fn rejects_row_unknown_status() {
    let mut doc = full_traceability();
    set(&mut doc, "/content/documents/0/requirements/0/status", json!("DONE"));
    assert_rejected(&doc);
}

#[test]
fn rejects_row_level_mlr() {
    let mut doc = full_traceability();
    set(&mut doc, "/content/documents/0/requirements/0/level", json!("MLR"));
    assert_rejected(&doc);
}

#[test]
fn rejects_case_column_misc() {
    let mut doc = full_traceability();
    set(&mut doc, "/content/documents/0/requirements/0/cases/0/column", json!("misc"));
    assert_rejected(&doc);
}

#[test]
fn rejects_case_outcome_passed() {
    let mut doc = full_traceability();
    set(&mut doc, "/content/documents/0/requirements/0/cases/0/outcome", json!("passed"));
    assert_rejected(&doc);
}

#[test]
fn rejects_top_level_extra_key() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/extra", json!(true));
    assert_rejected(&doc);
}

#[test]
fn rejects_documents_not_array() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/content/documents", json!({}));
    assert_rejected(&doc);
}

#[test]
fn rejects_obsolete_excluded_negative() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/header/obsolete_excluded", json!(-1));
    assert_rejected(&doc);
}

#[test]
fn rejects_run_ids_with_number() {
    let mut doc = minimal_traceability();
    set(&mut doc, "/header/run_ids", json!([1]));
    assert_rejected(&doc);
}

// ---------------------------------------------------------------- summary, gaps, problems

#[test]
fn rejects_summary_row_unknown_status() {
    let mut doc = full_summary();
    set(&mut doc, "/content/requirements/0/status", json!("DONE"));
    assert_rejected(&doc);
}

#[test]
fn rejects_summary_counts_missing_total() {
    let mut doc = full_summary();
    remove(&mut doc, "/content/linked/total");
    assert_rejected(&doc);
}

#[test]
fn rejects_problems_verdict_pass() {
    let mut doc = full_problems();
    set(&mut doc, "/content/rows/0/verdict", json!("pass"));
    assert_rejected(&doc);
}

#[test]
fn rejects_problems_missing_count_negative() {
    let mut doc = full_problems();
    set(&mut doc, "/content/missing_count", json!(-1));
    assert_rejected(&doc);
}

#[test]
fn rejects_gaps_extra_key_in_requirement() {
    let mut doc = full_gaps();
    set(&mut doc, "/content/no_tests/0/extra", json!("x"));
    assert_rejected(&doc);
}

#[test]
fn rejects_problems_kind_with_traceability_content() {
    let doc = envelope("problems", full_header(), full_traceability()["content"].clone());
    assert_rejected(&doc);
}

// ---------------------------------------------------------------- envelope meta-schema

#[test]
fn envelope_accepts_report_type() {
    let envelope_schema = read_json(&repo_file("schemas/tcms-envelope.schema.json"));
    let types = envelope_schema["properties"]["type"]["enum"]
        .as_array()
        .expect("type enum");
    assert!(
        types.iter().any(|t| t == "report"),
        "envelope type enum lacks report"
    );
    let envelope = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&envelope_schema)
        .expect("envelope compiles");
    let pair = json!({ "type": "report", "schema": SCHEMA_URI });
    assert!(envelope.is_valid(&pair), "envelope rejects the report pair");
}
