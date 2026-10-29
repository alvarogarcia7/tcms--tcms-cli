use anyhow::Result;
use jsonschema::JSONSchema;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

// Canonical paths:
// - Working schema (used by verifier): data/testcase_results_container/schema.json
// - Versioned schema (recommended): schemas/tcms/test-results-container.schema.v1.json
// - Sample data: schemas/tcms/samples/ (canonical location)
const SCHEMA_PATH: &str = "../../data/testcase_results_container/schema.json";
const SAMPLE_PATH: &str = "../../schemas/tcms/samples/testcase_results_container_sample.yml";

/// Helper function to load and compile schema
fn load_schema() -> Result<(Value, JSONSchema)> {
    let schema_path = PathBuf::from(SCHEMA_PATH);
    let schema_content = fs::read_to_string(&schema_path)?;
    let schema_json: Value = serde_json::from_str(&schema_content)?;
    let compiled_schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;
    Ok((schema_json, compiled_schema))
}

/// Test that the container schema file is valid JSON Schema
#[test]
fn test_schema_is_valid_json_schema() -> Result<()> {
    let schema_path = PathBuf::from(SCHEMA_PATH);
    assert!(
        schema_path.exists(),
        "Schema file should exist at {}",
        schema_path.display()
    );

    let (_schema_json, _compiled_schema) = load_schema()?;
    // If we got here, schema is valid
    Ok(())
}

/// Test that the sample data file validates against the schema
#[test]
fn test_sample_data_validates() -> Result<()> {
    let sample_path = PathBuf::from(SAMPLE_PATH);

    assert!(
        sample_path.exists(),
        "Sample data file should exist at {}",
        sample_path.display()
    );

    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Load sample data (YAML)
    let sample_content = fs::read_to_string(&sample_path)?;
    let sample_data: Value = serde_yaml::from_str(&sample_content)?;

    // Validate
    let result = compiled_schema.validate(&sample_data);
    if let Err(errors) = result {
        let error_messages: Vec<String> = errors
            .map(|e| format!("  - {} at {}", e, e.instance_path))
            .collect();
        panic!(
            "Sample data validation failed:\n{}",
            error_messages.join("\n")
        );
    }

    Ok(())
}

/// Test that a minimal valid container report validates
#[test]
fn test_minimal_valid_container_validates() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Create minimal valid container
    let minimal_container = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [],
        "metadata": {
            "execution_duration": 0.0,
            "total_test_cases": 0,
            "passed_test_cases": 0,
            "failed_test_cases": 0
        }
    });

    // Validate
    let result = compiled_schema.validate(&minimal_container);
    assert!(
        result.is_ok(),
        "Minimal valid container should pass validation"
    );

    Ok(())
}

/// Test that missing required fields are detected
#[test]
fn test_missing_required_fields_detected() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Create invalid container (missing "metadata")
    let invalid_container = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": []
    });

    // Validate - should fail
    let result = compiled_schema.validate(&invalid_container);
    assert!(
        result.is_err(),
        "Container missing 'metadata' should fail validation"
    );

    Ok(())
}

/// Test that invalid step status values are detected
#[test]
fn test_invalid_step_status_detected() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Create container with invalid step variant (externally tagged format)
    let invalid_container = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [{
            "test_case_id": "TC001",
            "description": "Test",
            "sequences": [{
                "sequence_id": 1,
                "name": "Seq1",
                "step_results": [{
                    "InvalidStatus": {  // Invalid variant!
                        "step": 1,
                        "description": "Step 1"
                    }
                }],
                "all_steps_passed": true
            }],
            "total_steps": 1,
            "passed_steps": 1,
            "failed_steps": 0,
            "not_executed_steps": 0,
            "overall_pass": true
        }],
        "metadata": {
            "execution_duration": 0.0,
            "total_test_cases": 1,
            "passed_test_cases": 1,
            "failed_test_cases": 0
        }
    });

    // Validate - should fail
    let result = compiled_schema.validate(&invalid_container);
    assert!(
        result.is_err(),
        "Container with invalid step status should fail validation"
    );

    Ok(())
}

