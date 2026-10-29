use testcase_manager::{
    SequenceVerificationResult, StepVerificationResultEnum, TestCaseVerificationResult,
};
use testcase_models::{Expected, TestCase};

/// Helper function to create a test case with metadata
fn create_test_case_with_metadata(requirement: &str, item: i64, tc: i64, id: &str) -> TestCase {
    TestCase::new(
        requirement.to_string(),
        item,
        tc,
        id.to_string(),
        format!("Test case for {}", requirement),
    )
}

// ============================================================================
// TestCaseVerificationResult Metadata Tests
// ============================================================================

#[test]
fn test_test_case_verification_result_metadata_fields() {
    let result = TestCaseVerificationResult {
        test_case_id: "TC001".to_string(),
        description: "Test case description".to_string(),
        sequences: vec![],
        total_steps: 0,
        passed_steps: 0,
        failed_steps: 0,
        not_executed_steps: 0,
        overall_pass: true,
        requirement: Some("REQ-001".to_string()),
        item: Some(42),
        tc: Some(99),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    assert_eq!(result.requirement, Some("REQ-001".to_string()));
    assert_eq!(result.item, Some(42));
    assert_eq!(result.tc, Some(99));
}

#[test]
fn test_test_case_verification_result_metadata_optional() {
    let result = TestCaseVerificationResult {
        test_case_id: "TC002".to_string(),
        description: "Test case without metadata".to_string(),
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
        run: Default::default(),
    };

    assert_eq!(result.requirement, None);
    assert_eq!(result.item, None);
    assert_eq!(result.tc, None);
}

#[test]
fn test_test_case_verification_result_serialization_with_metadata() {
    let result = TestCaseVerificationResult {
        test_case_id: "TC003".to_string(),
        description: "Test with metadata".to_string(),
        sequences: vec![],
        total_steps: 5,
        passed_steps: 4,
        failed_steps: 1,
        not_executed_steps: 0,
        overall_pass: false,
        requirement: Some("REQ-SERIAL-001".to_string()),
        item: Some(10),
        tc: Some(20),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"requirement\":\"REQ-SERIAL-001\""));
    assert!(json.contains("\"item\":10"));
    assert!(json.contains("\"tc\":20"));

    // Test JSON deserialization
    let deserialized: TestCaseVerificationResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement, Some("REQ-SERIAL-001".to_string()));
    assert_eq!(deserialized.item, Some(10));
    assert_eq!(deserialized.tc, Some(20));
    assert_eq!(deserialized.test_case_id, result.test_case_id);
    assert_eq!(deserialized.total_steps, result.total_steps);
}

#[test]
fn test_test_case_verification_result_yaml_serialization_with_metadata() {
    let result = TestCaseVerificationResult {
        test_case_id: "TC004".to_string(),
        description: "YAML test".to_string(),
        sequences: vec![],
        total_steps: 3,
        passed_steps: 3,
        failed_steps: 0,
        not_executed_steps: 0,
        overall_pass: true,
        requirement: Some("REQ-YAML-001".to_string()),
        item: Some(15),
        tc: Some(25),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Test YAML serialization
    let yaml = serde_yaml::to_string(&result).unwrap();
    assert!(yaml.contains("requirement: REQ-YAML-001"));
    assert!(yaml.contains("item: 15"));
    assert!(yaml.contains("tc: 25"));

    // Test YAML deserialization
    let deserialized: TestCaseVerificationResult = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(deserialized.requirement, Some("REQ-YAML-001".to_string()));
    assert_eq!(deserialized.item, Some(15));
    assert_eq!(deserialized.tc, Some(25));
}

#[test]
fn test_test_case_verification_result_serialization_without_metadata() {
    let result = TestCaseVerificationResult {
        test_case_id: "TC005".to_string(),
        description: "Test without metadata".to_string(),
        sequences: vec![],
        total_steps: 1,
        passed_steps: 1,
        failed_steps: 0,
        not_executed_steps: 0,
        overall_pass: true,
        requirement: None,
        item: None,
        tc: None,
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Test JSON serialization - optional fields should be omitted
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("\"requirement\""));
    assert!(!json.contains("\"item\""));
    // Note: "tc" might appear in test_case_id, so we need to be more specific
    assert!(!json.contains("\"tc\":"));

    // Test JSON roundtrip
    let deserialized: TestCaseVerificationResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement, None);
    assert_eq!(deserialized.item, None);
    assert_eq!(deserialized.tc, None);
}

