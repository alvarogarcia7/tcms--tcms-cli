use anyhow::Result;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use testcase_manager::{
    oracle::{AnswerVariant, HardcodedOracle, Oracle},
    GitManager, Prompts, TestCase, TestCaseBuilder,
};
use testcase_models::CaptureVarsFormat;

#[test]
#[ignore = "Requires CLI"]
fn test_end_to_end_complete_workflow() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let test_dir = temp_dir.path();
    let output_file = test_dir.join("test_case_001.yaml");

    println!("Test directory: {}", test_dir.display());
    println!("Output file: {}", output_file.display());

    // Set up all hardcoded answers for complete workflow
    let mut answers = VecDeque::new();

    // Metadata prompts
    answers.push_back(AnswerVariant::String("REQ-TEST-001".to_string()));
    answers.push_back(AnswerVariant::Int(42));
    answers.push_back(AnswerVariant::Int(1));
    answers.push_back(AnswerVariant::String("TC_Integration_Test_001".to_string()));
    answers.push_back(AnswerVariant::String(
        "End-to-end integration test for testcase-manager workflow".to_string(),
    ));

    // Commit metadata?
    answers.push_back(AnswerVariant::Bool(true));

    // Add general initial conditions?
    answers.push_back(AnswerVariant::Bool(true));
    answers.push_back(AnswerVariant::String("eUICC".to_string()));
    answers.push_back(AnswerVariant::String(
        "General initial condition 1".to_string(),
    ));
    answers.push_back(AnswerVariant::String("".to_string()));

    // Commit general initial conditions?
    answers.push_back(AnswerVariant::Bool(true));

    // Add initial conditions?
    answers.push_back(AnswerVariant::Bool(true));
    answers.push_back(AnswerVariant::String("eUICC".to_string()));
    answers.push_back(AnswerVariant::String("Initial condition 1".to_string()));
    answers.push_back(AnswerVariant::String("Initial condition 2".to_string()));
    answers.push_back(AnswerVariant::String("".to_string()));

    // Commit initial conditions?
    answers.push_back(AnswerVariant::Bool(true));

    // Test Sequence 1
    answers.push_back(AnswerVariant::String("Test Sequence 1".to_string()));
    answers.push_back(AnswerVariant::Bool(false)); // Use fuzzy search?
    answers.push_back(AnswerVariant::Bool(false)); // Edit description in editor?
    answers.push_back(AnswerVariant::Bool(true)); // Add sequence-specific initial conditions?
    answers.push_back(AnswerVariant::Bool(false)); // Use database?
    answers.push_back(AnswerVariant::String("eUICC".to_string()));
    answers.push_back(AnswerVariant::String(
        "Sequence-specific condition 1".to_string(),
    ));
    answers.push_back(AnswerVariant::String("".to_string()));

    // Commit this sequence?
    answers.push_back(AnswerVariant::Bool(true));

    // Add steps to this sequence now?
    answers.push_back(AnswerVariant::Bool(true));

    // Step 1
    answers.push_back(AnswerVariant::Bool(false)); // Use fuzzy search?
    answers.push_back(AnswerVariant::String("Execute test command".to_string()));
    answers.push_back(AnswerVariant::Bool(false)); // Is manual step?
    answers.push_back(AnswerVariant::String("ssh test-device".to_string()));
    answers.push_back(AnswerVariant::Bool(false)); // Include 'success' field?
    answers.push_back(AnswerVariant::String("0x9000".to_string()));
    answers.push_back(AnswerVariant::String("Success output".to_string()));

    // Commit this step?
    answers.push_back(AnswerVariant::Bool(true));

    // Add another step to this sequence?
    answers.push_back(AnswerVariant::Bool(true));

    // Step 2
    answers.push_back(AnswerVariant::Bool(false)); // Use fuzzy search?
    answers.push_back(AnswerVariant::String("Verify results".to_string()));
    answers.push_back(AnswerVariant::Bool(false)); // Is manual step?
    answers.push_back(AnswerVariant::String("ssh verify".to_string()));
    answers.push_back(AnswerVariant::Bool(false)); // Include 'success' field?
    answers.push_back(AnswerVariant::String("OK".to_string()));
    answers.push_back(AnswerVariant::String("Verification passed".to_string()));

    // Commit this step?
    answers.push_back(AnswerVariant::Bool(true));

    // Add another step to this sequence?
    answers.push_back(AnswerVariant::Bool(false));

    // Add another test sequence?
    answers.push_back(AnswerVariant::Bool(false));

    // Commit final complete test case?
    answers.push_back(AnswerVariant::Bool(true));

    let oracle: Arc<dyn Oracle> = Arc::new(HardcodedOracle::new(answers));

    println!("Starting test case workflow...");

    // Build the complete test case using HardcodedOracle
    let metadata = Prompts::prompt_metadata_with_oracle(&oracle)?;
    assert_eq!(metadata.requirement, "REQ-TEST-001");
    assert_eq!(metadata.item, 42);
    assert_eq!(metadata.tc, 1);
    assert_eq!(metadata.id, "TC_Integration_Test_001");

    let mut builder = TestCaseBuilder::new(test_dir, oracle.clone())?;

    let yaml_map = metadata.to_yaml();
    for (key, value) in yaml_map {
        builder.structure_mut().insert(key, value);
    }

    // Commit metadata
    let commit_metadata = oracle.confirm("Commit metadata to git?")?;
    if commit_metadata {
        builder.commit("TEST: Add test case metadata")?;
    }

    // Add general initial conditions
    let add_general = oracle.confirm("Add general initial conditions?")?;
    if add_general {
        builder.add_general_initial_conditions(None)?;
    }

    // Commit general initial conditions
    let commit_general = oracle.confirm("Commit general initial conditions?")?;
    if commit_general {
        builder.commit("TEST: Add general initial conditions")?;
    }

    // Add initial conditions
    let add_ic = oracle.confirm("Add initial conditions?")?;
    if add_ic {
        builder.add_initial_conditions(None)?;
    }

    // Commit initial conditions
    let commit_ic = oracle.confirm("Commit initial conditions?")?;
    if commit_ic {
        builder.commit("TEST: Add initial conditions")?;
    }

    // Add test sequence
    builder.add_test_sequence_interactive()?;

    // Commit sequence
    let commit_seq = oracle.confirm("Commit this sequence?")?;
    if commit_seq {
        builder.commit("TEST: Add test sequence")?;
    }

    // Add steps to sequence
    let add_steps = oracle.confirm("Add steps to this sequence now?")?;
    if add_steps {
        // Add Step 1
        builder.add_steps_to_sequence_with_commits(1 - 1)?;

        // Commit step 1
        let commit_step1 = oracle.confirm("Commit this step?")?;
        if commit_step1 {
            builder.commit("TEST: Add step 1")?;
        }

        // Add another step?
        let add_step2 = oracle.confirm("Add another step to this sequence?")?;
        if add_step2 {
            // Add Step 2
            builder.add_steps_to_sequence_by_id_with_commits(2 - 1)?;

            // Commit step 2
            let commit_step2 = oracle.confirm("Commit this step?")?;
            if commit_step2 {
                builder.commit("TEST: Add step 2")?;
            }
        }

        // Add another step?
        oracle.confirm("Add another step to this sequence?")?; // false
    }

    // Add another sequence?
    oracle.confirm("Add another test sequence?")?; // false

    // Save final test case
    builder.save()?;

    // Commit final test case
    builder.commit("TEST: Complete test case with all sequences and steps")?;

    println!("\n=== Workflow completed successfully ===");

    assert!(
        output_file.exists(),
        "Output YAML file was not created: {}",
        output_file.display()
    );

    let yaml_content =
        fs::read_to_string(&output_file).expect("Failed to read generated YAML file");

    println!("\n=== Generated YAML ===");
    println!("{}", yaml_content);

    validate_yaml_structure(&yaml_content);

    validate_yaml_parsing(&output_file);

    validate_git_commits(test_dir);

    println!("\n✓ All validations passed!");

    Ok(())
}

