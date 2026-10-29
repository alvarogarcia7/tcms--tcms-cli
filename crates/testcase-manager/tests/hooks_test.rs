use testcase_manager::TestExecutor;
use testcase_models::{
    Expected, HookConfig, HookType, Hooks, InitialConditions, OnError, Step, TestCase,
    TestSequence, TestStepExecutionEntry, Verification, VerificationExpression,
};

/// Helper to create a minimal test case for hooks testing
fn create_test_case_with_hooks(hooks: Option<Hooks>) -> TestCase {
    let step = Step {
        step: 1,
        manual: None,
        description: "Test step".to_string(),
        command: "echo 'test'".to_string(),
        capture_vars: None,
        expected: Expected {
            success: Some(true),
            result: "0".to_string(),
            output: "test".to_string(),
        },
        verification: Verification {
            result: VerificationExpression::Simple("[ $? -eq 0 ]".to_string()),
            output: VerificationExpression::Simple(
                "[ \"$COMMAND_OUTPUT\" = \"test\" ]".to_string(),
            ),
            output_file: None,
            general: None,
        },
        reference: None,
    };

    let sequence = TestSequence {
        id: 1,
        name: "Test Sequence".to_string(),
        description: "Test Description".to_string(),
        variables: None,
        initial_conditions: InitialConditions::default(),
        steps: vec![step],
        reference: None,
    };

    let mut test_case = TestCase::new(
        "REQ001".to_string(),
        1,
        1,
        "TC001".to_string(),
        "Test case with hooks".to_string(),
    );
    test_case.test_sequences = vec![sequence];
    test_case.hooks = hooks;
    test_case
}

// ============================================================================
// Hook Deserialization Tests
// ============================================================================

#[test]
fn test_hooks_deserialization_all_fields() {
    let yaml = r#"
script_start:
  command: "scripts/start.sh"
  on_error: fail
setup_test:
  command: "scripts/setup.sh"
  on_error: continue
before_sequence:
  command: "scripts/before_seq.sh"
  on_error: fail
after_sequence:
  command: "scripts/after_seq.sh"
  on_error: continue
before_step:
  command: "scripts/before_step.sh"
  on_error: fail
after_step:
  command: "scripts/after_step.sh"
  on_error: continue
teardown_test:
  command: "scripts/teardown.sh"
  on_error: fail
script_end:
  command: "scripts/end.sh"
  on_error: continue
"#;

    let hooks: Hooks = serde_yaml::from_str(yaml).unwrap();

    assert!(hooks.script_start.is_some());
    assert!(hooks.setup_test.is_some());
    assert!(hooks.before_sequence.is_some());
    assert!(hooks.after_sequence.is_some());
    assert!(hooks.before_step.is_some());
    assert!(hooks.after_step.is_some());
    assert!(hooks.teardown_test.is_some());
    assert!(hooks.script_end.is_some());

    // Verify script_start
    let script_start = hooks.script_start.unwrap();
    assert_eq!(script_start.command, "scripts/start.sh");
    assert_eq!(script_start.on_error, Some(OnError::Fail));

    // Verify setup_test
    let setup_test = hooks.setup_test.unwrap();
    assert_eq!(setup_test.command, "scripts/setup.sh");
    assert_eq!(setup_test.on_error, Some(OnError::Continue));
}

#[test]
fn test_hooks_deserialization_minimal() {
    let yaml = r#"
before_step:
  command: "scripts/hook.sh"
"#;

    let hooks: Hooks = serde_yaml::from_str(yaml).unwrap();

    assert!(hooks.script_start.is_none());
    assert!(hooks.before_step.is_some());

    let before_step = hooks.before_step.unwrap();
    assert_eq!(before_step.command, "scripts/hook.sh");
    assert_eq!(before_step.on_error, None);
}

#[test]
fn test_hooks_deserialization_empty() {
    let yaml = r#"{}"#;
    let hooks: Hooks = serde_yaml::from_str(yaml).unwrap();

    assert!(hooks.script_start.is_none());
    assert!(hooks.setup_test.is_none());
    assert!(hooks.before_sequence.is_none());
    assert!(hooks.after_sequence.is_none());
    assert!(hooks.before_step.is_none());
    assert!(hooks.after_step.is_none());
    assert!(hooks.teardown_test.is_none());
    assert!(hooks.script_end.is_none());
}

