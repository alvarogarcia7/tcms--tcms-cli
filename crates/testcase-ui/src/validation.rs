use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use serde_json::Value as JsonValue;
use std::fs;
use std::path::PathBuf;
use testcase_common::log_yaml_parse_error;
use testcase_models::ValidationErrorDetail;

pub struct SchemaValidator {
    schema: JSONSchema,
    schema_value: JsonValue,
}

/// Find the schema file in common locations
/// Uses canonical schema path: schemas/tcms-schemas/tcms/test-case-main.schema.json (legacy, backward compatible)
/// For new development, consider using: schemas/tcms-schemas/tcms/test-case-main.schema.json
fn find_schema_file() -> Result<PathBuf> {
    let possible_paths = vec![
        PathBuf::from("schemas/tcms-schemas/tcms/test-case-main.schema.json"),
        PathBuf::from("../../schemas/tcms-schemas/tcms/test-case-main.schema.json"),
    ];

    for path in &possible_paths {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    anyhow::bail!("Could not find schema file. Tried: {:?}", possible_paths)
}

impl SchemaValidator {
    pub fn new() -> Result<Self> {
        let schema_path = find_schema_file().context("Failed to locate schema file")?;
        let schema_content = fs::read_to_string(&schema_path)
            .context(format!("Failed to read {:?}", schema_path))?;

        let schema_value: JsonValue =
            serde_json::from_str(&schema_content).context("Failed to parse schema file")?;

        let schema = JSONSchema::compile(&schema_value)
            .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

        Ok(Self {
            schema,
            schema_value,
        })
    }

    /// Validate a chunk of input data against the schema
    ///
    /// This method validates only the provided chunk without requiring all fields.
    /// It validates the provided properties against the schema's definitions
    /// with proper handling of arrays, nested objects, and type constraints.
    ///
    /// # Arguments
    /// * `yaml_content` - YAML string containing the chunk to validate
    ///
    /// # Returns
    /// * `Ok(())` if the chunk is valid according to the schema
    /// * `Err` with validation errors if the chunk is invalid
    pub fn validate_chunk(&self, yaml_content: &str) -> Result<()> {
        log::debug!("Validating YAML chunk");
        let yaml_value: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
            Ok(value) => value,
            Err(e) => {
                log_yaml_parse_error(&e, yaml_content, "YAML chunk");
                return Err(anyhow::anyhow!("Failed to parse YAML content: {}", e));
            }
        };

        let json_value: JsonValue =
            serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

        if let JsonValue::Object(obj) = &json_value {
            let mut errors = Vec::new();

            if let Some(JsonValue::Object(schema_obj)) = self.schema_value.get("properties") {
                for (key, value) in obj.iter() {
                    if let Some(property_schema) = schema_obj.get(key) {
                        if let Err(validation_errors) =
                            self.validate_value(value, property_schema, key)
                        {
                            errors.extend(validation_errors);
                        }
                    }
                }
            }

            if !errors.is_empty() {
                log::error!("Schema validation failed for YAML chunk:");
                for error in &errors {
                    log::error!("{}", error);
                }
                anyhow::bail!("Schema validation failed:\n{}", errors.join("\n"));
            }
        }

        log::debug!("YAML chunk validation successful");
        Ok(())
    }

    /// Validate a single value against its schema definition
    fn validate_value(
        &self,
        value: &JsonValue,
        schema: &JsonValue,
        path: &str,
    ) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Handle oneOf constraint
        if let Some(JsonValue::Array(one_of_schemas)) = schema.get("oneOf") {
            let mut matched = false;
            let mut all_errors = Vec::new();

            for (idx, sub_schema) in one_of_schemas.iter().enumerate() {
                if self.validate_value(value, sub_schema, path).is_ok() {
                    matched = true;
                    break;
                } else if let Err(sub_errors) = self.validate_value(value, sub_schema, path) {
                    all_errors.push((idx, sub_errors));
                }
            }

            if !matched {
                let error_msg = format!(
                    "  - Path '{}': Value does not match any of the allowed schemas (oneOf constraint)",
                    path
                );
                log::error!(
                    "oneOf validation failed at path '{}': value did not match any of {} schemas",
                    path,
                    one_of_schemas.len()
                );
                errors.push(error_msg);
                return Err(errors);
            }

            // If oneOf matched, we're done with validation for this schema
            return Ok(());
        }

        // Check type constraint
        if let Some(expected_type) = schema.get("type") {
            if let Some(type_str) = expected_type.as_str() {
                let type_matches = match type_str {
                    "string" => value.is_string(),
                    "integer" => value.is_i64() || value.is_u64(),
                    "number" => value.is_f64() || value.is_i64() || value.is_u64(),
                    "boolean" => value.is_boolean(),
                    "array" => value.is_array(),
                    "object" => value.is_object(),
                    "null" => value.is_null(),
                    _ => true,
                };

                if !type_matches {
                    let error_msg = format!(
                        "  - Path '{}': Invalid type (expected {}, got {})",
                        path,
                        type_str,
                        get_value_type(value)
                    );
                    let actual_value = format_value_for_log(value);
                    log::error!(
                        "Type constraint violation at path '{}': expected type '{}', got type '{}', actual value: {}",
                        path,
                        type_str,
                        get_value_type(value),
                        actual_value
                    );
                    errors.push(error_msg);
                    return Err(errors);
                }
            }
        }

        // Validate array items
        if let JsonValue::Array(arr) = value {
            if let Some(items_schema) = schema.get("items") {
                // Handle tuple validation (items as array) vs single schema
                if let JsonValue::Array(item_schemas) = items_schema {
                    // Tuple validation: each position has its own schema
                    for (idx, item) in arr.iter().enumerate() {
                        // In JSON Schema Draft-04, if items is an array, additional items are allowed
                        // We only validate against the schema if one is defined for this position
                        if let Some(item_schema) = item_schemas.get(idx) {
                            let item_path = format!("{}[{}]", path, idx);
                            if let Err(item_errors) =
                                self.validate_value(item, item_schema, &item_path)
                            {
                                errors.extend(item_errors);
                            }
                        }
                    }
                } else {
                    // Single schema for all items
                    for (idx, item) in arr.iter().enumerate() {
                        let item_path = format!("{}[{}]", path, idx);
                        if let Err(item_errors) =
                            self.validate_value(item, items_schema, &item_path)
                        {
                            errors.extend(item_errors);
                        }
                    }
                }
            }
        }

        // Validate object properties
        if let JsonValue::Object(obj) = value {
            if let Some(JsonValue::Object(properties)) = schema.get("properties") {
                // Validate each property in the object
                for (key, val) in obj.iter() {
                    if let Some(prop_schema) = properties.get(key) {
                        let prop_path = format!("{}.{}", path, key);
                        if let Err(prop_errors) = self.validate_value(val, prop_schema, &prop_path)
                        {
                            errors.extend(prop_errors);
                        }
                    }
                }
            }

            // Validate additionalProperties if defined
            if let Some(additional_props_schema) = schema.get("additionalProperties") {
                let defined_properties = schema
                    .get("properties")
                    .and_then(|p| p.as_object())
                    .map(|obj| obj.keys().collect::<std::collections::HashSet<_>>())
                    .unwrap_or_default();

                for (key, val) in obj.iter() {
                    if !defined_properties.contains(key) {
                        let prop_path = format!("{}.{}", path, key);
                        if let Err(prop_errors) =
                            self.validate_value(val, additional_props_schema, &prop_path)
                        {
                            errors.extend(prop_errors);
                        }
                    }
                }
            }

            // Check required fields
            if let Some(JsonValue::Array(required)) = schema.get("required") {
                for req_field in required {
                    if let Some(field_name) = req_field.as_str() {
                        if !obj.contains_key(field_name) {
                            let error_msg = format!(
                                "  - Path '{}': Missing required property '{}'",
                                path, field_name
                            );
                            log::error!(
                                "Missing required property at path '{}': property '{}' is required but not present",
                                path,
                                field_name
                            );
                            errors.push(error_msg);
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate a complete test case document against the schema
    ///
    /// This method validates that all required fields are present and valid.
    /// Use this for complete test case documents.
    ///
    /// # Arguments
    /// * `yaml_content` - YAML string containing the complete document
    ///
    /// # Returns
    /// * `Ok(())` if the complete document is valid
    /// * `Err` with validation errors if the document is invalid or missing required fields
    pub fn validate_complete(&self, yaml_content: &str) -> Result<()> {
        log::debug!("Validating complete YAML document");
        let yaml_value: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
            Ok(value) => value,
            Err(e) => {
                log_yaml_parse_error(&e, yaml_content, "complete YAML document");
                return Err(anyhow::anyhow!("Failed to parse YAML content: {}", e));
            }
        };

        let json_value: JsonValue =
            serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

        let validation_result = self.schema.validate(&json_value);

        if let Err(errors) = validation_result {
            log::error!("Schema validation failed for complete YAML document:");
            let error_messages: Vec<String> = errors
                .map(|e| {
                    let path = if e.instance_path.to_string().is_empty() {
                        "root".to_string()
                    } else {
                        e.instance_path.to_string()
                    };
                    let formatted_error = format_validation_error(&e);
                    let error_str = e.to_string();
                    let actual_value =
                        extract_instance_value(&json_value, &e.instance_path.to_string());

                    log::error!(
                        "Validation error at path '{}': {} | Actual value: {}",
                        path,
                        error_str,
                        actual_value
                    );

                    format!("  - Path '{}': {}", path, formatted_error)
                })
                .collect();

            anyhow::bail!("Schema validation failed:\n{}", error_messages.join("\n"));
        }

        log::debug!("Complete YAML document validation successful");
        Ok(())
    }

    /// Validate and return detailed error information
    ///
    /// This method validates YAML content and returns structured error details
    /// that can be used for detailed error reporting.
    ///
    /// # Arguments
    /// * `yaml_content` - YAML string to validate
    ///
    /// # Returns
    /// * `Ok(())` if valid
    /// * `Err` with structured validation error details
    pub fn validate_with_details(&self, yaml_content: &str) -> Result<Vec<ValidationErrorDetail>> {
        let yaml_value: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
            Ok(value) => value,
            Err(e) => {
                log_yaml_parse_error(&e, yaml_content, "YAML content");
                return Err(anyhow::anyhow!("Failed to parse YAML content: {}", e));
            }
        };

        let json_value: JsonValue =
            serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

        let validation_result = self.schema.validate(&json_value);

        if let Err(errors) = validation_result {
            let mut error_details = Vec::new();

            for e in errors {
                let path = if e.instance_path.to_string().is_empty() {
                    "root".to_string()
                } else {
                    e.instance_path.to_string()
                };

                let error_str = e.to_string();
                let (constraint, expected_constraint) = extract_constraint_info(&error_str);

                let found_value = extract_instance_value(&json_value, &e.instance_path.to_string());

                log::error!(
                    "Schema validation error at path '{}': constraint='{}', expected='{}', found='{}'",
                    path,
                    constraint,
                    expected_constraint,
                    found_value
                );

                error_details.push(ValidationErrorDetail {
                    path: path.clone(),
                    constraint,
                    found_value,
                    expected_constraint,
                });
            }

            return Ok(error_details);
        }

        Ok(Vec::new())
    }

    /// Validate a partial chunk, allowing empty objects
    ///
    /// This is a convenience method that allows empty objects to pass validation.
    /// For non-empty objects, it delegates to validate_chunk.
    ///
    /// # Arguments
    /// * `yaml_content` - YAML string containing the partial chunk
    ///
    /// # Returns
    /// * `Ok(())` if the chunk is valid or empty
    /// * `Err` with validation errors if the chunk is invalid
    pub fn validate_partial_chunk(&self, yaml_content: &str) -> Result<()> {
        let yaml_value: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
            Ok(value) => value,
            Err(e) => {
                log_yaml_parse_error(&e, yaml_content, "partial YAML chunk");
                return Err(anyhow::anyhow!("Failed to parse YAML content: {}", e));
            }
        };

        let json_value: JsonValue =
            serde_json::to_value(&yaml_value).context("Failed to convert YAML to JSON")?;

        if let JsonValue::Object(obj) = &json_value {
            if obj.is_empty() {
                return Ok(());
            }
        }

        self.validate_chunk(yaml_content)
    }

    pub fn validate_initial_conditions(
        &self,
        initial_conditions: &serde_yaml::Value,
    ) -> Result<()> {
        log::debug!("Validating initial conditions structure");
        let json_value: JsonValue =
            serde_json::to_value(initial_conditions).context("Failed to convert to JSON")?;

        if let JsonValue::Object(obj) = &json_value {
            for (key, value) in obj.iter() {
                // Skip the "include" field - it has its own structure
                if key == "include" {
                    if !value.is_array() {
                        log::error!(
                            "Initial conditions validation error: 'include' field must be an array, got: {:?}",
                            value
                        );
                        anyhow::bail!("'include' field must be an array, got: {:?}", value);
                    }
                    // Validate include array items
                    if let JsonValue::Array(arr) = value {
                        for (idx, item) in arr.iter().enumerate() {
                            if !item.is_object() {
                                log::error!(
                                    "Initial conditions validation error at 'include', index {}: expected object, got {:?}",
                                    idx,
                                    item
                                );
                                anyhow::bail!(
                                    "Include item #{} must be an object, got: {:?}",
                                    idx + 1,
                                    item
                                );
                            }
                            if let JsonValue::Object(include_obj) = item {
                                if !include_obj.contains_key("id") {
                                    log::error!(
                                        "Initial conditions validation error at 'include', index {}: missing required 'id' field",
                                        idx
                                    );
                                    anyhow::bail!(
                                        "Include item #{} must have an 'id' field",
                                        idx + 1
                                    );
                                }
                            }
                        }
                    }
                    continue;
                }

                // Validate device-keyed condition arrays
                if !value.is_array() {
                    log::error!(
                        "Initial conditions validation error: device '{}' has invalid type. Expected: array of strings/objects, Found: {:?}",
                        key,
                        value
                    );
                    anyhow::bail!(
                        "Device '{}' must have an array of conditions, got: {:?}",
                        key,
                        value
                    );
                }

                if let JsonValue::Array(arr) = value {
                    for (idx, item) in arr.iter().enumerate() {
                        // Items can be strings, objects with "ref" field, or objects with "test_sequence" field
                        if !item.is_string() && !item.is_object() {
                            log::error!(
                                "Initial conditions validation error at device '{}', index {}: expected string or object, got {:?}",
                                key,
                                idx,
                                item
                            );
                            anyhow::bail!(
                                "Condition #{} for device '{}' must be a string or object, got: {:?}",
                                idx + 1,
                                key,
                                item
                            );
                        }

                        // If it's an object, validate it has either "ref" or "test_sequence"
                        if let JsonValue::Object(obj) = item {
                            let has_ref = obj.contains_key("ref");
                            let has_test_sequence = obj.contains_key("test_sequence");

                            if !has_ref && !has_test_sequence {
                                log::error!(
                                    "Initial conditions validation error at device '{}', index {}: object must have either 'ref' or 'test_sequence' field",
                                    key,
                                    idx
                                );
                                anyhow::bail!(
                                    "Condition #{} for device '{}' must have either 'ref' or 'test_sequence' field",
                                    idx + 1,
                                    key
                                );
                            }

                            // Validate test_sequence structure if present
                            if has_test_sequence {
                                if let Some(JsonValue::Object(ts_obj)) = obj.get("test_sequence") {
                                    if !ts_obj.contains_key("id") || !ts_obj.contains_key("step") {
                                        log::error!(
                                            "Initial conditions validation error at device '{}', index {}: test_sequence must have 'id' and 'step' fields",
                                            key,
                                            idx
                                        );
                                        anyhow::bail!(
                                            "Condition #{} for device '{}': test_sequence must have 'id' and 'step' fields",
                                            idx + 1,
                                            key
                                        );
                                    }
                                } else {
                                    log::error!(
                                        "Initial conditions validation error at device '{}', index {}: test_sequence must be an object",
                                        key,
                                        idx
                                    );
                                    anyhow::bail!(
                                        "Condition #{} for device '{}': test_sequence must be an object",
                                        idx + 1,
                                        key
                                    );
                                }
                            }
                        }
                    }
                }
            }
        } else {
            log::error!(
                "Initial conditions validation error: root structure must be an object with device names as keys, got: {:?}",
                json_value
            );
            anyhow::bail!("initial_conditions must be an object with device names as keys");
        }

        log::debug!("Initial conditions validation successful");
        Ok(())
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default SchemaValidator")
    }
}

fn get_value_type(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::String(_) => "string",
        JsonValue::Number(n) if n.is_f64() => "number",
        JsonValue::Number(_) => "integer",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
        JsonValue::Null => "null",
    }
}

fn format_value_for_log(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => {
            if s.len() > 100 {
                format!("\"{}...\" (truncated, length: {})", &s[..100], s.len())
            } else {
                format!("\"{}\"", s)
            }
        }
        JsonValue::Number(n) => format!("{}", n),
        JsonValue::Bool(b) => format!("{}", b),
        JsonValue::Null => "null".to_string(),
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else if arr.len() <= 3 {
                let items: Vec<String> = arr.iter().map(format_value_for_log).collect();
                format!("[{}]", items.join(", "))
            } else {
                format!("[array with {} items]", arr.len())
            }
        }
        JsonValue::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                format!("{{object with {} fields}}", obj.len())
            }
        }
    }
}