// ============================================================================
// SequenceVerificationResult Metadata Tests
// ============================================================================

#[test]
fn test_sequence_verification_result_metadata_fields() {
    let result = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![],
        all_steps_passed: true,
        requirement: Some("REQ-SEQ-001".to_string()),
        item: Some(100),
        tc: Some(200),
    };

    assert_eq!(result.requirement, Some("REQ-SEQ-001".to_string()));
    assert_eq!(result.item, Some(100));
    assert_eq!(result.tc, Some(200));
}

#[test]
fn test_sequence_verification_result_metadata_optional() {
    let result = SequenceVerificationResult {
        sequence_id: 2,
        name: "Sequence without metadata".to_string(),
        step_results: vec![],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    assert_eq!(result.requirement, None);
    assert_eq!(result.item, None);
    assert_eq!(result.tc, None);
}

#[test]
fn test_sequence_verification_result_serialization_with_metadata() {
    let result = SequenceVerificationResult {
        sequence_id: 3,
        name: "Serialization test sequence".to_string(),
        step_results: vec![],
        all_steps_passed: true,
        requirement: Some("REQ-SEQ-SERIAL-001".to_string()),
        item: Some(50),
        tc: Some(75),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"requirement\":\"REQ-SEQ-SERIAL-001\""));
    assert!(json.contains("\"item\":50"));
    assert!(json.contains("\"tc\":75"));

    // Test JSON deserialization
    let deserialized: SequenceVerificationResult = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.requirement,
        Some("REQ-SEQ-SERIAL-001".to_string())
    );
    assert_eq!(deserialized.item, Some(50));
    assert_eq!(deserialized.tc, Some(75));
    assert_eq!(deserialized.sequence_id, result.sequence_id);
}

#[test]
fn test_sequence_verification_result_yaml_serialization() {
    let result = SequenceVerificationResult {
        sequence_id: 4,
        name: "YAML sequence test".to_string(),
        step_results: vec![],
        all_steps_passed: false,
        requirement: Some("REQ-SEQ-YAML-001".to_string()),
        item: Some(30),
        tc: Some(40),
    };

    // Test YAML serialization
    let yaml = serde_yaml::to_string(&result).unwrap();
    assert!(yaml.contains("requirement: REQ-SEQ-YAML-001"));
    assert!(yaml.contains("item: 30"));
    assert!(yaml.contains("tc: 40"));

    // Test YAML deserialization
    let deserialized: SequenceVerificationResult = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        deserialized.requirement,
        Some("REQ-SEQ-YAML-001".to_string())
    );
    assert_eq!(deserialized.item, Some(30));
    assert_eq!(deserialized.tc, Some(40));
}

#[test]
fn test_sequence_verification_result_serialization_without_metadata() {
    let result = SequenceVerificationResult {
        sequence_id: 5,
        name: "No metadata sequence".to_string(),
        step_results: vec![],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    // Test JSON serialization - optional fields should be omitted
    let json = serde_json::to_string(&result).unwrap();
    assert!(!json.contains("\"requirement\""));
    assert!(!json.contains("\"item\""));
    assert!(!json.contains("\"tc\":"));

    // Test JSON roundtrip
    let deserialized: SequenceVerificationResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement, None);
    assert_eq!(deserialized.item, None);
    assert_eq!(deserialized.tc, None);
}

