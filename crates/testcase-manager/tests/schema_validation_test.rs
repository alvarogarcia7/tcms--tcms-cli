use jsonschema::JSONSchema;
use testcase_manager::{
    SequenceVerificationResult, StepVerificationResultEnum, TestCaseVerificationResult,
};
use testcase_models::Expected;

/// Load and compile a JSON schema from the schemas directory
/// Canonical schema paths:
/// - Legacy schemas: schemas/*.schema.json
/// - Versioned schemas: schemas/tcms/*.schema.v1.json
fn load_schema(schema_name: &str) -> JSONSchema {
    let schema_path = format!("schemas/{}", schema_name);
    let schema_content = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|_| panic!("Failed to read schema file: {}", schema_path));
    let schema_json: serde_json::Value = serde_json::from_str(&schema_content)
        .unwrap_or_else(|_| panic!("Failed to parse schema JSON: {}", schema_path));

    JSONSchema::compile(&schema_json)
        .unwrap_or_else(|_| panic!("Failed to compile schema: {}", schema_path))
}

/// Helper function to create a Pass variant with optional metadata
fn create_pass_result(
    step: i64,
    description: &str,
    requirement: Option<String>,
    item: Option<i64>,
    tc: Option<i64>,
) -> StepVerificationResultEnum {
    StepVerificationResultEnum::Pass {
        step,
        description: description.to_string(),
        requirement,
        item,
        tc,
    }
}

/// Helper function to create a Fail variant with optional metadata
#[allow(clippy::too_many_arguments)]
fn create_fail_result(
    step: i64,
    description: &str,
    expected: Expected,
    actual_result: &str,
    actual_output: &str,
    reason: &str,
    requirement: Option<String>,
    item: Option<i64>,
    tc: Option<i64>,
) -> StepVerificationResultEnum {
    StepVerificationResultEnum::Fail {
        step,
        description: description.to_string(),
        expected,
        actual_result: actual_result.to_string(),
        actual_output: actual_output.to_string(),
        reason: reason.to_string(),
        requirement,
        item,
        tc,
    }
}

/// Helper function to create a NotExecuted variant with optional metadata
fn create_not_executed_result(
    step: i64,
    description: &str,
    requirement: Option<String>,
    item: Option<i64>,
    tc: Option<i64>,
) -> StepVerificationResultEnum {
    StepVerificationResultEnum::NotExecuted {
        step,
        description: description.to_string(),
        requirement,
        item,
        tc,
    }
}

/// Helper function to create a TestCaseVerificationResult
fn create_test_case_result(
    test_case_id: &str,
    description: &str,
    sequences: Vec<SequenceVerificationResult>,
    requirement: Option<String>,
    item: Option<i64>,
    tc: Option<i64>,
) -> TestCaseVerificationResult {
    let mut total_steps = 0;
    let mut passed_steps = 0;
    let mut failed_steps = 0;
    let mut not_executed_steps = 0;

    for seq in &sequences {
        for step_result in &seq.step_results {
            total_steps += 1;
            match step_result {
                StepVerificationResultEnum::Pass { .. } => passed_steps += 1,
                StepVerificationResultEnum::Fail { .. } => failed_steps += 1,
                StepVerificationResultEnum::NotExecuted { .. } => not_executed_steps += 1,
            }
        }
    }

    let overall_pass = failed_steps == 0 && not_executed_steps == 0;

    TestCaseVerificationResult {
        test_case_id: test_case_id.to_string(),
        description: description.to_string(),
        sequences,
        total_steps,
        passed_steps,
        failed_steps,
        not_executed_steps,
        overall_pass,
        requirement,
        item,
        tc,
        source_yaml_sha256: None,
        run: Default::default(),
    }
}

