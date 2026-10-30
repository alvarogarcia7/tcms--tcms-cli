use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory under the schemas root that holds the pinned copy of tcms-schemas
/// (see `schemas/tcms-schemas/TCMS_SCHEMAS_VERSION` and `scripts/sync-tcms-schemas.sh`)
pub const VENDORED_SCHEMAS_DIR: &str = "tcms-schemas";

/// Envelope structure for extracting schema field from YAML/JSON documents
#[derive(Debug, Deserialize)]
pub struct Envelope {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub schema: Option<String>,
}

/// Resolve schema from payload by reading the `schema` field and mapping to local path
///
/// Canonical schema paths resolved:
/// - Legacy schemas: schemas/*.schema.json
/// - Versioned schemas: schemas/tcms/*.schema.v1.json
///
/// # Arguments
/// * `file_path` - Path to the YAML or JSON file
/// * `schemas_root` - Root directory containing schema files (default: "schemas/")
///
/// # Returns
/// * `Ok(PathBuf)` - Resolved schema file path
/// * `Err` - If schema field is missing or schema file doesn't exist
pub fn resolve_schema_from_payload<P: AsRef<Path>>(
    file_path: P,
    schemas_root: &str,
) -> Result<PathBuf> {
    let file_path = file_path.as_ref();
    let content = fs::read_to_string(file_path)
        .context(format!("Failed to read file: {}", file_path.display()))?;

    // Determine file type and parse accordingly
    let envelope: Envelope = if file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false)
    {
        serde_json::from_str(&content).context(format!(
            "Failed to parse JSON file for schema resolution: {}",
            file_path.display()
        ))?
    } else {
        serde_yaml::from_str(&content).context(format!(
            "Failed to parse YAML file for schema resolution: {}",
            file_path.display()
        ))?
    };

    // Extract schema field
    let schema_uri = envelope.schema.ok_or_else(|| {
        anyhow::anyhow!("Missing 'schema' field in file: {}", file_path.display())
    })?;

    // Resolve to local path
    let schema_path = resolve_schema_uri(&schema_uri, schemas_root)?;

    // Verify schema file exists
    if !schema_path.exists() {
        anyhow::bail!(
            "Schema file not found: {} (resolved from URI: '{}')",
            schema_path.display(),
            schema_uri
        );
    }

    log::debug!(
        "Resolved schema '{}' to '{}'",
        schema_uri,
        schema_path.display()
    );

    Ok(schema_path)
}

/// The versioned URIs that documents carry, and the vendored tcms-schemas file each one is an alias of
pub const SCHEMA_URI_ALIASES: [(&str, &str); 3] = [
    (
        "tcms/test-case.schema.v1.json",
        "tcms/test-case-main.schema.json",
    ),
    (
        "tcms/test-execution.schema.v1.json",
        "tcms/execution-log.schema.json",
    ),
    (
        "tcms/test-verification.schema.v1.json",
        "tcms/verification-result.schema.json",
    ),
];

/// The `type` each versioned URI requires in the document (the CLI schemas used to enforce it)
const ALIAS_TYPES: [(&str, &str); 3] = [
    ("tcms/test-case.schema.v1.json", "test_case"),
    ("tcms/test-execution.schema.v1.json", "test_execution"),
    ("tcms/test-verification.schema.v1.json", "test_verification"),
];

/// Envelope violations of a document that selects one of the versioned schemas: the vendored
/// tcms-schemas are envelope-free, so the CLI checks that `type` is present and matches
/// (the schemas it replaced had `type` required with a constant)
pub fn envelope_violations(document: &serde_json::Value) -> Vec<String> {
    let Some(uri) = document.get("schema").and_then(|v| v.as_str()) else {
        return Vec::new();
    };
    let Some((_, expected)) = ALIAS_TYPES.iter().find(|(u, _)| *u == uri) else {
        return Vec::new();
    };
    match document.get("type") {
        None => vec![format!(
            "'type' is a required property (expected \"{expected}\" for schema {uri})"
        )],
        Some(serde_json::Value::String(t)) if t == expected => Vec::new(),
        Some(other) => vec![format!(
            "{other} is not the type of schema {uri} (expected \"{expected}\")"
        )],
    }
}