/// Test that valid step statuses are accepted
#[test]
fn test_valid_step_statuses_accepted() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Test each valid status - externally tagged format
    for (variant_name, is_pass, is_fail, is_not_executed) in &[
        ("Pass", true, false, false),
        ("Fail", false, true, false),
        ("NotExecuted", false, false, true),
    ] {
        // Build step with dynamic variant name
        let step_data = if *is_fail {
            // Fail variant requires additional fields
            serde_json::json!({
                "step": 1,
                "description": "Test step",
                "expected": {
                    "result": "SUCCESS",
                    "output": "Expected output"
                },
                "actual_result": "FAILURE",
                "actual_output": "Actual output",
                "reason": "Test reason"
            })
        } else {
            // Pass and NotExecuted variants
            serde_json::json!({
                "step": 1,
                "description": "Test step"
            })
        };

        // Create the externally tagged enum with dynamic key
        let mut step = serde_json::Map::new();
        step.insert(variant_name.to_string(), step_data);
        let step = serde_json::Value::Object(step);

        let container = serde_json::json!({
            "title": "Test Report",
            "project": "Test Project",
            "test_date": "2024-03-16T12:00:00Z",
            "test_results": [{
                "test_case_id": "TC001",
                "description": "Test",
                "sequences": [{
                    "sequence_id": 1,
                    "name": "Seq1",
                    "step_results": [step],
                    "all_steps_passed": *is_pass
                }],
                "total_steps": 1,
                "passed_steps": if *is_pass { 1 } else { 0 },
                "failed_steps": if *is_fail { 1 } else { 0 },
                "not_executed_steps": if *is_not_executed { 1 } else { 0 },
                "overall_pass": *is_pass
            }],
            "metadata": {
                "execution_duration": 0.0,
                "total_test_cases": 1,
                "passed_test_cases": if *is_pass { 1 } else { 0 },
                "failed_test_cases": if *is_pass { 0 } else { 1 }
            }
        });

        let result = compiled_schema.validate(&container);
        if let Err(errors) = result {
            let error_messages: Vec<String> = errors.map(|e| format!("{}", e)).collect();
            panic!(
                "Container with status '{}' failed validation. Errors: {:?}",
                variant_name, error_messages
            );
        }
    }

    Ok(())
}

/// Test that string length constraints are enforced
#[test]
fn test_string_length_constraints() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Test title too long (max 200 chars)
    let title_too_long = "x".repeat(201);
    let invalid_container = serde_json::json!({
        "title": title_too_long,
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [],
        "metadata": {
            "execution_duration": 0.0,
            "total_test_cases": 0,
            "passed_test_cases": 0,
            "failed_test_cases": 0
        }
    });

    let result = compiled_schema.validate(&invalid_container);
    assert!(
        result.is_err(),
        "Container with title > 200 chars should fail validation"
    );

    Ok(())
}

/// Test that numeric minimum constraints are enforced
#[test]
fn test_numeric_minimum_constraints() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Test negative execution_duration (minimum 0)
    let invalid_container = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [],
        "metadata": {
            "execution_duration": -1.0,  // Invalid!
            "total_test_cases": 0,
            "passed_test_cases": 0,
            "failed_test_cases": 0
        }
    });

    let result = compiled_schema.validate(&invalid_container);
    assert!(
        result.is_err(),
        "Container with negative execution_duration should fail validation"
    );

    Ok(())
}

/// Test that optional fields can be omitted
#[test]
fn test_optional_fields_can_be_omitted() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Container without optional metadata fields
    let container_without_optionals = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [],
        "metadata": {
            // No environment, platform, or executor
            "execution_duration": 0.0,
            "total_test_cases": 0,
            "passed_test_cases": 0,
            "failed_test_cases": 0
        }
    });

    let result = compiled_schema.validate(&container_without_optionals);
    assert!(
        result.is_ok(),
        "Container without optional metadata fields should pass validation"
    );

    Ok(())
}