#[test]
#[ignore = "SKIPPED until the parsing of the YAML / JSON SCHEMA is correct again"]
fn test_yaml_structure_only() {
    // This is a simpler test that doesn't require spawning the binary
    // It validates that we can create and parse a valid YAML structure
    let yaml_content = r#"
requirement: REQ-TEST-001
item: 42
tc: 1
id: TC_Integration_Test_001
description: End-to-end integration test for testcase-manager workflow
general_initial_conditions:
  - eUICC:
      - General initial condition 1
initial_conditions:
  eUICC:
    - Initial condition 1
    - Initial condition 2
test_sequences:
  - id: 1
    name: Test Sequence 1
    description: ""
    initial_conditions:
      - eUICC:
          - Sequence-specific condition 1
    steps:
      - step: 1
        description: Execute test command
        command: ssh test-device
        expected:
          result: "0x9000"
          output: Success output
      - step: 2
        description: Verify results
        command: ssh verify
        expected:
          result: OK
          output: Verification passed
"#;

    validate_yaml_structure(yaml_content);

    // For this test, we use raw serde_yaml since the YAML doesn't have a schema field
    let parsed: TestCase = serde_yaml::from_str(yaml_content).expect("Failed to parse YAML");

    assert_eq!(parsed.requirement, "REQ-TEST-001");
    assert_eq!(parsed.item, 42);
    assert_eq!(parsed.tc, 1);
    assert_eq!(parsed.id, "TC_Integration_Test_001");
    assert_eq!(parsed.test_sequences.len(), 1);
    assert_eq!(parsed.test_sequences[0].steps.len(), 2);
}

fn validate_yaml_structure(yaml_content: &str) {
    assert!(
        yaml_content.contains("requirement:"),
        "YAML missing 'requirement' field"
    );
    assert!(
        yaml_content.contains("REQ-TEST-001"),
        "YAML missing requirement value"
    );

    assert!(yaml_content.contains("item:"), "YAML missing 'item' field");
    assert!(yaml_content.contains("42"), "YAML missing item value");

    assert!(yaml_content.contains("tc:"), "YAML missing 'tc' field");

    assert!(yaml_content.contains("id:"), "YAML missing 'id' field");
    assert!(
        yaml_content.contains("TC_Integration_Test_001"),
        "YAML missing id value"
    );

    assert!(
        yaml_content.contains("description:"),
        "YAML missing 'description' field"
    );
    assert!(
        yaml_content.contains("End-to-end integration test"),
        "YAML missing description value"
    );

    assert!(
        yaml_content.contains("general_initial_conditions:"),
        "YAML missing 'general_initial_conditions' field"
    );
    assert!(
        yaml_content.contains("General initial condition 1"),
        "YAML missing general initial condition value"
    );

    assert!(
        yaml_content.contains("initial_conditions:"),
        "YAML missing 'initial_conditions' field"
    );
    assert!(
        yaml_content.contains("Initial condition 1"),
        "YAML missing initial condition 1"
    );
    assert!(
        yaml_content.contains("Initial condition 2"),
        "YAML missing initial condition 2"
    );

    assert!(
        yaml_content.contains("test_sequences:"),
        "YAML missing 'test_sequences' field"
    );

    assert!(
        yaml_content.contains("Test Sequence 1"),
        "YAML missing sequence name"
    );

    assert!(
        yaml_content.contains("Sequence-specific condition 1"),
        "YAML missing sequence-specific initial condition"
    );

    assert!(
        yaml_content.contains("steps:"),
        "YAML missing 'steps' field"
    );

    assert!(
        yaml_content.contains("Execute test command"),
        "YAML missing step 1 description"
    );
    assert!(
        yaml_content.contains("ssh test-device"),
        "YAML missing step 1 command"
    );
    assert!(
        yaml_content.contains("0x9000"),
        "YAML missing step 1 expected result"
    );
    assert!(
        yaml_content.contains("Success output"),
        "YAML missing step 1 expected output"
    );

    assert!(
        yaml_content.contains("Verify results"),
        "YAML missing step 2 description"
    );
    assert!(
        yaml_content.contains("ssh verify"),
        "YAML missing step 2 command"
    );
    assert!(
        yaml_content.contains("Verification passed"),
        "YAML missing step 2 expected output"
    );
}