/// The vendored file (relative to `schemas/tcms-schemas/`) that a versioned URI is an alias of
pub fn vendored_alias(schema_uri: &str) -> Option<&'static str> {
    SCHEMA_URI_ALIASES
        .iter()
        .find(|(uri, _)| *uri == schema_uri)
        .map(|(_, vendored)| *vendored)
}

/// Resolve schema URI to local file path
///
/// # Arguments
/// * `schema_uri` - Schema URI from the document (e.g., "tcms/test-case.schema.v1.json" or,
///   for the vendored tcms-schemas, "tcms/test-run.schema.json")
/// * `schemas_root` - Root directory for schemas
///
/// # Returns
/// * `Ok(PathBuf)` - Local schema file path
pub fn resolve_schema_uri(schema_uri: &str, schemas_root: &str) -> Result<PathBuf> {
    // The versioned URIs of the documents are aliases of the vendored tcms-schemas files: the CLI
    // has no copy of these three schemas of its own any more (TCMS-40.1). A file that exists at the
    // URI under the given root (a schemas root of its own) still wins.
    if let Some(vendored_uri) = vendored_alias(schema_uri) {
        let own = PathBuf::from(schemas_root).join(schema_uri);
        if own.exists() {
            return Ok(own);
        }
        return Ok(PathBuf::from(schemas_root)
            .join(VENDORED_SCHEMAS_DIR)
            .join(vendored_uri));
    }

    // Schema URI is expected to be a relative path like "tcms/test-run.schema.json"
    // We join it with the schemas root
    let mut schema_path = PathBuf::from(schemas_root).join(schema_uri);

    // URIs of the vendored tcms-schemas copy ("tcms/test-run.schema.json") live under
    // <root>/tcms-schemas/; the CLI's own schemas keep priority
    if !schema_path.exists() {
        let vendored = PathBuf::from(schemas_root)
            .join(VENDORED_SCHEMAS_DIR)
            .join(schema_uri);
        if vendored.exists() {
            schema_path = vendored;
        }
    }

    log::debug!(
        "Resolving schema URI '{}' with root '{}' to '{}'",
        schema_uri,
        schemas_root,
        schema_path.display()
    );

    Ok(schema_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_schema_from_yaml_payload() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_file = temp_dir.path().join("test.yaml");
        let schemas_dir = temp_dir.path().join("schemas");
        let tcms_dir = schemas_dir.join("tcms");
        std::fs::create_dir_all(&tcms_dir).unwrap();

        let schema_file = tcms_dir.join("test-case.schema.v1.json");
        File::create(&schema_file)
            .unwrap()
            .write_all(b"{}")
            .unwrap();

        let yaml_content = r#"
type: test_case
schema: tcms/test-case.schema.v1.json
id: TEST_001
description: Test case
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        let result = resolve_schema_from_payload(&yaml_file, schemas_dir.to_str().unwrap());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved, schema_file);
    }

    #[test]
    fn test_resolve_schema_from_json_payload() {
        let temp_dir = TempDir::new().unwrap();
        let json_file = temp_dir.path().join("test.json");
        let schemas_dir = temp_dir.path().join("schemas");
        let tcms_dir = schemas_dir.join("tcms");
        std::fs::create_dir_all(&tcms_dir).unwrap();

        let schema_file = tcms_dir.join("test-result.schema.v1.json");
        File::create(&schema_file)
            .unwrap()
            .write_all(b"{}")
            .unwrap();

        let json_content = r#"{
  "type": "test_result",
  "schema": "tcms/test-result.schema.v1.json",
  "id": "RESULT_001"
}"#;
        File::create(&json_file)
            .unwrap()
            .write_all(json_content.as_bytes())
            .unwrap();

        let result = resolve_schema_from_payload(&json_file, schemas_dir.to_str().unwrap());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved, schema_file);
    }

    #[test]
    fn test_missing_schema_field() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_file = temp_dir.path().join("test.yaml");

        let yaml_content = r#"
