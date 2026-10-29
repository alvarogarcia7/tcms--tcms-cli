use chrono::{Local, Utc};
use testcase_manager::{
    BatchVerificationReport, SequenceVerificationResult, StepVerificationResultEnum,
    TestCaseVerificationResult,
};
use testcase_models::Expected;

/// Helper function to create a test case verification result
#[allow(clippy::too_many_arguments)]
fn create_test_case_result(
    test_case_id: &str,
    description: &str,
    total_steps: usize,
    passed_steps: usize,
    failed_steps: usize,
    not_executed_steps: usize,
    overall_pass: bool,
    requirement: Option<String>,
    item: Option<i64>,
    tc: Option<i64>,
) -> TestCaseVerificationResult {
    TestCaseVerificationResult {
        test_case_id: test_case_id.to_string(),
        description: description.to_string(),
        sequences: vec![],
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

/// Helper function to create a test case with sequences
fn create_test_case_with_sequences(
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

// ============================================================================
// BatchVerificationReport::new() Tests
// ============================================================================

#[test]
fn test_batch_verification_report_new_empty() {
    let report = BatchVerificationReport::new();

    assert_eq!(report.test_cases.len(), 0);
    assert_eq!(report.total_test_cases, 0);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 0);
    assert_eq!(report.passed_steps, 0);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
}

#[test]
fn test_batch_verification_report_new_has_timestamp() {
    let before = Local::now().with_timezone(&Utc);
    let report = BatchVerificationReport::new();
    let after = Local::now().with_timezone(&Utc);

    assert!(report.generated_at >= before);
    assert!(report.generated_at <= after);
}

#[test]
fn test_batch_verification_report_default() {
    let report = BatchVerificationReport::default();

    assert_eq!(report.total_test_cases, 0);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 0);
}

// ============================================================================
// add_test_case_result() Tests - Single Test Case
// ============================================================================

#[test]
fn test_add_test_case_result_single_passing() {
    let mut report = BatchVerificationReport::new();
    let result = create_test_case_result("TC001", "Test 1", 5, 5, 0, 0, true, None, None, None);

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 5);
    assert_eq!(report.passed_steps, 5);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
    assert_eq!(report.test_cases.len(), 1);
    assert_eq!(report.test_cases[0].test_case_id, "TC001");
}

#[test]
fn test_add_test_case_result_single_failing() {
    let mut report = BatchVerificationReport::new();
    let result = create_test_case_result("TC002", "Test 2", 5, 3, 2, 0, false, None, None, None);

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 5);
    assert_eq!(report.passed_steps, 3);
    assert_eq!(report.failed_steps, 2);
    assert_eq!(report.not_executed_steps, 0);
}

#[test]
fn test_add_test_case_result_with_not_executed_steps() {
    let mut report = BatchVerificationReport::new();
    let result = create_test_case_result("TC003", "Test 3", 10, 5, 2, 3, false, None, None, None);

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 10);
    assert_eq!(report.passed_steps, 5);
    assert_eq!(report.failed_steps, 2);
    assert_eq!(report.not_executed_steps, 3);
}

#[test]
fn test_add_test_case_result_all_steps_not_executed() {
    let mut report = BatchVerificationReport::new();
    let result = create_test_case_result("TC004", "Test 4", 3, 0, 0, 3, false, None, None, None);

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 3);
    assert_eq!(report.passed_steps, 0);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 3);
}

#[test]
fn test_add_test_case_result_empty_test_case() {
    let mut report = BatchVerificationReport::new();
    let result = create_test_case_result("TC005", "Empty test", 0, 0, 0, 0, true, None, None, None);

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 0);
    assert_eq!(report.passed_steps, 0);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
}

// ============================================================================
// add_test_case_result() Tests - Multiple Test Cases
// ============================================================================

#[test]
fn test_add_multiple_test_cases_all_passing() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 3, 3, 0, 0, true, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 4, 4, 0, 0, true, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC003", "Test 3", 5, 5, 0, 0, true, None, None, None,
    ));

    assert_eq!(report.total_test_cases, 3);
    assert_eq!(report.passed_test_cases, 3);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 12);
    assert_eq!(report.passed_steps, 12);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
}

#[test]
fn test_add_multiple_test_cases_all_failing() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 3, 2, 1, 0, false, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 4, 3, 1, 0, false, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC003", "Test 3", 5, 4, 1, 0, false, None, None, None,
    ));

    assert_eq!(report.total_test_cases, 3);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 3);
    assert_eq!(report.total_steps, 12);
    assert_eq!(report.passed_steps, 9);
    assert_eq!(report.failed_steps, 3);
    assert_eq!(report.not_executed_steps, 0);
}

