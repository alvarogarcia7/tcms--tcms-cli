use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::Path;

use crate::envelope::resolve_schema_from_payload;
use crate::yaml_utils::log_yaml_parse_error;

/// Load and validate a YAML file, then deserialize to type T
///
/// # Arguments
///
/// * `file_path` - Path to the YAML file to load
/// * `schemas_root` - Root directory containing schema files for validation
///
/// # Returns
///
/// Returns `Ok(T)` with the deserialized object if successful, or an error if:
/// - The file cannot be read
/// - Schema resolution fails
/// - Schema validation fails
/// - YAML deserialization fails
///
/// # Example
///
/// ```no_run
/// use serde::Deserialize;
/// use testcase_common::load_and_validate_yaml;
///
/// #[derive(Deserialize)]
/// struct TestCase {
///     id: String,
///     description: String,
/// }
///
/// let test_case: TestCase = load_and_validate_yaml("test.yaml", "schemas/").unwrap();
/// ```
pub fn load_and_validate_yaml<T, P, S>(file_path: P, schemas_root: S) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let file_path = file_path.as_ref();
    let schemas_root = schemas_root.as_ref();

    // Step 1: Read the YAML file
    let yaml_content = fs::read_to_string(file_path)
        .context(format!("Failed to read YAML file: {}", file_path.display()))?;

    // Step 2: Resolve schema from the payload's type field
    let schema_path = resolve_schema_from_payload(file_path, schemas_root).context(format!(
        "Failed to resolve schema for file: {}",
        file_path.display()
    ))?;

    log::debug!(
        "Resolved schema '{}' for file '{}'",
        schema_path.display(),
        file_path.display()
    );

    // Step 3: Validate against the schema
    validate_yaml_against_schema(&yaml_content, &schema_path, &file_path.to_string_lossy())
        .context(format!(
            "Schema validation failed for file: {}",
            file_path.display()
        ))?;

    log::debug!("Schema validation successful for '{}'", file_path.display());

    // Step 4: Deserialize to type T
    let result: T = serde_yaml::from_str(&yaml_content).map_err(|e| {
        log_yaml_parse_error(&e, &yaml_content, &file_path.to_string_lossy());
        anyhow::anyhow!(
            "Failed to deserialize YAML file '{}': {}",
            file_path.display(),
            e
        )
    })?;

    log::debug!(
        "Successfully loaded and validated YAML file: {}",
        file_path.display()
    );

    Ok(result)
}

/// Parse and validate a YAML string, then deserialize to type T
///
/// This function is designed for working with YAML content from editor buffers
/// or other string sources where you don't have a file path.
///
/// # Arguments
///
/// * `yaml_content` - The YAML content as a string
/// * `schemas_root` - Root directory containing schema files for validation
/// * `source_name` - Name to use in error messages (e.g., "editor buffer")
///
/// # Returns
///
/// Returns `Ok(T)` with the deserialized object if successful, or an error if:
/// - Schema resolution fails (missing 'schema' field in YAML)
/// - Schema validation fails
/// - YAML deserialization fails
///
/// # Example
///
/// ```no_run
/// use serde::Deserialize;
/// use testcase_common::parse_and_validate_yaml_string;
///
/// #[derive(Deserialize)]
/// struct TestCase {
///     schema: String,
///     id: String,
///     description: String,
/// }
///
/// let yaml = r#"
/// schema: tcms/test-case.schema.v1.json
/// id: TC001
/// description: Example test case
/// "#;
///
/// let test_case: TestCase = parse_and_validate_yaml_string(yaml, "schemas/", "editor buffer").unwrap();
/// ```
pub fn parse_and_validate_yaml_string<T, S>(
    yaml_content: &str,
    schemas_root: S,
    source_name: &str,
) -> Result<T>
where
    T: DeserializeOwned,
    S: AsRef<str>,
{
    let schemas_root = schemas_root.as_ref();

    // Step 1: Parse YAML to extract schema field
    let envelope: crate::envelope::Envelope = serde_yaml::from_str(yaml_content).map_err(|e| {
        log_yaml_parse_error(&e, yaml_content, source_name);
        anyhow::anyhow!("Failed to parse YAML from {}: {}", source_name, e)
    })?;

    // Step 2: Resolve schema from the payload's schema field
    let schema_uri = envelope
        .schema
        .ok_or_else(|| anyhow::anyhow!("Missing 'schema' field in YAML from {}", source_name))?;

    let schema_path = crate::envelope::resolve_schema_uri(&schema_uri, schemas_root)?;

    // Verify schema file exists
    if !schema_path.exists() {
        anyhow::bail!(
            "Schema file not found: {} (resolved from URI: '{}')",
            schema_path.display(),
            schema_uri
        );
    }

    log::debug!(
        "Resolved schema '{}' for {} to '{}'",
        schema_uri,
        source_name,
        schema_path.display()
    );

    // Step 3: Validate against the schema
    validate_yaml_against_schema(yaml_content, &schema_path, source_name)
        .context(format!("Schema validation failed for {}", source_name))?;

    log::debug!("Schema validation successful for {}", source_name);

    // Step 4: Deserialize to type T
    let result: T = serde_yaml::from_str(yaml_content).map_err(|e| {
        log_yaml_parse_error(&e, yaml_content, source_name);
        anyhow::anyhow!("Failed to deserialize YAML from {}: {}", source_name, e)
    })?;

    log::debug!(
        "Successfully parsed and validated YAML from {}",
        source_name
    );

    Ok(result)
}