// ============================================================================
// StepVerificationResultEnum::Pass Metadata Tests
// ============================================================================

#[test]
fn test_step_pass_metadata_fields() {
    let step = StepVerificationResultEnum::Pass {
        step: 1,
        description: "Passed step".to_string(),
        requirement: Some("REQ-STEP-PASS-001".to_string()),
        item: Some(5),
        tc: Some(10),
    };

    assert_eq!(step.requirement(), Some(&"REQ-STEP-PASS-001".to_string()));
    assert_eq!(step.item(), Some(5));
    assert_eq!(step.tc(), Some(10));
    assert!(step.is_pass());
}

#[test]
fn test_step_pass_metadata_optional() {
    let step = StepVerificationResultEnum::Pass {
        step: 2,
        description: "Pass without metadata".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    assert_eq!(step.requirement(), None);
    assert_eq!(step.item(), None);
    assert_eq!(step.tc(), None);
    assert!(step.is_pass());
}

#[test]
fn test_step_pass_serialization_with_metadata() {
    let step = StepVerificationResultEnum::Pass {
        step: 3,
        description: "Pass serialization test".to_string(),
        requirement: Some("REQ-PASS-SERIAL-001".to_string()),
        item: Some(7),
        tc: Some(14),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"requirement\":\"REQ-PASS-SERIAL-001\""));
    assert!(json.contains("\"item\":7"));
    assert!(json.contains("\"tc\":14"));
    assert!(json.contains("\"Pass\""));

    // Test JSON deserialization
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-PASS-SERIAL-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(7));
    assert_eq!(deserialized.tc(), Some(14));
    assert!(deserialized.is_pass());
}

#[test]
fn test_step_pass_yaml_serialization() {
    let step = StepVerificationResultEnum::Pass {
        step: 4,
        description: "YAML pass test".to_string(),
        requirement: Some("REQ-PASS-YAML-001".to_string()),
        item: Some(11),
        tc: Some(22),
    };

    // Test YAML serialization
    let yaml = serde_yaml::to_string(&step).unwrap();
    assert!(yaml.contains("requirement: REQ-PASS-YAML-001"));
    assert!(yaml.contains("item: 11"));
    assert!(yaml.contains("tc: 22"));

    // Test YAML deserialization
    let deserialized: StepVerificationResultEnum = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-PASS-YAML-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(11));
    assert_eq!(deserialized.tc(), Some(22));
}

#[test]
fn test_step_pass_serialization_without_metadata() {
    let step = StepVerificationResultEnum::Pass {
        step: 5,
        description: "Pass without metadata".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    // Test JSON serialization - optional fields should be omitted
    let json = serde_json::to_string(&step).unwrap();
    assert!(!json.contains("\"requirement\""));
    assert!(!json.contains("\"item\""));
    assert!(!json.contains("\"tc\":"));

    // Test JSON roundtrip
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement(), None);
    assert_eq!(deserialized.item(), None);
    assert_eq!(deserialized.tc(), None);
}

// ============================================================================
// StepVerificationResultEnum::Fail Metadata Tests
// ============================================================================

#[test]
fn test_step_fail_metadata_fields() {
    let step = StepVerificationResultEnum::Fail {
        step: 1,
        description: "Failed step".to_string(),
        expected: Expected {
            success: Some(true),
            result: "expected_result".to_string(),
            output: "expected_output".to_string(),
        },
        actual_result: "actual_result".to_string(),
        actual_output: "actual_output".to_string(),
        reason: "Mismatch".to_string(),
        requirement: Some("REQ-STEP-FAIL-001".to_string()),
        item: Some(8),
        tc: Some(16),
    };

    assert_eq!(step.requirement(), Some(&"REQ-STEP-FAIL-001".to_string()));
    assert_eq!(step.item(), Some(8));
    assert_eq!(step.tc(), Some(16));
    assert!(!step.is_pass());
}

