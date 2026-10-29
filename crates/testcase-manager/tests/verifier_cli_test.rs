use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper function to get the schema path for verifier
fn get_schema_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut workspace_root = PathBuf::from(manifest_dir);
    workspace_root.pop(); // Go up from crate dir
    workspace_root.pop(); // Go up from crates dir
    workspace_root.join("data/testcase_results_container/schema.json")
}

/// Helper to create a minimal valid test case YAML file
fn create_test_case_yaml(dir: &std::path::Path, test_case_id: &str) {
    let yaml_content = format!(
        r#"test_case_id: "{}"
name: "Test Case"
sequences:
  - sequence_id: 1
    name: "Test Sequence"
    steps:
      - step: 1
        description: "Test step"
        command: "echo 'test'"
        expected:
          result: ""
          output: "test"
        verification:
          result: "[[ $? -eq 0 ]]"
          output: "cat $COMMAND_OUTPUT | grep -q 'test'"
"#,
        test_case_id
    );
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(format!("{}.yaml", test_case_id)), yaml_content).unwrap();
}

/// Helper to create a minimal valid execution log JSON file
fn create_execution_log(path: &std::path::Path, test_case_id: &str) {
    let log_content = format!(
        r#"[
  {{
    "test_case_id": "{}",
    "sequence_id": 1,
    "step_number": 1,
    "success": true,
    "actual_result": "",
    "actual_output": "test",
    "timestamp": "2024-01-01T00:00:00Z"
  }}
]"#,
        test_case_id
    );
    fs::write(path, log_content).unwrap();
}

// ============================================================================
// Mutually Exclusive Mode Tests
// ============================================================================

#[test]
fn test_missing_required_args() {
    let temp_dir = TempDir::new().unwrap();
    let test_case_dir = temp_dir.path().join("testcases");
    create_test_case_yaml(&test_case_dir, "TC001");

    // No arguments provided - should fail
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Must specify either single-file mode")
            || stderr.contains("--log and --test-case")
            || stderr.contains("--folder")
    );
}

#[test]
fn test_single_file_mode_missing_test_case_id() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Only --log provided without --test-case
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Must specify either single-file mode")
            || stderr.contains("--log and --test-case")
            || stderr.contains("--folder")
    );
}

#[test]
fn test_single_file_mode_missing_log_file() {
    let temp_dir = TempDir::new().unwrap();
    let test_case_dir = temp_dir.path().join("testcases");
    create_test_case_yaml(&test_case_dir, "TC001");

    // Only --test-case provided without --log
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Must specify either single-file mode")
            || stderr.contains("--log and --test-case")
            || stderr.contains("--folder")
    );
}

#[test]
fn test_mutually_exclusive_modes_conflict() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");
    fs::create_dir_all(&folder_path).unwrap();

    // Both single-file mode and folder mode provided
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cannot use both single-file mode")
            || stderr.contains("simultaneously")
            || stderr.contains("folder mode")
    );
}

// ============================================================================
// Format Validation Tests
// ============================================================================

#[test]
fn test_invalid_format_uppercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Invalid format: XML (not supported)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "XML",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Format must be 'yaml' or 'json'") || stderr.contains("xml"));
}

#[test]
fn test_invalid_format_lowercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Invalid format: html
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "html",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Format must be 'yaml' or 'json'") || stderr.contains("html"));
}

#[test]
fn test_valid_format_yaml_lowercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Valid format: yaml (lowercase)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "yaml",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    // Should succeed (format validation passes)
    // The command might still fail for other reasons, but not format validation
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Format must be 'yaml' or 'json'"));
}

#[test]
fn test_valid_format_yaml_uppercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Valid format: YAML (uppercase - should be normalized to lowercase)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "YAML",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Format must be 'yaml' or 'json'"));
}

#[test]
fn test_valid_format_json_lowercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Valid format: json (lowercase)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "json",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Format must be 'yaml' or 'json'"));
}