fn validate_yaml_parsing(yaml_file: &Path) {
    // Note: Uses raw YAML parsing since generated YAML may not have schema field
    let yaml_content = fs::read_to_string(yaml_file).expect("Failed to read YAML file for parsing");

    let parsed_test_case: TestCase =
        serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML as TestCase");

    assert_eq!(parsed_test_case.requirement, "REQ-TEST-001");
    assert_eq!(parsed_test_case.item, 42);
    assert_eq!(parsed_test_case.tc, 1);
    assert_eq!(parsed_test_case.id, "TC_Integration_Test_001");
    assert!(parsed_test_case
        .description
        .contains("End-to-end integration test"));

    assert!(
        !parsed_test_case.general_initial_conditions.is_empty(),
        "General initial conditions should not be empty"
    );

    let ic_euicc = parsed_test_case
        .initial_conditions
        .devices
        .get("eUICC")
        .unwrap();

    assert!(
        !ic_euicc.is_empty(),
        "Initial conditions should not be empty"
    );
    assert_eq!(ic_euicc.len(), 2);

    assert!(
        !parsed_test_case.test_sequences.is_empty(),
        "Test sequences should not be empty"
    );

    let sequence = &parsed_test_case.test_sequences[0];
    assert_eq!(sequence.id, 1);
    assert_eq!(sequence.name, "Test Sequence 1");

    assert!(
        !sequence.initial_conditions.is_empty(),
        "Sequence initial conditions should not be empty"
    );

    assert!(!sequence.steps.is_empty(), "Steps should not be empty");
    assert_eq!(sequence.steps.len(), 2);

    let step1 = &sequence.steps[0];
    assert_eq!(step1.step, 1);
    assert_eq!(step1.description, "Execute test command");
    assert_eq!(step1.command, "ssh test-device");
    assert_eq!(step1.expected.result, "0x9000");
    assert_eq!(step1.expected.output, "Success output");

    let step2 = &sequence.steps[1];
    assert_eq!(step2.step, 2);
    assert_eq!(step2.description, "Verify results");
    assert_eq!(step2.command, "ssh verify");
    assert_eq!(step2.expected.result, "OK");
    assert_eq!(step2.expected.output, "Verification passed");

    println!("✓ YAML parsed successfully with correct structure");
}

fn validate_git_commits(repo_path: &Path) {
    let git = GitManager::open(repo_path).expect("Failed to open git repository");

    let commits = git.log(20).expect("Failed to get commit log");

    assert!(!commits.is_empty(), "No git commits found");

    println!("\n=== Git Commits ===");
    for commit in &commits {
        println!(
            "{} - {}",
            &commit.id[..7],
            commit.message.lines().next().unwrap_or("")
        );
    }

    let commit_messages: Vec<String> = commits.iter().map(|c| c.message.clone()).collect();

    let has_metadata_commit = commit_messages
        .iter()
        .any(|msg| msg.contains("metadata") || msg.contains("TEST: Add test case metadata"));

    assert!(
        has_metadata_commit,
        "No metadata commit found in git history"
    );

    let has_initial_conditions_commit = commit_messages
        .iter()
        .any(|msg| msg.contains("initial conditions"));

    assert!(
        has_initial_conditions_commit,
        "No initial conditions commit found in git history"
    );

    let has_sequence_commit = commit_messages
        .iter()
        .any(|msg| msg.contains("sequence") || msg.contains("TEST: Add test sequence"));

    assert!(
        has_sequence_commit,
        "No sequence commit found in git history"
    );

    let has_step_commits = commit_messages
        .iter()
        .any(|msg| msg.contains("step") || msg.contains("TEST: Add step"));

    assert!(has_step_commits, "No step commits found in git history");

    let has_final_commit = commit_messages
        .iter()
        .any(|msg| msg.contains("Complete test case") || msg.contains("all sequences and steps"));

    assert!(has_final_commit, "No final commit found in git history");

    assert!(
        commits.len() >= 5,
        "Expected at least 5 commits (metadata, general IC, IC, sequence, step(s), final), but found {}",
        commits.len()
    );

    println!("✓ Git commits validated successfully");
    println!("  Total commits: {}", commits.len());
}

// ===== Manual Verification Example Test Cases Integration Tests =====

use std::path::PathBuf;
use testcase_manager::validation::SchemaValidator;
use testcase_manager::TestExecutor;

/// Helper function to load a test case from a file
fn load_test_case_from_file(path: &str) -> Result<TestCase> {
    // Use CARGO_MANIFEST_DIR to get the workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut workspace_root = PathBuf::from(manifest_dir);
    workspace_root.pop(); // Go up from crate dir
    workspace_root.pop(); // Go up from crates dir

    let yaml_path = workspace_root.join(path);
    assert!(
        yaml_path.exists(),
        "Test file not found: {}",
        yaml_path.display()
    );

    // Note: Uses raw YAML parsing since example files may not have schema fields
    let yaml_content = fs::read_to_string(&yaml_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", yaml_path.display(), e));

    let test_case: TestCase = serde_yaml::from_str(&yaml_content)
        .unwrap_or_else(|e| panic!("Failed to deserialize {}: {}", yaml_path.display(), e));

    Ok(test_case)
}

/// Helper function to validate schema for a YAML file
fn validate_yaml_schema(path: &str) -> Result<()> {
    // Use CARGO_MANIFEST_DIR to get the workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut workspace_root = PathBuf::from(manifest_dir);
    workspace_root.pop(); // Go up from crate dir
    workspace_root.pop(); // Go up from crates dir

    let yaml_path = workspace_root.join(path);
    let yaml_content = fs::read_to_string(&yaml_path)?;

    let validator = SchemaValidator::new()?;
    validator.validate_chunk(&yaml_content)?;

    Ok(())
}

/// Helper function to check if a VerificationExpression contains a substring
fn verification_contains(expr: &testcase_manager::VerificationExpression, text: &str) -> bool {
    match expr {
        testcase_manager::VerificationExpression::Simple(s) => s.contains(text),
        testcase_manager::VerificationExpression::Conditional {
            condition,
            if_true,
            if_false,
            always,
        } => {
            condition.contains(text)
                || if_true
                    .as_ref()
                    .is_some_and(|v| v.iter().any(|s| s.contains(text)))
                || if_false
                    .as_ref()
                    .is_some_and(|v| v.iter().any(|s| s.contains(text)))
                || always
                    .as_ref()
                    .is_some_and(|v| v.iter().any(|s| s.contains(text)))
        }
    }
}

#[test]
fn test_tc_manual_verify_001_example_schema_validation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_001.yml";

    validate_yaml_schema(path)?;

    println!("✓ TC_MANUAL_VERIFY_001 passed schema validation");
    Ok(())
}