#[test]
fn test_step_fail_metadata_optional() {
    let step = StepVerificationResultEnum::Fail {
        step: 2,
        description: "Fail without metadata".to_string(),
        expected: Expected {
            success: None,
            result: "result".to_string(),
            output: "output".to_string(),
        },
        actual_result: "actual".to_string(),
        actual_output: "actual_out".to_string(),
        reason: "Error".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    assert_eq!(step.requirement(), None);
    assert_eq!(step.item(), None);
    assert_eq!(step.tc(), None);
    assert!(!step.is_pass());
}

#[test]
fn test_step_fail_serialization_with_metadata() {
    let step = StepVerificationResultEnum::Fail {
        step: 3,
        description: "Fail serialization test".to_string(),
        expected: Expected {
            success: Some(false),
            result: "SW=0x9000".to_string(),
            output: "Success".to_string(),
        },
        actual_result: "SW=0x6A82".to_string(),
        actual_output: "Error".to_string(),
        reason: "Result mismatch".to_string(),
        requirement: Some("REQ-FAIL-SERIAL-001".to_string()),
        item: Some(9),
        tc: Some(18),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"requirement\":\"REQ-FAIL-SERIAL-001\""));
    assert!(json.contains("\"item\":9"));
    assert!(json.contains("\"tc\":18"));
    assert!(json.contains("\"Fail\""));
    assert!(json.contains("\"expected\""));
    assert!(json.contains("\"actual_result\""));

    // Test JSON deserialization
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-FAIL-SERIAL-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(9));
    assert_eq!(deserialized.tc(), Some(18));
    assert!(!deserialized.is_pass());
}

#[test]
fn test_step_fail_yaml_serialization() {
    let step = StepVerificationResultEnum::Fail {
        step: 4,
        description: "YAML fail test".to_string(),
        expected: Expected {
            success: None,
            result: "expected".to_string(),
            output: "output".to_string(),
        },
        actual_result: "different".to_string(),
        actual_output: "different_output".to_string(),
        reason: "Values don't match".to_string(),
        requirement: Some("REQ-FAIL-YAML-001".to_string()),
        item: Some(12),
        tc: Some(24),
    };

    // Test YAML serialization
    let yaml = serde_yaml::to_string(&step).unwrap();
    assert!(yaml.contains("requirement: REQ-FAIL-YAML-001"));
    assert!(yaml.contains("item: 12"));
    assert!(yaml.contains("tc: 24"));

    // Test YAML deserialization
    let deserialized: StepVerificationResultEnum = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-FAIL-YAML-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(12));
    assert_eq!(deserialized.tc(), Some(24));
}

#[test]
fn test_step_fail_serialization_without_metadata() {
    let step = StepVerificationResultEnum::Fail {
        step: 5,
        description: "Fail without metadata".to_string(),
        expected: Expected {
            success: None,
            result: "expected".to_string(),
            output: "output".to_string(),
        },
        actual_result: "actual".to_string(),
        actual_output: "actual_output".to_string(),
        reason: "Error".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    // Test JSON serialization - optional fields should be omitted
    let json = serde_json::to_string(&step).unwrap();
    assert!(!json.contains("\"requirement\""));
    assert!(!json.contains("\"item\""));
    assert!(!json.contains("\"tc\":"));

    // Test JSON roundtrip
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement(), None);
    assert_eq!(deserialized.item(), None);
    assert_eq!(deserialized.tc(), None);
}

// ============================================================================
// StepVerificationResultEnum::NotExecuted Metadata Tests
// ============================================================================

#[test]
fn test_step_not_executed_metadata_fields() {
    let step = StepVerificationResultEnum::NotExecuted {
        step: 1,
        description: "Not executed step".to_string(),
        requirement: Some("REQ-STEP-NE-001".to_string()),
        item: Some(6),
        tc: Some(12),
    };

    assert_eq!(step.requirement(), Some(&"REQ-STEP-NE-001".to_string()));
    assert_eq!(step.item(), Some(6));
    assert_eq!(step.tc(), Some(12));
    assert!(!step.is_pass());
}

