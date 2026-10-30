// Edge-case tests for the verifier. Selected by `make test-verifier-edge-cases`
// (`cargo test verification_edge_cases_test`).

mod verification_edge_cases_test {
    use std::path::{Path, PathBuf};
    use testcase_models::{Step, TestCase, TestSequence};
    use testcase_verification::{TestExecutionLog, TestVerifier};

    fn make_step(step_num: i64, result: &str, output: &str) -> Step {
        Step::new(
            step_num,
            format!("Step {step_num}"),
            "echo test".to_string(),
            result.to_string(),
            output.to_string(),
        )
    }

    fn make_sequence(id: i64, steps: Vec<Step>) -> TestSequence {
        let mut seq =
            TestSequence::new(id, format!("Sequence {id}"), format!("Test sequence {id}"));
        seq.steps = steps;
        seq
    }

    fn make_test_case(id: &str, sequences: Vec<TestSequence>) -> TestCase {
        let mut tc = TestCase::new(
            "REQ-001".to_string(),
            1,
            1,
            id.to_string(),
            "Test case".to_string(),
        );
        tc.test_sequences = sequences;
        tc
    }

    fn make_log(
        test_case_id: &str,
        sequence_id: i64,
        step_number: i64,
        actual_result: &str,
        actual_output: &str,
    ) -> TestExecutionLog {
        TestExecutionLog {
            test_case_id: test_case_id.to_string(),
            sequence_id,
            step_number,
            success: Some(actual_result == "0"),
            actual_result: actual_result.to_string(),
            actual_output: actual_output.to_string(),
            timestamp: None,
            log_file_path: PathBuf::from("test_execution_log.json"),
            result_verification_pass: None,
            output_verification_pass: None,
            source_yaml_sha256: None,
        }
    }

    #[test]
    fn empty_log_marks_every_step_not_executed() {
        let tc = make_test_case(
            "TC_EMPTY_LOG",
            vec![make_sequence(
                1,
                vec![make_step(1, "0", "a"), make_step(2, "0", "b")],
            )],
        );

        let result = TestVerifier::with_exact_matching().verify_test_case(&tc, &[]);

        assert!(!result.overall_pass, "no logs must never be a pass");
        assert_eq!(result.total_steps, 2);
        assert_eq!(result.passed_steps, 0);
        assert_eq!(result.failed_steps, 0);
        assert_eq!(result.not_executed_steps, 2);
    }

    #[test]
    fn test_case_without_sequences_has_no_steps() {
        let tc = make_test_case("TC_NO_SEQUENCES", vec![]);
        let logs = vec![make_log("TC_NO_SEQUENCES", 1, 1, "0", "x")];

        let result = TestVerifier::with_exact_matching().verify_test_case(&tc, &logs);

        assert_eq!(result.total_steps, 0);
        assert_eq!(result.passed_steps, 0);
        assert_eq!(result.failed_steps, 0);
        assert_eq!(result.not_executed_steps, 0);
    }

    #[test]
    fn log_entries_for_undefined_steps_are_ignored() {
        let tc = make_test_case(
            "TC_EXTRA_LOGS",
            vec![make_sequence(1, vec![make_step(1, "0", "ok")])],
        );
        let logs = vec![
            make_log("TC_EXTRA_LOGS", 1, 1, "0", "ok"),
            make_log("TC_EXTRA_LOGS", 1, 99, "1", "unexpected"),
            make_log("TC_EXTRA_LOGS", 7, 1, "1", "unexpected sequence"),
        ];

        let result = TestVerifier::with_exact_matching().verify_test_case(&tc, &logs);

        assert!(
            result.overall_pass,
            "extra log entries must not fail the run"
        );
        assert_eq!(result.total_steps, 1);
        assert_eq!(result.passed_steps, 1);
        assert_eq!(result.failed_steps, 0);
    }

    #[test]
    fn same_step_number_in_two_sequences_is_matched_per_sequence() {
        let tc = make_test_case(
            "TC_TWO_SEQUENCES",
            vec![
                make_sequence(1, vec![make_step(1, "0", "first")]),
                make_sequence(2, vec![make_step(1, "0", "second")]),
            ],
        );
        let logs = vec![
            make_log("TC_TWO_SEQUENCES", 1, 1, "0", "first"),
            make_log("TC_TWO_SEQUENCES", 2, 1, "0", "second"),
        ];

        let result = TestVerifier::with_exact_matching().verify_test_case(&tc, &logs);

        assert!(result.overall_pass);
        assert_eq!(result.total_steps, 2);
        assert_eq!(result.passed_steps, 2);
    }

    // Decision D-41f: a file with no JSON array and no text-format line is not a log. It used to
    // parse to "no entries", a failed verdict that --success-on-completion forgave.
    #[test]
    fn empty_log_content_is_not_a_log() {
        let err = TestVerifier::with_exact_matching()
            .parse_log_content("", Path::new("empty.log"))
            .unwrap_err();

        assert!(
            err.to_string().contains("not a test execution log"),
            "{err}"
        );
    }

    #[test]
    fn whitespace_only_log_content_is_not_a_log() {
        let err = TestVerifier::with_exact_matching()
            .parse_log_content("\n   \n\t\n", Path::new("blank.log"))
            .unwrap_err();

        assert!(
            err.to_string().contains("not a test execution log"),
            "{err}"
        );
    }

    #[test]
    fn an_empty_json_array_is_still_a_log_with_no_entries() {
        let logs = TestVerifier::with_exact_matching()
            .parse_log_content("[]", Path::new("empty-array.log"))
            .unwrap();

        assert!(logs.is_empty());
    }
}