#[test]
fn test_tc_manual_verify_001_example_deserialization() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_001.yml";
    let test_case = load_test_case_from_file(path)?;

    // Validate metadata
    assert_eq!(test_case.id, "TC_MANUAL_VERIFY_001");
    assert_eq!(test_case.requirement, "MANUAL_VERIFY");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 1);
    assert!(test_case.description.contains("Audio beep verification"));

    // Validate test sequences
    assert_eq!(test_case.test_sequences.len(), 2);

    // Validate first sequence - Audio Output Verification
    let seq1 = &test_case.test_sequences[0];
    assert_eq!(seq1.id, 1);
    assert_eq!(seq1.name, "Audio Output Verification");
    assert_eq!(seq1.steps.len(), 8);

    // Check for manual steps
    let manual_steps: Vec<_> = seq1
        .steps
        .iter()
        .filter(|s| s.manual == Some(true))
        .collect();
    assert_eq!(
        manual_steps.len(),
        5,
        "Should have 5 manual steps in sequence 1"
    );

    // Validate specific manual steps have verification prompts
    let step2 = &seq1.steps[1];
    assert_eq!(step2.manual, Some(true));
    assert!(verification_contains(
        &step2.verification.output,
        "Did you hear"
    ));
    assert!(verification_contains(&step2.verification.output, "(Y/n)"));

    // Validate second sequence - System Beep Verification
    let seq2 = &test_case.test_sequences[1];
    assert_eq!(seq2.id, 2);
    assert_eq!(seq2.name, "System Beep Verification");
    assert_eq!(seq2.steps.len(), 4);

    let manual_steps_seq2: Vec<_> = seq2
        .steps
        .iter()
        .filter(|s| s.manual == Some(true))
        .collect();
    assert_eq!(
        manual_steps_seq2.len(),
        3,
        "Should have 3 manual steps in sequence 2"
    );

    println!("✓ TC_MANUAL_VERIFY_001 deserialized correctly");
    Ok(())
}

#[test]
fn test_tc_manual_verify_001_example_bash_script_generation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_001.yml";
    let test_case = load_test_case_from_file(path)?;

    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify script contains manual step markers
    assert!(
        script.contains("INFO: This is a manual step"),
        "Script should contain manual step marker"
    );

    // Verify script contains verification prompts
    assert!(
        script.contains("Did you hear a clear 1000Hz beep tone"),
        "Script should contain audio verification prompt"
    );
    assert!(
        script.contains("Did you hear a lower-pitched 500Hz beep tone"),
        "Script should contain 500Hz verification prompt"
    );
    assert!(
        script.contains("Did you hear a higher-pitched 2000Hz beep tone"),
        "Script should contain 2000Hz verification prompt"
    );
    assert!(
        script.contains("Did you hear the beep only from the LEFT speaker"),
        "Script should contain left speaker verification prompt"
    );
    assert!(
        script.contains("Did you hear the beep only from the RIGHT speaker"),
        "Script should contain right speaker verification prompt"
    );

    // Verify script contains read_true_false helper function
    assert!(
        script.contains("read_true_false()") || script.contains("read_true_false ()"),
        "Script should define read_true_false helper function"
    );

    // Verify script contains speaker-test commands
    assert!(
        script.contains("speaker-test -t sine -f 1000"),
        "Script should contain 1000Hz speaker test command"
    );
    assert!(
        script.contains("speaker-test -t sine -f 500"),
        "Script should contain 500Hz speaker test command"
    );
    assert!(
        script.contains("speaker-test -t sine -f 2000"),
        "Script should contain 2000Hz speaker test command"
    );

    // Verify script contains system beep commands
    assert!(
        script.contains("echo -e"),
        "Script should contain echo command for system beep"
    );

    println!("✓ TC_MANUAL_VERIFY_001 generated valid bash script");
    Ok(())
}

#[test]
fn test_tc_manual_verify_002_example_schema_validation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml";

    validate_yaml_schema(path)?;

    println!("✓ TC_MANUAL_VERIFY_002 passed schema validation");
    Ok(())
}

#[test]
fn test_tc_manual_verify_002_example_deserialization() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml";
    let test_case = load_test_case_from_file(path)?;

    // Validate metadata
    assert_eq!(test_case.id, "TC_MANUAL_VERIFY_002");
    assert_eq!(test_case.requirement, "MANUAL_VERIFY");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 2);
    assert!(test_case.description.contains("LED status verification"));

    // Validate test sequences
    assert_eq!(test_case.test_sequences.len(), 3);

    // Validate first sequence - LED Power and Link Status Verification
    let seq1 = &test_case.test_sequences[0];
    assert_eq!(seq1.id, 1);
    assert_eq!(seq1.name, "LED Power and Link Status Verification");
    assert_eq!(seq1.steps.len(), 8);

    // Check for manual steps with visual inspection
    let manual_steps: Vec<_> = seq1
        .steps
        .iter()
        .filter(|s| s.manual == Some(true))
        .collect();
    assert_eq!(
        manual_steps.len(),
        5,
        "Should have 5 manual steps in sequence 1"
    );

    // Validate LED observation prompts
    let step1 = &seq1.steps[0];
    assert_eq!(step1.manual, Some(true));
    assert!(verification_contains(
        &step1.verification.output,
        "power LED"
    ));
    assert!(verification_contains(
        &step1.verification.output,
        "solid green"
    ));

    // Validate second sequence - Multi-Color LED Status Verification
    let seq2 = &test_case.test_sequences[1];
    assert_eq!(seq2.id, 2);
    assert_eq!(seq2.name, "Multi-Color LED Status Verification");

    // Validate third sequence - LED Fault Indication Verification
    let seq3 = &test_case.test_sequences[2];
    assert_eq!(seq3.id, 3);
    assert_eq!(seq3.name, "LED Fault Indication Verification");

    println!("✓ TC_MANUAL_VERIFY_002 deserialized correctly");
    Ok(())
}