#[test]
fn test_step_not_executed_metadata_optional() {
    let step = StepVerificationResultEnum::NotExecuted {
        step: 2,
        description: "Not executed without metadata".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    assert_eq!(step.requirement(), None);
    assert_eq!(step.item(), None);
    assert_eq!(step.tc(), None);
    assert!(!step.is_pass());
}

#[test]
fn test_step_not_executed_serialization_with_metadata() {
    let step = StepVerificationResultEnum::NotExecuted {
        step: 3,
        description: "NotExecuted serialization test".to_string(),
        requirement: Some("REQ-NE-SERIAL-001".to_string()),
        item: Some(13),
        tc: Some(26),
    };

    // Test JSON serialization
    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("\"requirement\":\"REQ-NE-SERIAL-001\""));
    assert!(json.contains("\"item\":13"));
    assert!(json.contains("\"tc\":26"));
    assert!(json.contains("\"NotExecuted\""));

    // Test JSON deserialization
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-NE-SERIAL-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(13));
    assert_eq!(deserialized.tc(), Some(26));
    assert!(!deserialized.is_pass());
}

#[test]
fn test_step_not_executed_yaml_serialization() {
    let step = StepVerificationResultEnum::NotExecuted {
        step: 4,
        description: "YAML not executed test".to_string(),
        requirement: Some("REQ-NE-YAML-001".to_string()),
        item: Some(14),
        tc: Some(28),
    };

    // Test YAML serialization
    let yaml = serde_yaml::to_string(&step).unwrap();
    assert!(yaml.contains("requirement: REQ-NE-YAML-001"));
    assert!(yaml.contains("item: 14"));
    assert!(yaml.contains("tc: 28"));

    // Test YAML deserialization
    let deserialized: StepVerificationResultEnum = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(
        deserialized.requirement(),
        Some(&"REQ-NE-YAML-001".to_string())
    );
    assert_eq!(deserialized.item(), Some(14));
    assert_eq!(deserialized.tc(), Some(28));
}

#[test]
fn test_step_not_executed_serialization_without_metadata() {
    let step = StepVerificationResultEnum::NotExecuted {
        step: 5,
        description: "NotExecuted without metadata".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    // Test JSON serialization - optional fields should be omitted
    let json = serde_json::to_string(&step).unwrap();
    assert!(!json.contains("\"requirement\""));
    assert!(!json.contains("\"item\""));
    assert!(!json.contains("\"tc\":"));

    // Test JSON roundtrip
    let deserialized: StepVerificationResultEnum = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.requirement(), None);
    assert_eq!(deserialized.item(), None);
    assert_eq!(deserialized.tc(), None);
}

// ============================================================================
// Metadata Propagation Integration Tests
// ============================================================================