fn format_validation_error(error: &jsonschema::ValidationError) -> String {
    let error_str = error.to_string();

    if error_str.contains("is not of type") {
        return format!("Invalid type ({})", error_str);
    }

    if error_str.contains("is a required property") {
        if let Some(prop) = error_str.split('\'').nth(1) {
            return format!("Missing required property '{}'", prop);
        }
    }

    error_str
}

/// Extract constraint information from a validation error message
fn extract_constraint_info(error_str: &str) -> (String, String) {
    if error_str.contains("is not of type") {
        let parts: Vec<&str> = error_str.split("is not of type").collect();
        if parts.len() == 2 {
            let expected_type = parts[1].trim().trim_matches('\'').trim_matches('"');
            return (
                "type_mismatch".to_string(),
                format!("Expected type: {}", expected_type),
            );
        }
    }

    if error_str.contains("is a required property") {
        if let Some(prop) = error_str.split('\'').nth(1) {
            return (
                "missing_property".to_string(),
                format!("Required property '{}' is missing", prop),
            );
        }
    }

    if error_str.contains("is not valid under any of the given schemas") {
        return (
            "oneOf_validation".to_string(),
            "Value does not match any of the allowed schemas".to_string(),
        );
    }

    if error_str.contains("does not match pattern") {
        return ("pattern_mismatch".to_string(), error_str.to_string());
    }

    if error_str.contains("is less than the minimum") {
        return ("minimum_value".to_string(), error_str.to_string());
    }

    if error_str.contains("is greater than the maximum") {
        return ("maximum_value".to_string(), error_str.to_string());
    }

    ("validation_error".to_string(), error_str.to_string())
}