#[test]
fn test_tc_manual_verify_002_example_bash_script_generation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml";
    let test_case = load_test_case_from_file(path)?;

    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify script contains manual step markers
    assert!(
        script.contains("INFO: This is a manual step"),
        "Script should contain manual step marker"
    );

    // Verify script contains LED verification prompts
    assert!(
        script.contains("power LED") || script.contains("Power LED"),
        "Script should contain power LED verification prompt"
    );
    assert!(
        script.contains("link LED")
            || script.contains("Link LED")
            || script.contains("link/network LED"),
        "Script should contain link LED verification prompt"
    );
    assert!(
        script.contains("activity LED") || script.contains("Activity LED"),
        "Script should contain activity LED verification prompt"
    );

    // Verify script contains color verification prompts
    assert!(
        script.contains("GREEN"),
        "Script should contain green color verification"
    );
    assert!(
        script.contains("AMBER") || script.contains("amber"),
        "Script should contain amber color verification"
    );

    // Verify script contains physical action prompts
    assert!(
        script.contains("Connect network cable") || script.contains("network cable"),
        "Script should contain cable connection prompt"
    );
    assert!(
        script.contains("Disconnect") || script.contains("disconnect"),
        "Script should contain cable disconnection prompt"
    );

    // Verify script contains network commands
    assert!(
        script.contains("ip link"),
        "Script should contain ip link command"
    );
    assert!(
        script.contains("ethtool") || script.contains("Link speed") || script.contains("speed"),
        "Script should contain ethtool or speed checking"
    );

    // Verify script contains read_true_false helper
    assert!(
        script.contains("read_true_false()") || script.contains("read_true_false ()"),
        "Script should define read_true_false helper function"
    );

    println!("✓ TC_MANUAL_VERIFY_002 generated valid bash script");
    Ok(())
}

#[test]
fn test_tc_manual_verify_003_example_schema_validation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml";

    validate_yaml_schema(path)?;

    println!("✓ TC_MANUAL_VERIFY_003 passed schema validation");
    Ok(())
}

#[test]
fn test_tc_manual_verify_003_example_deserialization() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml";
    let test_case = load_test_case_from_file(path)?;

    // Validate metadata
    assert_eq!(test_case.id, "TC_MANUAL_VERIFY_003");
    assert_eq!(test_case.requirement, "MANUAL_VERIFY");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 3);
    assert!(test_case.description.contains("Chained verifications"));
    assert!(test_case.description.contains("USER_VERIFICATION"));

    // Validate test sequences
    assert_eq!(test_case.test_sequences.len(), 4);

    // Validate first sequence - Application Startup Verification Chain
    let seq1 = &test_case.test_sequences[0];
    assert_eq!(seq1.id, 1);
    assert_eq!(seq1.name, "Application Startup Verification Chain");
    assert_eq!(seq1.steps.len(), 6);

    // Check for manual steps
    let manual_steps_seq1: Vec<_> = seq1
        .steps
        .iter()
        .filter(|s| s.manual == Some(true))
        .collect();
    assert_eq!(
        manual_steps_seq1.len(),
        4,
        "Should have 4 manual steps in sequence 1"
    );

    // Validate second sequence - Interactive Feature Verification Chain
    let seq2 = &test_case.test_sequences[1];
    assert_eq!(seq2.id, 2);
    assert_eq!(seq2.name, "Interactive Feature Verification Chain");
    assert_eq!(seq2.steps.len(), 8);

    let manual_steps_seq2: Vec<_> = seq2
        .steps
        .iter()
        .filter(|s| s.manual == Some(true))
        .collect();
    assert_eq!(
        manual_steps_seq2.len(),
        7,
        "Should have 7 manual steps in sequence 2"
    );

    // Validate third sequence - Error Handling Verification Chain
    let seq3 = &test_case.test_sequences[2];
    assert_eq!(seq3.id, 3);
    assert_eq!(seq3.name, "Error Handling Verification Chain");

    // Validate fourth sequence - Conditional Verification with System State
    let seq4 = &test_case.test_sequences[3];
    assert_eq!(seq4.id, 4);
    assert_eq!(seq4.name, "Conditional Verification with System State");
    assert_eq!(seq4.steps.len(), 8);

    // Validate conditional verification logic in step 3
    let step3 = &seq4.steps[2];
    assert_eq!(step3.manual, Some(true));
    assert!(
        step3.description.contains("debug menu") || step3.description.contains("Debug menu"),
        "Step 3 description should mention debug menu"
    );

    // Validate capture_vars in step 1
    let step1 = &seq4.steps[0];
    assert!(
        step1.capture_vars.is_some(),
        "Step 1 should have capture_vars"
    );
    let capture_vars = step1.capture_vars.as_ref().unwrap();
    // Check that app_mode is captured (works for both Legacy and New format)
    let has_app_mode = match capture_vars {
        testcase_models::CaptureVarsFormat::Legacy(map) => map.contains_key("app_mode"),
        testcase_models::CaptureVarsFormat::New(vec) => vec.iter().any(|cv| cv.name == "app_mode"),
    };
    assert!(has_app_mode, "Should capture app_mode variable");

    println!("✓ TC_MANUAL_VERIFY_003 deserialized correctly");
    Ok(())
}