#[test]
fn test_metadata_propagation_from_test_case_to_results() {
    // Create a test case with metadata
    let test_case = create_test_case_with_metadata("REQ-PROP-001", 100, 200, "TC-PROP-001");

    // Verify metadata exists on test case
    assert_eq!(test_case.requirement, "REQ-PROP-001");
    assert_eq!(test_case.item, 100);
    assert_eq!(test_case.tc, 200);

    // Create verification result structures that would be built during verification
    let step_result_pass = StepVerificationResultEnum::Pass {
        step: 1,
        description: "Step 1".to_string(),
        requirement: Some(test_case.requirement.clone()),
        item: Some(test_case.item),
        tc: Some(test_case.tc),
    };

    let step_result_fail = StepVerificationResultEnum::Fail {
        step: 2,
        description: "Step 2".to_string(),
        expected: Expected {
            success: Some(true),
            result: "expected".to_string(),
            output: "output".to_string(),
        },
        actual_result: "actual".to_string(),
        actual_output: "actual_output".to_string(),
        reason: "mismatch".to_string(),
        requirement: Some(test_case.requirement.clone()),
        item: Some(test_case.item),
        tc: Some(test_case.tc),
    };

    let step_result_not_executed = StepVerificationResultEnum::NotExecuted {
        step: 3,
        description: "Step 3".to_string(),
        requirement: Some(test_case.requirement.clone()),
        item: Some(test_case.item),
        tc: Some(test_case.tc),
    };

    // Create sequence result with metadata
    let sequence_result = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![step_result_pass, step_result_fail, step_result_not_executed],
        all_steps_passed: false,
        requirement: Some(test_case.requirement.clone()),
        item: Some(test_case.item),
        tc: Some(test_case.tc),
    };

    // Create test case verification result with metadata
    let test_case_result = TestCaseVerificationResult {
        test_case_id: test_case.id.clone(),
        description: test_case.description.clone(),
        sequences: vec![sequence_result.clone()],
        total_steps: 3,
        passed_steps: 1,
        failed_steps: 1,
        not_executed_steps: 1,
        overall_pass: false,
        requirement: Some(test_case.requirement.clone()),
        item: Some(test_case.item),
        tc: Some(test_case.tc),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Verify metadata is propagated at all levels
    assert_eq!(
        test_case_result.requirement,
        Some("REQ-PROP-001".to_string())
    );
    assert_eq!(test_case_result.item, Some(100));
    assert_eq!(test_case_result.tc, Some(200));

    assert_eq!(
        sequence_result.requirement,
        Some("REQ-PROP-001".to_string())
    );
    assert_eq!(sequence_result.item, Some(100));
    assert_eq!(sequence_result.tc, Some(200));

    for step_result in &sequence_result.step_results {
        assert_eq!(step_result.requirement(), Some(&"REQ-PROP-001".to_string()));
        assert_eq!(step_result.item(), Some(100));
        assert_eq!(step_result.tc(), Some(200));
    }
}

#[test]
fn test_complete_verification_result_serialization_roundtrip_json() {
    // Create a complete verification result with metadata at all levels
    let step_results = vec![
        StepVerificationResultEnum::Pass {
            step: 1,
            description: "First step".to_string(),
            requirement: Some("REQ-RT-JSON-001".to_string()),
            item: Some(50),
            tc: Some(100),
        },
        StepVerificationResultEnum::Fail {
            step: 2,
            description: "Second step".to_string(),
            expected: Expected {
                success: Some(true),
                result: "0x9000".to_string(),
                output: "OK".to_string(),
            },
            actual_result: "0x6A82".to_string(),
            actual_output: "Error".to_string(),
            reason: "Status code mismatch".to_string(),
            requirement: Some("REQ-RT-JSON-001".to_string()),
            item: Some(50),
            tc: Some(100),
        },
        StepVerificationResultEnum::NotExecuted {
            step: 3,
            description: "Third step".to_string(),
            requirement: Some("REQ-RT-JSON-001".to_string()),
            item: Some(50),
            tc: Some(100),
        },
    ];

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "JSON Roundtrip Sequence".to_string(),
        step_results,
        all_steps_passed: false,
        requirement: Some("REQ-RT-JSON-001".to_string()),
        item: Some(50),
        tc: Some(100),
    };

    let original = TestCaseVerificationResult {
        test_case_id: "TC-RT-JSON-001".to_string(),
        description: "JSON roundtrip test".to_string(),
        sequences: vec![sequence],
        total_steps: 3,
        passed_steps: 1,
        failed_steps: 1,
        not_executed_steps: 1,
        overall_pass: false,
        requirement: Some("REQ-RT-JSON-001".to_string()),
        item: Some(50),
        tc: Some(100),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&original).unwrap();

    // Deserialize from JSON
    let deserialized: TestCaseVerificationResult = serde_json::from_str(&json).unwrap();

    // Verify all metadata fields are preserved
    assert_eq!(
        deserialized.requirement,
        Some("REQ-RT-JSON-001".to_string())
    );
    assert_eq!(deserialized.item, Some(50));
    assert_eq!(deserialized.tc, Some(100));

    assert_eq!(deserialized.sequences.len(), 1);
    let seq = &deserialized.sequences[0];
    assert_eq!(seq.requirement, Some("REQ-RT-JSON-001".to_string()));
    assert_eq!(seq.item, Some(50));
    assert_eq!(seq.tc, Some(100));

    assert_eq!(seq.step_results.len(), 3);
    for step in &seq.step_results {
        assert_eq!(step.requirement(), Some(&"REQ-RT-JSON-001".to_string()));
        assert_eq!(step.item(), Some(50));
        assert_eq!(step.tc(), Some(100));
    }
}