/// Validates YAML content against a JSON schema file
///
/// # Arguments
///
/// * `yaml_content` - The YAML content as a string
/// * `schema_path` - Path to the JSON schema file
/// * `source_name` - Name of the source for error messages
///
/// # Returns
///
/// Returns `Ok(())` if validation succeeds, or an error describing validation failures
fn validate_yaml_against_schema<P: AsRef<Path>>(
    yaml_content: &str,
    schema_path: P,
    source_name: &str,
) -> Result<()> {
    let schema_path = schema_path.as_ref();

    // Read and parse the JSON schema
    let schema_content = fs::read_to_string(schema_path).context(format!(
        "Failed to read schema file: {}",
        schema_path.display()
    ))?;

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_content).context("Failed to parse JSON schema")?;

    // Compile the schema with options to handle external references
    // Pre-load the tcms-envelope schema to avoid URL resolution failures
    let mut options = jsonschema::JSONSchema::options();

    // If this schema references the envelope schema, load it
    // The envelope schema is in the parent directory (e.g., schemas/tcms-envelope.schema.json)
    // when the main schema is in schemas/tcms-schemas/tcms/test-case-main.schema.json
    if let Some(schema_dir) = schema_path.parent() {
        // Try the parent directory first (schemas/ when we're in schemas/tcms/)
        let envelope_schema_path = schema_dir
            .parent()
            .map(|parent| parent.join("tcms-envelope.schema.json"))
            .unwrap_or_else(|| schema_dir.join("tcms-envelope.schema.json"));

        if envelope_schema_path.exists() {
            if let Ok(envelope_content) = fs::read_to_string(&envelope_schema_path) {
                if let Ok(envelope_value) =
                    serde_json::from_str::<serde_json::Value>(&envelope_content)
                {
                    // Add the envelope schema with its $id as the key
                    if let Some(id) = envelope_value.get("$id").and_then(|v| v.as_str()) {
                        let id_string = id.to_string();
                        options.with_document(id_string.clone(), envelope_value);
                        log::debug!(
                            "Loaded referenced schema: {} from {}",
                            id_string,
                            envelope_schema_path.display()
                        );
                    }
                }
            }
        }
    }

    let compiled_schema = options
        .compile(&schema_value)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    // Parse YAML content
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_content).map_err(|e| {
        log_yaml_parse_error(&e, yaml_content, source_name);
        anyhow::anyhow!("Failed to parse YAML: {}", e)
    })?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value =
        serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

    // Validate against schema
    let envelope_errors = crate::envelope::envelope_violations(&json_value);
    let schema_errors = compiled_schema.validate(&json_value).err();
    if schema_errors.is_some() || !envelope_errors.is_empty() {
        let mut error_messages = vec!["Schema constraint violations:".to_string()];
        for message in &envelope_errors {
            error_messages.push(format!(
                "  Error #0: Path 'root'\n    Constraint: {message}"
            ));
        }

        for (idx, error) in schema_errors.into_iter().flatten().enumerate() {
            let path = if error.instance_path.to_string().is_empty() {
                "root".to_string()
            } else {
                error.instance_path.to_string()
            };

            error_messages.push(format!("  Error #{}: Path '{}'", idx + 1, path));
            error_messages.push(format!("    Constraint: {}", error));

            let instance = error.instance.as_ref();
            error_messages.push(format!("    Found value: {}", instance));
        }

        return Err(anyhow::anyhow!(error_messages.join("\n")));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestData {
        schema: String,
        name: String,
        value: i32,
    }

    #[test]
    fn test_load_and_validate_yaml_success() {
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path().join("schemas");
        std::fs::create_dir(&schemas_root).unwrap();

        // Create a schema file
        let schema_path = schemas_root.join("test.schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "schema": { "type": "string" },
                "name": { "type": "string" },
                "value": { "type": "integer" }
            },
            "required": ["schema", "name", "value"]
        }"#;
        File::create(&schema_path)
            .unwrap()
            .write_all(schema_content.as_bytes())
            .unwrap();

        // Create a YAML file that references the schema
        let yaml_file = temp_dir.path().join("test.yaml");
        let yaml_content = r#"schema: test.schema.json
name: Test Case
value: 42
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        // Load and validate
        let result: Result<TestData> =
            load_and_validate_yaml(&yaml_file, schemas_root.to_str().unwrap());

        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.name, "Test Case");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_load_and_validate_yaml_validation_failure() {
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path().join("schemas");
        std::fs::create_dir(&schemas_root).unwrap();

        // Create a schema file with strict requirements
        let schema_path = schemas_root.join("test.schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "schema": { "type": "string" },
                "name": { "type": "string" },
                "value": { "type": "integer", "minimum": 100 }
            },
            "required": ["schema", "name", "value"]
        }"#;
        File::create(&schema_path)
            .unwrap()
            .write_all(schema_content.as_bytes())
            .unwrap();

        // Create a YAML file that violates the schema (value < 100)
        let yaml_file = temp_dir.path().join("test.yaml");
        let yaml_content = r#"schema: test.schema.json