#[test]
fn test_valid_format_json_uppercase() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Valid format: JSON (uppercase)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "JSON",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Format must be 'yaml' or 'json'"));
}

// ============================================================================
// File Existence Tests
// ============================================================================

#[test]
fn test_single_file_mode_log_file_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("nonexistent_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");

    // Log file does not exist
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Log file does not exist") || stderr.contains("nonexistent_log.json"));
}

#[test]
fn test_single_file_mode_log_file_exists() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Log file exists - validation should pass (other errors may occur but not file existence)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Log file does not exist"));
}

// ============================================================================
// Directory Existence Tests
// ============================================================================

#[test]
fn test_folder_mode_folder_not_exists() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("nonexistent_folder");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");

    // Folder does not exist
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Folder does not exist") || stderr.contains("nonexistent_folder"));
}

#[test]
fn test_folder_mode_path_is_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("not_a_directory.txt");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::write(&file_path, "not a directory").unwrap();

    // Path exists but is a file, not a directory
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            file_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Path is not a directory")
            || stderr.contains("not_a_directory.txt")
            || stderr.contains("is not a dir")
    );
}

#[test]
fn test_folder_mode_folder_exists() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();

    // Folder exists - validation should pass
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Folder does not exist"));
    assert!(!stderr.contains("Path is not a directory"));
}

// ============================================================================
// Successful Execution Tests
// ============================================================================

#[test]
fn test_single_file_mode_success_yaml_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "yaml",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));
    assert!(!stderr.contains("is not a directory"));

    // Output should contain YAML-formatted report (on stdout or stderr)
    let all_output = format!("{}{}", stdout, stderr);
    assert!(
        all_output.contains("test_cases:")
            || all_output.contains("test_case_id:")
            || all_output.contains("Processing log file")
    );
}

#[test]
fn test_single_file_mode_success_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "json",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));

    // Output should contain JSON-formatted report (on stdout or stderr)
    let all_output = format!("{}{}", stdout, stderr);
    assert!(
        (all_output.contains("{")
            && (all_output.contains("test_cases") || all_output.contains("testCases")))
            || all_output.contains("Processing log file")
    );
}

#[test]
fn test_single_file_mode_with_output_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let output_file = temp_dir.path().join("report.yaml");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--output",
            output_file.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));

    // Output file should be created or processing should have occurred
    if output_file.exists() {
        // Read and verify output file content
        let content = fs::read_to_string(&output_file).unwrap();
        assert!(
            content.contains("test_cases:")
                || content.contains("test_case_id:")
                || !content.is_empty()
        );
    } else {
        // If output file wasn't created, at least processing should have occurred
        assert!(stderr.contains("Processing log file") || stderr.contains("Failed"));
    }
}

#[test]
fn test_folder_mode_success_empty_folder() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));
    assert!(!stderr.contains("is not a directory"));

    // Should warn about no log files found
    assert!(
        stderr.contains("No execution log files")
            || stderr.contains("0")
            || output.status.success()
    );
}

#[test]
fn test_folder_mode_success_with_logs() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");
    let log_file = folder_path.join("TC001_execution_log.json");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();
    create_execution_log(&log_file, "TC001");

    let schema_path = get_schema_path();

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
            "--schema",
            schema_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));
    assert!(!stderr.contains("is not a directory"));

    // Should find and process log file
    assert!(stderr.contains("Found 1") || stderr.contains("Processing"));

    // Output should contain report (verifier outputs YAML to stderr via log::info!)
    assert!(
        stderr.contains("test_cases:")
            || stderr.contains("test_case_id:")
            || stderr.contains("type:")
    );
}

// ============================================================================
// Short Flag Tests
// ============================================================================

#[test]
fn test_short_flags_single_file_mode() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Use short flags: -l (log), -c (test-case), -F (format), -d (test-case-dir)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "-l",
            log_file.to_str().unwrap(),
            "-c",
            "TC001",
            "-F",
            "json",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));
}

