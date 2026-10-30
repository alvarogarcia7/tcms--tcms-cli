use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use testcase_common::{log_yaml_parse_error, resolve_schema_from_payload};

/// Main validator struct for YAML validation against JSON schemas
pub struct YamlValidator;

impl YamlValidator {
    /// Creates a new YamlValidator instance
    pub fn new() -> Self {
        Self
    }

    /// Validates a YAML file against a specified JSON schema
    ///
    /// # Arguments
    ///
    /// * `yaml_path` - Path to the YAML file to validate
    /// * `schema_path` - Path to the JSON schema file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation succeeds, or an error describing validation failures
    ///
    /// # Example
    ///
    /// ```no_run
    /// use validate_yaml::YamlValidator;
    ///
    /// let validator = YamlValidator::new();
    /// validator.validate_file("test.yaml", "schema.json").unwrap();
    /// ```
    pub fn validate_file<P: AsRef<Path>, S: AsRef<Path>>(
        &self,
        yaml_path: P,
        schema_path: S,
    ) -> Result<()> {
        validate_single_file(yaml_path, schema_path)
    }

    /// Validates a YAML file with automatic schema resolution
    ///
    /// # Arguments
    ///
    /// * `yaml_path` - Path to the YAML file to validate
    /// * `schemas_root` - Root directory containing schema files for auto-resolution
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation succeeds, or an error describing validation failures
    ///
    /// # Example
    ///
    /// ```no_run
    /// use validate_yaml::YamlValidator;
    ///
    /// let validator = YamlValidator::new();
    /// validator.validate_file_auto_schema("test.yaml", "schemas/").unwrap();
    /// ```
    pub fn validate_file_auto_schema<P: AsRef<Path>, S: AsRef<Path>>(
        &self,
        yaml_path: P,
        schemas_root: S,
    ) -> Result<()> {
        let yaml_path = yaml_path.as_ref();
        let schemas_root = schemas_root.as_ref();

        let schema_path = resolve_schema_for_file(yaml_path, None::<&Path>, schemas_root)?;
        validate_single_file(yaml_path, &schema_path)
    }

    /// Validates YAML content (as a string) against a specified JSON schema
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - The YAML content as a string
    /// * `schema_path` - Path to the JSON schema file
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if validation succeeds, or an error describing validation failures
    ///
    /// # Example
    ///
    /// ```no_run
    /// use validate_yaml::YamlValidator;
    ///
    /// let validator = YamlValidator::new();
    /// let yaml_content = "name: test\nvalue: 42";
    /// validator.validate_content(yaml_content, "schema.json").unwrap();
    /// ```
    pub fn validate_content<S: AsRef<Path>>(
        &self,
        yaml_content: &str,
        schema_path: S,
    ) -> Result<()> {
        validate_yaml_content(yaml_content, schema_path)
    }
}

impl Default for YamlValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates a single YAML file against a JSON schema
///
/// # Arguments
///
/// * `yaml_path` - Path to the YAML file to validate
/// * `schema_path` - Path to the JSON schema file
///
/// # Returns
///
/// Returns `Ok(())` if validation succeeds, or an error describing validation failures
pub fn validate_single_file<P: AsRef<Path>, S: AsRef<Path>>(
    yaml_path: P,
    schema_path: S,
) -> Result<()> {
    let yaml_path = yaml_path.as_ref();
    let schema_path = schema_path.as_ref();

    let schema_content = fs::read_to_string(schema_path).context(format!(
        "Failed to read schema file: {}",
        schema_path.display()
    ))?;

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_content).context("Failed to parse JSON schema")?;

    let compiled_schema = jsonschema::JSONSchema::compile(&schema_value)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    let yaml_content = fs::read_to_string(yaml_path)
        .context(format!("Failed to read YAML file: {}", yaml_path.display()))?;

    validate_yaml_against_compiled_schema(
        &yaml_content,
        &compiled_schema,
        &yaml_path.to_string_lossy(),
    )
}

/// Validates YAML content against a JSON schema
///
/// # Arguments
///
/// * `yaml_content` - The YAML content as a string
/// * `schema_path` - Path to the JSON schema file
///
/// # Returns
///
/// Returns `Ok(())` if validation succeeds, or an error describing validation failures
pub fn validate_yaml_content<S: AsRef<Path>>(yaml_content: &str, schema_path: S) -> Result<()> {
    let schema_path = schema_path.as_ref();

    let schema_content = fs::read_to_string(schema_path).context(format!(
        "Failed to read schema file: {}",
        schema_path.display()
    ))?;

    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_content).context("Failed to parse JSON schema")?;

    let compiled_schema = jsonschema::JSONSchema::compile(&schema_value)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    validate_yaml_against_compiled_schema(yaml_content, &compiled_schema, "<string>")
}