#[test]
fn test_on_error_deserialization() {
    let yaml_fail = r#"fail"#;
    let on_error: OnError = serde_yaml::from_str(yaml_fail).unwrap();
    assert_eq!(on_error, OnError::Fail);

    let yaml_continue = r#"continue"#;
    let on_error: OnError = serde_yaml::from_str(yaml_continue).unwrap();
    assert_eq!(on_error, OnError::Continue);
}

#[test]
fn test_hook_config_deserialization() {
    let yaml = r#"
command: "scripts/test.sh"
on_error: fail
"#;

    let config: HookConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.command, "scripts/test.sh");
    assert_eq!(config.on_error, Some(OnError::Fail));
}

#[test]
fn test_hook_config_without_on_error() {
    let yaml = r#"
command: "scripts/test.sh"
"#;

    let config: HookConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.command, "scripts/test.sh");
    assert_eq!(config.on_error, None);
}

// ============================================================================
// Hook Type Tests
// ============================================================================

#[test]
fn test_hook_type_serialization() {
    assert_eq!(
        serde_yaml::to_string(&HookType::ScriptStart)
            .unwrap()
            .trim(),
        "script_start"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::SetupTest).unwrap().trim(),
        "setup_test"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::BeforeSequence)
            .unwrap()
            .trim(),
        "before_sequence"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::AfterSequence)
            .unwrap()
            .trim(),
        "after_sequence"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::BeforeStep).unwrap().trim(),
        "before_step"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::AfterStep).unwrap().trim(),
        "after_step"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::TeardownTest)
            .unwrap()
            .trim(),
        "teardown_test"
    );
    assert_eq!(
        serde_yaml::to_string(&HookType::ScriptEnd).unwrap().trim(),
        "script_end"
    );
}

#[test]
fn test_hook_type_deserialization() {
    let hook_type: HookType = serde_yaml::from_str("script_start").unwrap();
    assert_eq!(hook_type, HookType::ScriptStart);

    let hook_type: HookType = serde_yaml::from_str("setup_test").unwrap();
    assert_eq!(hook_type, HookType::SetupTest);

    let hook_type: HookType = serde_yaml::from_str("before_sequence").unwrap();
    assert_eq!(hook_type, HookType::BeforeSequence);

    let hook_type: HookType = serde_yaml::from_str("after_sequence").unwrap();
    assert_eq!(hook_type, HookType::AfterSequence);

    let hook_type: HookType = serde_yaml::from_str("before_step").unwrap();
    assert_eq!(hook_type, HookType::BeforeStep);

    let hook_type: HookType = serde_yaml::from_str("after_step").unwrap();
    assert_eq!(hook_type, HookType::AfterStep);

    let hook_type: HookType = serde_yaml::from_str("teardown_test").unwrap();
    assert_eq!(hook_type, HookType::TeardownTest);

    let hook_type: HookType = serde_yaml::from_str("script_end").unwrap();
    assert_eq!(hook_type, HookType::ScriptEnd);
}

#[test]
fn test_hook_type_display() {
    assert_eq!(format!("{}", HookType::ScriptStart), "script_start");
    assert_eq!(format!("{}", HookType::SetupTest), "setup_test");
    assert_eq!(format!("{}", HookType::BeforeSequence), "before_sequence");
    assert_eq!(format!("{}", HookType::AfterSequence), "after_sequence");
    assert_eq!(format!("{}", HookType::BeforeStep), "before_step");
    assert_eq!(format!("{}", HookType::AfterStep), "after_step");
    assert_eq!(format!("{}", HookType::TeardownTest), "teardown_test");
    assert_eq!(format!("{}", HookType::ScriptEnd), "script_end");
}

// ============================================================================
// Script Generation Tests
// ============================================================================