name: Test Case
value: 42
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        // Load and validate - should fail
        let result: Result<TestData> =
            load_and_validate_yaml(&yaml_file, schemas_root.to_str().unwrap());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema validation failed"));
    }

    #[test]
    fn test_load_and_validate_yaml_missing_schema_field() {
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path().join("schemas");
        std::fs::create_dir(&schemas_root).unwrap();

        // Create a YAML file without schema field
        let yaml_file = temp_dir.path().join("test.yaml");
        let yaml_content = r#"name: Test Case
value: 42
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        // Load and validate - should fail during schema resolution
        let result: Result<TestData> =
            load_and_validate_yaml(&yaml_file, schemas_root.to_str().unwrap());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to resolve schema"));
    }

    #[test]
    fn test_load_and_validate_yaml_invalid_yaml_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path().join("schemas");
        std::fs::create_dir(&schemas_root).unwrap();

        // Create a schema file
        let schema_path = schemas_root.join("test.schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "schema": { "type": "string" },
                "name": { "type": "string" }
            },
            "required": ["schema", "name"]
        }"#;
        File::create(&schema_path)
            .unwrap()
            .write_all(schema_content.as_bytes())
            .unwrap();

        // Create a YAML file with invalid syntax
        let yaml_file = temp_dir.path().join("test.yaml");
        let yaml_content = r#"schema: test.schema.json
name: [unclosed array
"#;
        File::create(&yaml_file)
            .unwrap()
            .write_all(yaml_content.as_bytes())
            .unwrap();

        // Load and validate - should fail during validation
        let result: Result<TestData> =
            load_and_validate_yaml(&yaml_file, schemas_root.to_str().unwrap());

        assert!(result.is_err());
    }

    #[test]
    fn test_load_and_validate_yaml_file_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path().join("schemas");
        std::fs::create_dir(&schemas_root).unwrap();

        let yaml_file = temp_dir.path().join("nonexistent.yaml");

        let result: Result<TestData> =
            load_and_validate_yaml(&yaml_file, schemas_root.to_str().unwrap());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to read YAML file"));
    }

    #[test]
    fn test_validate_yaml_against_schema_success() {
        let temp_dir = TempDir::new().unwrap();
        let schema_path = temp_dir.path().join("test.schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        }"#;
        File::create(&schema_path)
            .unwrap()
            .write_all(schema_content.as_bytes())
            .unwrap();

        let yaml_content = "name: John Doe\nage: 30";

        let result = validate_yaml_against_schema(yaml_content, &schema_path, "test.yaml");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_against_schema_failure() {
        let temp_dir = TempDir::new().unwrap();
        let schema_path = temp_dir.path().join("test.schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        }"#;
        File::create(&schema_path)
            .unwrap()
            .write_all(schema_content.as_bytes())
            .unwrap();

        let yaml_content = "age: 30";

        let result = validate_yaml_against_schema(yaml_content, &schema_path, "test.yaml");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema constraint violations"));
    }
}
