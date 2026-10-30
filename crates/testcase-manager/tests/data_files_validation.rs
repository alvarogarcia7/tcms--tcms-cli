use std::fs;
use std::path::Path;
use testcase_manager::validation::SchemaValidator;

/// Test that validates all YAML files in the testcases/ directory
/// Valid files (gsma_*) should pass validation
/// Invalid files should fail validation appropriately

#[test]
fn test_valid_gsma_files() {
    let _ = env_logger::builder().is_test(true).try_init();

    let validator = SchemaValidator::new().expect("Failed to create validator");

    let valid_files = vec!["tests/sample/gsma_4.4.2.2_TC.yml"];

    for file_path in valid_files {
        log::info!("\n=== Testing valid file: {} ===", file_path);

        // Check file exists
        assert!(
            Path::new(file_path).exists(),
            "File does not exist: {}",
            file_path
        );

        // Read file content
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

        // Validate with complete validation (requires all fields)
        let result = validator.validate_complete(&content);

        assert!(
            result.is_ok(),
            "Valid GSMA file {} should pass validation, but got error: {:?}",
            file_path,
            result.err()
        );

        log::info!("✓ {} passed validation", file_path);
    }
}

#[test]
fn test_invalid_sgp_file_missing_general_conditions() {
    let _ = env_logger::builder().is_test(true).try_init();

    let validator = SchemaValidator::new().expect("Failed to create validator");

    let file_path = "tests/sample/SGP.22_4.4.2.yaml";

    log::info!("\n=== Testing invalid file: {} ===", file_path);

    // Check file exists
    assert!(
        Path::new(file_path).exists(),
        "File does not exist: {}",
        file_path
    );

    // Read file content
    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

    // Validate with complete validation - should fail due to missing general_initial_conditions
    let result = validator.validate_complete(&content);

    assert!(
        result.is_err(),
        "Invalid file {} should fail validation (missing general_initial_conditions)",
        file_path
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("general_initial_conditions") || error_msg.contains("required"),
        "Error should mention missing general_initial_conditions, got: {}",
        error_msg
    );

    log::info!(
        "✓ {} correctly failed validation: {}",
        file_path,
        error_msg.lines().next().unwrap_or("")
    );
}

#[test]
fn test_invalid_data_yml_wrong_structure() {
    let _ = env_logger::builder().is_test(true).try_init();

    let validator = SchemaValidator::new().expect("Failed to create validator");

    let file_path = "tests/sample/data.yml";

    log::info!("\n=== Testing invalid file: {} ===", file_path);

    // Check file exists
    assert!(
        Path::new(file_path).exists(),
        "File does not exist: {}",
        file_path
    );

    // Read file content
    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

    // Validate with complete validation - should fail due to wrong structure
    let result = validator.validate_complete(&content);

    assert!(
        result.is_err(),
        "Invalid file {} should fail validation (wrong structure)",
        file_path
    );

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("required") || error_msg.contains("Missing"),
        "Error should mention missing required fields, got: {}",
        error_msg
    );

    log::info!(
        "✓ {} correctly failed validation: {}",
        file_path,
        error_msg.lines().next().unwrap_or("")
    );
}

#[test]
fn test_chunk_validation_on_valid_files() {
    let _ = env_logger::builder().is_test(true).try_init();

    let validator = SchemaValidator::new().expect("Failed to create validator");

    let valid_files = vec!["tests/sample/gsma_4.4.2.2_TC.yml"];

    for file_path in valid_files {
        log::info!(
            "\n=== Testing chunk validation on valid file: {} ===",
            file_path
        );

        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

        // Chunk validation should also pass for complete valid documents
        let result = validator.validate_chunk(&content);

        assert!(
            result.is_ok(),
            "Valid GSMA file {} should pass chunk validation, but got error: {:?}",
            file_path,
            result.err()
        );

        log::info!("✓ {} passed chunk validation", file_path);
    }
}

#[test]
fn test_all_data_files_exist() {
    let _ = env_logger::builder().is_test(true).try_init();

    // Canonical schema path: schemas/tcms-schemas/tcms/test-case-main.schema.json (legacy, backward compatible)
    let expected_files = vec![
        "schemas/tcms-schemas/tcms/test-case-main.schema.json",
        "tests/sample/SGP.22_4.4.2.yaml",
        "tests/sample/data.yml",
    ];

    for file_path in expected_files {
        assert!(
            Path::new(file_path).exists(),
            "Expected file does not exist: {}",
            file_path
        );
    }

    log::info!("✓ All expected data files exist");
}

