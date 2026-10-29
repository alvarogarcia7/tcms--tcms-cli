//! The pinned copy of tcms-schemas (schemas/tcms-schemas, see TCMS-40.1 and TCMS-40.9)
//! is reachable through the envelope and its samples validate.

use serde_json::Value as Json;
use std::fs;
use std::path::{Path, PathBuf};
use testcase_common::load_and_validate_yaml;

fn schemas_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn vendored() -> PathBuf {
    schemas_root().join("tcms-schemas")
}

fn read_json(path: &Path) -> Json {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// `test-run` for `tcms/test-run.schema.json`
fn schema_names() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(vendored().join("tcms"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter_map(|f| f.strip_suffix(".schema.json").map(str::to_string))
        .collect();
    names.sort();
    names
}

fn envelope_type(schema_name: &str) -> String {
    schema_name.replace('-', "_")
}

/// Sample folder to schema name (tcms-schemas validator.py SCHEMA_MAP)
fn schema_for_category(category: &str) -> String {
    match category {
        "test-case" => "test-case-main".to_string(),
        other => other.to_string(),
    }
}

fn samples() -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for category in fs::read_dir(vendored().join("samples")).unwrap() {
        let category = category.unwrap();
        let name = category.file_name().to_string_lossy().to_string();
        for file in fs::read_dir(category.path()).unwrap() {
            let path = file.unwrap().path();
            // Like tcms-schemas scripts/validate-schemas.sh: *.yml and *.yaml samples
            let ext = path.extension().and_then(|e| e.to_str());
            if ext == Some("yml") || ext == Some("yaml") {
                out.push((name.clone(), path));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn pinned_version_is_recorded() {
    let version = fs::read_to_string(vendored().join("TCMS_SCHEMAS_VERSION")).unwrap();
    let commit = version
        .lines()
        .find_map(|l| l.strip_prefix("commit: "))
        .expect("commit line");
    assert_eq!(
        commit.len(),
        40,
        "pinned commit must be a full SHA: {commit}"
    );
}

#[test]
fn every_vendored_schema_is_a_valid_draft7_schema_known_to_the_envelope() {
    let envelope = read_json(&schemas_root().join("tcms-envelope.schema.json"));
    let types: Vec<&str> = envelope["properties"]["type"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    let names = schema_names();
    assert!(
        names.len() >= 24,
        "expected the full tcms-schemas set, got {names:?}"
    );
    for name in names {
        let schema = read_json(&vendored().join("tcms").join(format!("{name}.schema.json")));
        assert_eq!(
            schema["$schema"], "http://json-schema.org/draft-07/schema#",
            "{name} is not Draft-07"
        );
        jsonschema::JSONSchema::compile(&schema)
            .unwrap_or_else(|e| panic!("{name} does not compile: {e}"));
        assert!(
            types.contains(&envelope_type(&name).as_str()),
            "envelope type enum lacks {}",
            envelope_type(&name)
        );
    }
}

#[test]
fn envelope_accepts_unversioned_tcms_schemas_uris_and_the_versioned_ones() {
    let mut envelope = read_json(&schemas_root().join("tcms-envelope.schema.json"));
    // The relative $id cannot be compiled standalone; the loader registers it by id instead
    envelope["$id"] =
        serde_json::json!("https://tcms.example.com/schemas/tcms-envelope.schema.json");
    let compiled = jsonschema::JSONSchema::compile(&envelope).unwrap();

    for name in schema_names() {
        let doc = serde_json::json!({
            "type": envelope_type(&name),
            "schema": format!("tcms/{name}.schema.json"),
        });
        assert!(compiled.is_valid(&doc), "envelope rejects {doc}");
    }
    // The CLI's own versioned URIs keep working
    let legacy =
        serde_json::json!({"type": "test_case", "schema": "tcms/test-case.schema.v1.json"});
    assert!(compiled.is_valid(&legacy));
    // Unknown type or malformed URI still fail
    assert!(
        !compiled.is_valid(&serde_json::json!({"type": "nope", "schema": "tcms/x.schema.json"}))
    );
    assert!(
        !compiled.is_valid(&serde_json::json!({"type": "test_run", "schema": "elsewhere/x.json"}))
    );
}

#[test]
fn all_tcms_schemas_samples_validate_through_their_envelope() {
    let samples = samples();
    assert!(samples.len() >= 20, "expected the tcms-schemas samples");

    let dir = tempfile::tempdir().unwrap();
    for (category, path) in samples {
        let name = schema_for_category(&category);
        let body = fs::read_to_string(&path).unwrap();
        // The samples carry no envelope: add it, as a user would. Entities with their own
        // domain `type` (requirement, design-element, ...) get only `schema`, which is
        // all that auto-resolution reads.
        let has_own_type = body.lines().any(|l| l.starts_with("type:"));
        let type_line = if has_own_type {
            String::new()
        } else {
            format!("type: {}\n", envelope_type(&name))
        };
        let doc = format!("{type_line}schema: tcms/{name}.schema.json\n{body}");
        let file = dir.path().join(path.file_name().unwrap());
        fs::write(&file, doc).unwrap();

        load_and_validate_yaml::<serde_yaml::Value, _, _>(&file, schemas_root().to_str().unwrap())
            .unwrap_or_else(|e| panic!("{} failed: {e:#}", path.display()));
    }
}

#[test]
fn unknown_schema_still_errors() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.yaml");
    fs::write(
        &file,
        "type: nope\nschema: tcms/not-there.schema.json\nid: 1\n",
    )
    .unwrap();
    let err =
        load_and_validate_yaml::<serde_yaml::Value, _, _>(&file, schemas_root().to_str().unwrap())
            .unwrap_err();
    assert!(format!("{err:#}").contains("not-there"), "{err:#}");
}

#[test]
fn an_invalid_document_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("bad.yaml");
    // a review needs more than an id
    fs::write(
        &file,
        "type: review\nschema: tcms/review.schema.json\nid: 1\n",
    )
    .unwrap();
    assert!(load_and_validate_yaml::<serde_yaml::Value, _, _>(
        &file,
        schemas_root().to_str().unwrap()
    )
    .is_err());
}