#[test]
fn test_add_multiple_test_cases_mixed_results() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001",
        "Passing test",
        5,
        5,
        0,
        0,
        true,
        None,
        None,
        None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002",
        "Failing test",
        10,
        7,
        3,
        0,
        false,
        None,
        None,
        None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC003",
        "Passing test",
        3,
        3,
        0,
        0,
        true,
        None,
        None,
        None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC004",
        "Partially executed",
        8,
        5,
        1,
        2,
        false,
        None,
        None,
        None,
    ));

    assert_eq!(report.total_test_cases, 4);
    assert_eq!(report.passed_test_cases, 2);
    assert_eq!(report.failed_test_cases, 2);
    assert_eq!(report.total_steps, 26);
    assert_eq!(report.passed_steps, 20);
    assert_eq!(report.failed_steps, 4);
    assert_eq!(report.not_executed_steps, 2);
}

#[test]
fn test_add_many_test_cases_aggregation() {
    let mut report = BatchVerificationReport::new();

    for i in 0..100 {
        let passed = i % 3 == 0;
        let result = create_test_case_result(
            &format!("TC{:03}", i),
            &format!("Test {}", i),
            5,
            if passed { 5 } else { 4 },
            if passed { 0 } else { 1 },
            0,
            passed,
            None,
            None,
            None,
        );
        report.add_test_case_result(result);
    }

    assert_eq!(report.total_test_cases, 100);
    assert_eq!(report.passed_test_cases, 34);
    assert_eq!(report.failed_test_cases, 66);
    assert_eq!(report.total_steps, 500);
    assert_eq!(report.passed_steps, 500 - 66);
    assert_eq!(report.failed_steps, 66);
    assert_eq!(report.not_executed_steps, 0);
}

// ============================================================================
// Counter Aggregation Logic Tests
// ============================================================================

#[test]
fn test_counter_aggregation_incremental() {
    let mut report = BatchVerificationReport::new();

    assert_eq!(report.total_test_cases, 0);
    assert_eq!(report.total_steps, 0);

    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 10, 8, 2, 0, false, None, None, None,
    ));
    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.total_steps, 10);
    assert_eq!(report.passed_steps, 8);
    assert_eq!(report.failed_steps, 2);

    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 15, 12, 1, 2, false, None, None, None,
    ));
    assert_eq!(report.total_test_cases, 2);
    assert_eq!(report.total_steps, 25);
    assert_eq!(report.passed_steps, 20);
    assert_eq!(report.failed_steps, 3);
    assert_eq!(report.not_executed_steps, 2);

    report.add_test_case_result(create_test_case_result(
        "TC003", "Test 3", 5, 5, 0, 0, true, None, None, None,
    ));
    assert_eq!(report.total_test_cases, 3);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.failed_test_cases, 2);
    assert_eq!(report.total_steps, 30);
    assert_eq!(report.passed_steps, 25);
    assert_eq!(report.failed_steps, 3);
    assert_eq!(report.not_executed_steps, 2);
}

#[test]
fn test_counter_aggregation_edge_cases() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001",
        "All passed",
        100,
        100,
        0,
        0,
        true,
        None,
        None,
        None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002",
        "All failed",
        50,
        0,
        50,
        0,
        false,
        None,
        None,
        None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC003",
        "All not executed",
        25,
        0,
        0,
        25,
        false,
        None,
        None,
        None,
    ));

    assert_eq!(report.total_test_cases, 3);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.failed_test_cases, 2);
    assert_eq!(report.total_steps, 175);
    assert_eq!(report.passed_steps, 100);
    assert_eq!(report.failed_steps, 50);
    assert_eq!(report.not_executed_steps, 25);
}

#[test]
fn test_counter_aggregation_zero_values() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001", "Empty", 0, 0, 0, 0, true, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Empty", 0, 0, 0, 0, true, None, None, None,
    ));

    assert_eq!(report.total_test_cases, 2);
    assert_eq!(report.passed_test_cases, 2);
    assert_eq!(report.failed_test_cases, 0);
    assert_eq!(report.total_steps, 0);
    assert_eq!(report.passed_steps, 0);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
}

// ============================================================================
// Summary Statistics Tests
// ============================================================================

#[test]
fn test_summary_all_passing() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 5, 5, 0, 0, true, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 3, 3, 0, 0, true, None, None, None,
    ));

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 2/2 passed, Steps: 8/8 passed (0 failed, 0 not executed)"
    );
}