#[test]
fn test_short_flags_folder_mode() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let output_file = temp_dir.path().join("report.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();
    // A folder without logs verified nothing, which is an error (TCMS-41), so give it one
    create_execution_log(&folder_path.join("TC001_execution_log.json"), "TC001");

    let schema_path = get_schema_path();

    // Use short flags: -f (folder), -F (format), -o (output), -d (test-case-dir)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "-f",
            folder_path.to_str().unwrap(),
            "-F",
            "json",
            "-o",
            output_file.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
            "--schema",
            schema_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Must specify either"));
    assert!(!stderr.contains("Cannot use both"));
    assert!(!stderr.contains("Format must be"));
    assert!(!stderr.contains("does not exist"));
    assert!(!stderr.contains("is not a directory"));

    // Output file should be created
    assert!(output_file.exists());
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_mixed_case_format() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Mixed case format: YaMl
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "--format",
            "YaMl",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should accept mixed case (normalized to lowercase)
    assert!(!stderr.contains("Format must be 'yaml' or 'json'"));
}

#[test]
fn test_default_format_is_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // No format specified - should default to yaml
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have validation errors
    assert!(!stderr.contains("Format must be"));

    // Output should be in YAML format (default) - check both stdout and stderr
    let all_output = format!("{}{}", stdout, stderr);
    assert!(
        all_output.contains("test_results:")
            || all_output.contains("test_case_id:")
            || all_output.contains("Processing log file")
    );
}

#[test]
fn test_default_test_case_dir() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");

    // Create test case in default location "testcases" (relative to current dir)
    // Note: This test might fail if run from a different directory
    // We'll just verify that the default value is accepted
    create_execution_log(&log_file, "TC001");

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not complain about missing test-case-dir argument
    // (but might fail because test case doesn't exist in default location)
    assert!(!stderr.contains("required arguments"));
}

// ============================================================================
// Verbose Flag Tests
// ============================================================================

#[test]
fn test_verbose_flag_enables_debug_logging_single_file_mode() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Run with --verbose flag
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages are present
    // Check for debug messages that should appear in single-file mode
    assert!(
        stderr.contains("Single-file mode: test case ID")
            || stderr.contains("Parsing log file:")
            || stderr.contains("Loading test case definition")
            || stderr.contains("Successfully parsed log file")
            || stderr.contains("Verifying test case")
            || stderr.contains("Creating batch report"),
        "Expected debug messages in verbose mode, but stderr was: {}",
        stderr
    );
}

#[test]
fn test_verbose_flag_short_form_enables_debug_logging() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Run with -v flag (short form)
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
            "-v",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages are present
    assert!(
        stderr.contains("Single-file mode: test case ID")
            || stderr.contains("Parsing log file:")
            || stderr.contains("Loading test case definition")
            || stderr.contains("Successfully parsed log file")
            || stderr.contains("Verifying test case")
            || stderr.contains("Creating batch report"),
        "Expected debug messages in verbose mode, but stderr was: {}",
        stderr
    );
}

#[test]
fn test_without_verbose_flag_debug_messages_suppressed_single_file_mode() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.path().join("TC001_execution_log.json");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_execution_log(&log_file, "TC001");

    // Run WITHOUT --verbose flag
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--log",
            log_file.to_str().unwrap(),
            "--test-case",
            "TC001",
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages are NOT present
    // These specific debug messages should not appear without verbose flag
    assert!(
        !stderr.contains("Single-file mode: test case ID"),
        "Debug message 'Single-file mode: test case ID' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Successfully parsed log file with"),
        "Debug message 'Successfully parsed log file with' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Successfully loaded test case")
            || !stderr.contains("with")
            || !stderr.contains("test sequences"),
        "Debug message about successfully loaded test case should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Creating batch report with single test case result"),
        "Debug message 'Creating batch report' should not appear without verbose flag"
    );

    // Info-level messages should still be present
    assert!(
        stderr.contains("Processing log file:") || output.status.success(),
        "Info-level messages should still be present"
    );
}