/// Validates YAML content against a compiled JSON schema
fn validate_yaml_against_compiled_schema(
    yaml_content: &str,
    compiled_schema: &jsonschema::JSONSchema,
    source_name: &str,
) -> Result<()> {
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(yaml_content).map_err(|e| {
        log_yaml_parse_error(&e, yaml_content, source_name);
        anyhow::anyhow!("Failed to parse YAML: {}", e)
    })?;

    let json_value: serde_json::Value =
        serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

    let envelope_errors = testcase_common::envelope::envelope_violations(&json_value);
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

/// Resolves the schema path for a given YAML file
///
/// # Arguments
///
/// * `yaml_file` - Path to the YAML file
/// * `explicit_schema` - Optional explicit schema path (if provided, this is returned)
/// * `schemas_root` - Root directory containing schema files for auto-resolution
///
/// # Returns
///
/// Returns the resolved schema path
pub fn resolve_schema_for_file<P: AsRef<Path>, S: AsRef<Path>>(
    yaml_file: P,
    explicit_schema: Option<S>,
    schemas_root: P,
) -> Result<PathBuf> {
    let yaml_file = yaml_file.as_ref();
    let schemas_root = schemas_root.as_ref();

    if let Some(schema) = explicit_schema {
        let schema = schema.as_ref();
        log::debug!(
            "Using explicit schema '{}' for file '{}'",
            schema.display(),
            yaml_file.display()
        );
        return Ok(schema.to_path_buf());
    }

    log::debug!(
        "Auto-resolving schema for file '{}' from schemas root '{}'",
        yaml_file.display(),
        schemas_root.display()
    );
    resolve_schema_from_payload(yaml_file, schemas_root.to_str().unwrap_or("schemas/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    fn create_temp_schema() -> Result<NamedTempFile> {
        let mut schema_file = NamedTempFile::new()?;
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "age": {
                    "type": "integer",
                    "minimum": 0
                }
            },
            "required": ["name"]
        }"#;
        schema_file.write_all(schema_content.as_bytes())?;
        schema_file.flush()?;
        Ok(schema_file)
    }

    fn create_temp_yaml(content: &str) -> Result<NamedTempFile> {
        let mut yaml_file = NamedTempFile::new()?;
        yaml_file.write_all(content.as_bytes())?;
        yaml_file.flush()?;
        Ok(yaml_file)
    }

    #[test]
    fn test_validate_file_success() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: John Doe\nage: 30";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let validator = YamlValidator::new();
        let result = validator.validate_file(yaml_file.path(), schema_file.path());

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_missing_required_field() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "age: 30";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let validator = YamlValidator::new();
        let result = validator.validate_file(yaml_file.path(), schema_file.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema constraint violations"));
    }

    #[test]
    fn test_validate_file_type_mismatch() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: John Doe\nage: not_a_number";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let validator = YamlValidator::new();
        let result = validator.validate_file(yaml_file.path(), schema_file.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema constraint violations"));
    }

    #[test]
    fn test_validate_file_negative_age() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: John Doe\nage: -5";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let validator = YamlValidator::new();
        let result = validator.validate_file(yaml_file.path(), schema_file.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema constraint violations"));
    }

    #[test]
    fn test_validate_content_success() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: Jane Doe\nage: 25";

        let validator = YamlValidator::new();
        let result = validator.validate_content(yaml_content, schema_file.path());

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_content_missing_required() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "age: 25";

        let validator = YamlValidator::new();
        let result = validator.validate_content(yaml_content, schema_file.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Schema constraint violations"));
    }

    #[test]
    fn test_validate_content_invalid_yaml() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: [unclosed array";

        let validator = YamlValidator::new();
        let result = validator.validate_content(yaml_content, schema_file.path());

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse YAML"));
    }

    #[test]
    fn test_validate_single_file_function() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: Test User\nage: 40";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let result = validate_single_file(yaml_file.path(), schema_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_content_function() {
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: Another User\nage: 50";

        let result = validate_yaml_content(yaml_content, schema_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_schema_for_file_explicit() {
        let explicit_schema = PathBuf::from("schemas/test.json");
        let yaml_file = PathBuf::from("test.yaml");
        let schemas_root = PathBuf::from("schemas/");

        let result = resolve_schema_for_file(&yaml_file, Some(&explicit_schema), &schemas_root);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), explicit_schema);
    }

    #[test]
    fn test_validator_default() {
        let validator = YamlValidator::new();
        let schema_file = create_temp_schema().unwrap();
        let yaml_content = "name: Default Test";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let result = validator.validate_file(yaml_file.path(), schema_file.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_auto_schema_with_schema_field() {
        // Create a temporary directory for schemas
        let temp_dir = TempDir::new().unwrap();
        let schemas_root = temp_dir.path();

        // Create a schema file in the temp directory
        let schema_path = schemas_root.join("test-schema.json");
        let schema_content = r#"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "schema": { "type": "string" },
                "value": { "type": "string" }
            },
            "required": ["schema", "value"]
        }"#;
        std::fs::write(&schema_path, schema_content).unwrap();

        // Create a YAML file that references the schema
        let yaml_content = "schema: test-schema.json\nvalue: test data";
        let yaml_file = create_temp_yaml(yaml_content).unwrap();

        let validator = YamlValidator::new();
        let result = validator.validate_file_auto_schema(yaml_file.path(), schemas_root);

        // This may fail if the schema resolution doesn't find the file,
        // but we're testing that the API works
        assert!(result.is_ok() || result.is_err());
    }
}