/// Test that additional properties are not allowed at container level
#[test]
fn test_additional_properties_not_allowed() -> Result<()> {
    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Container with additional property
    let container_with_extra = serde_json::json!({
        "title": "Test Report",
        "project": "Test Project",
        "test_date": "2024-03-16T12:00:00Z",
        "test_results": [],
        "metadata": {
            "execution_duration": 0.0,
            "total_test_cases": 0,
            "passed_test_cases": 0,
            "failed_test_cases": 0
        },
        "extra_field": "not allowed"  // This should cause validation failure
    });

    let result = compiled_schema.validate(&container_with_extra);
    assert!(
        result.is_err(),
        "Container with additional properties should fail validation"
    );

    Ok(())
}

/// Test that ContainerReport with StepVerificationResultEnum instances validates correctly
#[test]
fn test_container_report_with_step_verification_enum_validates() -> Result<()> {
    use chrono::Utc;
    use testcase_manager::ContainerReportMetadata;
    use testcase_manager::{
        ContainerReport, Expected, SequenceVerificationResult, StepVerificationResultEnum,
        TestCaseVerificationResult,
    };

    // Load schema
    let (_schema_json, compiled_schema) = load_schema()?;

    // Create a ContainerReport with all three StepVerificationResultEnum variants
    let container_report = ContainerReport {
        doc_type: None,
        schema: None,
        title: "Test Execution Results".to_string(),
        project: "Test Case Manager - Verification Results".to_string(),
        test_date: Utc::now(),
        test_results: vec![TestCaseVerificationResult {
            test_case_id: "TC001".to_string(),
            description: "Test case with all step result variants".to_string(),
            source_yaml_sha256: None,
            sequences: vec![SequenceVerificationResult {
                sequence_id: 1,
                name: "Main Test Sequence".to_string(),
                step_results: vec![
                    // Pass variant
                    StepVerificationResultEnum::Pass {
                        step: 1,
                        description: "Step that passed".to_string(),
                        requirement: Some("REQ-001".to_string()),
                        item: Some(1),
                        tc: Some(100),
                    },
                    // Fail variant
                    StepVerificationResultEnum::Fail {
                        step: 2,
                        description: "Step that failed".to_string(),
                        expected: Expected {
                            success: Some(true),
                            result: "SUCCESS".to_string(),
                            output: "Expected output".to_string(),
                        },
                        actual_result: "FAILURE".to_string(),
                        actual_output: "Actual output".to_string(),
                        reason: "Output mismatch".to_string(),
                        requirement: Some("REQ-002".to_string()),
                        item: Some(2),
                        tc: Some(200),
                    },
                    // NotExecuted variant
                    StepVerificationResultEnum::NotExecuted {
                        step: 3,
                        description: "Step not executed".to_string(),
                        requirement: Some("REQ-003".to_string()),
                        item: Some(3),
                        tc: Some(300),
                    },
                ],
                all_steps_passed: false,
                requirement: Some("REQ-SEQ".to_string()),
                item: Some(10),
                tc: Some(1000),
            }],
            total_steps: 3,
            passed_steps: 1,
            failed_steps: 1,
            not_executed_steps: 1,
            overall_pass: false,
            requirement: Some("REQ-TC".to_string()),
            item: Some(20),
            tc: Some(2000),
            run: Default::default(),
        }],
        metadata: ContainerReportMetadata {
            environment: Some("Test Environment".to_string()),
            platform: Some("Linux x86_64".to_string()),
            executor: Some("Test Executor".to_string()),
            execution_duration: 123.45,
            total_test_cases: 1,
            passed_test_cases: 0,
            failed_test_cases: 1,
            source_hashes: None,
        },
    };

    // Serialize to JSON Value (this is the canonical representation for validation)
    // Note: We serialize to JSON Value first, then validate, as YAML tags (!Pass, !Fail, etc.)
    // are YAML-specific and not part of the JSON schema validation
    let json_value = serde_json::to_value(&container_report)?;

    // Validate JSON representation
    let json_result = compiled_schema.validate(&json_value);
    if let Err(errors) = json_result {
        let error_messages: Vec<String> = errors
            .map(|e| format!("  - {} at {}", e, e.instance_path))
            .collect();
        panic!("JSON validation failed:\n{}", error_messages.join("\n"));
    }

    // Verify that we can serialize to both YAML and JSON strings
    let yaml = serde_yaml::to_string(&container_report)?;
    assert!(!yaml.is_empty(), "YAML serialization should produce output");

    let json = serde_json::to_string_pretty(&container_report)?;
    assert!(!json.is_empty(), "JSON serialization should produce output");

    // Verify JSON string can be parsed back and validated
    let json_value_from_string: Value = serde_json::from_str(&json)?;
    let json_reparse_result = compiled_schema.validate(&json_value_from_string);
    if let Err(errors) = json_reparse_result {
        let error_messages: Vec<String> = errors
            .map(|e| format!("  - {} at {}", e, e.instance_path))
            .collect();
        panic!(
            "JSON re-parse validation failed:\n{}",
            error_messages.join("\n")
        );
    }

    // Verify externally tagged format for Pass variant
    let pass_json = serde_json::to_value(&StepVerificationResultEnum::Pass {
        step: 1,
        description: "Test".to_string(),
        requirement: None,
        item: None,
        tc: None,
    })?;
    assert!(pass_json.is_object(), "Pass variant should be an object");
    assert!(
        pass_json.get("Pass").is_some(),
        "Pass variant should have 'Pass' key"
    );

    // Verify externally tagged format for Fail variant
    let fail_json = serde_json::to_value(&StepVerificationResultEnum::Fail {
        step: 2,
        description: "Test".to_string(),
        expected: Expected {
            success: None,
            result: "OK".to_string(),
            output: "Output".to_string(),
        },
        actual_result: "FAIL".to_string(),
        actual_output: "Error".to_string(),
        reason: "Mismatch".to_string(),
        requirement: None,
        item: None,
        tc: None,
    })?;
    assert!(fail_json.is_object(), "Fail variant should be an object");
    assert!(
        fail_json.get("Fail").is_some(),
        "Fail variant should have 'Fail' key"
    );

    // Verify externally tagged format for NotExecuted variant
    let not_executed_json = serde_json::to_value(&StepVerificationResultEnum::NotExecuted {
        step: 3,
        description: "Test".to_string(),
        requirement: None,
        item: None,
        tc: None,
    })?;
    assert!(
        not_executed_json.is_object(),
        "NotExecuted variant should be an object"
    );
    assert!(
        not_executed_json.get("NotExecuted").is_some(),
        "NotExecuted variant should have 'NotExecuted' key"
    );

    Ok(())
}