#[test]
fn test_summary_all_failing() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 5, 3, 2, 0, false, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 4, 2, 2, 0, false, None, None, None,
    ));

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 0/2 passed, Steps: 5/9 passed (4 failed, 0 not executed)"
    );
}

#[test]
fn test_summary_mixed_results() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 10, 10, 0, 0, true, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC002", "Test 2", 8, 5, 2, 1, false, None, None, None,
    ));
    report.add_test_case_result(create_test_case_result(
        "TC003", "Test 3", 5, 5, 0, 0, true, None, None, None,
    ));

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 2/3 passed, Steps: 20/23 passed (2 failed, 1 not executed)"
    );
}

#[test]
fn test_summary_empty_report() {
    let report = BatchVerificationReport::new();
    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 0/0 passed, Steps: 0/0 passed (0 failed, 0 not executed)"
    );
}

#[test]
fn test_summary_with_not_executed_only() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 5, 0, 0, 5, false, None, None, None,
    ));

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 0/1 passed, Steps: 0/5 passed (0 failed, 5 not executed)"
    );
}

#[test]
fn test_summary_large_numbers() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001",
        "Large test",
        10000,
        9500,
        400,
        100,
        false,
        None,
        None,
        None,
    ));

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 0/1 passed, Steps: 9500/10000 passed (400 failed, 100 not executed)"
    );
}

// ============================================================================
// Tests with Sequences and Detailed Step Results
// ============================================================================

#[test]
fn test_with_sequences_all_steps_passing() {
    let mut report = BatchVerificationReport::new();

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Test Sequence".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Step 1".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Pass {
                step: 2,
                description: "Step 2".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Pass {
                step: 3,
                description: "Step 3".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
        ],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let result = create_test_case_with_sequences(
        "TC001",
        "Test with sequences",
        vec![sequence],
        None,
        None,
        None,
    );

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.total_steps, 3);
    assert_eq!(report.passed_steps, 3);
    assert_eq!(report.failed_steps, 0);
    assert_eq!(report.not_executed_steps, 0);
}

#[test]
fn test_with_sequences_mixed_step_results() {
    let mut report = BatchVerificationReport::new();

    let sequence = SequenceVerificationResult {
        sequence_id: 1,
        name: "Mixed Sequence".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Step 1".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Fail {
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
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::NotExecuted {
                step: 3,
                description: "Step 3".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Pass {
                step: 4,
                description: "Step 4".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
        ],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let result = create_test_case_with_sequences(
        "TC001",
        "Test with mixed results",
        vec![sequence],
        None,
        None,
        None,
    );

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 4);
    assert_eq!(report.passed_steps, 2);
    assert_eq!(report.failed_steps, 1);
    assert_eq!(report.not_executed_steps, 1);
}

#[test]
fn test_with_multiple_sequences() {
    let mut report = BatchVerificationReport::new();

    let sequence1 = SequenceVerificationResult {
        sequence_id: 1,
        name: "Sequence 1".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Step 1".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Pass {
                step: 2,
                description: "Step 2".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
        ],
        all_steps_passed: true,
        requirement: None,
        item: None,
        tc: None,
    };

    let sequence2 = SequenceVerificationResult {
        sequence_id: 2,
        name: "Sequence 2".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Fail {
                step: 3,
                description: "Step 3".to_string(),
                expected: Expected {
                    success: None,
                    result: "expected".to_string(),
                    output: "output".to_string(),
                },
                actual_result: "actual".to_string(),
                actual_output: "actual_output".to_string(),
                reason: "mismatch".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::NotExecuted {
                step: 4,
                description: "Step 4".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
        ],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let result = create_test_case_with_sequences(
        "TC001",
        "Test with multiple sequences",
        vec![sequence1, sequence2],
        None,
        None,
        None,
    );

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.total_steps, 4);
    assert_eq!(report.passed_steps, 2);
    assert_eq!(report.failed_steps, 1);
    assert_eq!(report.not_executed_steps, 1);
}

#[test]
fn test_multiple_test_cases_with_sequences() {
    let mut report = BatchVerificationReport::new();

    let seq1 = SequenceVerificationResult {
        sequence_id: 1,
        name: "Sequence 1".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Step 1".to_string(),
                requirement: Some("REQ001".to_string()),
                item: Some(1),
                tc: Some(1),
            },
            StepVerificationResultEnum::Pass {
                step: 2,
                description: "Step 2".to_string(),
                requirement: Some("REQ002".to_string()),
                item: Some(1),
                tc: Some(1),
            },
        ],
        all_steps_passed: true,
        requirement: Some("REQ-SEQ1".to_string()),
        item: Some(1),
        tc: Some(1),
    };

    let result1 = create_test_case_with_sequences(
        "TC001",
        "First test case",
        vec![seq1],
        Some("REQ-TC001".to_string()),
        Some(1),
        Some(1),
    );

    let seq2 = SequenceVerificationResult {
        sequence_id: 1,
        name: "Sequence 1".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Step 1".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Fail {
                step: 2,
                description: "Step 2".to_string(),
                expected: Expected {
                    success: Some(false),
                    result: "FAIL".to_string(),
                    output: "error".to_string(),
                },
                actual_result: "OK".to_string(),
                actual_output: "success".to_string(),
                reason: "Expected failure but got success".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
            StepVerificationResultEnum::Pass {
                step: 3,
                description: "Step 3".to_string(),
                requirement: None,
                item: None,
                tc: None,
            },
        ],
        all_steps_passed: false,
        requirement: None,
        item: None,
        tc: None,
    };

    let result2 =
        create_test_case_with_sequences("TC002", "Second test case", vec![seq2], None, None, None);

    report.add_test_case_result(result1);
    report.add_test_case_result(result2);

    assert_eq!(report.total_test_cases, 2);
    assert_eq!(report.passed_test_cases, 1);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 5);
    assert_eq!(report.passed_steps, 4);
    assert_eq!(report.failed_steps, 1);
    assert_eq!(report.not_executed_steps, 0);

    let summary = report.summary();
    assert_eq!(
        summary,
        "Test Cases: 1/2 passed, Steps: 4/5 passed (1 failed, 0 not executed)"
    );
}