/// Extract the actual value at a given JSON path
fn extract_instance_value(json_value: &JsonValue, path: &str) -> String {
    if path.is_empty() || path == "/" {
        return format!("{}", json_value);
    }

    let path_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let mut current = json_value;

    for part in path_parts {
        if let Ok(index) = part.parse::<usize>() {
            if let Some(arr) = current.as_array() {
                if let Some(value) = arr.get(index) {
                    current = value;
                } else {
                    return format!("<index {} out of bounds>", index);
                }
            } else {
                return format!("<not an array at {}>", part);
            }
        } else if let Some(obj) = current.as_object() {
            if let Some(value) = obj.get(part) {
                current = value;
            } else {
                return format!("<missing field '{}'>", part);
            }
        } else {
            return format!("<cannot access field '{}'>", part);
        }
    }

    match current {
        JsonValue::String(s) => format!("\"{}\"", s),
        JsonValue::Number(n) => format!("{}", n),
        JsonValue::Bool(b) => format!("{}", b),
        JsonValue::Null => "null".to_string(),
        JsonValue::Array(_) => "[array]".to_string(),
        JsonValue::Object(_) => "{object}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validator_creation() {
        let validator = SchemaValidator::new();
        assert!(validator.is_ok());
    }

    #[test]
    fn test_validate_complete_valid_yaml() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
requirement: XXX100
item: 1
tc: 4
id: '4.2.2.2.1 TC_eUICC_ES6.UpdateMetadata'
description: 'Test description'
general_initial_conditions:
  eUICC:
    - "Some condition"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
test_sequences:
  - id: 1
    name: "Test Sequence"
    description: "Test description"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: true
        description: "Step description"
        command: "ssh"
        expected:
          success: true
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2 description"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
  - id: 2
    name: "Test Sequence 2"
    description: "Test description 2"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: false
        description: "Step description"
        command: "ssh"
        expected:
          success: false
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2 description"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
"#;

        let result = validator.validate_complete(yaml_content);
        assert!(
            result.is_ok(),
            "Validation should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_invalid_yaml_missing_required() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
requirement: XXX100
item: 1
"#;

        let result = validator.validate_complete(yaml_content);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Missing required property") || error_msg.contains("required"));
    }

    #[test]
    fn test_validate_chunk_metadata_only() {
        let validator = SchemaValidator::new().unwrap();

        // Validate only metadata fields as a chunk (not requiring all other fields)
        let yaml_content = r#"
requirement: XXX100
item: 1
tc: 4
id: 'TC001'
description: 'Test description'
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Chunk validation should succeed for metadata only: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_invalid_yaml_wrong_type() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
requirement: XXX100
item: "not_an_integer"
tc: 4
id: '4.2.2.2.1'
description: 'Test'
general_initial_conditions:
  eUICC:
    - "Condition"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step"
        command: "ssh"
        expected:
          success: true
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
  - id: 2
    name: "Test 2"
    description: "Test 2"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step"
        command: "ssh"
        expected:
          success: true
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
"#;

        let result = validator.validate_complete(yaml_content);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid type") || error_msg.contains("type"));
    }

    #[test]
    fn test_validate_chunk_wrong_type() {
        let validator = SchemaValidator::new().unwrap();

        // Test that chunk validation catches type errors
        let yaml_content = r#"
requirement: XXX100
item: "not_an_integer"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Invalid type")
                || error_msg.contains("type")
                || error_msg.contains("item")
        );
    }

    #[test]
    fn test_validate_partial_chunk_empty() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = "{}";

        let result = validator.validate_partial_chunk(yaml_content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_initial_conditions_valid() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC:
  - "Condition 1"
  - "Condition 2"
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(
            result.is_ok(),
            "Validation should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_initial_conditions_invalid_not_array() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC: "not an array"
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have an array"));
    }

    #[test]
    fn test_validate_initial_conditions_invalid_not_strings() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC:
  - "Valid string"
  - 123
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be a string"));
    }

    #[test]
    fn test_validate_initial_conditions_multiple_devices() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC:
  - "Condition 1"
  - "Condition 2"