/// TCMS-40.3: a stamped report validates against the verifier's container schema and each
/// result against the verification-result schema of tcms-schemas
#[test]
fn test_stamped_results_validate_against_container_and_tcms_schemas() -> Result<()> {
    use testcase_verification::{
        BatchVerificationReport, ContainerReportConfig, RunInfo, TestCaseVerificationResult,
        TestVerifier,
    };

    let mut report = BatchVerificationReport::new();
    for id in ["TC_A", "TC_B"] {
        report.add_test_case_result(TestCaseVerificationResult {
            test_case_id: id.to_string(),
            description: "stamped".to_string(),
            sequences: vec![],
            total_steps: 0,
            passed_steps: 0,
            failed_steps: 0,
            not_executed_steps: 0,
            overall_pass: true,
            requirement: None,
            item: None,
            tc: None,
            source_yaml_sha256: None,
            run: RunInfo::default(),
        });
    }
    let yaml = TestVerifier::with_exact_matching().generate_report(
        &[report],
        "yaml",
        ContainerReportConfig {
            title: "T".to_string(),
            project: "P".to_string(),
            environment: Some("Staging".to_string()),
            platform: None,
            executor: None,
            run: RunInfo {
                run_id: Some("RUN-1".to_string()),
                build_id: Some("build-7".to_string()),
                tester: Some("alice".to_string()),
                ..Default::default()
            },
        },
    )?;
    let container: Value = serde_yaml::from_str(&yaml)?;

    // the verifier's own container schema
    let (_, container_schema) = load_schema()?;
    if let Err(errors) = container_schema.validate(&container) {
        let messages: Vec<String> = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        panic!("container schema: {}", messages.join("; "));
    }

    // every result against the extended verification-result schema
    let schema_json: Value = serde_json::from_str(&fs::read_to_string(
        "../../schemas/tcms-schemas/tcms/verification-result.schema.json",
    )?)?;
    let schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("compile: {}", e))?;
    for result in container["test_results"].as_array().unwrap() {
        assert_eq!(result["run_id"], "RUN-1");
        assert_eq!(result["build_id"], "build-7");
        assert_eq!(result["tester"], "alice");
        assert_eq!(result["environment"], "Staging");
        assert!(result["date"].is_string());
        if let Err(errors) = schema.validate(result) {
            let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
            panic!("verification-result schema: {}", messages.join("; "));
        }
    }
    Ok(())
}