#[test]
fn test_verbose_flag_enables_debug_logging_folder_mode() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");
    let log_file = folder_path.join("TC001_execution_log.json");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();
    create_execution_log(&log_file, "TC001");

    // Run with --verbose flag in folder mode
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages are present
    // Check for debug messages that should appear in folder mode
    assert!(
        stderr.contains("Starting folder discovery mode")
            || stderr.contains("Discovered log files:")
            || stderr.contains("Extracted test case ID:")
            || stderr.contains("Parsing log file:")
            || stderr.contains("Loading test case definition")
            || stderr.contains("Verifying test case")
            || stderr.contains("Added test case result to batch report")
            || stderr.contains("Folder mode processing complete"),
        "Expected debug messages in verbose mode, but stderr was: {}",
        stderr
    );
}

#[test]
fn test_without_verbose_flag_debug_messages_suppressed_folder_mode() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");
    let log_file = folder_path.join("TC001_execution_log.json");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();
    create_execution_log(&log_file, "TC001");

    // Run WITHOUT --verbose flag in folder mode
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages are NOT present
    assert!(
        !stderr.contains("Starting folder discovery mode for:"),
        "Debug message 'Starting folder discovery mode for:' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Discovered log files:"),
        "Debug message 'Discovered log files:' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Extracted test case ID:"),
        "Debug message 'Extracted test case ID:' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Successfully parsed log file with"),
        "Debug message 'Successfully parsed log file with' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Successfully loaded test case")
            || !stderr.contains("with")
            || !stderr.contains("test sequences"),
        "Debug message about successfully loaded test case should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Added test case result to batch report"),
        "Debug message 'Added test case result to batch report' should not appear without verbose flag"
    );
    assert!(
        !stderr.contains("Folder mode processing complete"),
        "Debug message 'Folder mode processing complete' should not appear without verbose flag"
    );

    // Info-level messages should still be present
    assert!(
        stderr.contains("Found 1 execution log file") || output.status.success(),
        "Info-level messages should still be present"
    );
}

#[test]
fn test_verbose_flag_with_empty_folder() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    fs::create_dir_all(&folder_path).unwrap();

    // Run with --verbose flag on empty folder
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug-level messages about empty folder are present
    assert!(
        stderr.contains("Starting folder discovery mode")
            || stderr.contains("Folder discovery completed with no files found")
            || stderr.contains("Log file discovery complete"),
        "Expected debug messages about empty folder in verbose mode, but stderr was: {}",
        stderr
    );
}

#[test]
fn test_verbose_flag_with_multiple_log_files() {
    let temp_dir = TempDir::new().unwrap();
    let folder_path = temp_dir.path().join("logs");
    let test_case_dir = temp_dir.path().join("testcases");

    create_test_case_yaml(&test_case_dir, "TC001");
    create_test_case_yaml(&test_case_dir, "TC002");
    fs::create_dir_all(&folder_path).unwrap();
    create_execution_log(&folder_path.join("TC001_execution_log.json"), "TC001");
    create_execution_log(&folder_path.join("TC002_execution_log.json"), "TC002");

    // Run with --verbose flag
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "verifier",
            "--bin",
            "verifier",
            "--",
            "--folder",
            folder_path.to_str().unwrap(),
            "-d",
            test_case_dir.to_str().unwrap(),
            "--verbose",
        ])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify that debug messages appear for multiple files
    assert!(
        stderr.contains("Discovered log files:"),
        "Expected debug message about discovered files"
    );
    assert!(
        stderr.contains("Processing log file 1/2") || stderr.contains("Processing log file 2/2"),
        "Expected debug messages about processing multiple files"
    );
    assert!(
        stderr.contains("Folder mode processing complete"),
        "Expected final debug message about completion"
    );
}