type: test_case
id: TEST_001
description: Test case without schema field
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        let result = resolve_schema_from_payload(&yaml_file, "schemas/");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing 'schema' field"));
    }

    #[test]
    fn test_schema_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let yaml_file = temp_dir.path().join("test.yaml");

        let yaml_content = r#"
type: test_case
schema: tcms/nonexistent.schema.v1.json
id: TEST_001
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        let result = resolve_schema_from_payload(
            &yaml_file,
            temp_dir.path().join("schemas").to_str().unwrap(),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Schema file not found"));
    }

    #[test]
    fn test_resolve_schema_uri_falls_back_to_vendored_tcms_schemas() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let vendored = root.join(VENDORED_SCHEMAS_DIR).join("tcms");
        std::fs::create_dir_all(&vendored).unwrap();
        std::fs::write(vendored.join("test-run.schema.json"), b"{}").unwrap();

        let resolved =
            resolve_schema_uri("tcms/test-run.schema.json", root.to_str().unwrap()).unwrap();
        assert_eq!(resolved, vendored.join("test-run.schema.json"));
    }

    #[test]
    fn test_resolve_schema_uri_prefers_cli_schema_over_vendored() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        for dir in [
            root.join("tcms"),
            root.join(VENDORED_SCHEMAS_DIR).join("tcms"),
        ] {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("review.schema.json"), b"{}").unwrap();
        }

        let resolved =
            resolve_schema_uri("tcms/review.schema.json", root.to_str().unwrap()).unwrap();
        assert_eq!(resolved, root.join("tcms/review.schema.json"));
    }

    #[test]
    fn test_resolve_schema_uri_missing_keeps_primary_path_for_the_error() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let resolved = resolve_schema_uri("tcms/nope.schema.json", root.to_str().unwrap()).unwrap();
        assert_eq!(resolved, root.join("tcms/nope.schema.json"));
    }

    #[test]
    fn versioned_uris_are_aliases_of_the_vendored_schemas() {
        let root = TempDir::new().unwrap();
        let root_str = root.path().to_str().unwrap();
        for (uri, vendored) in SCHEMA_URI_ALIASES {
            assert_eq!(
                resolve_schema_uri(uri, root_str).unwrap(),
                root.path().join("tcms-schemas").join(vendored),
                "{uri}"
            );
        }
    }

    #[test]
    fn a_file_at_the_uri_under_the_given_root_wins_over_the_alias() {
        let root = TempDir::new().unwrap();
        std::fs::create_dir_all(root.path().join("tcms")).unwrap();
        let own = root.path().join("tcms/test-case.schema.v1.json");
        File::create(&own).unwrap().write_all(b"{}").unwrap();
        assert_eq!(
            resolve_schema_uri(
                "tcms/test-case.schema.v1.json",
                root.path().to_str().unwrap()
            )
            .unwrap(),
            own
        );
    }

    #[test]
    fn envelope_violations_need_the_type_of_the_versioned_schema() {
        use serde_json::json;
        let ok = json!({"type": "test_case", "schema": "tcms/test-case.schema.v1.json"});
        assert!(envelope_violations(&ok).is_empty());
        let missing = json!({"schema": "tcms/test-case.schema.v1.json"});
        assert!(envelope_violations(&missing)[0].contains("'type' is a required property"));
        let wrong = json!({"type": "test_run", "schema": "tcms/test-case.schema.v1.json"});
        assert!(envelope_violations(&wrong)[0].contains("expected \"test_case\""));
        // other schemas have no such rule here (the vendored ones are envelope-free)
        let other = json!({"schema": "tcms/test-run.schema.json"});
        assert!(envelope_violations(&other).is_empty());
        assert!(envelope_violations(&json!({"id": "x"})).is_empty());
    }
}