// ============================================================================
// Complex Aggregation Scenarios
// ============================================================================

#[test]
fn test_complex_scenario_mixed_test_cases() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001",
        "All passing",
        20,
        20,
        0,
        0,
        true,
        Some("REQ001".to_string()),
        Some(1),
        Some(1),
    ));

    report.add_test_case_result(create_test_case_result(
        "TC002",
        "Some failures",
        15,
        12,
        3,
        0,
        false,
        Some("REQ002".to_string()),
        Some(2),
        Some(2),
    ));

    report.add_test_case_result(create_test_case_result(
        "TC003",
        "Partially executed",
        25,
        18,
        2,
        5,
        false,
        Some("REQ003".to_string()),
        Some(3),
        Some(3),
    ));

    report.add_test_case_result(create_test_case_result(
        "TC004",
        "All passing",
        10,
        10,
        0,
        0,
        true,
        Some("REQ004".to_string()),
        Some(4),
        Some(4),
    ));

    report.add_test_case_result(create_test_case_result(
        "TC005",
        "Mixed results",
        30,
        25,
        4,
        1,
        false,
        Some("REQ005".to_string()),
        Some(5),
        Some(5),
    ));

    assert_eq!(report.total_test_cases, 5);
    assert_eq!(report.passed_test_cases, 2);
    assert_eq!(report.failed_test_cases, 3);
    assert_eq!(report.total_steps, 100);
    assert_eq!(report.passed_steps, 85);
    assert_eq!(report.failed_steps, 9);
    assert_eq!(report.not_executed_steps, 6);

    assert_eq!(report.test_cases.len(), 5);
    assert_eq!(report.test_cases[0].test_case_id, "TC001");
    assert_eq!(report.test_cases[4].test_case_id, "TC005");
}

