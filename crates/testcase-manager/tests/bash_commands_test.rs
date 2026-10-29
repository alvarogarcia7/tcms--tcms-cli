use std::fs;
use std::path::PathBuf;
use testcase_manager::validation::SchemaValidator;

/// Integration test for bash_commands test cases
/// Validates YAML schema compliance for all bash command test cases
const BASH_COMMANDS_DIR: &str = "test-acceptance/00_test_cases/bash_commands";

fn get_bash_commands_test_cases() -> Vec<PathBuf> {
    let dir = PathBuf::from(BASH_COMMANDS_DIR);
    let mut test_cases = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("yml")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("TC_"))
            {
                test_cases.push(path);
            }
        }
    }

    test_cases.sort();
    test_cases
}

#[test]
fn test_bash_commands_directory_exists() {
    let dir = PathBuf::from(BASH_COMMANDS_DIR);
    assert!(
        dir.exists(),
        "Bash commands test directory should exist: {}",
        BASH_COMMANDS_DIR
    );
    assert!(
        dir.is_dir(),
        "Bash commands path should be a directory: {}",
        BASH_COMMANDS_DIR
    );
}

#[test]
fn test_bash_commands_test_cases_exist() {
    let test_cases = get_bash_commands_test_cases();
    assert!(
        !test_cases.is_empty(),
        "Should have at least one bash commands test case"
    );
    assert!(
        test_cases.len() >= 12,
        "Should have at least 12 bash commands test cases, found: {}",
        test_cases.len()
    );

    println!("\nFound {} bash commands test cases:", test_cases.len());
    for tc in &test_cases {
        println!("  - {}", tc.display());
    }
}

#[test]
fn test_bash_commands_yaml_schema_validation() {
    let _ = env_logger::builder().is_test(true).try_init();

    let validator = SchemaValidator::new().expect("Failed to create validator");
    let test_cases = get_bash_commands_test_cases();

    assert!(
        !test_cases.is_empty(),
        "No bash commands test cases found in {}",
        BASH_COMMANDS_DIR
    );

    let mut validation_results = Vec::new();

    for test_case_path in &test_cases {
        let file_name = test_case_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        log::info!("\n=== Validating: {} ===", file_name);

        let content = fs::read_to_string(test_case_path).unwrap_or_else(|e| {
            panic!(
                "Failed to read test case {}: {}",
                test_case_path.display(),
                e
            )
        });

        let result = validator.validate_complete(&content);

        match &result {
            Ok(_) => {
                log::info!("✓ {} passed schema validation", file_name);
                validation_results.push((file_name.to_string(), true, None));
            }
            Err(e) => {
                log::error!("✗ {} failed schema validation: {}", file_name, e);
                validation_results.push((file_name.to_string(), false, Some(e.to_string())));
            }
        }
    }

    // Print summary
    println!("\n=== Validation Summary ===");
    let passed = validation_results.iter().filter(|(_, ok, _)| *ok).count();
    let failed = validation_results.len() - passed;

    println!("Total: {}", validation_results.len());
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    if failed > 0 {
        println!("\nFailed test cases:");
        for (name, ok, error) in &validation_results {
            if !ok {
                println!("  ✗ {}", name);
                if let Some(err) = error {
                    println!("    Error: {}", err);
                }
            }
        }
    }

    // Assert all passed
    assert_eq!(
        failed, 0,
        "All bash commands test cases should pass schema validation"
    );
}