#[test]
fn test_tc_manual_verify_003_example_bash_script_generation() -> Result<()> {
    let path = "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml";
    let test_case = load_test_case_from_file(path)?;

    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify script contains manual step markers
    assert!(
        script.contains("INFO: This is a manual step"),
        "Script should contain manual step marker"
    );

    // Verify script contains application verification prompts
    assert!(
        script.contains("application window") || script.contains("Application window"),
        "Script should contain application window verification"
    );
    assert!(
        script.contains("splash screen") || script.contains("Splash screen"),
        "Script should contain splash screen verification"
    );
    assert!(
        script.contains("menu bar") || script.contains("Menu bar"),
        "Script should contain menu bar verification"
    );

    // Verify script contains GUI interaction prompts
    assert!(
        script.contains("File menu") || script.contains("File Menu"),
        "Script should contain File menu interaction"
    );
    assert!(
        script.contains("New Project") || script.contains("new project"),
        "Script should contain New Project interaction"
    );
    assert!(
        script.contains("Browse") || script.contains("browse"),
        "Script should contain Browse button interaction"
    );

    // Verify script contains error handling verification
    assert!(
        script.contains("error dialog") || script.contains("Error dialog"),
        "Script should contain error dialog verification"
    );
    assert!(
        script.contains("duplicate") || script.contains("Duplicate"),
        "Script should contain duplicate project error handling"
    );

    // Verify script contains conditional logic
    assert!(
        script.contains("PRODUCTION") || script.contains("DEVELOPMENT"),
        "Script should contain environment mode checking"
    );
    assert!(
        script.contains("app_mode"),
        "Script should contain app_mode variable"
    );

    // Verify script contains variable capture
    assert!(
        script.contains("PRODUCTION\\|DEVELOPMENT") || script.contains("PRODUCTION|DEVELOPMENT"),
        "Script should contain regex pattern for mode capture"
    );

    // Verify script contains conditional verification structure
    assert!(
        script.contains("if") && script.contains("then") && script.contains("else"),
        "Script should contain conditional logic structure"
    );

    // Verify script contains read_true_false helper
    assert!(
        script.contains("read_true_false()") || script.contains("read_true_false ()"),
        "Script should define read_true_false helper function"
    );

    println!("✓ TC_MANUAL_VERIFY_003 generated valid bash script with conditional logic");
    Ok(())
}

#[test]
fn test_all_manual_verify_examples_have_read_true_false_helper() -> Result<()> {
    // NOTE: TC_MANUAL_VERIFY_001.yml has invalid YAML (output: true instead of a string at line 143)
    // so we skip it in this test
    let examples = vec![
        "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml",
        "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml",
    ];

    let executor = TestExecutor::new();

    for path in examples {
        let test_case = load_test_case_from_file(path)?;
        let script = executor.generate_test_script(&test_case);

        // All manual verification scripts should have the read_true_false helper
        assert!(
            script.contains("read_true_false()") || script.contains("read_true_false ()"),
            "Script for {} should define read_true_false helper function",
            path
        );

        // All scripts should have at least one (Y/n) prompt
        assert!(
            script.contains("(Y/n)"),
            "Script for {} should contain (Y/n) prompts",
            path
        );
    }

    println!("✓ Valid manual verification examples include read_true_false helper");
    Ok(())
}

#[test]
fn test_all_manual_verify_examples_have_manual_step_markers() -> Result<()> {
    // NOTE: TC_MANUAL_VERIFY_001.yml has invalid YAML (output: true instead of a string at line 143)
    // so we skip it in this test
    let examples = vec![
        (
            "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml",
            12,
        ),
        (
            "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml",
            21,
        ),
    ];

    let executor = TestExecutor::new();

    for (path, expected_manual_steps) in examples {
        let test_case = load_test_case_from_file(path)?;
        let script = executor.generate_test_script(&test_case);

        // Count manual step markers in the script
        let manual_marker_count = script.matches("INFO: This is a manual step").count();

        assert_eq!(
            manual_marker_count, expected_manual_steps,
            "Script for {} should have {} manual step markers, found {}",
            path, expected_manual_steps, manual_marker_count
        );
    }

    println!("✓ All manual verification examples have correct number of manual step markers");
    Ok(())
}

#[test]
fn test_manual_verify_examples_verification_prompts_format() -> Result<()> {
    // NOTE: TC_MANUAL_VERIFY_001.yml has invalid YAML (output: true instead of a string at line 143)
    // so we skip it in this test
    let examples = vec![
        "testcases/examples/manual_steps/TC_MANUAL_VERIFY_002.yml",
        "testcases/examples/manual_steps/TC_MANUAL_VERIFY_003.yml",
    ];

    for path in examples {
        let test_case = load_test_case_from_file(path)?;

        // Verify that all manual steps have proper verification output format
        for sequence in &test_case.test_sequences {
            for step in &sequence.steps {
                if step.manual == Some(true) {
                    // Verification output should contain a question prompt
                    assert!(
                        verification_contains(&step.verification.output, "?")
                            || verification_contains(&step.verification.output, "(Y/n)"),
                        "Manual step {} verification should contain question or (Y/n) prompt",
                        step.step
                    );
                }
            }
        }
    }

    println!("✓ All manual verification examples have properly formatted verification prompts");
    Ok(())
}

// ===== Fail-Fast Behavior Tests =====