LPA:
  - "LPA Condition 1"
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(
            result.is_ok(),
            "Validation should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_initial_conditions_custom_device_types() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC:
  - "eUICC Condition 1"
LPA:
  - "LPA Condition 1"
  - "LPA Condition 2"
SM_DP_PLUS:
  - "SM-DP+ Condition 1"
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(
            result.is_ok(),
            "Validation should succeed with custom device types: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_initial_conditions_single_device() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
LPA:
  - "LPA Condition 1"
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(
            result.is_ok(),
            "Validation should succeed with single device: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_initial_conditions_empty_array_error() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
eUICC: []
"#;
        let initial_conditions: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();

        let result = validator.validate_initial_conditions(&initial_conditions);
        assert!(result.is_ok(), "Empty array should be valid");
    }

    #[test]
    fn test_validate_chunk_single_field() {
        let validator = SchemaValidator::new().unwrap();

        // Test validating just one field
        let yaml_content = r#"
requirement: XXX100
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should validate single field chunk: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_multiple_fields() {
        let validator = SchemaValidator::new().unwrap();

        // Test validating multiple fields but not all
        let yaml_content = r#"
requirement: XXX100
item: 1
tc: 4
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should validate multiple field chunk: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_general_initial_conditions_only() {
        let validator = SchemaValidator::new().unwrap();

        // Test validating just general_initial_conditions
        let yaml_content = r#"
general_initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should validate general_initial_conditions chunk: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_only() {
        let validator = SchemaValidator::new().unwrap();

        // Test validating just initial_conditions
        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should validate initial_conditions chunk: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_test_sequences_with_three_sequences() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Sequence 1"
    description: "First sequence"
    initial_conditions:
      eUICC:
        - "Condition 1"
    steps:
      - step: 1
        manual: true
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
  - id: 2
    name: "Sequence 2"
    description: "Second sequence"
    initial_conditions:
      eUICC:
        - "Condition 2"
    steps:
      - step: 1
        manual: false
        description: "Step 1"
        command: "cmd1"
        expected:
          success: false
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
  - id: 3
    name: "Sequence 3"
    description: "Third sequence"
    initial_conditions:
      eUICC:
        - "Condition 3"
    steps:
      - step: 1
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept 3+ test sequences: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_test_sequences_missing_required_in_step() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step without command"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_err(),
            "Should reject step missing required 'command'"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("command") || error_msg.contains("required"),
            "Error should mention missing command: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_test_sequences_missing_expected_fields() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: true
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_err(),
            "Should reject step with missing 'output' in first expected"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("output") || error_msg.contains("required"),
            "Error should mention missing output: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_general_initial_conditions_wrong_structure() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
general_initial_conditions:
  eUICC: "Should be array not string"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(result.is_err(), "Should reject non-array eUICC value");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("type") || error_msg.contains("Invalid"),
            "Error should mention type mismatch: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_euicc_not_array() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC: "Should be array"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(result.is_err(), "Should reject non-array eUICC");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("type") || error_msg.contains("Invalid"),
            "Error should mention type mismatch: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_euicc_items_not_strings() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Valid string"
    - 123
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_err(),
            "Should reject non-string/non-object array items (plain integers)"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("type")
                || error_msg.contains("Invalid")
                || error_msg.contains("oneOf")
                || error_msg.contains("string"),
            "Error should mention type mismatch: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_with_ref_object() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Valid string"
    - ref: "some_ref"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept objects with ref field: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_with_test_sequence_object() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Valid string"
    - test_sequence:
        id: 1
        step: "2"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept objects with test_sequence field: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_step_wrong_type_for_step_number() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: "should be integer"
        description: "Step"
        command: "cmd"
        expected:
          success: true
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(result.is_err(), "Should reject string step number");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("type")
                || error_msg.contains("Invalid")
                || error_msg.contains("integer"),
            "Error should mention type mismatch: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_test_sequences_single_sequence_valid() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Single Sequence"
    description: "Only one sequence"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: true
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept single test sequence: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_manual_field_optional() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step without manual field"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept steps without manual field: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_complete_empty_test_sequences() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
requirement: XXX100
item: 1
tc: 4
id: 'TC001'
description: 'Test'
general_initial_conditions:
  eUICC:
    - "Condition"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
test_sequences: []
"#;

        let result = validator.validate_complete(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept empty test_sequences array: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_success_field_optional_in_second_step() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: true
        description: "Step with success"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step without success"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
  - id: 2
    name: "Test 2"
    description: "Test 2"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step with success"
        command: "cmd1"
        expected:
          success: false
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step without success"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Steps should follow tuple validation pattern: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_accepts_valid_complete_payload() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
requirement: XXX100
item: 1
tc: 4
id: '4.2.2.2.1 TC_eUICC_ES6.UpdateMetadata'
description: 'Test description'
general_initial_conditions:
  eUICC:
    - "Some condition"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
test_sequences:
  - id: 1
    name: "Test Sequence"
    description: "Test description"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: true
        description: "Step description"
        command: "ssh"
        expected:
          success: true
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2 description"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
  - id: 2
    name: "Test Sequence 2"
    description: "Test description 2"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        manual: false
        description: "Step description"
        command: "ssh"
        expected:
          success: false
          result: "SW=0x9000"
          output: "Success"
      - step: 2
        description: "Step 2 description"
        command: "ssh"
        expected:
          result: "SW=0x9000"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept valid complete payload: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_rejects_invalid_integer_type() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
item: "not_an_integer"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_err(),
            "Should reject string where integer expected"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Invalid type") || error_msg.contains("integer"),
            "Error should mention type mismatch: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_accepts_additional_array_items() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
general_initial_conditions:
  eUICC:
     - "Condition 1"
     - "Condition 2"
     - "Condition 3"
     - "Condition 4"
     - "Condition 5"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept additional items in arrays (tuple validation allows extras): {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_accepts_additional_steps() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    name: "Test"
    description: "Test"
    initial_conditions:
      eUICC:
        - "Condition"
    steps:
      - step: 1
        description: "Step 1"
        command: "cmd1"
        expected:
          success: true
          result: "OK"
          output: "Success"
      - step: 2
        description: "Step 2"
        command: "cmd2"
        expected:
          result: "OK"
          output: "Success"
      - step: 3
        description: "Step 3"
        command: "cmd3"
        expected:
          result: "OK"
          output: "Success"
      - step: 4
        description: "Step 4"
        command: "cmd4"
        expected:
          result: "OK"
          output: "Success"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept more than 2 steps (tuple validation allows additional items): {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_rejects_missing_required_sequence_fields() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
test_sequences:
  - id: 1
    description: "Missing name and other required fields"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_err(),
            "Should reject sequence missing required fields"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("name") || error_msg.contains("required"),
            "Error should mention missing required field: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_chunk_initial_conditions_accepts_more_than_two_items() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Condition 1"
    - "Condition 2"
    - "Condition 3"
    - "Condition 4"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept more than 2 items in eUICC array (tuple validation allows extras): {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_accepts_single_euicc_item() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC:
    - "Single condition"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept single item in eUICC array: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_empty_euicc_array() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
initial_conditions:
  eUICC: []
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept empty eUICC array: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_chunk_general_initial_conditions_accepts_multiple_devices() {
        let validator = SchemaValidator::new().unwrap();

        let yaml_content = r#"
general_initial_conditions:
  eUICC:
    - "eUICC condition 1"
    - "eUICC condition 2"
  LPA:
    - "LPA condition 1"
  SM_DP_PLUS:
    - "SM-DP+ condition 1"
    - "SM-DP+ condition 2"
    - "SM-DP+ condition 3"
"#;

        let result = validator.validate_chunk(yaml_content);
        assert!(
            result.is_ok(),
            "Should accept multiple device types in general_initial_conditions: {:?}",
            result.err()
        );
    }
}