/// TCMS-40.4: the test run record validates against the tcms-schemas test-run schema, with
/// every verdict, a reviewer and a problem report
#[test]
fn test_run_record_validates_against_tcms_test_run_schema() -> Result<()> {
    use std::collections::{BTreeMap, BTreeSet};
    use testcase_verification::{
        BatchVerificationReport, RunInfo, TestCaseVerificationResult, TestRunInput, TestRunRecord,
    };

    let mut report = BatchVerificationReport::new();
    for (id, passed, failed, not_executed) in [
        ("TC_PASS", 2, 0, 0),
        ("TC_FAIL", 1, 1, 0),
        ("TC_SKIP", 0, 0, 2),
        ("TC_ERR", 0, 0, 2),
    ] {
        report.add_test_case_result(TestCaseVerificationResult {
            test_case_id: id.to_string(),
            description: "d".to_string(),
            sequences: vec![],
            total_steps: passed + failed + not_executed,
            passed_steps: passed,
            failed_steps: failed,
            not_executed_steps: not_executed,
            overall_pass: failed == 0 && not_executed == 0,
            requirement: None,
            item: None,
            tc: None,
            source_yaml_sha256: None,
            run: RunInfo::default(),
        });
    }
    let record = TestRunRecord::build(
        &report,
        &TestRunInput {
            run: RunInfo {
                run_id: Some("RUN-1".to_string()),
                build_id: Some("build-7".to_string()),
                environment: Some("Staging".to_string()),
                tester: Some("alice".to_string()),
                date: None,
            },
            reviewer: Some("bob"),
            problem_reports: BTreeMap::from([("TC_FAIL".to_string(), "PR-057".to_string())]),
            errored: BTreeSet::from(["TC_ERR".to_string()]),
            result_ref: Some("results.yaml"),
            ..Default::default()
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    // as read by PyYAML-style loaders: plain YAML, no tags
    let value: Value = serde_yaml::from_str(&record.to_yaml()?)?;
    let schema_json: Value = serde_json::from_str(&fs::read_to_string(
        "../../schemas/tcms-schemas/tcms/test-run.schema.json",
    )?)?;
    let schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("compile: {}", e))?;
    if let Err(errors) = schema.validate(&value) {
        let messages: Vec<String> = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        panic!("test-run schema: {}", messages.join("; "));
    }

    let verdicts: Vec<&str> = value["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["verdict"].as_str().unwrap())
        .collect();
    assert_eq!(verdicts, ["pass", "fail", "not_executed", "error"]);
    assert_eq!(value["reviewer"], "bob");
    assert_eq!(value["results"][1]["problem_report_id"], "PR-057");
    Ok(())
}

/// TCMS-40.5: the traceability matrix validates against the tcms-schemas trace-matrix schema
#[test]
fn test_trace_matrix_validates_against_tcms_trace_matrix_schema() -> Result<()> {
    use testcase_models::{TestCase, TestType};
    use testcase_verification::{build_matrix, Requirement, RequirementLevel};

    let mut normal = TestCase::new("L".into(), 1, 1, "N1".into(), "d".into());
    normal.requirements = vec!["HLR-1".to_string()];
    normal.test_type = Some(TestType::NormalRange);
    let requirements = vec![
        Requirement {
            id: "HLR-1".to_string(),
            level: RequirementLevel::Hlr,
        },
        Requirement {
            id: "LLR-2".to_string(),
            level: RequirementLevel::Llr,
        },
    ];
    let (matrix, findings) = build_matrix("TM-1", Some("BL-1"), &requirements, &[normal], &[]);
    assert!(!findings.complete());

    let value: Value = serde_yaml::from_str(&testcase_common::to_yaml_string(&matrix)?)?;
    let schema_json: Value = serde_json::from_str(&fs::read_to_string(
        "../../schemas/tcms-schemas/tcms/trace-matrix.schema.json",
    )?)?;
    let schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("compile: {}", e))?;
    if let Err(errors) = schema.validate(&value) {
        let messages: Vec<String> = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        panic!("trace-matrix schema: {}", messages.join("; "));
    }
    assert_eq!(value["links"][1]["level"], "LLR");
    Ok(())
}

/// TCMS-40.10: an imported coverage report validates against the tcms-schemas coverage-report
/// schema, for LCOV only, MC/DC only and requirements-based inputs
#[test]
fn test_coverage_report_validates_against_tcms_coverage_report_schema() -> Result<()> {
    use testcase_verification::{
        build_coverage_report, CoverageInput, GapAnnotation, Lcov, McdcInput,
    };

    let schema_json: Value = serde_json::from_str(&fs::read_to_string(
        "../../schemas/tcms-schemas/tcms/coverage-report.schema.json",
    )?)?;
    let schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("compile: {}", e))?;
    let check = |input: CoverageInput| -> Result<Value> {
        let (report, _) = build_coverage_report(input)?;
        let value: Value = serde_yaml::from_str(&testcase_common::to_yaml_string(&report)?)?;
        if let Err(errors) = schema.validate(&value) {
            let messages: Vec<String> = errors
                .map(|e| format!("{} at {}", e, e.instance_path))
                .collect();
            panic!("coverage-report schema: {}", messages.join("; "));
        }
        Ok(value)
    };
    let annotation = |location: &str, classification: &str, approver: Option<&str>| GapAnnotation {
        location: location.to_string(),
        coverage_type: None,
        classification: classification.to_string(),
        rationale: "because".to_string(),
        approver: approver.map(str::to_string),
        code_unit_id: None,
        requirement_id: None,
        test_case_id: None,
        problem_report_id: None,
        action_item_id: None,
    };

    let lcov = "SF:src/a.c\nDA:1,1\nDA:2,0\nBRDA:1,0,0,1\nBRDA:1,0,1,-\nend_of_record\n";
    let value = check(CoverageInput {
        id: "COV-1".into(),
        baseline_id: "BL-1".into(),
        run_id: Some("RUN-1".into()),
        tool: Some("gcov".into()),
        tool_version: Some("13".into()),
        lcov: Some(Lcov::parse(lcov)?),
        annotations: vec![
            annotation("src/a.c:2", "dead_code", None),
            annotation("src/a.c:1", "justified", Some("qa.lead")),
        ],
        ..Default::default()
    })?;
    assert_eq!(value["gaps"][0]["classification"], "dead_code");
    assert_eq!(value["structural"]["decision_percent"], 50.0);

    let mcdc: McdcInput = serde_yaml::from_str("mcdc_percent: 88.5\n")?;
    let value = check(CoverageInput {
        id: "COV-2".into(),
        baseline_id: "BL-1".into(),
        mcdc: Some(mcdc),
        ..Default::default()
    })?;
    assert_eq!(value["structural"]["mcdc_percent"], 88.5);
    Ok(())
}