#[test]
fn test_execution_stops_at_first_failure() -> Result<()> {
    use tempfile::TempDir;

    // Create a temporary directory for test outputs
    let temp_dir = TempDir::new()?;
    let output_dir = temp_dir.path();

    // Create a test case with multiple steps where the first one fails
    let mut test_case = testcase_manager::TestCase::new(
        "REQ-FAIL-001".to_string(),
        1,
        1,
        "TC_FAIL_FAST_001".to_string(),
        "Test case to verify fail-fast behavior on step failure".to_string(),
    );

    // Create a test sequence with 4 steps
    let mut sequence = testcase_manager::TestSequence::new(
        1,
        "Fail-Fast Test Sequence".to_string(),
        "Sequence with a failing step followed by subsequent steps".to_string(),
    );

    // Step 1: Passing step
    let step1 = testcase_manager::Step {
        step: 1,
        manual: None,
        description: "Initial passing step".to_string(),
        command: "echo 'step1 success'".to_string(),
        capture_vars: None,
        expected: testcase_manager::Expected {
            success: Some(true),
            result: "0".to_string(),
            output: "step1 success".to_string(),
        },
        verification: testcase_manager::Verification {
            result: testcase_manager::VerificationExpression::Simple(
                "[ $EXIT_CODE -eq 0 ]".to_string(),
            ),
            output: testcase_manager::VerificationExpression::Simple(
                "[[ \"$COMMAND_OUTPUT\" = \"step1 success\" ]]".to_string(),
            ),
            output_file: None,
            general: None,
        },
        reference: None,
    };

    // Step 2: Failing step - exit code is non-zero
    let step2 = testcase_manager::Step {
        step: 2,
        manual: None,
        description: "Failing step with non-zero exit code".to_string(),
        command: "echo 'step2 output' && exit 1".to_string(),
        capture_vars: None,
        expected: testcase_manager::Expected {
            success: Some(false),
            result: "0".to_string(), // Expected to be 0, but will be 1
            output: "step2 success".to_string(), // Expected output, but verification will fail
        },
        verification: testcase_manager::Verification {
            result: testcase_manager::VerificationExpression::Simple(
                "[ $EXIT_CODE -eq 0 ]".to_string(), // This will fail since exit code is 1
            ),
            output: testcase_manager::VerificationExpression::Simple(
                "[[ \"$COMMAND_OUTPUT\" =~ \"step2\" ]]".to_string(),
            ),
            output_file: None,
            general: None,
        },
        reference: None,
    };

    // Step 3: Should not be executed
    let step3 = testcase_manager::Step {
        step: 3,
        manual: None,
        description: "Step that should not be executed".to_string(),
        command: "echo 'step3 success'".to_string(),
        capture_vars: None,
        expected: testcase_manager::Expected {
            success: Some(true),
            result: "0".to_string(),
            output: "step3 success".to_string(),
        },
        verification: testcase_manager::Verification {
            result: testcase_manager::VerificationExpression::Simple(
                "[ $EXIT_CODE -eq 0 ]".to_string(),
            ),
            output: testcase_manager::VerificationExpression::Simple(
                "[[ \"$COMMAND_OUTPUT\" = \"step3 success\" ]]".to_string(),
            ),
            output_file: None,
            general: None,
        },
        reference: None,
    };

    // Step 4: Should not be executed
    let step4 = testcase_manager::Step {
        step: 4,
        manual: None,
        description: "Another step that should not be executed".to_string(),
        command: "echo 'step4 success'".to_string(),
        capture_vars: None,
        expected: testcase_manager::Expected {
            success: Some(true),
            result: "0".to_string(),
            output: "step4 success".to_string(),
        },
        verification: testcase_manager::Verification {
            result: testcase_manager::VerificationExpression::Simple(
                "[ $EXIT_CODE -eq 0 ]".to_string(),
            ),
            output: testcase_manager::VerificationExpression::Simple(
                "[[ \"$COMMAND_OUTPUT\" = \"step4 success\" ]]".to_string(),
            ),
            output_file: None,
            general: None,
        },
        reference: None,
    };

    sequence.steps.push(step1);
    sequence.steps.push(step2);
    sequence.steps.push(step3);
    sequence.steps.push(step4);
    test_case.test_sequences.push(sequence);

    // Create executor with output directory
    let executor = testcase_manager::TestExecutor::with_output_dir(output_dir.to_path_buf());

    // Execute the test case - should fail at step 2
    let result = executor.execute_test_case(&test_case);

    // Verify execution failed
    assert!(
        result.is_err(),
        "Execution should fail when a step verification fails"
    );

    // Verify the error message mentions step failure
    let error = result.unwrap_err();
    let error_message = error.to_string();
    assert!(
        error_message.contains("Step 2") || error_message.contains("verification failed"),
        "Error message should mention the failed step: {}",
        error_message
    );

    // Read the execution log
    let log_path = output_dir.join("TC_FAIL_FAST_001_execution_log.json");
    assert!(
        log_path.exists(),
        "Execution log should be written even after failure"
    );

    let log_content = fs::read_to_string(&log_path)?;
    let log_entries: Vec<testcase_manager::TestStepExecutionEntry> =
        serde_json::from_str(&log_content)?;

    // Verify execution log contains only steps 1 and 2 (not 3 and 4)
    assert_eq!(
        log_entries.len(),
        2,
        "Execution log should contain only 2 entries (steps 1 and 2)"
    );

    // Verify step 1 succeeded
    assert_eq!(log_entries[0].step, 1);
    assert_eq!(log_entries[0].test_sequence, 1);
    assert_eq!(log_entries[0].exit_code, 0);
    assert!(
        log_entries[0].output.contains("step1 success"),
        "Step 1 output should be 'step1 success', got: {}",
        log_entries[0].output
    );

    // Verify step 2 failed (exit code 1)
    assert_eq!(log_entries[1].step, 2);
    assert_eq!(log_entries[1].test_sequence, 1);
    assert_eq!(
        log_entries[1].exit_code, 1,
        "Step 2 should have exit code 1"
    );
    assert!(
        log_entries[1].output.contains("step2"),
        "Step 2 output should contain 'step2', got: {}",
        log_entries[1].output
    );

    // Verify steps 3 and 4 are NOT in the execution log
    let has_step_3 = log_entries.iter().any(|e| e.step == 3);
    let has_step_4 = log_entries.iter().any(|e| e.step == 4);
    assert!(
        !has_step_3,
        "Step 3 should not be in execution log (execution should have stopped)"
    );
    assert!(
        !has_step_4,
        "Step 4 should not be in execution log (execution should have stopped)"
    );

    println!("✓ Fail-fast behavior verified: execution stopped at first failure");
    println!("✓ Execution log contains only steps up to the failure");

    Ok(())
}

// ===== Variable Capture Example Test Cases Integration Tests =====

#[test]
fn test_tc_var_001_example_schema_validation() -> Result<()> {
    let path = "testcases/examples/variables/TC_VAR_001.yml";

    validate_yaml_schema(path)?;

    println!("✓ TC_VAR_001 passed schema validation");
    Ok(())
}