#[test]
fn test_verification_result_schema_with_pass_variant() {
    let schema = load_schema("verification-result.schema.json");

    // Create a Pass result without optional metadata
    let pass_result = create_pass_result(1, "Test step 1", None, None, None);
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC001",
        "Test case with Pass variant",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for Pass variant:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_pass_variant_with_metadata() {
    let schema = load_schema("verification-result.schema.json");

    // Create a Pass result with all optional metadata fields
    let pass_result = create_pass_result(
        1,
        "Test step 1",
        Some("REQ-001".to_string()),
        Some(42),
        Some(100),
    );
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: Some("REQ-001".to_string()),
        item: Some(42),
        tc: Some(100),
    };

    let test_result = create_test_case_result(
        "TC001",
        "Test case with Pass variant and metadata",
        vec![sequence],
        Some("REQ-001".to_string()),
        Some(42),
        Some(100),
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for Pass variant with metadata:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_fail_variant() {
    let schema = load_schema("verification-result.schema.json");

    // Create a Fail result without optional metadata
    let expected = Expected {
        success: Some(true),
        result: "0".to_string(),
        output: "Success".to_string(),
    };
    let fail_result = create_fail_result(
        1,
        "Test step 1",
        expected,
        "1",
        "Error occurred",
        "Exit code mismatch",
        None,
        None,
        None,
    );
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![fail_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC002",
        "Test case with Fail variant",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for Fail variant:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_fail_variant_with_metadata() {
    let schema = load_schema("verification-result.schema.json");

    // Create a Fail result with all optional metadata fields
    let expected = Expected {
        success: Some(false),
        result: "1".to_string(),
        output: "Error".to_string(),
    };
    let fail_result = create_fail_result(
        2,
        "Test step 2",
        expected,
        "0",
        "Success",
        "Expected failure but got success",
        Some("REQ-002".to_string()),
        Some(50),
        Some(200),
    );
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![fail_result],
        all_steps_passed: false,
        requirement: Some("REQ-002".to_string()),
        item: Some(50),
        tc: Some(200),
    };

    let test_result = create_test_case_result(
        "TC002",
        "Test case with Fail variant and metadata",
        vec![sequence],
        Some("REQ-002".to_string()),
        Some(50),
        Some(200),
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for Fail variant with metadata:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_fail_variant_with_null_success() {
    let schema = load_schema("verification-result.schema.json");

    // Create a Fail result with success field set to null
    let expected = Expected {
        success: None,
        result: "0".to_string(),
        output: "Expected output".to_string(),
    };
    let fail_result = create_fail_result(
        1,
        "Test step 1",
        expected,
        "0",
        "Actual output",
        "Output mismatch",
        None,
        None,
        None,
    );
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![fail_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC003",
        "Test case with Fail variant (null success)",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for Fail variant with null success:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_not_executed_variant() {
    let schema = load_schema("verification-result.schema.json");

    // Create a NotExecuted result without optional metadata
    let not_executed_result = create_not_executed_result(1, "Test step 1", None, None, None);
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![not_executed_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC004",
        "Test case with NotExecuted variant",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for NotExecuted variant:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_not_executed_variant_with_metadata() {
    let schema = load_schema("verification-result.schema.json");

    // Create a NotExecuted result with all optional metadata fields
    let not_executed_result = create_not_executed_result(
        1,
        "Test step 1",
        Some("REQ-003".to_string()),
        Some(75),
        Some(300),
    );
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![not_executed_result],
        all_steps_passed: false,
        requirement: Some("REQ-003".to_string()),
        item: Some(75),
        tc: Some(300),
    };

    let test_result = create_test_case_result(
        "TC004",
        "Test case with NotExecuted variant and metadata",
        vec![sequence],
        Some("REQ-003".to_string()),
        Some(75),
        Some(300),
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for NotExecuted variant with metadata:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_mixed_variants() {
    let schema = load_schema("verification-result.schema.json");

    // Create a sequence with all three variants
    let pass_result = create_pass_result(1, "Successful step", None, None, None);

    let expected = Expected {
        success: Some(true),
        result: "0".to_string(),
        output: "Expected".to_string(),
    };
    let fail_result = create_fail_result(
        2,
        "Failed step",
        expected,
        "1",
        "Actual",
        "Mismatch",
        None,
        None,
        None,
    );

    let not_executed_result = create_not_executed_result(3, "Skipped step", None, None, None);

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Mixed Test Sequence".to_string(),
        step_results: vec![pass_result, fail_result, not_executed_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC005",
        "Test case with mixed variants",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for mixed variants:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_multiple_sequences() {
    let schema = load_schema("verification-result.schema.json");

    // Create multiple sequences
    let pass_result1 = create_pass_result(1, "Step 1", None, None, None);
    let pass_result2 = create_pass_result(2, "Step 2", None, None, None);

    let sequence1 = SequenceVerificationResult {
        sequence_id: 1,
        name: "First Sequence".to_string(),
        step_results: vec![pass_result1, pass_result2],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let expected = Expected {
        success: Some(false),
        result: "1".to_string(),
        output: "Error".to_string(),
    };
    let fail_result = create_fail_result(
        1,
        "Failed step",
        expected,
        "0",
        "Success",
        "Unexpected success",
        None,
        None,
        None,
    );

    let sequence2 = SequenceVerificationResult {
        sequence_id: 2,
        name: "Second Sequence".to_string(),
        step_results: vec![fail_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC006",
        "Test case with multiple sequences",
        vec![sequence1, sequence2],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for multiple sequences:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_output_schema_with_pass_variant() {
    let schema = load_schema("verification-output.schema.json");

    // Create a Pass result
    let pass_result = create_pass_result(1, "Test step", None, None, None);
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC007",
        "Test for verification-output schema",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against verification-output schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed against verification-output schema:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_output_schema_with_all_variants() {
    let schema = load_schema("verification-output.schema.json");

    // Create all three variants
    let pass_result = create_pass_result(1, "Successful step", None, None, None);

    let expected = Expected {
        success: Some(true),
        result: "0".to_string(),
        output: "Expected output".to_string(),
    };
    let fail_result = create_fail_result(
        2,
        "Failed step",
        expected,
        "1",
        "Actual output",
        "Result mismatch",
        None,
        None,
        None,
    );

    let not_executed_result = create_not_executed_result(3, "Skipped step", None, None, None);

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Complete Test Sequence".to_string(),
        step_results: vec![pass_result, fail_result, not_executed_result],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC008",
        "Complete test with all variants",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against verification-output schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed against verification-output schema:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_yaml_serialization_conforms_to_schema() {
    let schema = load_schema("verification-result.schema.json");

    // Create a test case result
    let pass_result = create_pass_result(1, "Test step", Some("REQ-004".to_string()), None, None);
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "YAML Test Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: Some("REQ-004".to_string()),
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC009",
        "YAML serialization test",
        vec![sequence],
        Some("REQ-004".to_string()),
        None,
        None,
    );

    // Serialize to YAML (for verification that YAML serialization works)
    let yaml_output = serde_yaml::to_string(&test_result).expect("Failed to serialize to YAML");

    // Verify YAML is non-empty and contains expected content
    assert!(!yaml_output.is_empty());
    assert!(yaml_output.contains("TC009"));
    assert!(yaml_output.contains("REQ-004"));

    // For schema validation, serialize directly to JSON (the actual verification format)
    let json_value = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_value);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "YAML serialization validation failed:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_json_serialization_conforms_to_schema() {
    let schema = load_schema("verification-result.schema.json");

    // Create a complex test case result
    let pass_result = create_pass_result(
        1,
        "Pass step",
        Some("REQ-005".to_string()),
        Some(10),
        Some(20),
    );

    let expected = Expected {
        success: Some(false),
        result: "1".to_string(),
        output: "Error message".to_string(),
    };
    let fail_result = create_fail_result(
        2,
        "Fail step",
        expected,
        "0",
        "Success message",
        "Expected failure",
        Some("REQ-005".to_string()),
        Some(10),
        Some(20),
    );

    let not_executed_result = create_not_executed_result(
        3,
        "Not executed step",
        Some("REQ-005".to_string()),
        Some(10),
        Some(20),
    );

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "JSON Test Sequence".to_string(),
        step_results: vec![pass_result, fail_result, not_executed_result],
        all_steps_passed: false,
        requirement: Some("REQ-005".to_string()),
        item: Some(10),
        tc: Some(20),
    };

    let test_result = create_test_case_result(
        "TC010",
        "JSON serialization test",
        vec![sequence],
        Some("REQ-005".to_string()),
        Some(10),
        Some(20),
    );

    // Serialize to JSON
    let json_output =
        serde_json::to_string_pretty(&test_result).expect("Failed to serialize to JSON");

    // Parse back to value
    let json_value: serde_json::Value =
        serde_json::from_str(&json_output).expect("Failed to parse JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_value);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "JSON serialization validation failed:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_result_schema_with_empty_sequences() {
    let schema = load_schema("verification-result.schema.json");

    // Create a test case result with empty sequences (edge case)
    let test_result = create_test_case_result("TC011", "Empty test case", vec![], None, None, None);

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for empty sequences:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_verification_output_schema_with_metadata_fields() {
    let schema = load_schema("verification-output.schema.json");

    // Create results with all metadata fields populated
    let pass_result = create_pass_result(
        1,
        "Step with metadata",
        Some("REQ-006".to_string()),
        Some(100),
        Some(500),
    );

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Metadata Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: Some("REQ-006".to_string()),
        item: Some(100),
        tc: Some(500),
    };

    let test_result = create_test_case_result(
        "TC012",
        "Test with all metadata fields",
        vec![sequence],
        Some("REQ-006".to_string()),
        Some(100),
        Some(500),
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against verification-output schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed with metadata fields:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_both_schemas_accept_same_structure() {
    // Load both schemas
    let verification_result_schema = load_schema("verification-result.schema.json");
    let verification_output_schema = load_schema("verification-output.schema.json");

    // Create a test case result
    let pass_result = create_pass_result(1, "Compatible step", None, None, None);
    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Compatible Sequence".to_string(),
        step_results: vec![pass_result],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let test_result = create_test_case_result(
        "TC013",
        "Cross-schema compatibility test",
        vec![sequence],
        None,
        None,
        None,
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against both schemas
    let result1 = verification_result_schema.validate(&json_output);
    if let Err(errors) = result1 {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed against verification-result schema:\n{}",
            error_messages.join("\n")
        );
    }

    let result2 = verification_output_schema.validate(&json_output);
    if let Err(errors) = result2 {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed against verification-output schema:\n{}",
            error_messages.join("\n")
        );
    }
}

#[test]
fn test_large_scale_verification_result() {
    let schema = load_schema("verification-result.schema.json");

    // Create a large-scale test with multiple sequences and many steps
    let mut sequences = Vec::new();

    for seq_id in 1..=5 {
        let mut step_results = Vec::new();
        for step_num in 1..=20 {
            let result = match step_num % 4 {
                0 => create_pass_result(
                    step_num,
                    &format!("Pass step {}", step_num),
                    Some(format!("REQ-{:03}", seq_id)),
                    Some(seq_id * 10),
                    Some(seq_id * 100),
                ),
                1 => {
                    let expected = Expected {
                        success: Some(true),
                        result: "0".to_string(),
                        output: format!("Expected output {}", step_num),
                    };
                    create_fail_result(
                        step_num,
                        &format!("Fail step {}", step_num),
                        expected,
                        "1",
                        &format!("Actual output {}", step_num),
                        "Simulated failure",
                        Some(format!("REQ-{:03}", seq_id)),
                        Some(seq_id * 10),
                        Some(seq_id * 100),
                    )
                }
                2 => create_not_executed_result(
                    step_num,
                    &format!("Skipped step {}", step_num),
                    Some(format!("REQ-{:03}", seq_id)),
                    Some(seq_id * 10),
                    Some(seq_id * 100),
                ),
                _ => create_pass_result(
                    step_num,
                    &format!("Another pass step {}", step_num),
                    None,
                    None,
                    None,
                ),
            };
            step_results.push(result);
        }

        let sequence = SequenceVerificationResult {
            sequence_id: seq_id,
            name: format!("Large Sequence {}", seq_id),
            step_results,
            all_steps_passed: false,
            requirement: Some(format!("REQ-{:03}", seq_id)),
            item: Some(seq_id * 10),
            tc: Some(seq_id * 100),
        };
        sequences.push(sequence);
    }

    let test_result = create_test_case_result(
        "TC014",
        "Large-scale test case",
        sequences,
        Some("REQ-MASTER".to_string()),
        Some(1000),
        Some(5000),
    );

    // Serialize to JSON
    let json_output = serde_json::to_value(&test_result).expect("Failed to serialize to JSON");

    // Validate against schema
    let validation_result = schema.validate(&json_output);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!(
            "Validation failed for large-scale test:\n{}",
            error_messages.join("\n")
        );
    }

    // Also verify the structure is reasonable
    assert_eq!(test_result.sequences.len(), 5);
    assert_eq!(test_result.total_steps, 100); // 5 sequences * 20 steps each
}