#[test]
fn test_bash_commands_yaml_parsing() {
    let test_cases = get_bash_commands_test_cases();

    for test_case_path in &test_cases {
        let file_name = test_case_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let content = fs::read_to_string(test_case_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {} as YAML: {}", file_name, e));

        // Verify it's a mapping
        assert!(
            yaml_value.is_mapping(),
            "{} should be a YAML mapping",
            file_name
        );

        let mapping = yaml_value.as_mapping().unwrap();

        // Verify required fields exist
        assert!(
            mapping.contains_key(serde_yaml::Value::String("requirement".to_string())),
            "{} missing 'requirement' field",
            file_name
        );
        assert!(
            mapping.contains_key(serde_yaml::Value::String("id".to_string())),
            "{} missing 'id' field",
            file_name
        );
        assert!(
            mapping.contains_key(serde_yaml::Value::String("description".to_string())),
            "{} missing 'description' field",
            file_name
        );
        assert!(
            mapping.contains_key(serde_yaml::Value::String("test_sequences".to_string())),
            "{} missing 'test_sequences' field",
            file_name
        );
    }

    println!(
        "\n✓ All {} bash commands test cases are valid YAML",
        test_cases.len()
    );
}

#[test]
fn test_bash_commands_structure() {
    let test_cases = get_bash_commands_test_cases();

    for test_case_path in &test_cases {
        let file_name = test_case_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let content = fs::read_to_string(test_case_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", file_name, e));

        let mapping = yaml_value.as_mapping().unwrap();

        // Check test_sequences structure
        let sequences = mapping
            .get(serde_yaml::Value::String("test_sequences".to_string()))
            .unwrap_or_else(|| panic!("{} missing test_sequences", file_name));

        assert!(
            sequences.is_sequence(),
            "{} test_sequences should be an array",
            file_name
        );

        let sequences_array = sequences.as_sequence().unwrap();
        assert!(
            !sequences_array.is_empty(),
            "{} should have at least one test sequence",
            file_name
        );

        // Check first sequence has required fields
        let first_seq = &sequences_array[0];
        let seq_mapping = first_seq
            .as_mapping()
            .unwrap_or_else(|| panic!("{} sequence should be an object", file_name));

        assert!(
            seq_mapping.contains_key(serde_yaml::Value::String("id".to_string())),
            "{} sequence missing 'id'",
            file_name
        );
        assert!(
            seq_mapping.contains_key(serde_yaml::Value::String("name".to_string())),
            "{} sequence missing 'name'",
            file_name
        );
        assert!(
            seq_mapping.contains_key(serde_yaml::Value::String("steps".to_string())),
            "{} sequence missing 'steps'",
            file_name
        );

        // Check steps
        let steps = seq_mapping
            .get(serde_yaml::Value::String("steps".to_string()))
            .unwrap_or_else(|| panic!("{} sequence missing steps", file_name));

        assert!(
            steps.is_sequence(),
            "{} steps should be an array",
            file_name
        );

        let steps_array = steps.as_sequence().unwrap();
        assert!(
            !steps_array.is_empty(),
            "{} should have at least one step",
            file_name
        );

        // Check first step structure
        let first_step = &steps_array[0];
        let step_mapping = first_step
            .as_mapping()
            .unwrap_or_else(|| panic!("{} step should be an object", file_name));

        assert!(
            step_mapping.contains_key(serde_yaml::Value::String("step".to_string())),
            "{} step missing 'step' number",
            file_name
        );
        assert!(
            step_mapping.contains_key(serde_yaml::Value::String("description".to_string())),
            "{} step missing 'description'",
            file_name
        );
        assert!(
            step_mapping.contains_key(serde_yaml::Value::String("command".to_string())),
            "{} step missing 'command'",
            file_name
        );
        assert!(
            step_mapping.contains_key(serde_yaml::Value::String("expected".to_string())),
            "{} step missing 'expected'",
            file_name
        );
        assert!(
            step_mapping.contains_key(serde_yaml::Value::String("verification".to_string())),
            "{} step missing 'verification'",
            file_name
        );
    }

    println!(
        "\n✓ All {} bash commands test cases have correct structure",
        test_cases.len()
    );
}

#[test]
fn test_bash_commands_specific_files() {
    let expected_files = vec![
        "TC_BASH_SIMPLE_001.yml",
        "TC_BASH_INTERMEDIATE_001.yml",
        "TC_BASH_COMPLEX_001.yml",
        "TC_BASH_VERIFICATION_001.yml",
        "TC_BASH_ARRAYS_001.yml",
        "TC_BASH_STRING_OPS_001.yml",
        "TC_BASH_CONDITIONALS_001.yml",
        "TC_BASH_LOOPS_001.yml",
        "TC_BASH_FILE_OPS_001.yml",
        "TC_BASH_MATH_OPS_001.yml",
        "TC_BASH_ENV_VARS_001.yml",
        "TC_BASH_PROCESS_OPS_001.yml",
        "TC_BASH_REDIRECTION_001.yml",
    ];

    for file_name in expected_files {
        let file_path = PathBuf::from(BASH_COMMANDS_DIR).join(file_name);
        assert!(
            file_path.exists(),
            "Expected bash commands test case should exist: {}",
            file_name
        );
    }

    println!("\n✓ All expected bash commands test case files exist");
}

#[test]
fn test_bash_commands_readme_exists() {
    let readme_path = PathBuf::from(BASH_COMMANDS_DIR).join("README.md");
    assert!(
        readme_path.exists(),
        "bash_commands directory should have a README.md"
    );

    let content =
        fs::read_to_string(&readme_path).expect("Should be able to read bash_commands README.md");

    assert!(
        !content.is_empty(),
        "bash_commands README.md should not be empty"
    );
    assert!(
        content.contains("# Bash Commands Test Cases"),
        "README should have proper title"
    );

    println!("\n✓ bash_commands README.md exists and has content");
}

#[test]
fn test_bash_commands_verification_strictness() {
    let test_cases = get_bash_commands_test_cases();

    for test_case_path in &test_cases {
        let file_name = test_case_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let content = fs::read_to_string(test_case_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", file_name, e));

        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", file_name, e));

        let mapping = yaml_value.as_mapping().unwrap();
        let sequences = mapping
            .get(serde_yaml::Value::String("test_sequences".to_string()))
            .unwrap();
        let sequences_array = sequences.as_sequence().unwrap();

        for sequence in sequences_array {
            let seq_mapping = sequence.as_mapping().unwrap();
            let steps = seq_mapping
                .get(serde_yaml::Value::String("steps".to_string()))
                .unwrap();
            let steps_array = steps.as_sequence().unwrap();

            for step in steps_array {
                let step_mapping = step.as_mapping().unwrap();

                // Verify each step has verification field
                assert!(
                    step_mapping
                        .contains_key(serde_yaml::Value::String("verification".to_string())),
                    "{} should have verification for all steps",
                    file_name
                );

                let verification = step_mapping
                    .get(serde_yaml::Value::String("verification".to_string()))
                    .unwrap();
                let verification_mapping = verification.as_mapping().unwrap();

                // Verify result verification exists
                assert!(
                    verification_mapping
                        .contains_key(serde_yaml::Value::String("result".to_string())),
                    "{} should have result verification for all steps",
                    file_name
                );

                // Verify output verification exists
                assert!(
                    verification_mapping
                        .contains_key(serde_yaml::Value::String("output".to_string())),
                    "{} should have output verification for all steps",
                    file_name
                );
            }
        }
    }

    println!(
        "\n✓ All {} bash commands test cases have strict verification",
        test_cases.len()
    );
}