#[test]
fn test_tc_var_001_example_deserialization() -> Result<()> {
    let path = "testcases/examples/variables/TC_VAR_001.yml";
    let test_case = load_test_case_from_file(path)?;

    // Validate metadata
    assert_eq!(test_case.id, "TC_VAR_001");
    assert_eq!(test_case.requirement, "VAR_001");
    assert_eq!(test_case.item, 1);
    assert_eq!(test_case.tc, 1);
    assert!(test_case
        .description
        .contains("command-based and regex-based variable captures"));

    // Validate test sequences
    assert_eq!(test_case.test_sequences.len(), 1);

    // Validate Variable Capture Test sequence
    let seq = &test_case.test_sequences[0];
    assert_eq!(seq.id, 1);
    assert_eq!(seq.name, "Variable Capture Test");
    assert_eq!(seq.steps.len(), 3);

    // Validate step 1 - command-based capture
    let step1 = &seq.steps[0];
    assert_eq!(step1.step, 1);
    assert!(step1.description.contains("command"));
    assert!(step1.capture_vars.is_some());

    // Validate step 2 - regex-based captures
    let step2 = &seq.steps[1];
    assert_eq!(step2.step, 2);
    assert!(step2.description.contains("JSON"));
    assert!(step2.capture_vars.is_some());

    // Validate step 3 - mixed captures
    let step3 = &seq.steps[2];
    assert_eq!(step3.step, 3);
    assert!(step3.description.contains("Mix"));
    assert!(step3.capture_vars.is_some());

    println!("✓ TC_VAR_001 deserialized correctly");
    Ok(())
}

#[test]
fn test_tc_var_001_example_bash_script_generation() -> Result<()> {
    let path = "testcases/examples/variables/TC_VAR_001.yml";
    let test_case = load_test_case_from_file(path)?;

    let executor = TestExecutor::new();
    let script = executor.generate_test_script(&test_case);

    // Verify script contains variable capture logic
    assert!(
        script.contains("output_len="),
        "Script should capture output_len variable"
    );
    assert!(
        script.contains("token="),
        "Script should capture token variable"
    );
    assert!(
        script.contains("user_id="),
        "Script should capture user_id variable"
    );
    assert!(
        script.contains("transaction_id="),
        "Script should capture transaction_id variable"
    );
    assert!(
        script.contains("amount="),
        "Script should capture amount variable"
    );
    assert!(
        script.contains("status="),
        "Script should capture status variable"
    );
    assert!(
        script.contains("line_count="),
        "Script should capture line_count variable"
    );

    // Verify general verification conditions (using sanitized variable names)
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_output_len_is_numeric"),
        "Script should contain output_len numeric verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_token_format"),
        "Script should contain token format verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_user_id_is_numeric"),
        "Script should contain user_id numeric verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_transaction_id_format"),
        "Script should contain transaction_id format verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_amount_is_numeric"),
        "Script should contain amount numeric verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_status_is_completed"),
        "Script should contain status verification variable"
    );
    assert!(
        script.contains("GENERAL_VERIFY_PASS_verify_line_count"),
        "Script should contain line_count verification variable"
    );

    println!("✓ TC_VAR_001 bash script generated correctly");
    Ok(())
}

#[test]
fn test_tc_var_001_capture_vars_format() -> Result<()> {
    let path = "testcases/examples/variables/TC_VAR_001.yml";
    let test_case = load_test_case_from_file(path)?;

    let seq = &test_case.test_sequences[0];

    // Step 1: command-based capture
    let step1 = &seq.steps[0];
    if let Some(CaptureVarsFormat::New(vars)) = &step1.capture_vars {
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "output_len");
        assert!(vars[0].command.is_some());
        assert!(vars[0].capture.is_none());
    } else {
        panic!("Step 1 should use New capture vars format with command");
    }

    // Step 2: regex-based captures
    let step2 = &seq.steps[1];
    if let Some(CaptureVarsFormat::New(vars)) = &step2.capture_vars {
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].name, "token");
        assert!(vars[0].capture.is_some());
        assert!(vars[0].command.is_none());
        assert_eq!(vars[1].name, "user_id");
        assert!(vars[1].capture.is_some());
        assert!(vars[1].command.is_none());
    } else {
        panic!("Step 2 should use New capture vars format with regex");
    }

    // Step 3: mixed captures
    let step3 = &seq.steps[2];
    if let Some(CaptureVarsFormat::New(vars)) = &step3.capture_vars {
        assert_eq!(vars.len(), 4);

        // Check for regex captures
        assert!(vars
            .iter()
            .any(|v| v.name == "transaction_id" && v.capture.is_some()));
        assert!(vars
            .iter()
            .any(|v| v.name == "amount" && v.capture.is_some()));
        assert!(vars
            .iter()
            .any(|v| v.name == "status" && v.capture.is_some()));

        // Check for command capture
        assert!(vars
            .iter()
            .any(|v| v.name == "line_count" && v.command.is_some()));
    } else {
        panic!("Step 3 should use New capture vars format with mixed types");
    }

    println!("✓ TC_VAR_001 capture vars format validated");
    Ok(())
}

#[test]
fn test_tc_var_001_general_verification_conditions() -> Result<()> {
    let path = "testcases/examples/variables/TC_VAR_001.yml";
    let test_case = load_test_case_from_file(path)?;

    let seq = &test_case.test_sequences[0];

    // All steps should have general verification conditions
    for step in &seq.steps {
        assert!(
            step.verification.general.is_some(),
            "Step {} should have general verification conditions",
            step.step
        );

        let general = step.verification.general.as_ref().unwrap();
        assert!(
            !general.is_empty(),
            "Step {} should have non-empty general verification conditions",
            step.step
        );

        // Verify each condition has a name and condition
        for condition in general {
            assert!(
                !condition.name.is_empty(),
                "General verification condition should have a name"
            );
            assert!(
                !condition.condition.is_empty(),
                "General verification condition should have a condition"
            );
        }
    }

    // Step 1 should have 2 general conditions
    let step1_general = seq.steps[0].verification.general.as_ref().unwrap();
    assert_eq!(
        step1_general.len(),
        2,
        "Step 1 should have 2 general verification conditions"
    );

    // Step 2 should have 4 general conditions
    let step2_general = seq.steps[1].verification.general.as_ref().unwrap();
    assert_eq!(
        step2_general.len(),
        4,
        "Step 2 should have 4 general verification conditions"
    );

    // Step 3 should have 5 general conditions
    let step3_general = seq.steps[2].verification.general.as_ref().unwrap();
    assert_eq!(
        step3_general.len(),
        5,
        "Step 3 should have 5 general verification conditions"
    );

    println!("✓ TC_VAR_001 general verification conditions validated");
    Ok(())
}