#[test]
fn test_complete_verification_result_serialization_roundtrip_yaml() {
    // Create a complete verification result with metadata at all levels
    let step_results = vec![
        StepVerificationResultEnum::Pass {
            step: 1,
            description: "YAML step 1".to_string(),
            requirement: Some("REQ-RT-YAML-001".to_string()),
            item: Some(60),
            tc: Some(120),
        },
        StepVerificationResultEnum::Fail {
            step: 2,
            description: "YAML step 2".to_string(),
            expected: Expected {
                success: None,
                result: "expected_value".to_string(),
                output: "expected_output".to_string(),
            },
            actual_result: "actual_value".to_string(),
            actual_output: "actual_output".to_string(),
            reason: "Value mismatch".to_string(),
            requirement: Some("REQ-RT-YAML-001".to_string()),
            item: Some(60),
            tc: Some(120),
        },
        StepVerificationResultEnum::NotExecuted {
            step: 3,
            description: "YAML step 3".to_string(),
            requirement: Some("REQ-RT-YAML-001".to_string()),
            item: Some(60),
            tc: Some(120),
        },
    ];

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "YAML Roundtrip Sequence".to_string(),
        step_results,
        all_steps_passed: false,
        requirement: Some("REQ-RT-YAML-001".to_string()),
        item: Some(60),
        tc: Some(120),
    };

    let original = TestCaseVerificationResult {
        test_case_id: "TC-RT-YAML-001".to_string(),
        description: "YAML roundtrip test".to_string(),
        sequences: vec![sequence],
        total_steps: 3,
        passed_steps: 1,
        failed_steps: 1,
        not_executed_steps: 1,
        overall_pass: false,
        requirement: Some("REQ-RT-YAML-001".to_string()),
        item: Some(60),
        tc: Some(120),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&original).unwrap();

    // Deserialize from YAML
    let deserialized: TestCaseVerificationResult = serde_yaml::from_str(&yaml).unwrap();

    // Verify all metadata fields are preserved
    assert_eq!(
        deserialized.requirement,
        Some("REQ-RT-YAML-001".to_string())
    );
    assert_eq!(deserialized.item, Some(60));
    assert_eq!(deserialized.tc, Some(120));

    assert_eq!(deserialized.sequences.len(), 1);
    let seq = &deserialized.sequences[0];
    assert_eq!(seq.requirement, Some("REQ-RT-YAML-001".to_string()));
    assert_eq!(seq.item, Some(60));
    assert_eq!(seq.tc, Some(120));

    assert_eq!(seq.step_results.len(), 3);
    for step in &seq.step_results {
        assert_eq!(step.requirement(), Some(&"REQ-RT-YAML-001".to_string()));
        assert_eq!(step.item(), Some(60));
        assert_eq!(step.tc(), Some(120));
    }
}