#[test]
fn test_script_generation_with_all_hooks() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/start.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        setup_test: Some(HookConfig {
            command: "scripts/setup.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        before_sequence: Some(HookConfig {
            command: "scripts/before_seq.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        after_sequence: Some(HookConfig {
            command: "scripts/after_seq.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        before_step: Some(HookConfig {
            command: "scripts/before_step.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        after_step: Some(HookConfig {
            command: "scripts/after_step.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        teardown_test: Some(HookConfig {
            command: "scripts/teardown.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        script_end: Some(HookConfig {
            command: "scripts/end.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify all hooks are present in script
    assert!(script.contains("# Execute script_start hook"));
    assert!(script.contains("scripts/start.sh"));

    assert!(script.contains("# Execute setup_test hook"));
    assert!(script.contains("scripts/setup.sh"));

    assert!(script.contains("# Execute before_sequence hook"));
    assert!(script.contains("scripts/before_seq.sh"));

    assert!(script.contains("# Execute after_sequence hook"));
    assert!(script.contains("scripts/after_seq.sh"));

    assert!(script.contains("# Execute before_step hook"));
    assert!(script.contains("scripts/before_step.sh"));

    assert!(script.contains("# Execute after_step hook"));
    assert!(script.contains("scripts/after_step.sh"));

    assert!(script.contains("# Execute teardown_test hook"));
    assert!(script.contains("scripts/teardown.sh"));

    assert!(script.contains("# Execute script_end hook"));
    assert!(script.contains("scripts/end.sh"));
}

#[test]
fn test_script_generation_with_no_hooks() {
    let test_case = create_test_case_with_hooks(None);
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify no hook execution code is present
    assert!(!script.contains("# Execute script_start hook"));
    assert!(!script.contains("# Execute setup_test hook"));
    assert!(!script.contains("# Execute before_sequence hook"));
    assert!(!script.contains("# Execute after_sequence hook"));
    assert!(!script.contains("# Execute before_step hook"));
    assert!(!script.contains("# Execute after_step hook"));
    assert!(!script.contains("# Execute teardown_test hook"));
    assert!(!script.contains("# Execute script_end hook"));
}

#[test]
fn test_script_generation_with_selective_hooks() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/start.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        before_step: Some(HookConfig {
            command: "scripts/before_step.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        script_end: Some(HookConfig {
            command: "scripts/end.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify selected hooks are present
    assert!(script.contains("# Execute script_start hook"));
    assert!(script.contains("scripts/start.sh"));
    assert!(script.contains("# Execute before_step hook"));
    assert!(script.contains("scripts/before_step.sh"));
    assert!(script.contains("# Execute script_end hook"));
    assert!(script.contains("scripts/end.sh"));

    // Verify non-selected hooks are absent
    assert!(!script.contains("# Execute setup_test hook"));
    assert!(!script.contains("# Execute before_sequence hook"));
    assert!(!script.contains("# Execute after_sequence hook"));
    assert!(!script.contains("# Execute after_step hook"));
    assert!(!script.contains("# Execute teardown_test hook"));
}

// ============================================================================
// On Error Behavior Tests
// ============================================================================

#[test]
fn test_hook_fail_on_error_script_generation() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify fail behavior generates exit on error
    assert!(script.contains("# Execute before_step hook"));
    assert!(script.contains("scripts/hook.sh"));
    assert!(script.contains("Error: before_step hook failed"));
    assert!(script.contains("exit $HOOK_EXIT_CODE"));
}

#[test]
fn test_hook_continue_on_error_script_generation() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify continue behavior generates warning but no exit
    assert!(script.contains("# Execute before_step hook"));
    assert!(script.contains("scripts/hook.sh"));
    assert!(script.contains("Warning: before_step hook failed"));
    assert!(script.contains("(continuing)"));

    // Should not contain exit statement in continue mode
    let hook_section_start = script.find("# Execute before_step hook").unwrap();
    let hook_section = &script[hook_section_start..];
    let next_step = hook_section.find("# Step").unwrap_or(hook_section.len());
    let hook_code = &hook_section[..next_step];

    // Check that within the hook code, there's no "exit $HOOK_EXIT_CODE" after the warning
    assert!(!hook_code.contains("exit $HOOK_EXIT_CODE"));
}

#[test]
fn test_hook_default_on_error_behavior() {
    // When on_error is None, it should default to Fail behavior
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: None,
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Default should be fail behavior
    assert!(script.contains("# Execute before_step hook"));
    assert!(script.contains("Error: before_step hook failed"));
    assert!(script.contains("exit $HOOK_EXIT_CODE"));
}

// ============================================================================
// Hook Execution with Bash Code Validation Tests
// ============================================================================

#[test]
fn test_hook_bash_code_structure_sh_file() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/test.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // .sh files should be sourced
    assert!(script.contains("if [ -f \"scripts/test.sh\" ]; then"));
    assert!(script.contains("source \"scripts/test.sh\""));
    assert!(script.contains("HOOK_EXIT_CODE=$?"));
}

#[test]
fn test_hook_bash_code_structure_non_sh_file() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/test.py".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Non-.sh files should be executed directly
    assert!(script.contains("scripts/test.py"));
    assert!(script.contains("HOOK_EXIT_CODE=$?"));
    // Should not be sourced
    assert!(!script.contains("source \"scripts/test.py\""));
}

