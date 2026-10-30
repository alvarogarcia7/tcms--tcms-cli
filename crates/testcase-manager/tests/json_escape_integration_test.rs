// use anyhow::Result;
// use serde_json::Value;
// use std::fs;
// use std::path::PathBuf;
// use std::process::Command;
// use tempfile::TempDir;
// use testcase_manager::config::{
//     Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
// };
// use testcase_manager::executor::TestExecutor;
// use testcase_models::{
//     Expected, Step, TestCase, TestSequence, Verification, VerificationExpression,
// };
//
// // ============================================================================
// // Integration Test: Script Generation + Execution with JSON Escape
// // ============================================================================
// //
// // This test suite validates the JSON escaping functionality in generated test
// // scripts, with comprehensive cross-platform compatibility testing for macOS
// // and Linux environments.
// //
// // Test Categories:
// // ----------------
// //
// // 1. **Basic JSON Escape Tests**
// //    - test_generated_script_with_json_escape_special_chars
// //    - test_generated_script_auto_mode_with_binary
// //    - test_generated_script_auto_mode_without_binary
// //    - test_generated_script_shell_fallback_mode
// //
// // 2. **Complex Special Character Tests**
// //    - test_script_execution_complex_special_characters
// //    - test_shell_fallback_with_extreme_characters
// //    - test_large_output_handling
// //
// // 3. **Error Handling Tests**
// //    - test_json_escape_binary_crash_handling
// //    - test_json_escape_auto_fallback_on_crash
// //
// // 4. **Concurrency Tests**
// //    - test_concurrent_script_execution
// //    - test_concurrent_with_shared_output_dir
// //
// // 5. **Cross-Platform Compatibility Tests** ⭐ NEW ⭐
// //    - test_bash_32_compatibility                (verifies bash 3.2+ compatibility)
// //    - test_sed_bsd_compatibility                (verifies sed uses -E not -r)
// //    - test_awk_cross_platform_compatibility     (verifies portable awk patterns)
// //    - test_script_execution_cross_platform      (executes on current platform)
// //    - test_newline_handling_cross_platform      (tests LF/CRLF/CR handling)
// //    - test_sed_awk_fallback_special_chars       (tests backslash/quote/tab escaping)
// //    - test_printf_usage_for_portability         (verifies printf over echo)
// //    - test_posix_shell_compatibility            (checks POSIX-friendly constructs)
// //    - test_sed_uses_basic_patterns_only         (verifies basic sed syntax only)
// //    - test_awk_printf_pattern_compatibility     (verifies awk printf patterns)
// //    - test_sed_awk_execution_cross_platform     (executes sed/awk on platform)
// //    - test_empty_output_cross_platform          (handles empty output edge case)
// //    - test_printf_for_portability               (verifies printf usage)
// //    - test_no_trailing_whitespace_issues        (handles whitespace correctly)
// //
// // 6. **Utility Tests**
// //    - test_script_json_validation_with_jq
// //    - test_script_output_escaped_variable
// //
// // To run all tests:
// //   cargo test --all-features --test json_escape_integration_test
// //
// // To run specific test categories:
// //   cargo test --all-features --test json_escape_integration_test compatibility
// //   cargo test --all-features --test json_escape_integration_test cross_platform
// //   cargo test --all-features --test json_escape_integration_test fallback
// //
// // To run just the new compatibility tests (as requested):
// //   cargo test --all-features -- json_escape_integration_test::test_bash_32_compatibility
// //   cargo test --all-features -- json_escape_integration_test::test_sed_bsd_compatibility
// //   cargo test --all-features -- json_escape_integration_test::test_awk_cross_platform_compatibility
// //   cargo test --all-features -- json_escape_integration_test::test_script_execution_cross_platform
// //   cargo test --all-features -- json_escape_integration_test::test_newline_handling_cross_platform
// //   cargo test --all-features -- json_escape_integration_test::test_sed_awk_fallback_special_chars
// //   cargo test --all-features -- json_escape_integration_test::test_posix_shell_compatibility
// //   cargo test --all-features -- json_escape_integration_test::test_sed_uses_basic_patterns_only
// //   cargo test --all-features -- json_escape_integration_test::test_awk_printf_pattern_compatibility
// //   cargo test --all-features -- json_escape_integration_test::test_sed_awk_execution_cross_platform
// //   cargo test --all-features -- json_escape_integration_test::test_empty_output_cross_platform
// //   cargo test --all-features -- json_escape_integration_test::test_printf_for_portability
// //   cargo test --all-features -- json_escape_integration_test::test_no_trailing_whitespace_issues
// //
// // Or run all compatibility tests with pattern matching:
// //   cargo test --all-features -- json_escape_integration_test::test_.*compatibility
// //
// // ============================================================================
//
// // /// Test that generated scripts with json-escape produce valid JSON logs with special characters
// // #[test]
// // fn test_generated_script_with_json_escape_special_chars() -> Result<()> {
// //     let temp_dir = TempDir::new()?;
// //
// //     // Create config that uses RustBinary method
// //     let config = Config {
// //         script_generation: ScriptGenerationConfig {
// //             json_escaping: JsonEscapingConfig {
// //                 method: JsonEscapingMethod::RustBinary,
// //                 enabled: true,
// //                 binary_path: None, // Use default json-escape from PATH
// //             },
// //         },
// //         ..Default::default()
// //     };
// //
// //     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
// //
// //     // Create test case with commands that output special characters
// //     let test_case = create_test_case_with_special_characters();
// //
// //     // Generate the test script
// //     let script = executor.generate_test_script(&test_case);
// //     let script_path = temp_dir.path().join("test_script.sh");
// //     fs::write(&script_path, &script)?;
// //
// //     // Make script executable
// //     #[cfg(unix)]
// //     {
// //         use std::os::unix::fs::PermissionsExt;
// //         let mut perms = fs::metadata(&script_path)?.permissions();
// //         perms.set_mode(0o755);
// //         fs::set_permissions(&script_path, perms)?;
// //     }
// //
// //     // Build json-escape binary first to ensure it's available
// //     let build_output = Command::new("cargo")
// //         .args(["build", "--bin", "json-escape"])
// //         .output()?;
// //
// //     if !build_output.status.success() {
// //         eprintln!(
// //             "Failed to build json-escape: {}",
// //             String::from_utf8_lossy(&build_output.stderr)
// //         );
// //         return Err(anyhow::anyhow!("Failed to build json-escape binary"));
// //     }
// //
// //     // Execute the generated script
// //     let output = Command::new("bash")
// //         .arg(&script_path)
// //         .current_dir(temp_dir.path())
// //         .output()?;
// //
// //     // Script should succeed
// //     if !output.status.success() {
// //         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
// //         eprintln!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
// //     }
// //
// //     // Read the generated JSON log
// //     let json_log_path = temp_dir
// //         .path()
// //         .join(format!("{}_execution_log.json", test_case.id));
// //     assert!(
// //         json_log_path.exists(),
// //         "JSON log file should be created: {}",
// //         json_log_path.display()
// //     );
// //
// //     let json_content = fs::read_to_string(&json_log_path)?;
// //
// //     // Validate JSON is parseable
// //     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
// //     assert!(
// //         parsed.is_ok(),
// //         "Generated JSON should be valid, got error: {:?}",
// //         parsed.err()
// //     );
// //
// //     let json_value = parsed.unwrap();
// //     assert!(json_value.is_array(), "JSON should be an array");
// //
// //     let entries = json_value.as_array().unwrap();
// //     assert_eq!(
// //         entries.len(),
// //         4,
// //         "Should have 4 entries for 4 non-manual steps"
// //     );
// //
// //     // Validate that special characters are properly escaped in JSON
// //     for entry in entries {
// //         assert!(entry["output"].is_string(), "output should be a string");
// //         let output_str = entry["output"].as_str().unwrap();
// //
// //         // Verify no unescaped characters that would break JSON
// //         // The JSON parser already validated this by successfully parsing,
// //         // but let's do some sanity checks
// //         assert!(
// //             !output_str.contains("\n\n"),
// //             "Should not have literal newlines in JSON string"
// //         );
// //     }
// //
// //     Ok(())
// // }
//
// // /// Test script generation with Auto mode - tries json-escape, falls back to shell
// // #[test]
// // fn test_generated_script_auto_mode_with_binary() -> Result<()> {
// //     let temp_dir = TempDir::new()?;
// //
// //     // Create config with Auto mode
// //     let config = Config {
// //         script_generation: ScriptGenerationConfig {
// //             json_escaping: JsonEscapingConfig {
// //                 method: JsonEscapingMethod::Auto,
// //                 enabled: true,
// //                 binary_path: None,
// //             },
// //         },
// //         ..Default::default()
// //     };
// //
// //     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
// //     let test_case = create_test_case_with_special_characters();
// //
// //     let script = executor.generate_test_script(&test_case);
// //
// //     // Verify script contains both paths
// //     assert!(
// //         script.contains("if command -v json-escape"),
// //         "Script should check for json-escape binary"
// //     );
// //     assert!(
// //         script.contains("else"),
// //         "Script should have fallback branch"
// //     );
// //     assert!(
// //         script.contains("Shell fallback"),
// //         "Script should have shell fallback"
// //     );
// //
// //     let script_path = temp_dir.path().join("test_auto.sh");
// //     fs::write(&script_path, &script)?;
// //
// //     #[cfg(unix)]
// //     {
// //         use std::os::unix::fs::PermissionsExt;
// //         let mut perms = fs::metadata(&script_path)?.permissions();
// //         perms.set_mode(0o755);
// //         fs::set_permissions(&script_path, perms)?;
// //     }
// //
// //     // Build json-escape binary
// //     Command::new("cargo")
// //         .args(["build", "--bin", "json-escape"])
// //         .output()?;
// //
// //     // Execute script with json-escape available
// //     let output = Command::new("bash")
// //         .arg(&script_path)
// //         .current_dir(temp_dir.path())
// //         .output()?;
// //
// //     if !output.status.success() {
// //         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
// //     }
// //
// //     // Validate JSON output
// //     let json_log_path = temp_dir
// //         .path()
// //         .join(format!("{}_execution_log.json", test_case.id));
// //
// //     if json_log_path.exists() {
// //         let json_content = fs::read_to_string(&json_log_path)?;
// //         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
// //         assert!(parsed.is_ok(), "Generated JSON should be valid");
// //     }
// //
// //     Ok(())
// // }
//
// /// Test script with Auto mode when json-escape binary is NOT available
// #[test]
// fn test_generated_script_auto_mode_without_binary() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     // Create config with Auto mode but point to non-existent binary
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: Some(PathBuf::from("/nonexistent/json-escape")),
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_fallback.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script - should use shell fallback
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Validate JSON output - shell fallback should still produce valid JSON
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     if json_log_path.exists() {
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         assert!(
//             parsed.is_ok(),
//             "JSON from shell fallback should be valid, error: {:?}",
//             parsed.err()
//         );
//     }
//
//     Ok(())
// }
//
// /// Test script with ShellFallback mode - does not use json-escape binary
// #[test]
// fn test_generated_script_shell_fallback_mode() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     // Create config that only uses shell fallback
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify script only uses shell fallback
//     assert!(
//         script.contains("Shell fallback"),
//         "Script should use shell fallback"
//     );
//     assert!(
//         !script.contains("if command -v json-escape"),
//         "Script should not check for json-escape"
//     );
//     assert!(
//         script.contains("sed 's/\\\\/\\\\\\\\/g"),
//         "Script should have sed escaping"
//     );
//     assert!(script.contains("awk"), "Script should use awk for newlines");
//
//     let script_path = temp_dir.path().join("test_shell_only.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Validate JSON output
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     if json_log_path.exists() {
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         assert!(parsed.is_ok(), "JSON from shell fallback should be valid");
//     }
//
//     Ok(())
// }
//
// /// Test that script properly escapes complex scenarios with multiple special chars
// // #[test]
// // fn test_script_execution_complex_special_characters() -> Result<()> {
// //     let temp_dir = TempDir::new()?;
// //
// //     let config = Config {
// //         script_generation: ScriptGenerationConfig {
// //             json_escaping: JsonEscapingConfig {
// //                 method: JsonEscapingMethod::Auto,
// //                 enabled: true,
// //                 binary_path: None,
// //             },
// //         },
// //         ..Default::default()
// //     };
// //
// //     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
// //
// //     // Create test case with very complex special characters
// //     let mut test_case = TestCase::new(
// //         "REQ001".to_string(),
// //         1,
// //         1,
// //         "COMPLEX_CHARS_TC".to_string(),
// //         "Test complex character escaping".to_string(),
// //     );
// //
// //     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Complex chars".to_string());
// //
// //     // Step with backslashes
// //     sequence.steps.push(Step {
// //         step: 1,
// //         manual: None,
// //         description: "Backslashes test".to_string(),
// //         command: r#"echo 'Path: C:\Users\test\file.txt'"#.to_string(),
// //         capture_vars: None,
// //         expected: Expected {
// //             success: Some(true),
// //             result: "0".to_string(),
// //             output: "true".to_string(),
// //         },
// //         verification: Verification {
// //             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
// //             output: VerificationExpression::Simple("true".to_string()),
// //             output_file: None,
// //             general: None,
// //         },
// //         reference: None,
// //     });
// //
// //     // Step with quotes
// //     sequence.steps.push(Step {
// //         step: 2,
// //         manual: None,
// //         description: "Quotes test".to_string(),
// //         command: r#"echo 'He said "hello world"'"#.to_string(),
// //         capture_vars: None,
// //         expected: Expected {
// //             success: Some(true),
// //             result: "0".to_string(),
// //             output: "true".to_string(),
// //         },
// //         verification: Verification {
// //             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
// //             output: VerificationExpression::Simple("true".to_string()),
// //             output_file: None,
// //             general: None,
// //         },
// //         reference: None,
// //     });
// //
// //     // Step with mixed special characters
// //     sequence.steps.push(Step {
// //         step: 3,
// //         manual: None,
// //         description: "Mixed special chars".to_string(),
// //         command: r#"printf 'Line1\nLine2\tTabbed\rReturn'"#.to_string(),
// //         capture_vars: None,
// //         expected: Expected {
// //             success: Some(true),
// //             result: "0".to_string(),
// //             output: "true".to_string(),
// //         },
// //         verification: Verification {
// //             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
// //             output: VerificationExpression::Simple("true".to_string()),
// //             output_file: None,
// //             general: None,
// //         },
// //         reference: None,
// //     });
// //
// //     test_case.test_sequences.push(sequence);
// //
// //     let script = executor.generate_test_script(&test_case);
// //     let script_path = temp_dir.path().join("test_complex.sh");
// //     fs::write(&script_path, &script)?;
// //
// //     #[cfg(unix)]
// //     {
// //         use std::os::unix::fs::PermissionsExt;
// //         let mut perms = fs::metadata(&script_path)?.permissions();
// //         perms.set_mode(0o755);
// //         fs::set_permissions(&script_path, perms)?;
// //     }
// //
// //     // Build json-escape binary
// //     Command::new("cargo")
// //         .args(["build", "--bin", "json-escape"])
// //         .output()?;
// //
// //     // Execute script
// //     let output = Command::new("bash")
// //         .arg(&script_path)
// //         .current_dir(temp_dir.path())
// //         .output()?;
// //
// //     if !output.status.success() {
// //         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
// //     }
// //
// //     // Validate JSON
// //     let json_log_path = temp_dir
// //         .path()
// //         .join(format!("{}_execution_log.json", test_case.id));
// //
// //     assert!(
// //         json_log_path.exists(),
// //         "JSON log should be created: {}",
// //         json_log_path.display()
// //     );
// //
// //     let json_content = fs::read_to_string(&json_log_path)?;
// //
// //     // Must be valid JSON
// //     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
// //     assert!(
// //         parsed.is_ok(),
// //         "Complex JSON should be valid, error: {:?}",
// //         parsed.err()
// //     );
// //
// //     let json_value = parsed.unwrap();
// //     let entries = json_value.as_array().unwrap();
// //     assert_eq!(entries.len(), 3, "Should have 3 entries");
// //
// //     // Verify each entry has properly escaped output
// //     for entry in entries {
// //         let output_str = entry["output"].as_str().unwrap();
// //         // If it parsed as valid JSON, the escaping worked
// //         assert!(
// //             !output_str.is_empty() || entry["command"].as_str().unwrap().contains("printf"),
// //             "Output should exist or be empty for printf"
// //         );
// //     }
// //
// //     Ok(())
// // }
//
// /// Test script validation with jq (if available)
// #[test]
// fn test_script_json_validation_with_jq() -> Result<()> {
//     // Check if jq is available
//     let jq_check = Command::new("which").arg("jq").output();
//
//     if jq_check.is_err() || !jq_check.unwrap().status.success() {
//         println!("Skipping test: jq not available");
//         return Ok(());
//     }
//
//     let temp_dir = TempDir::new()?;
//
//     let config = Config::default();
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Script should contain jq validation
//     assert!(
//         script.contains("jq empty"),
//         "Script should validate JSON with jq"
//     );
//     assert!(
//         script.contains("if command -v jq"),
//         "Script should check for jq availability"
//     );
//
//     Ok(())
// }
//
// /// Test that OUTPUT_ESCAPED variable is properly set in generated scripts
// #[test]
// fn test_script_output_escaped_variable() -> Result<()> {
//     let config = Config::default();
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_simple_test_case();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify OUTPUT_ESCAPED is used
//     assert!(
//         script.contains("OUTPUT_ESCAPED="),
//         "Script should set OUTPUT_ESCAPED variable"
//     );
//     assert!(
//         script.contains(r#"\"output\": \"$OUTPUT_ESCAPED\""#),
//         "Script should use OUTPUT_ESCAPED in JSON log"
//     );
//     assert!(
//         script.contains("COMMAND_OUTPUT"),
//         "Script should capture COMMAND_OUTPUT"
//     );
//
//     Ok(())
// }
//
// // ============================================================================
// // Helper Functions
// // ============================================================================
//
// /// Create a test case with commands that output special characters
// fn create_test_case_with_special_characters() -> TestCase {
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "SPECIAL_CHARS_TC".to_string(),
//         "Test with special characters".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Special chars".to_string());
//
//     // Step with newlines
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Newline test".to_string(),
//         command: r#"printf 'Line1\nLine2\nLine3'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Step with tabs
//     sequence.steps.push(Step {
//         step: 2,
//         manual: None,
//         description: "Tab test".to_string(),
//         command: r#"printf 'Col1\tCol2\tCol3'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Step with quotes
//     sequence.steps.push(Step {
//         step: 3,
//         manual: None,
//         description: "Quote test".to_string(),
//         command: r#"echo 'Test "with" quotes'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Step with backslashes
//     sequence.steps.push(Step {
//         step: 4,
//         manual: None,
//         description: "Backslash test".to_string(),
//         command: r#"echo 'Path: C:\test\file'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//     test_case
// }
//
// /// Create a simple test case for basic testing
// fn create_simple_test_case() -> TestCase {
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "SIMPLE_TC".to_string(),
//         "Simple test".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Simple sequence".to_string());
//
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Echo test".to_string(),
//         command: "echo 'hello'".to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "hello".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//     test_case
// }
//
// // ============================================================================
// // Error Handling Tests
// // ============================================================================
//
// /// Test handling of json-escape binary crash/non-zero exit
// #[test]
// fn test_json_escape_binary_crash_handling() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     // Point to a script that simulates binary crash
//     let fake_binary = temp_dir.path().join("fake-json-escape");
//     fs::write(&fake_binary, "#!/bin/bash\nexit 1\n")?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&fake_binary)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&fake_binary, perms)?;
//     }
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::RustBinary,
//                 enabled: true,
//                 binary_path: Some(fake_binary.clone()),
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_simple_test_case();
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_crash.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script - should handle binary failure gracefully
//     let _output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     // Script might succeed with empty output or handle the error
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     // Check if JSON was created and is parseable
//     if json_log_path.exists() {
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         // JSON should be valid even if binary crashed (due to || echo "")
//         assert!(
//             parsed.is_ok(),
//             "JSON should still be valid after binary crash"
//         );
//     }
//
//     Ok(())
// }
//
// /// Test handling when json-escape binary crashes but Auto mode falls back to shell
// #[test]
// fn test_json_escape_auto_fallback_on_crash() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     // Create a fake json-escape binary that always crashes
//     let fake_binary = temp_dir.path().join("fake-json-escape-crash");
//     fs::write(
//         &fake_binary,
//         "#!/bin/bash\necho 'ERROR: Binary crashed' >&2\nexit 127\n",
//     )?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&fake_binary)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&fake_binary, perms)?;
//     }
//
//     // Use Auto mode which should fall back to shell
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: Some(fake_binary.clone()),
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_auto_fallback.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script - should use shell fallback
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created even with crashing binary in Auto mode"
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid using shell fallback after binary crash"
//     );
//
//     Ok(())
// }
//
// /// Test sed/awk fallback failures with invalid regex patterns
// #[test]
// fn test_shell_fallback_with_extreme_characters() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with extreme special characters that might challenge sed/awk
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "EXTREME_TC".to_string(),
//         "Test extreme characters".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Extreme chars".to_string());
//
//     // Binary data, control characters, null bytes (printed as string)
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Extreme characters test".to_string(),
//         command: r#"printf '\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f'"#
//             .to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_extreme.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     // This test documents that shell fallback has limitations with control characters
//     // Script execution may fail with validation errors for extreme binary data
//     if !output.status.success() {
//         // Expected failure - shell fallback cannot handle all control characters
//         let stderr = String::from_utf8_lossy(&output.stderr);
//         assert!(
//             stderr.contains("Internal Script Error") || stderr.contains("not valid"),
//             "Script should fail gracefully with validation error for control characters"
//         );
//     }
//
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     if json_log_path.exists() {
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         // If JSON was generated, it might be invalid due to control characters
//         if let Err(ref e) = parsed {
//             eprintln!(
//                 "Expected limitation: JSON parse error with control characters: {:?}",
//                 e
//             );
//         }
//     }
//
//     Ok(())
// }
//
// /// Test extremely large command output (1MB+)
// #[test]
// fn test_large_output_handling() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "LARGE_OUTPUT_TC".to_string(),
//         "Test large output".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Large output".to_string());
//
//     // Generate ~1MB of output with special characters
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Large output test".to_string(),
//         command: r#"for i in $(seq 1 10000); do echo "Line $i with special chars: \"quotes\" and \backslashes\ and tabs	and newlines"; done"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_large.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script with 60 second timeout for large output
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created for large output"
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//
//     // Verify JSON is parseable
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "Large output JSON should be valid, error: {:?}",
//         parsed.err()
//     );
//
//     // Verify the JSON contains the large output
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 1, "Should have 1 entry");
//
//     let output_str = entries[0]["output"].as_str().unwrap();
//     // Output should be large (>100KB in escaped form)
//     assert!(
//         output_str.len() > 100_000,
//         "Escaped output should be large, got {} bytes",
//         output_str.len()
//     );
//
//     Ok(())
// }
//
// /// Test concurrent execution with multiple scripts
// #[test]
// fn test_concurrent_script_execution() -> Result<()> {
//     use std::thread;
//
//     let temp_dir = TempDir::new()?;
//
//     // Build json-escape binary first
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create multiple test cases
//     let test_cases: Vec<TestCase> = (1..=5)
//         .map(|i| {
//             let mut test_case = TestCase::new(
//                 "REQ001".to_string(),
//                 1,
//                 i,
//                 format!("CONCURRENT_TC_{}", i),
//                 format!("Concurrent test {}", i),
//             );
//
//             let mut sequence = TestSequence::new(1, format!("Seq{}", i), format!("Sequence {}", i));
//
//             sequence.steps.push(Step {
//                 step: 1,
//                 manual: None,
//                 description: format!("Test step {}", i),
//                 command: format!(
//                     r#"echo "Test {} with special chars: \"quotes\" and newlines\nand tabs\t""#,
//                     i
//                 ),
//                 capture_vars: None,
//                 expected: Expected {
//                     success: Some(true),
//                     result: "0".to_string(),
//                     output: "true".to_string(),
//                 },
//                 verification: Verification {
//                     result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//                     output: VerificationExpression::Simple("true".to_string()),
//                     output_file: None,
//                     general: None,
//                 },
//                 reference: None,
//             });
//
//             test_case.test_sequences.push(sequence);
//             test_case
//         })
//         .collect();
//
//     // Generate scripts for all test cases
//     let script_paths: Vec<PathBuf> = test_cases
//         .iter()
//         .map(|tc| {
//             let script = executor.generate_test_script(tc);
//             let script_path = temp_dir
//                 .path()
//                 .join(format!("test_concurrent_{}.sh", tc.id));
//             fs::write(&script_path, &script).unwrap();
//
//             #[cfg(unix)]
//             {
//                 use std::os::unix::fs::PermissionsExt;
//                 let mut perms = fs::metadata(&script_path).unwrap().permissions();
//                 perms.set_mode(0o755);
//                 fs::set_permissions(&script_path, perms).unwrap();
//             }
//
//             script_path
//         })
//         .collect();
//
//     // Execute all scripts concurrently
//     let temp_dir_path = temp_dir.path().to_path_buf();
//     let handles: Vec<_> = script_paths
//         .into_iter()
//         .map(|script_path| {
//             let temp_dir_clone = temp_dir_path.clone();
//             thread::spawn(move || {
//                 Command::new("bash")
//                     .arg(&script_path)
//                     .current_dir(&temp_dir_clone)
//                     .output()
//             })
//         })
//         .collect();
//
//     // Wait for all threads to complete
//     let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
//
//     // Verify all executed successfully
//     for result in &results {
//         assert!(result.is_ok(), "Script execution should succeed");
//     }
//
//     // Verify all JSON logs were created and are valid
//     for test_case in &test_cases {
//         let json_log_path = temp_dir
//             .path()
//             .join(format!("{}_execution_log.json", test_case.id));
//
//         assert!(
//             json_log_path.exists(),
//             "JSON log should exist for {}",
//             test_case.id
//         );
//
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         assert!(
//             parsed.is_ok(),
//             "JSON should be valid for {}, error: {:?}",
//             test_case.id,
//             parsed.err()
//         );
//     }
//
//     Ok(())
// }
//
// // ============================================================================
// // Cross-Platform Compatibility Tests
// // ============================================================================
// //
// // These tests verify that generated scripts work correctly across different
// // platforms (macOS and Linux) and shell versions (bash 3.2+). They ensure:
// //
// // 1. **Bash 3.2+ Compatibility**: Scripts don't use bash 4.0+ features like
// //    associative arrays (declare -A), globstar (**), or case fallthrough (;& and ;;&).
// //    This is critical because macOS ships with bash 3.2 by default.
// //
// // 2. **BSD vs GNU Tool Compatibility**: sed/awk commands use only flags and
// //    syntax that work on both BSD (macOS) and GNU (Linux) versions:
// //    - sed uses basic substitution patterns: sed 's/pattern/replacement/g'
// //    - NO GNU-specific flags like sed -r (use sed -E for extended regex if needed)
// //    - awk uses portable printf patterns: awk '{printf "%s%s", (NR>1?"\\n":""), $0}'
// //    - This is what the shell fallback uses for JSON escaping
// //
// // 3. **Cross-Platform Execution**: Scripts execute correctly on both macOS
// //    and Linux, producing valid JSON output in both environments. The test
// //    reports which platform it's running on for debugging.
// //
// // 4. **Newline Handling**: Shell escaping properly handles platform-specific
// //    newline differences (LF, CRLF, CR) and escapes them consistently in JSON:
// //    - Unix LF (\n) -> JSON \\n
// //    - Windows CRLF (\r\n) -> JSON \\r\\n
// //    - Mac Classic CR (\r) -> JSON \\r
// //
// // 5. **Special Character Handling**: sed/awk fallback correctly escapes
// //    backslashes, quotes, tabs, and carriage returns for JSON strings:
// //    - Backslash (\) -> \\
// //    - Quote (") -> \"
// //    - Tab (\t) -> \t
// //    - Carriage return (\r) -> \r
// //
// // 6. **Portable Utilities**: Scripts use printf instead of echo for better
// //    cross-platform compatibility (echo behavior varies across shells).
// //
// // Implementation Details:
// // ----------------------
// // The shell fallback JSON escaping uses this portable command chain:
// //   sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r/\\r/g' | awk '{printf "%s%s", (NR>1?"\\n":""), $0}'
// //
// // This works because:
// // - sed 's/...' basic substitution is POSIX-compatible
// // - Multiple sed substitutions can be chained with semicolons
// // - awk printf with conditional NR>1 handles newlines portably
// // - printf '%s' prevents interpretation of escape sequences
// //
// // To run these tests:
// //   cargo test --all-features --test json_escape_integration_test
// //
// // To run only compatibility tests:
// //   cargo test --all-features --test json_escape_integration_test compatibility
// //   cargo test --all-features --test json_escape_integration_test cross_platform
// //
// // ============================================================================
//
// /// Test that generated scripts are compatible with bash 3.2+ (default on macOS)
// #[test]
// fn test_bash_32_compatibility() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify script does not use bash 4.0+ features
//     assert!(
//         !script.contains("declare -A"),
//         "Script should not use associative arrays (bash 4.0+)"
//     );
//     assert!(
//         !script.contains(";&"),
//         "Script should not use ;& case fallthrough (bash 4.0+)"
//     );
//     assert!(
//         !script.contains(";;&"),
//         "Script should not use ;;& case fallthrough (bash 4.0+)"
//     );
//     assert!(
//         !script.contains("**"),
//         "Script should not use globstar (bash 4.0+)"
//     );
//
//     // Verify bash 3.2 compatible constructs
//     assert!(
//         script.contains("#!/bin/bash"),
//         "Script should have bash shebang"
//     );
//
//     // Write script to file
//     let script_path = temp_dir.path().join("test_bash32.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Try to execute with bash (will use whatever version is available)
//     // Note: We use regular bash, not --posix, as the script uses bash-specific features
//     // like [[ ]] and pipefail, which are bash 3.2+ compatible but not POSIX
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//         eprintln!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
//     }
//
//     // Verify JSON was created
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created with bash 3.2 compatible script"
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid with bash 3.2 compatible script, error: {:?}",
//         parsed.err()
//     );
//
//     Ok(())
// }
//
// /// Test that sed commands use BSD-compatible flags (sed -E not -r)
// #[test]
// fn test_sed_bsd_compatibility() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify no GNU-specific sed flags
//     assert!(
//         !script.contains("sed -r"),
//         "Script should not use GNU-specific 'sed -r' flag"
//     );
//     assert!(
//         !script.contains("sed --regexp-extended"),
//         "Script should not use GNU-specific long option"
//     );
//
//     // Verify sed uses basic or extended regex only (both BSD and GNU compatible)
//     if script.contains("sed -E") {
//         // sed -E is BSD/GNU compatible for extended regex
//         assert!(
//             script.contains("sed -E") || script.contains("sed 's/"),
//             "sed -E should be used for extended regex if needed"
//         );
//     }
//
//     // Verify sed patterns use portable syntax
//     // The shell fallback uses basic sed substitution patterns
//     assert!(
//         script.contains("sed 's/"),
//         "Script should use basic sed substitution"
//     );
//
//     Ok(())
// }
//
// /// Test that awk commands work on both BSD and GNU versions
// #[test]
// fn test_awk_cross_platform_compatibility() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify awk pattern for newline handling is present
//     assert!(
//         script.contains("awk"),
//         "Script should use awk for newline handling"
//     );
//
//     // Verify awk uses portable printf pattern
//     assert!(
//         script.contains(r#"awk '{printf "%s%s""#),
//         "Script should use portable awk printf pattern"
//     );
//
//     Ok(())
// }
//
// /// Test script execution on current platform (works on both macOS and Linux)
// #[test]
// fn test_script_execution_cross_platform() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     // Test with shell fallback to ensure cross-platform compatibility
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_cross_platform.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script on current platform
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Platform: {}", std::env::consts::OS);
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//         eprintln!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
//     }
//
//     // Verify JSON output
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created on {}",
//         std::env::consts::OS
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid on {}, error: {:?}",
//         std::env::consts::OS,
//         parsed.err()
//     );
//
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(
//         entries.len(),
//         4,
//         "Should have 4 entries on {}",
//         std::env::consts::OS
//     );
//
//     Ok(())
// }
//
// /// Test shell escaping handles platform-specific newline differences
// #[test]
// fn test_newline_handling_cross_platform() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with various newline scenarios
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "NEWLINE_TC".to_string(),
//         "Test newline handling".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Newline tests".to_string());
//
//     // Unix newlines (LF) - using echo to produce actual newlines
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Unix newlines".to_string(),
//         command: r#"echo "line1"; echo "line2"; echo "line3""#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Multiple lines in a single echo
//     sequence.steps.push(Step {
//         step: 2,
//         manual: None,
//         description: "Multiline output".to_string(),
//         command: r#"echo "line1"; echo "line2""#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Simple output with newline from echo
//     sequence.steps.push(Step {
//         step: 3,
//         manual: None,
//         description: "Echo with newline".to_string(),
//         command: r#"echo "text with spaces""#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_newlines.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Verify JSON output
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should be created");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON with various newline types should be valid"
//     );
//
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 3, "Should have 3 entries");
//
//     // Verify that all output fields are valid JSON strings
//     // The fact that the JSON parsed successfully means escaping worked correctly
//     // The sed/awk pipeline handles newlines via awk's printf with NR>1
//     // Just check that we can access each entry's output field
//     for entry in entries.iter() {
//         let _output = entry["output"]
//             .as_str()
//             .expect("output should be a valid string");
//     }
//
//     // If JSON parsed successfully, newlines were properly handled
//     assert_eq!(entries.len(), 3, "All 3 test steps should be present");
//
//     Ok(())
// }
//
// /// Test that sed/awk fallback works correctly with special characters
// #[test]
// fn test_sed_awk_fallback_special_chars() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with challenging special characters for sed/awk
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "SED_AWK_TC".to_string(),
//         "Test sed/awk with special chars".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Special chars".to_string());
//
//     // Test quotes (challenging for sed)
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Quotes test".to_string(),
//         command: r#"echo 'He said "hello" to me'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test newlines (echo produces actual newlines)
//     sequence.steps.push(Step {
//         step: 2,
//         manual: None,
//         description: "Newlines test".to_string(),
//         command: r#"echo 'line1'; echo 'line2'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test backslashes in path (echo output)
//     sequence.steps.push(Step {
//         step: 3,
//         manual: None,
//         description: "Path with backslashes".to_string(),
//         command: r#"echo 'C:\test\path'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_sed_awk.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Verify JSON output
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created with sed/awk"
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON from sed/awk should be valid, error: {:?}",
//         parsed.err()
//     );
//
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 3, "Should have 3 entries");
//
//     // Verify specific escaping
//     // Entry 1: quotes should be escaped
//     let entry1_output = entries[0]["output"].as_str().unwrap();
//     assert!(
//         entry1_output.contains("\\\"") || entry1_output.contains("hello"),
//         "Quotes should be escaped or output should contain the text"
//     );
//
//     // Entry 2: newlines should be present (from echo command)
//     let entry2_output = entries[1]["output"].as_str().unwrap();
//     assert!(
//         entry2_output.contains("\\n") || entry2_output.contains("line1"),
//         "Newlines should be escaped or output should contain line1"
//     );
//
//     // Entry 3: backslashes in path (echo with single quotes preserves them literally)
//     let entry3_output = entries[2]["output"].as_str().unwrap();
//     assert!(
//         entry3_output.contains("test") && entry3_output.contains("path"),
//         "Output should contain path components"
//     );
//
//     Ok(())
// }
//
// /// Test that printf is used instead of echo for cross-platform compatibility
// #[test]
// fn test_printf_usage_for_portability() -> Result<()> {
//     let config = Config::default();
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_simple_test_case();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify printf is used for variable output instead of echo -n or echo -e
//     assert!(
//         script.contains("printf"),
//         "Script should use printf for portable output"
//     );
//
//     Ok(())
// }
//
// /// Test compatibility with sh (POSIX shell) in addition to bash
// #[test]
// fn test_posix_shell_compatibility() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_simple_test_case();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // While we target bash, verify no bash-specific constructs that break POSIX
//     // (except arrays which are documented as bash-only)
//
//     // Verify [[ ]] is allowed (bash-specific but acceptable)
//     // Verify [ ] is used for basic tests
//
//     // Verify no process substitution
//     assert!(
//         !script.contains("<("),
//         "Script should avoid process substitution for broader compatibility"
//     );
//
//     Ok(())
// }
//
// // ============================================================================
// // Additional Cross-Platform Compatibility Tests
// // ============================================================================
//
// /// Test that generated scripts use only basic sed patterns without GNU extensions
// #[test]
// fn test_sed_uses_basic_patterns_only() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify sed uses basic substitution syntax (s/pattern/replacement/g)
//     assert!(
//         script.contains("sed 's/"),
//         "Script should use sed basic substitution"
//     );
//
//     // Verify sed patterns are chained with semicolons (BSD/GNU compatible)
//     assert!(
//         script.contains("; s/"),
//         "Script should chain sed substitutions with semicolons"
//     );
//
//     // Count that all key characters are being escaped
//     assert!(
//         script.contains(r#"s/\\/\\\\/g"#),
//         "Script should escape backslashes"
//     );
//     assert!(
//         script.contains(r#"s/"/\\"/g"#),
//         "Script should escape double quotes"
//     );
//     assert!(
//         script.contains(r#"s/\t/\\t/g"#),
//         "Script should escape tabs"
//     );
//     assert!(
//         script.contains(r#"s/\r/\\r/g"#),
//         "Script should escape carriage returns"
//     );
//
//     Ok(())
// }
//
// /// Test that awk uses printf pattern for cross-platform newline handling
// #[test]
// fn test_awk_printf_pattern_compatibility() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify awk uses portable printf with NR (number of records) check
//     assert!(
//         script.contains(r#"awk '{printf "%s%s""#),
//         "Script should use awk printf pattern"
//     );
//     assert!(
//         script.contains("NR>1"),
//         "Script should use NR>1 for newline handling"
//     );
//     assert!(
//         script.contains(r#"\\n"#),
//         "Script should escape newlines in awk pattern"
//     );
//
//     Ok(())
// }
//
// /// Test that scripts work with both macOS and Linux sed/awk implementations
// #[test]
// fn test_sed_awk_execution_cross_platform() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test with complex characters to stress test sed/awk
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "CROSS_PLAT_TC".to_string(),
//         "Cross-platform sed/awk test".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Platform test".to_string());
//
//     // Test multiple special characters at once
//     // Note: Using echo to avoid issues with printf interpretation of escape sequences
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Multiple special chars".to_string(),
//         command: r#"echo 'Path: C:\test\file.txt' && echo 'Quoted: "value" text'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_cross_platform_exec.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Platform: {}", std::env::consts::OS);
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//         eprintln!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
//     }
//
//     // Verify JSON output is valid
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log should be created on {}",
//         std::env::consts::OS
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid on {} with sed/awk fallback, error: {:?}",
//         std::env::consts::OS,
//         parsed.err()
//     );
//
//     // Verify the output contains properly escaped characters
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 1, "Should have 1 entry");
//
//     let output_str = entries[0]["output"].as_str().unwrap();
//     // Verify the output contains expected text
//     // The echo commands produce output with newlines that get escaped by the sed/awk pipeline
//     assert!(!output_str.is_empty(), "Output should not be empty");
//     // Verify basic content is present
//     assert!(
//         output_str.contains("test") || output_str.contains("Quoted") || output_str.contains("Path"),
//         "Output should contain some of the expected text"
//     );
//
//     Ok(())
// }
//
// /// Test handling of empty output and edge cases
// #[test]
// fn test_empty_output_cross_platform() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "EMPTY_OUTPUT_TC".to_string(),
//         "Test empty output".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Empty output test".to_string());
//
//     // Test with command that produces no output
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Empty output".to_string(),
//         command: "true".to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_empty_output.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     assert!(
//         output.status.success(),
//         "Script should succeed with empty output"
//     );
//
//     // Verify JSON output is valid even with empty output
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should be created");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid with empty output, error: {:?}",
//         parsed.err()
//     );
//
//     Ok(())
// }
//
// /// Test printf usage for portability (printf is more consistent than echo)
// #[test]
// fn test_printf_for_portability() -> Result<()> {
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_config(config);
//     let test_case = create_test_case_with_special_characters();
//
//     let script = executor.generate_test_script(&test_case);
//
//     // Verify that printf is used for outputting command output to sed
//     assert!(
//         script.contains("printf '%s' \"$COMMAND_OUTPUT\""),
//         "Script should use printf '%s' for safe output (prevents interpretation of escape sequences)"
//     );
//
//     // This is important because echo behavior varies across shells and platforms
//     // printf '%s' ensures the string is output exactly as-is without interpretation
//
//     Ok(())
// }
//
// /// Test that scripts don't contain trailing whitespace issues
// #[test]
// fn test_no_trailing_whitespace_issues() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::ShellFallback,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "WHITESPACE_TC".to_string(),
//         "Test whitespace handling".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Whitespace test".to_string());
//
//     // Test with trailing/leading whitespace
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Whitespace handling".to_string(),
//         command: r#"printf '  spaces before and after  '"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_whitespace.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     assert!(
//         output.status.success(),
//         "Script should handle whitespace correctly"
//     );
//
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should be created");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be valid with whitespace, error: {:?}",
//         parsed.err()
//     );
//
//     Ok(())
// }
//
// /// Test race conditions with concurrent writes to same output
// #[test]
// fn test_concurrent_with_shared_output_dir() -> Result<()> {
//     use std::thread;
//
//     let temp_dir = TempDir::new()?;
//
//     // Build json-escape binary first
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::RustBinary,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     // All tests use the same output directory
//     let shared_output_dir = temp_dir.path().join("shared_output");
//     fs::create_dir_all(&shared_output_dir)?;
//
//     // Create multiple test cases with DIFFERENT IDs
//     let test_cases: Vec<TestCase> = (1..=10)
//         .map(|i| {
//             let mut test_case = TestCase::new(
//                 format!("REQ{:03}", i),
//                 1,
//                 i,
//                 format!("RACE_TC_{}", i),
//                 format!("Race test {}", i),
//             );
//
//             let mut sequence = TestSequence::new(1, format!("Seq{}", i), format!("Sequence {}", i));
//
//             sequence.steps.push(Step {
//                 step: 1,
//                 manual: None,
//                 description: format!("Test step {}", i),
//                 command: format!(r#"echo "Race test {} with chars: \"quotes\"""#, i),
//                 capture_vars: None,
//                 expected: Expected {
//                     success: Some(true),
//                     result: "0".to_string(),
//                     output: "true".to_string(),
//                 },
//                 verification: Verification {
//                     result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//                     output: VerificationExpression::Simple("true".to_string()),
//                     output_file: None,
//                     general: None,
//                 },
//                 reference: None,
//             });
//
//             test_case.test_sequences.push(sequence);
//             test_case
//         })
//         .collect();
//
//     // Generate and execute scripts concurrently
//     let handles: Vec<_> = test_cases
//         .into_iter()
//         .map(|tc| {
//             let config_clone = config.clone();
//             let shared_output_clone = shared_output_dir.clone();
//
//             thread::spawn(move || {
//                 let executor =
//                     TestExecutor::with_output_dir_and_config(&shared_output_clone, config_clone);
//                 let script = executor.generate_test_script(&tc);
//                 let script_path = shared_output_clone.join(format!("test_race_{}.sh", tc.id));
//                 fs::write(&script_path, &script).unwrap();
//
//                 #[cfg(unix)]
//                 {
//                     use std::os::unix::fs::PermissionsExt;
//                     let mut perms = fs::metadata(&script_path).unwrap().permissions();
//                     perms.set_mode(0o755);
//                     fs::set_permissions(&script_path, perms).unwrap();
//                 }
//
//                 let output = Command::new("bash")
//                     .arg(&script_path)
//                     .current_dir(&shared_output_clone)
//                     .output()
//                     .unwrap();
//
//                 (tc.id.clone(), output)
//             })
//         })
//         .collect();
//
//     // Wait for all threads and collect results
//     let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
//
//     // Verify all JSON logs are valid and separate
//     for (test_id, output) in results {
//         if !output.status.success() {
//             eprintln!(
//                 "Test {} stderr: {}",
//                 test_id,
//                 String::from_utf8_lossy(&output.stderr)
//             );
//         }
//
//         let json_log_path = shared_output_dir.join(format!("{}_execution_log.json", test_id));
//
//         assert!(
//             json_log_path.exists(),
//             "JSON log should exist for {}",
//             test_id
//         );
//
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//         assert!(
//             parsed.is_ok(),
//             "JSON should be valid for {}, error: {:?}",
//             test_id,
//             parsed.err()
//         );
//     }
//
//     Ok(())
// }
//
// // ============================================================================
// // JSON Log Validation Tests
// // ============================================================================
// //
// // These tests validate that generated scripts produce JSON logs that:
// // 1. Are parseable by serde_json
// // 2. Have properly escaped special characters in all fields
// // 3. Validate against the execution-log.schema.json schema
// // 4. Can be validated with jq when available
// //
// // To run these tests:
// //   cargo test --all-features -- json_escape_integration_test::test_.*json_log
// //
// // ============================================================================
//
// /// Test that generated scripts produce valid JSON logs parseable by serde_json
// #[test]
// fn test_json_log_serde_parsing() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with special characters
//     let test_case = create_test_case_with_special_characters();
//
//     // Generate and execute script
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_json_log.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//         eprintln!("Script stdout: {}", String::from_utf8_lossy(&output.stdout));
//     }
//
//     // Read and parse JSON log
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(
//         json_log_path.exists(),
//         "JSON log file should exist: {}",
//         json_log_path.display()
//     );
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//
//     // Validate JSON is parseable by serde_json
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON should be parseable by serde_json, error: {:?}",
//         parsed.err()
//     );
//
//     let json_value = parsed.unwrap();
//     assert!(json_value.is_array(), "JSON log should be an array");
//
//     let entries = json_value.as_array().unwrap();
//     assert!(!entries.is_empty(), "JSON log should have entries");
//
//     // Verify each entry has required fields
//     for entry in entries {
//         assert!(entry.is_object(), "Each entry should be an object");
//         assert!(
//             entry.get("test_sequence").is_some(),
//             "Entry should have test_sequence field"
//         );
//         assert!(entry.get("step").is_some(), "Entry should have step field");
//         assert!(
//             entry.get("command").is_some(),
//             "Entry should have command field"
//         );
//         assert!(
//             entry.get("exit_code").is_some(),
//             "Entry should have exit_code field"
//         );
//         assert!(
//             entry.get("output").is_some(),
//             "Entry should have output field"
//         );
//     }
//
//     Ok(())
// }
//
// /// Test that special characters in output fields are properly escaped
// #[test]
// fn test_json_log_special_character_escaping() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with comprehensive special characters
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "SPECIAL_CHARS_LOG_TC".to_string(),
//         "Test special character escaping in JSON log".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Special chars".to_string());
//
//     // Test 1: Newlines
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Newlines".to_string(),
//         command: r#"printf 'Line1\nLine2\nLine3'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test 2: Double quotes
//     sequence.steps.push(Step {
//         step: 2,
//         manual: None,
//         description: "Double quotes".to_string(),
//         command: r#"echo 'Test "quotes" here'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test 3: Backslashes
//     sequence.steps.push(Step {
//         step: 3,
//         manual: None,
//         description: "Backslashes".to_string(),
//         command: r#"echo 'Path: C:\test\file.txt'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test 4: Tabs
//     sequence.steps.push(Step {
//         step: 4,
//         manual: None,
//         description: "Tabs".to_string(),
//         command: r#"printf 'Col1\tCol2\tCol3'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test 5: Carriage returns
//     sequence.steps.push(Step {
//         step: 5,
//         manual: None,
//         description: "Carriage returns".to_string(),
//         command: r#"printf 'Text\rOverwrite'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test 6: Mixed special characters
//     sequence.steps.push(Step {
//         step: 6,
//         manual: None,
//         description: "Mixed special characters".to_string(),
//         command: r#"printf 'Line1\nQuoted: "value"\tTab\rReturn\\Backslash'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     // Generate and execute script
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_escaping.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Read JSON log
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should exist");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//
//     // Parse JSON - if this succeeds, escaping worked
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON with special characters should be parseable, error: {:?}",
//         parsed.err()
//     );
//
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 6, "Should have 6 entries");
//
//     // Verify that output fields are strings (not broken by unescaped characters)
//     for entry in entries {
//         assert!(
//             entry["output"].is_string(),
//             "Output field should be a valid string"
//         );
//
//         // Verify no literal unescaped newlines in the JSON string representation
//         // (they should be escaped as \n)
//         let output_json = serde_json::to_string(&entry["output"])?;
//         assert!(
//             !output_json.contains("\n\n"),
//             "JSON output should not contain literal newlines"
//         );
//     }
//
//     Ok(())
// }
//
// /// Test JSON log validation against execution-log.schema.json
// #[test]
// fn test_json_log_schema_validation() -> Result<()> {
//     use jsonschema::JSONSchema;
//
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     // Generate and execute script
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_schema.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Read JSON log
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should exist");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//     let json_value: Value = serde_json::from_str(&json_content)?;
//
//     // Load schema
//     let schema_path = PathBuf::from("schemas/tcms-schemas/tcms/execution-log.schema.json");
//     assert!(
//         schema_path.exists(),
//         "Schema file should exist at {:?}",
//         schema_path
//     );
//
//     let schema_content = fs::read_to_string(&schema_path)?;
//     let schema_json: Value = serde_json::from_str(&schema_content)?;
//
//     // Compile schema
//     let compiled_schema = JSONSchema::compile(&schema_json)
//         .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;
//
//     // Validate each entry in the array
//     let entries = json_value.as_array().unwrap();
//     for (idx, entry) in entries.iter().enumerate() {
//         let validation_result = compiled_schema.validate(entry);
//         if let Err(errors) = validation_result {
//             let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
//             panic!(
//                 "Entry {} failed schema validation:\n{}",
//                 idx,
//                 error_messages.join("\n")
//             );
//         }
//     }
//
//     Ok(())
// }
//
// /// Test JSON log validation with jq (if available)
// #[test]
// fn test_json_log_jq_validation() -> Result<()> {
//     // Check if jq is available
//     let jq_check = Command::new("which").arg("jq").output();
//
//     if jq_check.is_err() || !jq_check.unwrap().status.success() {
//         println!("Skipping test: jq not available");
//         return Ok(());
//     }
//
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//     let test_case = create_test_case_with_special_characters();
//
//     // Generate and execute script
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_jq.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Read JSON log
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should exist");
//
//     // Validate with jq
//     let jq_output = Command::new("jq")
//         .arg("empty")
//         .arg(&json_log_path)
//         .output()?;
//
//     assert!(
//         jq_output.status.success(),
//         "jq validation failed: {}",
//         String::from_utf8_lossy(&jq_output.stderr)
//     );
//
//     // Also test extracting fields with jq
//     let jq_extract = Command::new("jq")
//         .arg(".[].output")
//         .arg(&json_log_path)
//         .output()?;
//
//     assert!(
//         jq_extract.status.success(),
//         "jq field extraction failed: {}",
//         String::from_utf8_lossy(&jq_extract.stderr)
//     );
//
//     // Verify we got output from jq
//     let jq_result = String::from_utf8_lossy(&jq_extract.stdout);
//     assert!(
//         !jq_result.is_empty(),
//         "jq should extract output fields successfully"
//     );
//
//     Ok(())
// }
//
// /// Test JSON log with all escaping methods (RustBinary, ShellFallback, Auto)
// #[test]
// fn test_json_log_all_escaping_methods() -> Result<()> {
//     let methods = vec![
//         ("RustBinary", JsonEscapingMethod::RustBinary),
//         ("ShellFallback", JsonEscapingMethod::ShellFallback),
//         ("Auto", JsonEscapingMethod::Auto),
//     ];
//
//     // Build json-escape binary once
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     for (method_name, method) in methods {
//         let temp_dir = TempDir::new()?;
//
//         let config = Config {
//             script_generation: ScriptGenerationConfig {
//                 json_escaping: JsonEscapingConfig {
//                     method,
//                     enabled: true,
//                     binary_path: None,
//                 },
//             },
//             ..Default::default()
//         };
//
//         let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//         let test_case = create_test_case_with_special_characters();
//
//         // Generate and execute script
//         let script = executor.generate_test_script(&test_case);
//         let script_path = temp_dir.path().join(format!("test_{}.sh", method_name));
//         fs::write(&script_path, &script)?;
//
//         #[cfg(unix)]
//         {
//             use std::os::unix::fs::PermissionsExt;
//             let mut perms = fs::metadata(&script_path)?.permissions();
//             perms.set_mode(0o755);
//             fs::set_permissions(&script_path, perms)?;
//         }
//
//         // Execute script
//         let output = Command::new("bash")
//             .arg(&script_path)
//             .current_dir(temp_dir.path())
//             .output()?;
//
//         if !output.status.success() {
//             eprintln!(
//                 "[{}] Script stderr: {}",
//                 method_name,
//                 String::from_utf8_lossy(&output.stderr)
//             );
//         }
//
//         // Read and validate JSON log
//         let json_log_path = temp_dir
//             .path()
//             .join(format!("{}_execution_log.json", test_case.id));
//
//         assert!(
//             json_log_path.exists(),
//             "[{}] JSON log should exist",
//             method_name
//         );
//
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//
//         assert!(
//             parsed.is_ok(),
//             "[{}] JSON should be valid, error: {:?}",
//             method_name,
//             parsed.err()
//         );
//
//         let json_value = parsed.unwrap();
//         let entries = json_value.as_array().unwrap();
//
//         assert!(
//             !entries.is_empty(),
//             "[{}] JSON log should have entries",
//             method_name
//         );
//
//         // Verify each entry has required fields
//         for entry in entries {
//             assert!(
//                 entry.get("test_sequence").is_some(),
//                 "[{}] Entry should have test_sequence",
//                 method_name
//             );
//             assert!(
//                 entry.get("step").is_some(),
//                 "[{}] Entry should have step",
//                 method_name
//             );
//             assert!(
//                 entry.get("command").is_some(),
//                 "[{}] Entry should have command",
//                 method_name
//             );
//             assert!(
//                 entry.get("exit_code").is_some(),
//                 "[{}] Entry should have exit_code",
//                 method_name
//             );
//             assert!(
//                 entry.get("output").is_some(),
//                 "[{}] Entry should have output",
//                 method_name
//             );
//             assert!(
//                 entry["output"].is_string(),
//                 "[{}] Output should be a string",
//                 method_name
//             );
//         }
//     }
//
//     Ok(())
// }
//
// /// Test JSON log with edge case characters that are challenging to escape
// #[test]
// fn test_json_log_edge_case_characters() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create test case with edge case characters
//     let mut test_case = TestCase::new(
//         "REQ001".to_string(),
//         1,
//         1,
//         "EDGE_CASE_TC".to_string(),
//         "Test edge case character escaping".to_string(),
//     );
//
//     let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Edge cases".to_string());
//
//     // Test: Unicode characters
//     sequence.steps.push(Step {
//         step: 1,
//         manual: None,
//         description: "Unicode characters".to_string(),
//         command: r#"echo 'Unicode: 你好 мир 🎉'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test: Multiple consecutive special characters
//     sequence.steps.push(Step {
//         step: 2,
//         manual: None,
//         description: "Multiple special chars".to_string(),
//         command: r#"echo '"""\\\\"'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     // Test: Empty lines and whitespace
//     sequence.steps.push(Step {
//         step: 3,
//         manual: None,
//         description: "Empty lines and whitespace".to_string(),
//         command: r#"printf '\n\n   \n\t\t\n'"#.to_string(),
//         capture_vars: None,
//         expected: Expected {
//             success: Some(true),
//             result: "0".to_string(),
//             output: "true".to_string(),
//         },
//         verification: Verification {
//             result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//             output: VerificationExpression::Simple("true".to_string()),
//             output_file: None,
//             general: None,
//         },
//         reference: None,
//     });
//
//     test_case.test_sequences.push(sequence);
//
//     // Generate and execute script
//     let script = executor.generate_test_script(&test_case);
//     let script_path = temp_dir.path().join("test_edge_cases.sh");
//     fs::write(&script_path, &script)?;
//
//     #[cfg(unix)]
//     {
//         use std::os::unix::fs::PermissionsExt;
//         let mut perms = fs::metadata(&script_path)?.permissions();
//         perms.set_mode(0o755);
//         fs::set_permissions(&script_path, perms)?;
//     }
//
//     // Build json-escape binary
//     Command::new("cargo")
//         .args(["build", "--bin", "json-escape"])
//         .output()?;
//
//     // Execute script
//     let output = Command::new("bash")
//         .arg(&script_path)
//         .current_dir(temp_dir.path())
//         .output()?;
//
//     if !output.status.success() {
//         eprintln!("Script stderr: {}", String::from_utf8_lossy(&output.stderr));
//     }
//
//     // Read JSON log
//     let json_log_path = temp_dir
//         .path()
//         .join(format!("{}_execution_log.json", test_case.id));
//
//     assert!(json_log_path.exists(), "JSON log should exist");
//
//     let json_content = fs::read_to_string(&json_log_path)?;
//
//     // Parse JSON - if this succeeds, edge case escaping worked
//     let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//     assert!(
//         parsed.is_ok(),
//         "JSON with edge case characters should be parseable, error: {:?}",
//         parsed.err()
//     );
//
//     let json_value = parsed.unwrap();
//     let entries = json_value.as_array().unwrap();
//     assert_eq!(entries.len(), 3, "Should have 3 entries");
//
//     // Verify all entries are valid
//     for entry in entries {
//         assert!(entry["output"].is_string(), "Output should be a string");
//     }
//
//     Ok(())
// }
//
// /// Test JSON log validation with comprehensive special character matrix
// #[test]
// fn test_json_log_comprehensive_escaping_matrix() -> Result<()> {
//     let temp_dir = TempDir::new()?;
//
//     let config = Config {
//         script_generation: ScriptGenerationConfig {
//             json_escaping: JsonEscapingConfig {
//                 method: JsonEscapingMethod::Auto,
//                 enabled: true,
//                 binary_path: None,
//             },
//         },
//         ..Default::default()
//     };
//
//     let executor = TestExecutor::with_output_dir_and_config(temp_dir.path(), config);
//
//     // Create comprehensive test matrix
//     let test_cases = [
//         ("newline", r#"printf 'A\nB'"#),
//         ("tab", r#"printf 'A\tB'"#),
//         ("carriage_return", r#"printf 'A\rB'"#),
//         ("backslash", r#"echo 'A\B'"#),
//         ("double_quote", r#"echo 'A"B'"#),
//         ("single_quote", r#"echo "A'B"#),
//         ("multiple_newlines", r#"printf 'A\n\nB'"#),
//         ("mixed", r#"printf 'A\nB\tC"D\\E'"#),
//     ];
//
//     for (idx, (desc, command)) in test_cases.iter().enumerate() {
//         let mut test_case = TestCase::new(
//             "REQ001".to_string(),
//             1,
//             (idx + 1) as i64,
//             format!("MATRIX_TC_{}", idx),
//             format!("Test {}", desc),
//         );
//
//         let mut sequence = TestSequence::new(1, "Seq1".to_string(), desc.to_string());
//
//         sequence.steps.push(Step {
//             step: 1,
//             manual: None,
//             description: desc.to_string(),
//             command: command.to_string(),
//             capture_vars: None,
//             expected: Expected {
//                 success: Some(true),
//                 result: "0".to_string(),
//                 output: "true".to_string(),
//             },
//             verification: Verification {
//                 result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
//                 output: VerificationExpression::Simple("true".to_string()),
//                 output_file: None,
//                 general: None,
//             },
//             reference: None,
//         });
//
//         test_case.test_sequences.push(sequence);
//
//         // Generate and execute script
//         let script = executor.generate_test_script(&test_case);
//         let script_path = temp_dir.path().join(format!("test_{}.sh", desc));
//         fs::write(&script_path, &script)?;
//
//         #[cfg(unix)]
//         {
//             use std::os::unix::fs::PermissionsExt;
//             let mut perms = fs::metadata(&script_path)?.permissions();
//             perms.set_mode(0o755);
//             fs::set_permissions(&script_path, perms)?;
//         }
//
//         // Execute script
//         let output = Command::new("bash")
//             .arg(&script_path)
//             .current_dir(temp_dir.path())
//             .output()?;
//
//         if !output.status.success() {
//             eprintln!(
//                 "[{}] Script stderr: {}",
//                 desc,
//                 String::from_utf8_lossy(&output.stderr)
//             );
//         }
//
//         // Read and validate JSON log
//         let json_log_path = temp_dir
//             .path()
//             .join(format!("{}_execution_log.json", test_case.id));
//
//         assert!(json_log_path.exists(), "[{}] JSON log should exist", desc);
//
//         let json_content = fs::read_to_string(&json_log_path)?;
//         let parsed: Result<Value, _> = serde_json::from_str(&json_content);
//
//         assert!(
//             parsed.is_ok(),
//             "[{}] JSON should be valid, error: {:?}",
//             desc,
//             parsed.err()
//         );
//     }
//
//     Ok(())
// }