#[test]
fn test_mixed_metadata_presence_serialization() {
    // Test with some metadata present and some absent
    let step_with_metadata = StepVerificationResultEnum::Pass {
        step: 1,
        description: "Step with metadata".to_string(),
        requirement: Some("REQ-MIXED-001".to_string()),
        item: Some(1),
        tc: Some(1),
    };

    let step_without_metadata = StepVerificationResultEnum::Pass {
        step: 2,
        description: "Step without metadata".to_string(),
        requirement: None,
        item: None,
        tc: None,
    };

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Mixed Sequence".to_string(),
        step_results: vec![step_with_metadata, step_without_metadata],
        all_steps_passed: true,
        requirement: Some("REQ-MIXED-001".to_string()),
        item: Some(1),
        tc: Some(1),
    };

    let test_case_result = TestCaseVerificationResult {
        test_case_id: "TC-MIXED-001".to_string(),
        description: "Mixed metadata test".to_string(),
        sequences: vec![sequence],
        total_steps: 2,
        passed_steps: 2,
        failed_steps: 0,
        not_executed_steps: 0,
        overall_pass: true,
        requirement: Some("REQ-MIXED-001".to_string()),
        item: Some(1),
        tc: Some(1),
        source_yaml_sha256: None,
        run: Default::default(),
    };

    // JSON roundtrip
    let json = serde_json::to_string(&test_case_result).unwrap();
    let deserialized: TestCaseVerificationResult = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.requirement, Some("REQ-MIXED-001".to_string()));
    assert_eq!(deserialized.item, Some(1));
    assert_eq!(deserialized.tc, Some(1));

    let seq = &deserialized.sequences[0];
    assert_eq!(
        seq.step_results[0].requirement(),
        Some(&"REQ-MIXED-001".to_string())
    );
    assert_eq!(seq.step_results[0].item(), Some(1));
    assert_eq!(seq.step_results[0].tc(), Some(1));

    assert_eq!(seq.step_results[1].requirement(), None);
    assert_eq!(seq.step_results[1].item(), None);
    assert_eq!(seq.step_results[1].tc(), None);
}

#[test]
fn test_step_verification_result_enum_methods_with_metadata() {
    // Test that the helper methods work correctly with metadata
    let pass = StepVerificationResultEnum::Pass {
        step: 1,
        description: "Pass with metadata".to_string(),
        requirement: Some("REQ-METHOD-001".to_string()),
        item: Some(10),
        tc: Some(20),
    };

    assert_eq!(pass.step_number(), 1);
    assert!(pass.is_pass());
    assert_eq!(pass.requirement(), Some(&"REQ-METHOD-001".to_string()));
    assert_eq!(pass.item(), Some(10));
    assert_eq!(pass.tc(), Some(20));

    let fail = StepVerificationResultEnum::Fail {
        step: 2,
        description: "Fail with metadata".to_string(),
        expected: Expected {
            success: None,
            result: "r".to_string(),
            output: "o".to_string(),
        },
        actual_result: "ar".to_string(),
        actual_output: "ao".to_string(),
        reason: "reason".to_string(),
        requirement: Some("REQ-METHOD-002".to_string()),
        item: Some(11),
        tc: Some(21),
    };

    assert_eq!(fail.step_number(), 2);
    assert!(!fail.is_pass());
    assert_eq!(fail.requirement(), Some(&"REQ-METHOD-002".to_string()));
    assert_eq!(fail.item(), Some(11));
    assert_eq!(fail.tc(), Some(21));

    let not_executed = StepVerificationResultEnum::NotExecuted {
        step: 3,
        description: "NotExecuted with metadata".to_string(),
        requirement: Some("REQ-METHOD-003".to_string()),
        item: Some(12),
        tc: Some(22),
    };

    assert_eq!(not_executed.step_number(), 3);
    assert!(!not_executed.is_pass());
    assert_eq!(
        not_executed.requirement(),
        Some(&"REQ-METHOD-003".to_string())
    );
    assert_eq!(not_executed.item(), Some(12));
    assert_eq!(not_executed.tc(), Some(22));
}