#[test]
fn test_hook_bash_code_missing_file_handling() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/nonexistent.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Should check for file existence
    assert!(script.contains("if [ -f \"scripts/nonexistent.sh\" ]; then"));
    assert!(script.contains("Warning: Hook script 'scripts/nonexistent.sh' not found"));
    assert!(script.contains("HOOK_EXIT_CODE=127"));
}

#[test]
fn test_hook_bash_code_set_unset_errexit() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Find the hook execution section
    let hook_start = script.find("# Execute before_step hook").unwrap();
    let hook_section = &script[hook_start..];
    let hook_end = hook_section.find("\n\n").unwrap_or(hook_section.len());
    let hook_code = &hook_section[..hook_end];

    // Should disable errexit before running hook, then re-enable it
    assert!(hook_code.contains("set +e"));
    assert!(hook_code.contains("set -e"));

    // Verify order: set +e comes before the hook execution
    let set_plus_e_pos = hook_code.find("set +e").unwrap();
    let source_pos = hook_code
        .find("source")
        .or_else(|| hook_code.find("scripts/hook.sh"))
        .unwrap();
    assert!(set_plus_e_pos < source_pos);
}

// ============================================================================
// Environment Variable Passing Tests
// ============================================================================

#[test]
fn test_hook_environment_variables_sequence_level() {
    let hooks = Hooks {
        before_sequence: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Find the before_sequence hook section
    let hook_pos = script.find("# Execute before_sequence hook").unwrap();
    let before_hook = &script[..hook_pos];

    // Should set TEST_SEQUENCE_ID and TEST_SEQUENCE_NAME before hook execution
    assert!(before_hook.contains("TEST_SEQUENCE_ID=1"));
    assert!(before_hook.contains("TEST_SEQUENCE_NAME="));
    assert!(before_hook.contains("export TEST_SEQUENCE_ID TEST_SEQUENCE_NAME"));
}

#[test]
fn test_hook_environment_variables_step_level() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Find the before_step hook section
    let hook_pos = script.find("# Execute before_step hook").unwrap();
    let before_hook = &script[..hook_pos];

    // Should set TEST_STEP_NUMBER and TEST_STEP_DESCRIPTION before hook execution
    assert!(before_hook.contains("TEST_STEP_NUMBER=1"));
    assert!(before_hook.contains("TEST_STEP_DESCRIPTION="));
    assert!(before_hook.contains("export TEST_STEP_NUMBER TEST_STEP_DESCRIPTION"));
}

#[test]
fn test_hook_test_case_id_available() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Test case ID should be in the script as a comment
    assert!(script.contains("# Test Case: TC001"));
}

// ============================================================================
// Hook Log Entry Serialization Tests
// ============================================================================

#[test]
fn test_hook_execution_entry_with_hook_type() {
    let entry = TestStepExecutionEntry {
        doc_type: None,
        schema: None,
        test_sequence: 1,
        step: 2,
        command: "scripts/hook.sh".to_string(),
        exit_code: 0,
        output: "hook output".to_string(),
        timestamp: Some("2024-01-01T00:00:00Z".to_string()),
        hook_type: Some(HookType::BeforeStep),
        hook_path: Some("scripts/hook.sh".to_string()),
        result_verification_pass: false,
        output_verification_pass: false,
        run_id: None,
        test_case_id: None,
    };

    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("\"hook_type\":\"before_step\""));
    assert!(json.contains("\"hook_path\":\"scripts/hook.sh\""));

    // Deserialize and verify
    let deserialized: TestStepExecutionEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.hook_type, Some(HookType::BeforeStep));
    assert_eq!(deserialized.hook_path, Some("scripts/hook.sh".to_string()));
}

#[test]
fn test_hook_execution_entry_display_with_hook() {
    let entry = TestStepExecutionEntry {
        doc_type: None,
        schema: None,
        test_sequence: 1,
        step: 2,
        command: "scripts/hook.sh".to_string(),
        exit_code: 0,
        output: "output".to_string(),
        timestamp: None,
        hook_type: Some(HookType::AfterSequence),
        hook_path: Some("scripts/after.sh".to_string()),
        result_verification_pass: false,
        output_verification_pass: false,
        run_id: None,
        test_case_id: None,
    };

    let display = format!("{}", entry);
    assert!(display.contains("Hook: after_sequence"));
    assert!(display.contains("path: scripts/after.sh"));
}