#[test]
fn test_complex_scenario_with_sequences_and_metadata() {
    let mut report = BatchVerificationReport::new();

    let seq1 = SequenceVerificationResult {
        sequence_id: 1,
        name: "Setup".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 1,
                description: "Initialize".to_string(),
                requirement: Some("REQ-INIT".to_string()),
                item: Some(1),
                tc: Some(100),
            },
            StepVerificationResultEnum::Pass {
                step: 2,
                description: "Configure".to_string(),
                requirement: Some("REQ-CONFIG".to_string()),
                item: Some(1),
                tc: Some(100),
            },
        ],
        all_steps_passed: true,
        requirement: Some("REQ-SETUP".to_string()),
        item: Some(1),
        tc: Some(100),
    };

    let seq2 = SequenceVerificationResult {
        sequence_id: 2,
        name: "Execution".to_string(),
        step_results: vec![
            StepVerificationResultEnum::Pass {
                step: 3,
                description: "Execute".to_string(),
                requirement: Some("REQ-EXEC".to_string()),
                item: Some(2),
                tc: Some(100),
            },
            StepVerificationResultEnum::Fail {
                step: 4,
                description: "Verify".to_string(),
                expected: Expected {
                    success: Some(true),
                    result: "0x9000".to_string(),
                    output: "Success".to_string(),
                },
                actual_result: "0x6A82".to_string(),
                actual_output: "Security condition not satisfied".to_string(),
                reason: "Status word mismatch".to_string(),
                requirement: Some("REQ-VERIFY".to_string()),
                item: Some(2),
                tc: Some(100),
            },
        ],
        all_steps_passed: false,
        requirement: Some("REQ-EXEC-SEQ".to_string()),
        item: Some(2),
        tc: Some(100),
    };

    let seq3 = SequenceVerificationResult {
        sequence_id: 3,
        name: "Cleanup".to_string(),
        step_results: vec![StepVerificationResultEnum::NotExecuted {
            step: 5,
            description: "Cleanup resources".to_string(),
            requirement: Some("REQ-CLEANUP".to_string()),
            item: Some(3),
            tc: Some(100),
        }],
        all_steps_passed: false,
        requirement: Some("REQ-CLEANUP-SEQ".to_string()),
        item: Some(3),
        tc: Some(100),
    };

    let result = create_test_case_with_sequences(
        "TC100",
        "Complex test with metadata",
        vec![seq1, seq2, seq3],
        Some("REQ-TOP".to_string()),
        Some(100),
        Some(100),
    );

    report.add_test_case_result(result);

    assert_eq!(report.total_test_cases, 1);
    assert_eq!(report.passed_test_cases, 0);
    assert_eq!(report.failed_test_cases, 1);
    assert_eq!(report.total_steps, 5);
    assert_eq!(report.passed_steps, 3);
    assert_eq!(report.failed_steps, 1);
    assert_eq!(report.not_executed_steps, 1);

    assert_eq!(
        report.test_cases[0].requirement,
        Some("REQ-TOP".to_string())
    );
    assert_eq!(report.test_cases[0].item, Some(100));
    assert_eq!(report.test_cases[0].tc, Some(100));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_batch_report_json_serialization() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 5, 5, 0, 0, true, None, None, None,
    ));

    let json = serde_json::to_string_pretty(&report).unwrap();
    let deserialized: BatchVerificationReport = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.total_test_cases, report.total_test_cases);
    assert_eq!(deserialized.passed_test_cases, report.passed_test_cases);
    assert_eq!(deserialized.total_steps, report.total_steps);
    assert_eq!(deserialized.passed_steps, report.passed_steps);
}

#[test]
fn test_batch_report_yaml_serialization() {
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 3, 2, 1, 0, false, None, None, None,
    ));

    let yaml = serde_yaml::to_string(&report).unwrap();
    let deserialized: BatchVerificationReport = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(deserialized.total_test_cases, report.total_test_cases);
    assert_eq!(deserialized.failed_test_cases, report.failed_test_cases);
    assert_eq!(deserialized.failed_steps, report.failed_steps);
}

// ============================================================================
// Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_very_large_step_counts() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001",
        "Large test",
        usize::MAX / 4,
        usize::MAX / 4,
        0,
        0,
        true,
        None,
        None,
        None,
    ));

    assert_eq!(report.total_steps, usize::MAX / 4);
    assert_eq!(report.passed_steps, usize::MAX / 4);
}

#[test]
fn test_test_case_order_preserved() {
    let mut report = BatchVerificationReport::new();

    for i in 0..10 {
        report.add_test_case_result(create_test_case_result(
            &format!("TC{:03}", i),
            &format!("Test {}", i),
            1,
            1,
            0,
            0,
            true,
            None,
            None,
            None,
        ));
    }

    for i in 0..10 {
        assert_eq!(report.test_cases[i].test_case_id, format!("TC{:03}", i));
    }
}

#[test]
fn test_report_with_special_characters_in_ids() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC-001_test.case",
        "Test with special chars: <>\"'&",
        5,
        5,
        0,
        0,
        true,
        Some("REQ-001_requirement".to_string()),
        Some(123),
        Some(456),
    ));

    assert_eq!(report.test_cases[0].test_case_id, "TC-001_test.case");
    assert_eq!(
        report.test_cases[0].description,
        "Test with special chars: <>\"'&"
    );
}

#[test]
fn test_summary_formatting_consistency() {
    let mut report = BatchVerificationReport::new();

    report.add_test_case_result(create_test_case_result(
        "TC001", "Test 1", 100, 50, 25, 25, false, None, None, None,
    ));

    let summary = report.summary();

    assert!(summary.contains("Test Cases:"));
    assert!(summary.contains("Steps:"));
    assert!(summary.contains("passed"));
    assert!(summary.contains("failed"));
    assert!(summary.contains("not executed"));
}