#[test]
fn test_valid_files_can_be_parsed_as_yaml() {
    let valid_files = vec!["tests/sample/gsma_4.4.2.2_TC.yml"];

    for file_path in valid_files {
        log::info!("\n=== Testing YAML parsing for: {} ===", file_path);

        // Note: This test uses raw YAML parsing since the test file doesn't have a schema field
        let content = fs::read_to_string(file_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {} as YAML: {}", file_path, e));

        // Verify it's an object/mapping
        assert!(
            yaml_value.is_mapping(),
            "Expected {} to be a YAML mapping/object",
            file_path
        );

        // Verify required top-level fields exist
        let mapping = yaml_value.as_mapping().unwrap();

        assert!(
            mapping.contains_key(serde_yaml::Value::String("requirement".to_string())),
            "{} missing 'requirement' field",
            file_path
        );

        assert!(
            mapping.contains_key(serde_yaml::Value::String("item".to_string())),
            "{} missing 'item' field",
            file_path
        );

        assert!(
            mapping.contains_key(serde_yaml::Value::String("tc".to_string())),
            "{} missing 'tc' field",
            file_path
        );

        assert!(
            mapping.contains_key(serde_yaml::Value::String("id".to_string())),
            "{} missing 'id' field",
            file_path
        );

        assert!(
            mapping.contains_key(serde_yaml::Value::String("description".to_string())),
            "{} missing 'description' field",
            file_path
        );

        assert!(
            mapping.contains_key(serde_yaml::Value::String("test_sequences".to_string())),
            "{} missing 'test_sequences' field",
            file_path
        );

        log::info!("✓ {} is valid YAML with expected structure", file_path);
    }
}

#[test]
fn test_invalid_files_structure() {
    let _ = env_logger::builder().is_test(true).try_init();

    let file_path = "tests/sample/SGP.22_4.4.2.yaml";
    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {} as YAML: {}", file_path, e));

    let mapping = yaml_value.as_mapping().unwrap();

    // Should be missing general_initial_conditions
    assert!(
        !mapping.contains_key(serde_yaml::Value::String(
            "general_initial_conditions".to_string()
        )),
        "{} should be missing 'general_initial_conditions'",
        file_path
    );

    log::info!(
        "✓ {} is correctly missing general_initial_conditions",
        file_path
    );
}

#[test]
fn test_valid_files_detailed_structure() {
    let _ = env_logger::builder().is_test(true).try_init();

    let file_path = "tests/sample/gsma_4.4.2.2_TC.yml";

    log::info!("\n=== Testing detailed structure of: {} ===", file_path);

    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_path, e));

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {} as YAML: {}", file_path, e));

    let mapping = yaml_value.as_mapping().unwrap();

    // Check general_initial_conditions
    let general_ic = mapping
        .get(serde_yaml::Value::String(
            "general_initial_conditions".to_string(),
        ))
        .expect("Missing general_initial_conditions");

    assert!(
        general_ic.is_mapping(),
        "general_initial_conditions should be an object"
    );

    // Check initial_conditions
    let ic = mapping
        .get(serde_yaml::Value::String("initial_conditions".to_string()))
        .expect("Missing initial_conditions");

    assert!(ic.is_mapping(), "initial_conditions should be an object");

    // Check test_sequences
    let sequences = mapping
        .get(serde_yaml::Value::String("test_sequences".to_string()))
        .expect("Missing test_sequences");

    assert!(sequences.is_sequence(), "test_sequences should be an array");

    let sequences_array = sequences.as_sequence().unwrap();
    assert!(
        sequences_array.len() >= 2,
        "Should have at least 2 test sequences"
    );

    // Check first sequence structure
    let first_seq = &sequences_array[0];
    assert!(first_seq.is_mapping(), "Sequence should be an object");

    let seq_mapping = first_seq.as_mapping().unwrap();

    assert!(
        seq_mapping.contains_key(serde_yaml::Value::String("id".to_string())),
        "Sequence missing 'id'"
    );

    assert!(
        seq_mapping.contains_key(serde_yaml::Value::String("name".to_string())),
        "Sequence missing 'name'"
    );

    assert!(
        seq_mapping.contains_key(serde_yaml::Value::String("steps".to_string())),
        "Sequence missing 'steps'"
    );

    // Check steps
    let steps = seq_mapping
        .get(serde_yaml::Value::String("steps".to_string()))
        .expect("Missing steps");

    assert!(steps.is_sequence(), "steps should be an array");

    let steps_array = steps.as_sequence().unwrap();
    assert!(steps_array.len() >= 2, "Should have at least 2 steps");

    // Check first step structure
    let first_step = &steps_array[0];
    let step_mapping = first_step.as_mapping().unwrap();

    assert!(
        step_mapping.contains_key(serde_yaml::Value::String("step".to_string())),
        "Step missing 'step' number"
    );

    assert!(
        step_mapping.contains_key(serde_yaml::Value::String("description".to_string())),
        "Step missing 'description'"
    );

    assert!(
        step_mapping.contains_key(serde_yaml::Value::String("command".to_string())),
        "Step missing 'command'"
    );

    assert!(
        step_mapping.contains_key(serde_yaml::Value::String("expected".to_string())),
        "Step missing 'expected'"
    );

    log::info!("✓ {} has correct detailed structure", file_path);
}