#[test]
fn test_hook_execution_entry_yaml_roundtrip() {
    let entry = TestStepExecutionEntry {
        doc_type: None,
        schema: None,
        test_sequence: 1,
        step: 1,
        command: "scripts/hook.sh".to_string(),
        exit_code: 0,
        output: "success".to_string(),
        timestamp: Some("2024-01-01T12:00:00Z".to_string()),
        hook_type: Some(HookType::SetupTest),
        hook_path: Some("scripts/setup.sh".to_string()),
        result_verification_pass: false,
        output_verification_pass: false,
        run_id: None,
        test_case_id: None,
    };

    let yaml = serde_yaml::to_string(&entry).unwrap();
    let deserialized: TestStepExecutionEntry = serde_yaml::from_str(&yaml).unwrap();

    assert_eq!(entry, deserialized);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_hook_with_special_characters_in_path() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/with space/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Path should be properly quoted
    assert!(script.contains("\"scripts/with space/hook.sh\""));
}

#[test]
fn test_hook_with_absolute_path() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "/usr/local/bin/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    assert!(script.contains("/usr/local/bin/hook.sh"));
}

#[test]
fn test_hook_with_command_not_script() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "echo 'inline hook'".to_string(),
            on_error: Some(OnError::Continue),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Inline commands should be executed, not sourced
    assert!(script.contains("echo 'inline hook'"));
    assert!(!script.contains("source \"echo 'inline hook'\""));
}

#[test]
fn test_multiple_hooks_same_type_not_allowed() {
    // This test verifies that the Hooks struct only allows one hook per type
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook1.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    // You cannot have two before_step hooks in the same Hooks struct
    // This is enforced by the struct design
    assert!(hooks.before_step.is_some());
}

#[test]
fn test_hooks_serialization_skip_none_fields() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/start.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let yaml = serde_yaml::to_string(&hooks).unwrap();

    // Only script_start should be serialized
    assert!(yaml.contains("script_start"));

    // Other fields should not appear
    assert!(!yaml.contains("setup_test"));
    assert!(!yaml.contains("before_sequence"));
    assert!(!yaml.contains("after_sequence"));
    assert!(!yaml.contains("before_step"));
    assert!(!yaml.contains("after_step"));
    assert!(!yaml.contains("teardown_test"));
    assert!(!yaml.contains("script_end"));
}

#[test]
fn test_test_case_with_hooks_yaml_roundtrip() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/start.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        before_step: Some(HookConfig {
            command: "scripts/before.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));

    let yaml = serde_yaml::to_string(&test_case).unwrap();
    let deserialized: TestCase = serde_yaml::from_str(&yaml).unwrap();

    assert!(deserialized.hooks.is_some());
    let deserialized_hooks = deserialized.hooks.unwrap();
    assert!(deserialized_hooks.script_start.is_some());
    assert!(deserialized_hooks.before_step.is_some());

    let script_start = deserialized_hooks.script_start.unwrap();
    assert_eq!(script_start.command, "scripts/start.sh");
    assert_eq!(script_start.on_error, Some(OnError::Fail));
}

#[test]
fn test_hook_execution_order_in_script() {
    let hooks = Hooks {
        script_start: Some(HookConfig {
            command: "scripts/start.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        setup_test: Some(HookConfig {
            command: "scripts/setup.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        before_sequence: Some(HookConfig {
            command: "scripts/before_seq.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        before_step: Some(HookConfig {
            command: "scripts/before_step.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        after_step: Some(HookConfig {
            command: "scripts/after_step.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        after_sequence: Some(HookConfig {
            command: "scripts/after_seq.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        teardown_test: Some(HookConfig {
            command: "scripts/teardown.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        script_end: Some(HookConfig {
            command: "scripts/end.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify hook execution order
    let script_start_pos = script.find("# Execute script_start hook").unwrap();
    let setup_test_pos = script.find("# Execute setup_test hook").unwrap();
    let before_sequence_pos = script.find("# Execute before_sequence hook").unwrap();
    let before_step_pos = script.find("# Execute before_step hook").unwrap();
    let after_step_pos = script.find("# Execute after_step hook").unwrap();
    let after_sequence_pos = script.find("# Execute after_sequence hook").unwrap();
    let teardown_test_pos = script.find("# Execute teardown_test hook").unwrap();
    let script_end_pos = script.find("# Execute script_end hook").unwrap();

    // Verify correct order
    assert!(script_start_pos < setup_test_pos);
    assert!(setup_test_pos < before_sequence_pos);
    assert!(before_sequence_pos < before_step_pos);
    assert!(before_step_pos < after_step_pos);
    assert!(after_step_pos < after_sequence_pos);
    assert!(after_sequence_pos < teardown_test_pos);
    assert!(teardown_test_pos < script_end_pos);
}

#[test]
fn test_hook_with_non_zero_exit_code_fail_mode() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/failing_hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // In fail mode, hook failure should exit the script
    let hook_start = script.find("# Execute before_step hook").unwrap();
    let hook_section = &script[hook_start..];

    assert!(hook_section.contains("if [ $HOOK_EXIT_CODE -ne 0 ]; then"));
    assert!(hook_section.contains("Error: before_step hook failed"));
    assert!(hook_section.contains("exit $HOOK_EXIT_CODE"));
}

#[test]
fn test_hook_with_non_zero_exit_code_continue_mode() {
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/failing_hook.sh".to_string(),
            on_error: Some(OnError::Continue),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // In continue mode, hook failure should log warning but not exit
    let hook_start = script.find("# Execute before_step hook").unwrap();
    let hook_section = &script[hook_start..];

    assert!(hook_section.contains("if [ $HOOK_EXIT_CODE -ne 0 ]; then"));
    assert!(hook_section.contains("Warning: before_step hook failed"));
    assert!(hook_section.contains("(continuing)"));
}

#[test]
fn test_hook_execution_with_missing_file() {
    let hooks = Hooks {
        before_sequence: Some(HookConfig {
            command: "scripts/missing.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Should handle missing file gracefully
    assert!(script.contains("if [ -f \"scripts/missing.sh\" ]; then"));
    assert!(script.contains("Warning: Hook script 'scripts/missing.sh' not found"));
    assert!(script.contains("HOOK_EXIT_CODE=127"));
}

#[test]
fn test_hook_non_executable_handling() {
    // Non-executable hooks should still be handled by bash
    // .sh files are sourced, others are executed
    let hooks = Hooks {
        before_step: Some(HookConfig {
            command: "scripts/hook.sh".to_string(),
            on_error: Some(OnError::Fail),
        }),
        ..Default::default()
    };

    let test_case = create_test_case_with_hooks(Some(hooks));
    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // .sh files are sourced, so they don't need execute permission
    assert!(script.contains("source \"scripts/hook.sh\""));
}

#[test]
fn test_hook_execution_entry_without_hook_type() {
    let entry = TestStepExecutionEntry {
        doc_type: None,
        schema: None,
        test_sequence: 1,
        step: 1,
        command: "echo test".to_string(),
        exit_code: 0,
        output: "test".to_string(),
        timestamp: None,
        hook_type: None,
        hook_path: None,
        result_verification_pass: true,
        output_verification_pass: true,
        run_id: None,
        test_case_id: None,
    };

    let json = serde_json::to_string(&entry).unwrap();

    // hook_type and hook_path should not be in JSON when None
    assert!(!json.contains("hook_type"));
    assert!(!json.contains("hook_path"));
}

#[test]
fn test_hooks_default() {
    let hooks = Hooks::default();

    assert!(hooks.script_start.is_none());
    assert!(hooks.setup_test.is_none());
    assert!(hooks.before_sequence.is_none());
    assert!(hooks.after_sequence.is_none());
    assert!(hooks.before_step.is_none());
    assert!(hooks.after_step.is_none());
    assert!(hooks.teardown_test.is_none());
    assert!(hooks.script_end.is_none());
}

#[test]
fn test_test_case_without_hooks_field() {
    let yaml = r#"
requirement: REQ001
item: 1
tc: 1
id: TC001
description: Test case without hooks
general_initial_conditions: {}
initial_conditions: {}
test_sequences: []
"#;

    let test_case: TestCase = serde_yaml::from_str(yaml).unwrap();
    assert!(test_case.hooks.is_none());
}

#[test]
fn test_hook_config_equality() {
    let config1 = HookConfig {
        command: "scripts/hook.sh".to_string(),
        on_error: Some(OnError::Fail),
    };

    let config2 = HookConfig {
        command: "scripts/hook.sh".to_string(),
        on_error: Some(OnError::Fail),
    };

    let config3 = HookConfig {
        command: "scripts/other.sh".to_string(),
        on_error: Some(OnError::Fail),
    };

    assert_eq!(config1, config2);
    assert_ne!(config1, config3);
}

#[test]
fn test_on_error_equality() {
    assert_eq!(OnError::Fail, OnError::Fail);
    assert_eq!(OnError::Continue, OnError::Continue);
    assert_ne!(OnError::Fail, OnError::Continue);
}
