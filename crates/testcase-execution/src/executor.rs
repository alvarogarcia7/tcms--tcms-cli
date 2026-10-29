use crate::bdd_parser::BddStepRegistry;
use crate::hydration::VarHydrator;
use anyhow::{Context, Result};
use chrono::Local;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use testcase_common::{Config, JsonEscapingMethod};
use testcase_models::{
    CaptureVarsFormat, TestCase, TestStepExecutionEntry, VerificationExpression,
};

/// Trait for prompting user for confirmation
pub trait ConfirmPrompt {
    fn confirm(&self, prompt: &str) -> Result<bool>;
}

/// Default implementation that uses stdin/stdout
pub struct DefaultConfirmPrompt;

impl ConfirmPrompt for DefaultConfirmPrompt {
    fn confirm(&self, prompt: &str) -> Result<bool> {
        use std::io::{self, Write};

        print!("{} [y/N]: ", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }
}

pub struct TestExecutor {
    output_dir: Option<PathBuf>,
    config: Config,
    /// Run ID written to execution log entries when `TCMS_RUN_ID` is not set at run time
    run_id: Option<String>,
}

/// Quote `value` as a bash single-quoted string
fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Helper function to check if capture_vars is empty
fn capture_vars_is_empty(capture_vars: &CaptureVarsFormat) -> bool {
    match capture_vars {
        CaptureVarsFormat::Legacy(map) => map.is_empty(),
        CaptureVarsFormat::New(vec) => vec.is_empty(),
    }
}

/// Helper function to convert CaptureVarsFormat to a vector of (name, capture_pattern, command) tuples
fn capture_vars_to_vec(
    capture_vars: &CaptureVarsFormat,
) -> Vec<(String, Option<String>, Option<String>)> {
    match capture_vars {
        CaptureVarsFormat::Legacy(map) => map
            .iter()
            .map(|(name, pattern)| (name.clone(), Some(pattern.clone()), None))
            .collect(),
        CaptureVarsFormat::New(vec) => vec
            .iter()
            .map(|cv| (cv.name.clone(), cv.capture.clone(), cv.command.clone()))
            .collect(),
    }
}

/// Compute SHA-256 hash of YAML content and return hex-encoded string
pub fn compute_yaml_sha256(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Extract strings from VerificationExpression for pattern checking
fn extract_verification_strings(expr: &VerificationExpression) -> Vec<String> {
    match expr {
        VerificationExpression::Simple(s) => vec![s.clone()],
        VerificationExpression::Conditional {
            condition,
            if_true,
            if_false,
            always,
        } => {
            let mut strings = vec![condition.clone()];
            if let Some(commands) = if_true {
                strings.extend(commands.clone());
            }
            if let Some(commands) = if_false {
                strings.extend(commands.clone());
            }
            if let Some(commands) = always {
                strings.extend(commands.clone());
            }
            strings
        }
    }
}

/// Check if a test case uses variables that require bash 4.0+ associative arrays
fn test_case_uses_variables(test_case: &TestCase) -> bool {
    // Check if any sequence has variables
    for sequence in &test_case.test_sequences {
        if let Some(ref vars) = sequence.variables {
            if !vars.is_empty() {
                return true;
            }
        }

        // Check if any step has capture_vars or uses variable substitution
        for step in &sequence.steps {
            if let Some(ref capture_vars) = step.capture_vars {
                if !capture_vars_is_empty(capture_vars) {
                    return true;
                }
            }

            // Check if command uses variable substitution
            if step.command.contains("${") || step.command.contains("$STEP_VARS") {
                return true;
            }

            // Check if verification expressions use variables
            for s in extract_verification_strings(&step.verification.result) {
                if s.contains("${") || s.contains("$STEP_VARS") {
                    return true;
                }
            }
            for s in extract_verification_strings(&step.verification.output) {
                if s.contains("${") || s.contains("$STEP_VARS") {
                    return true;
                }
            }
            if let Some(ref output_file) = step.verification.output_file {
                for s in extract_verification_strings(output_file) {
                    if s.contains("${") || s.contains("$STEP_VARS") {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Check if a test case uses hydration variables (${#VAR_NAME} pattern)
fn test_case_uses_hydration_vars(test_case: &TestCase) -> bool {
    let hydrator = VarHydrator::new();

    // Check all test sequences
    for sequence in &test_case.test_sequences {
        // Check each step's command and verification expressions
        for step in &sequence.steps {
            // Check command
            if !hydrator.extract_placeholders(&step.command).is_empty() {
                return true;
            }

            // Check verification result
            for s in extract_verification_strings(&step.verification.result) {
                if !hydrator.extract_placeholders(&s).is_empty() {
                    return true;
                }
            }

            // Check verification output
            for s in extract_verification_strings(&step.verification.output) {
                if !hydrator.extract_placeholders(&s).is_empty() {
                    return true;
                }
            }

            // Check verification output_file
            if let Some(ref output_file) = step.verification.output_file {
                for s in extract_verification_strings(output_file) {
                    if !hydrator.extract_placeholders(&s).is_empty() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Replace ${#VAR_NAME} with ${VAR_NAME} (remove the # marker)
fn convert_hydration_placeholder_to_bash(text: &str) -> String {
    let placeholder_pattern = Regex::new(r"\$\{#([A-Z_][A-Z0-9_]*)\}").unwrap();
    placeholder_pattern
        .replace_all(text, |caps: &regex::Captures| format!("${{{}}}", &caps[1]))
        .to_string()
}

/// Convert hydration placeholders in a VerificationExpression
fn convert_verification_expr_hydration(expr: &VerificationExpression) -> VerificationExpression {
    match expr {
        VerificationExpression::Simple(s) => {
            VerificationExpression::Simple(convert_hydration_placeholder_to_bash(s))
        }
        VerificationExpression::Conditional {
            condition,
            if_true,
            if_false,
            always,
        } => VerificationExpression::Conditional {
            condition: convert_hydration_placeholder_to_bash(condition),
            if_true: if_true.as_ref().map(|cmds| {
                cmds.iter()
                    .map(|s| convert_hydration_placeholder_to_bash(s))
                    .collect()
            }),
            if_false: if_false.as_ref().map(|cmds| {
                cmds.iter()
                    .map(|s| convert_hydration_placeholder_to_bash(s))
                    .collect()
            }),
            always: always.as_ref().map(|cmds| {
                cmds.iter()
                    .map(|s| convert_hydration_placeholder_to_bash(s))
                    .collect()
            }),
        },
    }
}

/// Generate verification script with variable substitution support
fn generate_verification_with_var_subst(expr: &VerificationExpression, var_name: &str) -> String {
    let bash_expr: bash_eval::BashExpression = expr.into();
    bash_eval::generate_verification_with_var_subst(&bash_expr, var_name)
}

/// Format a multi-line text as shell comments, with the first line prefixed with a label
/// and subsequent lines properly commented
fn format_as_comment(prefix: &str, text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();

    for (idx, line) in lines.iter().enumerate() {
        if idx == 0 {
            result.push_str("# ");
            result.push_str(prefix);
            result.push_str(line);
        } else {
            result.push_str("# ");
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

impl TestExecutor {
    pub fn new() -> Self {
        Self {
            output_dir: None,
            config: Config::load_or_default(),
            run_id: None,
        }
    }

    /// Set the run ID written to execution log entries. The `TCMS_RUN_ID` environment
    /// variable of the process that runs the generated script takes precedence.
    pub fn with_run_id(mut self, run_id: Option<String>) -> Self {
        self.run_id = run_id
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        self
    }

    pub fn with_output_dir<P: Into<PathBuf>>(output_dir: P) -> Self {
        Self {
            output_dir: Some(output_dir.into()),
            config: Config::load_or_default(),
            run_id: None,
        }
    }

    pub fn with_config(config: Config) -> Self {
        Self {
            output_dir: None,
            config,
            run_id: None,
        }
    }

    pub fn with_output_dir_and_config<P: Into<PathBuf>>(output_dir: P, config: Config) -> Self {
        Self {
            output_dir: Some(output_dir.into()),
            config,
            run_id: None,
        }
    }

    /// Generate JSON escaping code based on configuration
    ///
    /// Returns bash script code that reads from $COMMAND_OUTPUT and sets $OUTPUT_ESCAPED
    /// based on the configured JSON escaping method.
    fn generate_json_escaping_code(&self) -> String {
        let mut script = String::new();

        script.push_str("# Escape output for JSON (BSD/GNU compatible)\n");

        let method = &self.config.script_generation.json_escaping.method;
        let binary_path = &self.config.script_generation.json_escaping.binary_path;
        let jq_path = &self.config.script_generation.json_escaping.jq_path;

        match method {
            JsonEscapingMethod::Jq => {
                // Use jq reading from COMMAND_OUTPUT
                let jq_cmd = jq_path.as_ref().and_then(|p| p.to_str()).unwrap_or("jq");
                script.push_str(&format!(
                    "OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | {} -Rs . 2>/dev/null | sed 's/^\"//;s/\"$//' || echo \"\")\n",
                    jq_cmd
                ));
            }
            JsonEscapingMethod::RustBinary => {
                // Use json-escape binary reading from COMMAND_OUTPUT
                let bin_path = binary_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("json-escape");
                script.push_str(&format!(
                    "OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | {} 2>/dev/null || echo \"\")\n",
                    bin_path
                ));
            }
            JsonEscapingMethod::ShellFallback => {
                // Use sed/awk fallback reading from COMMAND_OUTPUT
                script.push_str("# Shell fallback: escape backslashes, quotes, tabs, and convert newlines to \\n\n");
                script.push_str("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | sed 's/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\t/\\\\t/g' | awk '{printf \"%s%s\", (NR>1?\"\\\\n\":\"\"), $0}')\n");
            }
            JsonEscapingMethod::Auto => {
                // Try json-escape binary first, then jq, then fallback to sed/awk
                let jq_cmd = jq_path.as_ref().and_then(|p| p.to_str()).unwrap_or("jq");
                let bin_path = binary_path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("json-escape");

                script.push_str(&format!(
                    "if command -v {} >/dev/null 2>&1; then\n",
                    bin_path
                ));
                script.push_str(&format!(
                    "    OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | {} 2>/dev/null || echo \"\")\n",
                    bin_path
                ));
                script.push_str(&format!(
                    "elif command -v {} >/dev/null 2>&1; then\n",
                    jq_cmd
                ));
                script.push_str(&format!(
                    "    OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | {} -Rs . 2>/dev/null | sed 's/^\"//;s/\"$//' || echo \"\")\n",
                    jq_cmd
                ));
                script.push_str("else\n");
                script.push_str("    # Shell fallback: escape backslashes, quotes, tabs, and convert newlines to \\n\n");
                script.push_str("    OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | sed 's/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\t/\\\\t/g' | awk '{printf \"%s%s\", (NR>1?\"\\\\n\":\"\"), $0}')\n");
                script.push_str("fi\n");
            }
        }

        script
    }

    /// Detect if a verification expression string is a bash command or a human-readable prompt
    /// Returns true if it looks like a bash expression (starts with bash operators or common commands)
    /// Returns false if it looks like human-readable text (a prompt for manual verification)
    fn is_bash_expression(expr_str: &str) -> bool {
        let trimmed = expr_str.trim();

        // Check for common bash expression patterns
        if trimmed.starts_with("[[")
            || trimmed.starts_with("[")
            || trimmed.starts_with("grep")
            || trimmed.starts_with("test")
            || trimmed.starts_with("echo")
            || trimmed.starts_with("cat")
            || trimmed.starts_with("if ")
            || trimmed.starts_with("!")
            || trimmed.starts_with("$")
            || trimmed.starts_with("-f ")
            || trimmed.starts_with("-d ")
            || trimmed.starts_with("command ")
            || trimmed.starts_with("hash ")
            || trimmed.starts_with("read_true_false ")
            || trimmed.starts_with("read_verification ")
            || trimmed.starts_with("tail ")
            || trimmed.starts_with("head ")
            || trimmed.starts_with("wc ")
            || trimmed.starts_with("find ")
            || trimmed.starts_with("ls ")
            || trimmed.starts_with("ps ")
            || trimmed.starts_with("pgrep ")
            || trimmed.starts_with("curl ")
            || trimmed.starts_with("wget ")
            || trimmed.starts_with("nc ")
            || trimmed.starts_with("netcat ")
            || trimmed.contains(" || ")
            || trimmed.contains(" && ")
            || trimmed.contains(" -eq ")
            || trimmed.contains(" -ne ")
            || trimmed.contains(" -lt ")
            || trimmed.contains(" -gt ")
            || trimmed.contains(" = ")
            || trimmed.contains(" == ")
            || trimmed.contains(" != ")
            || trimmed.starts_with("awk ")
            || trimmed.starts_with("sed ")
        {
            return true;
        }

        // If it's just "true" or "false", it's a bash expression
        if trimmed == "true" || trimmed == "false" {
            return true;
        }

        // Default to false - treat as prompt text
        false
    }

    /// Generate bash script code for a VerificationExpression
    /// For Simple expressions, generates an if statement that sets the variable on success
    /// For Conditional expressions, generates bash code to evaluate the condition
    /// and execute the appropriate commands based on the result
    fn generate_verification_script(expr: &VerificationExpression, var_name: &str) -> String {
        let bash_expr: bash_eval::BashExpression = expr.into();
        bash_eval::generate_verification_script(&bash_expr, var_name)
    }

    /// Generate bash code to execute a hook
    fn generate_hook_execution(
        hook_name: &str,
        hook_config: &testcase_models::HookConfig,
        yaml_dir: Option<&Path>,
    ) -> String {
        let mut code = String::new();

        code.push_str(&format!("# Execute {} hook\n", hook_name));

        // Check if command is a .sh file (source it) or another executable (execute it)
        let command = &hook_config.command;
        let is_sh_file = command.ends_with(".sh");

        // Resolve absolute path for relative hook commands
        let resolved_command = if command.starts_with('/') {
            // Absolute path - use as is
            command.to_string()
        } else if let Some(dir) = yaml_dir {
            // Relative path - convert to absolute path relative to yaml_dir
            let full_path = dir.join(command);
            // Try to get absolute path
            match std::fs::canonicalize(&full_path) {
                Ok(abs_path) => abs_path.to_string_lossy().to_string(),
                Err(_) => {
                    // If canonicalize fails (file doesn't exist), construct absolute path manually
                    match std::env::current_dir() {
                        Ok(cwd) => cwd.join(&full_path).to_string_lossy().to_string(),
                        Err(_) => full_path.to_string_lossy().to_string(),
                    }
                }
            }
        } else {
            // No yaml_dir - use command as is
            command.to_string()
        };

        // Determine on_error behavior (default is Fail)
        let on_error = hook_config
            .on_error
            .as_ref()
            .map(|e| matches!(e, testcase_models::OnError::Continue))
            .unwrap_or(false);

        code.push_str("set +e\n");

        if is_sh_file {
            // Source .sh files in a subshell to prevent exit from terminating main script
            code.push_str(&format!("if [ -f \"{}\" ]; then\n", resolved_command));
            code.push_str(&format!("    (source \"{}\")\n", resolved_command));
            code.push_str("    HOOK_EXIT_CODE=$?\n");
            code.push_str("else\n");
            code.push_str(&format!(
                "    echo \"Warning: Hook script '{}' not found\" >&2\n",
                resolved_command
            ));
            code.push_str("    HOOK_EXIT_CODE=127\n");
            code.push_str("fi\n");
        } else {
            // Execute other files
            code.push_str(&format!("{}\n", resolved_command));
            code.push_str("HOOK_EXIT_CODE=$?\n");
        }

        code.push_str("set -e\n");

        // Handle hook failure based on on_error setting
        if on_error {
            // Continue on error
            code.push_str("if [ $HOOK_EXIT_CODE -ne 0 ]; then\n");
            code.push_str(&format!("    echo \"Warning: {} hook failed with exit code $HOOK_EXIT_CODE (continuing)\" >&2\n", hook_name));
            code.push_str("fi\n");
        } else {
            // Fail on error (default)
            code.push_str("if [ $HOOK_EXIT_CODE -ne 0 ]; then\n");
            code.push_str(&format!(
                "    echo \"Error: {} hook failed with exit code $HOOK_EXIT_CODE\" >&2\n",
                hook_name
            ));
            // Only record hook failure in JSON for hooks that run BEFORE JSON is closed
            // (script_start, setup_test, before_sequence, before_step, after_step, after_sequence)
            // Don't record for teardown_test and script_end which run AFTER JSON array is closed
            let record_failure = !matches!(hook_name, "teardown_test" | "script_end");

            if record_failure {
                code.push_str("    # Record hook failure in JSON before exiting\n");
                code.push_str("    if [ \"$FIRST_ENTRY\" = \"true\" ]; then\n");
                code.push_str("        FIRST_ENTRY=false\n");
                code.push_str("    else\n");
                code.push_str("        echo ',' >> \"$JSON_LOG\"\n");
                code.push_str("    fi\n");
                // Write hook failure entry using echo instead of heredoc to ensure variable substitution
                code.push_str("    cat >> \"$JSON_LOG\" << EOF\n");
                // The entry must satisfy the execution-log schema (test_sequence, step,
                // command, exit_code, output) and TestStepExecutionEntry, which is also
                // what the verifier parses. Hooks outside a sequence or step log 0.
                // Escape for JSON, then for the unquoted heredoc below.
                let mut hook_path = String::new();
                for c in resolved_command.chars() {
                    match c {
                        // JSON `\\`, written as `\\\\` so the heredoc yields `\\`
                        '\\' => hook_path.push_str("\\\\\\\\"),
                        // JSON `\"`; the heredoc keeps a backslash before a quote
                        '"' => hook_path.push_str("\\\""),
                        // heredoc must not expand these
                        '$' | '`' => {
                            hook_path.push('\\');
                            hook_path.push(c);
                        }
                        // inline hooks are multi-line scripts; JSON forbids raw control characters
                        '\n' => hook_path.push_str("\\n"),
                        '\r' => hook_path.push_str("\\r"),
                        '\t' => hook_path.push_str("\\t"),
                        c if (c as u32) < 0x20 => {
                            hook_path.push_str(&format!("\\u{:04x}", c as u32))
                        }
                        c => hook_path.push(c),
                    }
                }
                code.push_str("  {\n");
                code.push_str("    \"test_case_id\": \"$TEST_CASE_ID_ESCAPED\",\n");
                code.push_str("    $RUN_ID_JSON\n");
                code.push_str("    \"test_sequence\": ${TEST_SEQUENCE_ID:-0},\n");
                code.push_str("    \"step\": ${TEST_STEP_NUMBER:-0},\n");
                code.push_str(&format!("    \"command\": \"{}\",\n", hook_path));
                code.push_str("    \"exit_code\": $HOOK_EXIT_CODE,\n");
                code.push_str(&format!(
                    "    \"output\": \"{} hook failed with exit code $HOOK_EXIT_CODE\",\n",
                    hook_name
                ));
                code.push_str("    \"timestamp\": \"$TIMESTAMP\",\n");
                code.push_str(&format!("    \"hook_type\": \"{}\",\n", hook_name));
                code.push_str(&format!("    \"hook_path\": \"{}\",\n", hook_path));
                code.push_str("    \"result_verification_pass\": false,\n");
                code.push_str("    \"output_verification_pass\": false\n");
                code.push_str("  }\n");
                code.push_str("EOF\n");
            }

            code.push_str("    exit $HOOK_EXIT_CODE\n");
            code.push_str("fi\n");
        }

        code.push('\n');
        code
    }

    pub fn generate_test_script_with_json_output(
        &self,
        test_case: &TestCase,
        json_output_path: &Path,
        source_hash: Option<String>,
        yaml_dir: Option<&Path>,
    ) -> String {
        let mut script = String::new();

        script.push_str("#!/bin/bash\n");
        script.push_str("set -euo pipefail\n\n");

        // Initialize JSON logging FIRST before any hooks (so JSON file exists even if hooks fail)
        script.push_str(&format!("JSON_LOG=\"{}\"\n", json_output_path.display()));
        script.push_str("TIMESTAMP=$(date +\"%Y-%m-%dT%H:%M:%S\")\n");
        // Identify the test case and the run in every log entry. The run ID comes from
        // TCMS_RUN_ID when the script runs, else from the value given at generation;
        // without one, no run_id is written.
        script.push_str(&format!(
            "TEST_CASE_ID={}\n",
            shell_single_quote(&test_case.id)
        ));
        script.push_str(&format!(
            "RUN_ID=\"${{TCMS_RUN_ID:-}}\"\n[ -n \"$RUN_ID\" ] || RUN_ID={}\n",
            shell_single_quote(self.run_id.as_deref().unwrap_or(""))
        ));
        // JSON-escape both with the configured method (the same code as for command output)
        let escaping = self.generate_json_escaping_code();
        script.push_str("COMMAND_OUTPUT=\"$TEST_CASE_ID\"\n");
        script.push_str(&escaping);
        script.push_str("TEST_CASE_ID_ESCAPED=\"$OUTPUT_ESCAPED\"\n");
        script.push_str("COMMAND_OUTPUT=\"$RUN_ID\"\n");
        script.push_str(&escaping);
        script.push_str("RUN_ID_ESCAPED=\"$OUTPUT_ESCAPED\"\n");
        script.push_str("COMMAND_OUTPUT=\"\"\nOUTPUT_ESCAPED=\"\"\n");
        script.push_str("RUN_ID_JSON=\"\"\n");
        script.push_str("if [ -n \"$RUN_ID\" ]; then RUN_ID_JSON=\"\\\"run_id\\\": \\\"$RUN_ID_ESCAPED\\\",\"; fi\n");
        if let Some(ref hash) = source_hash {
            script.push_str(&format!("SOURCE_YAML_SHA256=\"{}\"\n", hash));
        }
        script.push('\n');

        // Check if test case uses variables
        let uses_variables = test_case_uses_variables(test_case);

        if uses_variables {
            // Initialize variable storage (bash 3.2+ compatible using space-separated string)
            script.push_str(
                "# Initialize variable storage for captured variables (bash 3.2+ compatible)\n",
            );
            script.push_str("CAPTURED_VAR_NAMES=\"\"\n\n");
        }

        // Add trap to ensure JSON file is properly closed on any exit and temp files are cleaned
        script.push_str(
            "# Trap to ensure JSON file is closed properly on exit and temp files are cleaned\n",
        );
        script.push_str("cleanup() {\n");
        script.push_str("    # Clean up temp capture file if it exists\n");
        script.push_str(
            "    if [ -n \"${_CAPTURE_TMPFILE:-}\" ] && [ -f \"$_CAPTURE_TMPFILE\" ]; then\n",
        );
        script.push_str("        rm -f \"$_CAPTURE_TMPFILE\"\n");
        script.push_str("    fi\n");
        script.push_str("    if [ -f \"$JSON_LOG\" ]; then\n");
        script.push_str("        # Check if JSON_LOG ends with '[' or ','\n");
        script.push_str("        LAST_CHAR=$(tail -c 2 \"$JSON_LOG\" | head -c 1)\n");
        script.push_str("        if [ \"$LAST_CHAR\" != \"]\" ]; then\n");
        script.push_str("            echo '' >> \"$JSON_LOG\"\n");
        script.push_str("            echo ']' >> \"$JSON_LOG\"\n");
        script.push_str("        fi\n");
        script.push_str("        # Validate JSON\n");
        script.push_str("        if command -v jq >/dev/null 2>&1; then\n");
        script.push_str("            if ! jq empty \"$JSON_LOG\" >/dev/null 2>&1; then\n");
        script.push_str("                echo \"500 - Internal Script Error: Generated JSON is not valid\" >&2\n");
        script.push_str("                exit 1\n");
        script.push_str("            fi\n");
        script.push_str("        fi\n");
        script.push_str("    fi\n");
        script.push_str("}\n");
        script.push_str("trap cleanup EXIT\n\n");

        // Initialize JSON file BEFORE any hooks
        script.push_str("echo '[' > \"$JSON_LOG\"\n");
        script.push_str("FIRST_ENTRY=true\n\n");

        // Execute script_start hook after JSON is initialized
        if let Some(ref hooks) = test_case.hooks {
            if let Some(ref hook) = hooks.script_start {
                script.push_str(&Self::generate_hook_execution(
                    "script_start",
                    hook,
                    yaml_dir,
                ));
            }
        }

        // Add bash helper functions for Y/n prompts
        script.push_str(include_str!("../../../scripts/lib/read_true_false.sh"));
        script.push('\n');

        script.push_str("# Prompts user for verification with Y/n input\n");
        script.push_str("# Returns: 0 for yes, 1 for no (standard bash convention)\n");
        script
            .push_str("# Supports both interactive and non-interactive modes with TTY detection\n");
        script.push_str("read_verification() {\n");
        script.push_str("    local prompt=\"$1\"\n");
        script.push_str("    local default=\"${2:-y}\"\n");
        script.push_str("    \n");
        script.push_str("    # Check if running in non-interactive mode\n");
        script.push_str(
            "    if [[ \"${DEBIAN_FRONTEND:-}\" == 'noninteractive' ]] || ! [ -t 0 ]; then\n",
        );
        script.push_str("        # Non-interactive mode: return default\n");
        script.push_str("        if [[ \"$default\" =~ ^[Yy]$ ]]; then\n");
        script.push_str("            return 0\n");
        script.push_str("        else\n");
        script.push_str("            return 1\n");
        script.push_str("        fi\n");
        script.push_str("    fi\n");
        script.push_str("    \n");
        script.push_str("    # Interactive mode: prompt user\n");
        script.push_str("    while true; do\n");
        script.push_str("        if [[ \"$default\" =~ ^[Yy]$ ]]; then\n");
        script.push_str("            read -p \"$prompt [Y/n]: \" response\n");
        script.push_str("        else\n");
        script.push_str("            read -p \"$prompt [y/N]: \" response\n");
        script.push_str("        fi\n");
        script.push_str("        \n");
        script.push_str("        # Empty response uses default\n");
        script.push_str("        if [[ -z \"$response\" ]]; then\n");
        script.push_str("            response=\"$default\"\n");
        script.push_str("        fi\n");
        script.push_str("        \n");
        script.push_str("        # Validate response\n");
        script.push_str("        case \"$response\" in\n");
        script.push_str("            [Yy]|[Yy][Ee][Ss])\n");
        script.push_str("                return 0\n");
        script.push_str("                ;;\n");
        script.push_str("            [Nn]|[Nn][Oo])\n");
        script.push_str("                return 1\n");
        script.push_str("                ;;\n");
        script.push_str("            *)\n");
        script.push_str("                echo \"Invalid response. Please enter Y or n.\" >&2\n");
        script.push_str("                ;;\n");
        script.push_str("        esac\n");
        script.push_str("    done\n");
        script.push_str("}\n\n");

        script.push_str("# Test Case: ");
        script.push_str(&test_case.id);
        script.push('\n');
        script.push_str(&format_as_comment("Description: ", &test_case.description));
        if let Some(ref hash) = source_hash {
            script.push_str("# Source YAML SHA-256: ");
            script.push_str(hash);
            script.push('\n');
        }
        script.push('\n');

        // Check if test case uses hydration variables
        let uses_hydration_vars = test_case_uses_hydration_vars(test_case);

        // Source the export file if hydration variables are present
        if uses_hydration_vars {
            script.push_str("# Source environment variables for hydration\n");
            script.push_str(&format!("EXPORT_FILE=\"{}.env\"\n", test_case.id));
            script.push_str("if [ -f \"$EXPORT_FILE\" ]; then\n");
            script.push_str("    source \"$EXPORT_FILE\"\n");
            script.push_str("fi\n\n");
        }

        // Generate prerequisite checks
        if let Some(ref prerequisites) = test_case.prerequisites {
            if !prerequisites.is_empty() {
                script.push_str("# Prerequisites\n");
                script.push_str("echo \"Checking prerequisites...\"\n\n");

                for (idx, prereq) in prerequisites.iter().enumerate() {
                    script.push_str(&format!(
                        "# Prerequisite {}: {}\n",
                        idx + 1,
                        prereq.description
                    ));

                    match prereq.prerequisite_type {
                        testcase_models::PrerequisiteType::Manual => {
                            // Manual prerequisite: output description and prompt for confirmation
                            script.push_str(&format!(
                                "echo \"[MANUAL PREREQUISITE {}] {}\"\n",
                                idx + 1,
                                prereq.description.replace("\"", "\\\"")
                            ));

                            // Check if we're in interactive mode (TTY available and DEBIAN_FRONTEND not set to noninteractive)
                            script.push_str("if [[ \"${DEBIAN_FRONTEND:-}\" != 'noninteractive' && -t 0 ]]; then\n");
                            script.push_str("    read -p \"Press ENTER to confirm this prerequisite is satisfied...\"\n");
                            script.push_str("else\n");
                            script.push_str("    echo \"Non-interactive mode: assuming prerequisite is satisfied.\"\n");
                            script.push_str("fi\n");
                        }
                        testcase_models::PrerequisiteType::Automatic => {
                            // Automatic prerequisite: execute verification_command
                            if let Some(ref verification_cmd) = prereq.verification_command {
                                script.push_str(&format!(
                                    "echo \"[AUTOMATIC PREREQUISITE {}] Verifying: {}\"\n",
                                    idx + 1,
                                    prereq.description.replace("\"", "\\\"")
                                ));
                                script.push_str("set +e\n");
                                script.push_str(&format!(
                                    "PREREQ_OUTPUT=$({{ {}; }} 2>&1)\n",
                                    verification_cmd
                                ));
                                script.push_str("PREREQ_EXIT_CODE=$?\n");
                                script.push_str("set -e\n");
                                script.push_str("if [ $PREREQ_EXIT_CODE -ne 0 ]; then\n");
                                script.push_str(&format!(
                                    "    echo \"WARNING: Prerequisite {} failed: {}\"\n",
                                    idx + 1,
                                    prereq.description.replace("\"", "\\\"")
                                ));
                                script.push_str("    echo \"Verification command: ");
                                script.push_str(&verification_cmd.replace("\"", "\\\""));
                                script.push_str("\"\n");
                                script.push_str("    echo \"Exit code: $PREREQ_EXIT_CODE\"\n");
                                script.push_str("    echo \"Output: $PREREQ_OUTPUT\"\n");
                                script.push_str("    echo \"Continuing with test execution despite prerequisite failure...\"\n");
                                script.push_str("else\n");
                                script.push_str(&format!(
                                    "    echo \"[PASS] Prerequisite {} verified\"\n",
                                    idx + 1
                                ));
                                script.push_str("fi\n");
                            } else {
                                // Automatic prerequisite without verification command - treat as error
                                script.push_str(&format!(
                                    "echo \"ERROR: Automatic prerequisite {} has no verification_command\"\n",
                                    idx + 1
                                ));
                                script.push_str("exit 1\n");
                            }
                        }
                    }
                    script.push('\n');
                }

                script.push_str("echo \"All prerequisites satisfied\"\n");
                script.push_str("echo \"\"\n\n");
            }
        }

        // Execute setup_test hook after prerequisites
        if let Some(ref hooks) = test_case.hooks {
            if let Some(ref hook) = hooks.setup_test {
                script.push_str(&Self::generate_hook_execution("setup_test", hook, yaml_dir));
            }
        }

        // Instantiate BDD step registry
        let bdd_registry = BddStepRegistry::load_from_default_paths().unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to load BDD step definitions: {}",
                e
            );
            eprintln!("BDD patterns will not be available. Initial conditions will be treated as comments.");
            BddStepRegistry::new()
        });

        if !test_case.general_initial_conditions.is_empty() {
            script.push_str("# General Initial Conditions\n");

            // Output include array as comments
            if let Some(ref includes) = test_case.general_initial_conditions.include {
                for include_ref in includes {
                    if let Some(ref test_seq) = include_ref.test_sequence {
                        script.push_str(&format!(
                            "# Include: {} (test_sequence: {})\n",
                            include_ref.id, test_seq
                        ));
                    } else {
                        script.push_str(&format!("# Include: {}\n", include_ref.id));
                    }
                }
            }

            for (key, values) in &test_case.general_initial_conditions.devices {
                for value in values {
                    let value_str = match value {
                        testcase_models::InitialConditionItem::String(s) => s.clone(),
                        testcase_models::InitialConditionItem::RefItem { reference } => {
                            format!("ref: {}", reference)
                        }
                        testcase_models::InitialConditionItem::TestSequenceRef {
                            test_sequence,
                        } => {
                            format!(
                                "test_sequence: id={}, step={}",
                                test_sequence.id, test_sequence.step
                            )
                        }
                    };
                    if let Some(command) = bdd_registry.try_parse_as_bdd(&value_str) {
                        script.push_str(&format!("# {}: {}\n", key, value_str));
                        // Wrap BDD commands in error handling so failures don't cause script exit
                        script.push_str("set +e\n");
                        script.push_str(&format!("{}\n", command));
                        script.push_str("set -e\n");
                    } else {
                        script.push_str(&format!("# {}: {}\n", key, value_str));
                    }
                }
            }
            script.push('\n');
        }

        if !test_case.initial_conditions.is_empty() {
            script.push_str("# Initial Conditions\n");

            // Output include array as comments
            if let Some(ref includes) = test_case.initial_conditions.include {
                for include_ref in includes {
                    if let Some(ref test_seq) = include_ref.test_sequence {
                        script.push_str(&format!(
                            "# Include: {} (test_sequence: {})\n",
                            include_ref.id, test_seq
                        ));
                    } else {
                        script.push_str(&format!("# Include: {}\n", include_ref.id));
                    }
                }
            }

            for (key, values) in &test_case.initial_conditions.devices {
                for value in values {
                    let value_str = match value {
                        testcase_models::InitialConditionItem::String(s) => s.clone(),
                        testcase_models::InitialConditionItem::RefItem { reference } => {
                            format!("ref: {}", reference)
                        }
                        testcase_models::InitialConditionItem::TestSequenceRef {
                            test_sequence,
                        } => {
                            format!(
                                "test_sequence: id={}, step={}",
                                test_sequence.id, test_sequence.step
                            )
                        }
                    };
                    if let Some(command) = bdd_registry.try_parse_as_bdd(&value_str) {
                        script.push_str(&format!("# {}: {}\n", key, value_str));
                        // Wrap BDD commands in error handling so failures don't cause script exit
                        script.push_str("set +e\n");
                        script.push_str(&format!("{}\n", command));
                        script.push_str("set -e\n");
                    } else {
                        script.push_str(&format!("# {}: {}\n", key, value_str));
                    }
                }
            }
            script.push('\n');
        }

        for sequence in &test_case.test_sequences {
            script.push_str(&format!(
                "# Test Sequence {}: {}\n",
                sequence.id, sequence.name
            ));
            for line in sequence.description.split('\n') {
                script.push_str(&format!("# {}\n", line));
            }

            // Execute before_sequence hook with TEST_SEQUENCE_ID and TEST_SEQUENCE_NAME
            if let Some(ref hooks) = test_case.hooks {
                if let Some(ref hook) = hooks.before_sequence {
                    script.push_str(&format!("TEST_SEQUENCE_ID={}\n", sequence.id));
                    script.push_str(&format!(
                        "TEST_SEQUENCE_NAME={}\n",
                        bash_escape(&sequence.name)
                    ));
                    script.push_str("export TEST_SEQUENCE_ID TEST_SEQUENCE_NAME\n");
                    script.push_str(&Self::generate_hook_execution(
                        "before_sequence",
                        hook,
                        yaml_dir,
                    ));
                }
            }

            if !sequence.initial_conditions.is_empty() {
                script.push_str("# Sequence Initial Conditions\n");

                // Output include array as comments
                if let Some(ref includes) = sequence.initial_conditions.include {
                    for include_ref in includes {
                        if let Some(ref test_seq) = include_ref.test_sequence {
                            script.push_str(&format!(
                                "# Include: {} (test_sequence: {})\n",
                                include_ref.id, test_seq
                            ));
                        } else {
                            script.push_str(&format!("# Include: {}\n", include_ref.id));
                        }
                    }
                }

                for (key, values) in &sequence.initial_conditions.devices {
                    for value in values {
                        let value_str = match value {
                            testcase_models::InitialConditionItem::String(s) => s.clone(),
                            testcase_models::InitialConditionItem::RefItem { reference } => {
                                format!("ref: {}", reference)
                            }
                            testcase_models::InitialConditionItem::TestSequenceRef {
                                test_sequence,
                            } => format!(
                                "test_sequence: id={}, step={}",
                                test_sequence.id, test_sequence.step
                            ),
                        };
                        if let Some(command) = bdd_registry.try_parse_as_bdd(&value_str) {
                            script.push_str(&format!("# {}: {}\n", key, value_str));
                            // Wrap BDD commands in error handling so failures don't cause script exit
                            script.push_str("set +e\n");
                            script.push_str(&format!("{}\n", command));
                            script.push_str("set -e\n");
                        } else {
                            script.push_str(&format!("# {}: {}\n", key, value_str));
                        }
                    }
                }
            }
            script.push('\n');

            // Initialize sequence-level variables
            if let Some(ref variables) = sequence.variables {
                if !variables.is_empty() {
                    script.push_str("# Initialize sequence variables\n");
                    for (var_name, var_value) in variables {
                        script.push_str(&format!("{}={}\n", var_name, bash_escape(var_value)));
                        script.push_str(&format!(
                            "if ! echo \" $CAPTURED_VAR_NAMES \" | grep -q \" {} \"; then CAPTURED_VAR_NAMES=\"$CAPTURED_VAR_NAMES {}\"; fi\n",
                            var_name, var_name
                        ));
                    }
                    script.push('\n');
                }
            }

            for step in &sequence.steps {
                let comment =
                    format_multiline_comment(&format!("# Step {}: ", step.step), &step.description);
                script.push_str(&comment);
                script.push('\n');

                // Execute before_step hook with TEST_STEP_NUMBER and TEST_STEP_DESCRIPTION
                if let Some(ref hooks) = test_case.hooks {
                    if let Some(ref hook) = hooks.before_step {
                        script.push_str(&format!("TEST_STEP_NUMBER={}\n", step.step));
                        script.push_str(&format!(
                            "TEST_STEP_DESCRIPTION={}\n",
                            bash_escape(&step.description)
                        ));
                        script.push_str("export TEST_STEP_NUMBER TEST_STEP_DESCRIPTION\n");
                        script.push_str(&Self::generate_hook_execution(
                            "before_step",
                            hook,
                            yaml_dir,
                        ));
                    }
                }

                if step.manual == Some(true) {
                    // Check if manual step has verification fields (not just "true")
                    let has_result_verification = !matches!(&step.verification.result,
                        VerificationExpression::Simple(s) if s.trim() == "true");
                    let has_output_verification = !matches!(&step.verification.output,
                        VerificationExpression::Simple(s) if s.trim() == "true");
                    let has_verification = has_result_verification || has_output_verification;

                    // Format multi-line description for echo statements
                    let echo_desc = step
                        .description
                        .lines()
                        .map(|line| line.trim())
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join(" ");
                    script.push_str(&format!(
                        "echo \"Step {}: {}\"\n",
                        step.step,
                        echo_desc.replace("\"", "\\\"")
                    ));
                    script.push_str(&format!(
                        "echo \"Command: {}\"\n",
                        step.command.replace("\"", "\\\"")
                    ));
                    script.push_str("echo \"INFO: This is a manual step. You must perform this action manually.\"\n");

                    if has_verification {
                        // Generate interactive prompt for action using read_true_false
                        script.push_str("# Prompt user to confirm manual action completion\n");
                        script.push_str("MANUAL_STEP_CONFIRMED=false\n");
                        script.push_str(
                            "if read_true_false \"Have you completed the manual action?\"; then\n",
                        );
                        script
                            .push_str("    # User confirmed (read_true_false returns 0 for yes)\n");
                        script.push_str("    MANUAL_STEP_CONFIRMED=true\n");
                        script.push_str("else\n");
                        script.push_str("    # User rejected (read_true_false returns 1 for no)\n");
                        script.push_str("    MANUAL_STEP_CONFIRMED=false\n");
                        script.push_str("fi\n\n");

                        // Initialize COMMAND_OUTPUT for manual steps (no actual command output)
                        script.push_str("COMMAND_OUTPUT=\"Manual step\"\n\n");

                        // Convert hydration placeholders in verification expressions
                        let converted_result_expr =
                            convert_verification_expr_hydration(&step.verification.result);
                        let converted_output_expr =
                            convert_verification_expr_hydration(&step.verification.output);

                        // Evaluate verification expressions
                        script.push_str("# Manual step verification\n");
                        script.push_str("USER_VERIFICATION_RESULT=false\n");
                        script.push_str("USER_VERIFICATION_OUTPUT=false\n");

                        // Generate verification script for result
                        if has_result_verification {
                            let result_verification_script = if uses_variables {
                                generate_verification_with_var_subst(
                                    &converted_result_expr,
                                    "USER_VERIFICATION_RESULT",
                                )
                            } else {
                                Self::generate_verification_script(
                                    &converted_result_expr,
                                    "USER_VERIFICATION_RESULT",
                                )
                            };
                            script.push_str(&result_verification_script);
                        } else {
                            script.push_str("USER_VERIFICATION_RESULT=true\n");
                        }

                        // Generate verification script for output
                        if has_output_verification {
                            // Determine if this is a bash expression based on the expression type
                            let is_bash_expr = match &converted_output_expr {
                                VerificationExpression::Conditional { .. } => true, // Conditionals are always bash expressions
                                VerificationExpression::Simple(s) => Self::is_bash_expression(s),
                            };

                            // Check if this looks like a prompt (not a bash expression)
                            if !is_bash_expr && !uses_variables {
                                // Treat as a manual verification prompt
                                // Use read_verification to prompt the user
                                script.push_str("# Use read_verification for manual step prompt\n");
                                if let VerificationExpression::Simple(s) = &converted_output_expr {
                                    let escaped_prompt =
                                        s.replace("\\", "\\\\").replace("\"", "\\\"");
                                    script.push_str(&format!(
                                        "if read_verification \"{}\"; then\n",
                                        escaped_prompt
                                    ));
                                }
                                script.push_str("    USER_VERIFICATION_OUTPUT=true\n");
                                script.push_str("else\n");
                                script.push_str("    USER_VERIFICATION_OUTPUT=false\n");
                                script.push_str("fi\n");
                            } else {
                                // Generate verification script for bash expressions
                                let output_verification_script = if uses_variables {
                                    generate_verification_with_var_subst(
                                        &converted_output_expr,
                                        "USER_VERIFICATION_OUTPUT",
                                    )
                                } else {
                                    Self::generate_verification_script(
                                        &converted_output_expr,
                                        "USER_VERIFICATION_OUTPUT",
                                    )
                                };
                                script.push_str(&output_verification_script);
                            }
                        } else {
                            script.push_str("USER_VERIFICATION_OUTPUT=true\n");
                        }

                        // Set USER_VERIFICATION variable with combined results
                        script
                            .push_str("\n# Set USER_VERIFICATION based on verification results\n");
                        script.push_str("if [ \"$USER_VERIFICATION_RESULT\" = true ] && [ \"$USER_VERIFICATION_OUTPUT\" = true ]; then\n");
                        script.push_str("    USER_VERIFICATION=true\n");
                        script.push_str("else\n");
                        script.push_str("    USER_VERIFICATION=false\n");
                        script.push_str("fi\n\n");

                        // Output [PASS]/[FAIL] messages based on verification outcome
                        script.push_str("if [ \"$USER_VERIFICATION\" = true ]; then\n");
                        script.push_str(&format!(
                            "    echo \"[PASS] Step {}: {}\"\n",
                            step.step,
                            echo_desc.replace("\"", "\\\"")
                        ));
                        script.push_str("else\n");
                        script.push_str(&format!(
                            "    echo \"[FAIL] Step {}: {}\"\n",
                            step.step,
                            echo_desc.replace("\"", "\\\"")
                        ));
                        script.push_str(
                            "    echo \"  Result verification: $USER_VERIFICATION_RESULT\"\n",
                        );
                        script.push_str(
                            "    echo \"  Output verification: $USER_VERIFICATION_OUTPUT\"\n",
                        );
                        script.push_str("    exit 1\n");
                        script.push_str("fi\n\n");
                    } else {
                        // No verification fields - just prompt to continue using read_true_false
                        script.push_str("# Initialize COMMAND_OUTPUT for manual step\n");
                        script.push_str("COMMAND_OUTPUT=\"Manual step\"\n\n");
                        script.push_str("# Prompt user to confirm they want to continue\n");
                        script.push_str("MANUAL_STEP_CONFIRMED=false\n");
                        script.push_str(
                            "if read_true_false \"Have you completed the manual step?\"; then\n",
                        );
                        script
                            .push_str("    # User confirmed (read_true_false returns 0 for yes)\n");
                        script.push_str("    MANUAL_STEP_CONFIRMED=true\n");
                        script.push_str("else\n");
                        script.push_str("    # User rejected (read_true_false returns 1 for no)\n");
                        script.push_str("    MANUAL_STEP_CONFIRMED=false\n");
                        script.push_str("fi\n\n");

                        // Set verification variables for logging (no explicit verification)
                        script.push_str("if [ \"$MANUAL_STEP_CONFIRMED\" = true ]; then\n");
                        script.push_str("    USER_VERIFICATION_RESULT=true\n");
                        script.push_str("    USER_VERIFICATION_OUTPUT=true\n");
                        script.push_str("else\n");
                        script.push_str("    USER_VERIFICATION_RESULT=false\n");
                        script.push_str("    USER_VERIFICATION_OUTPUT=false\n");
                        script.push_str("fi\n\n");
                    }

                    // Log manual step to JSON execution log
                    script.push_str("# Log manual step to execution log\n");
                    script.push_str("if [ \"$FIRST_ENTRY\" = false ]; then\n");
                    script.push_str("    echo ',' >> \"$JSON_LOG\"\n");
                    script.push_str("fi\n");
                    script.push_str("FIRST_ENTRY=false\n\n");

                    // Escape command for JSON
                    let escaped_command = step
                        .command
                        .replace("\\", "\\\\")
                        .replace("'", "\"")
                        .replace("\"", "\\\"")
                        .replace("\n", "\\n")
                        .replace("\r", "\\r")
                        .replace("\t", "\\t");

                    script.push_str("# Write manual step JSON entry\n");
                    script.push_str("{\n");
                    script.push_str("    echo '  {\'\n");
                    script.push_str(
                        "    echo \"    \\\"test_case_id\\\": \\\"$TEST_CASE_ID_ESCAPED\\\",\"\n",
                    );
                    script.push_str(
                        "    if [ -n \"$RUN_ID\" ]; then echo \"    $RUN_ID_JSON\"; fi\n",
                    );
                    script.push_str(&format!(
                        "    echo '    \"test_sequence\": {},'\n",
                        sequence.id
                    ));
                    script.push_str(&format!("    echo '    \"step\": {},'\n", step.step));
                    script.push_str(&format!(
                        "    echo '    \"command\": \"{}\",'\n",
                        escaped_command
                    ));
                    script.push_str("    echo \"    \\\"exit_code\\\": 0,\"\n");
                    script.push_str("    if [ \"$MANUAL_STEP_CONFIRMED\" = true ]; then\n");
                    script.push_str(
                        "        echo \"    \\\"output\\\": \\\"Manual step confirmed by user\\\",\"\n",
                    );
                    script.push_str("    else\n");
                    script.push_str(
                        "        echo \"    \\\"output\\\": \\\"Manual step rejected by user\\\",\"\n",
                    );
                    script.push_str("    fi\n");
                    script.push_str("    echo \"    \\\"timestamp\\\": \\\"$TIMESTAMP\\\",\"\n");
                    if source_hash.is_some() {
                        script.push_str("    echo \"    \\\"source_yaml_sha256\\\": \\\"$SOURCE_YAML_SHA256\\\",\"\n");
                    }

                    if has_verification {
                        script.push_str("    echo \"    \\\"result_verification_pass\\\": $USER_VERIFICATION_RESULT,\"\n");
                        script.push_str("    echo \"    \\\"output_verification_pass\\\": $USER_VERIFICATION_OUTPUT\"\n");
                    } else {
                        script.push_str("    echo '    \"result_verification_pass\": true,'\n");
                        script.push_str("    echo '    \"output_verification_pass\": true'\n");
                    }
                    script.push_str("    echo '  }'\n");
                    script.push_str("} >> \"$JSON_LOG\"\n\n");

                    // Check if manual step was confirmed and handle rejection
                    script.push_str("if [ \"$MANUAL_STEP_CONFIRMED\" = false ]; then\n");
                    script.push_str("    echo \"Manual step not completed. Exiting.\" >&2\n");
                    script.push_str("    exit 1\n");
                    script.push_str("fi\n\n");

                    // Execute after_step hook for manual step
                    if let Some(ref hooks) = test_case.hooks {
                        if let Some(ref hook) = hooks.after_step {
                            // Set step context variables for after_step hook (manual steps don't have real exit code/output)
                            script.push_str("STEP_EXIT_CODE=0\n");
                            script.push_str("COMMAND_OUTPUT=\"\"\n");
                            script.push_str("export STEP_EXIT_CODE COMMAND_OUTPUT\n");
                            script.push_str(&Self::generate_hook_execution(
                                "after_step",
                                hook,
                                yaml_dir,
                            ));
                        }
                    }

                    continue;
                }

                // Format multi-line description for echo statements (automated steps)
                let echo_desc = step
                    .description
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");

                script.push_str(&format!(
                    "LOG_FILE=\"{}_sequence-{}_step-{}.actual.log\"\n",
                    test_case.id, sequence.id, step.step
                ));
                script.push_str("COMMAND_OUTPUT=\"\"\n");
                script.push_str("_CAPTURE_TMPFILE=$(mktemp)\n");
                script.push_str("set +e\n");

                // Convert hydration placeholders (${#VAR_NAME} -> ${VAR_NAME})
                let converted_command = convert_hydration_placeholder_to_bash(&step.command);

                // Trim trailing whitespace and newlines from the command
                let normalized_command = converted_command.trim_end().to_string();

                // For commands without here-documents, remove internal newlines to avoid bash syntax errors
                // This handles YAML commands that span multiple lines
                let is_heredoc_command = normalized_command.contains("<<");
                let normalized_command = if is_heredoc_command {
                    // Keep internal newlines for heredocs
                    normalized_command
                } else {
                    // Remove internal newlines for regular commands
                    normalized_command
                        .replace(['\n', '\r'], " ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                };

                // Check if the command needs variable substitution
                let needs_substitution =
                    normalized_command.contains("${") || normalized_command.contains("$STEP_VARS");

                if needs_substitution {
                    // Generate bash code to perform variable substitution on the command
                    // Store the original command in a variable
                    // Escape backslashes, quotes, and dollar signs to prevent premature expansion
                    let escaped_command = normalized_command
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("$", "\\$");
                    script.push_str(&format!("ORIGINAL_COMMAND=\"{}\"\n", escaped_command));

                    // Perform variable substitution: replace ${var_name} patterns using eval
                    script.push_str("SUBSTITUTED_COMMAND=\"$ORIGINAL_COMMAND\"\n");
                    script.push_str("if [ -n \"$CAPTURED_VAR_NAMES\" ]; then\n");
                    script.push_str("    for var_name in $CAPTURED_VAR_NAMES; do\n");
                    script.push_str("        eval \"var_value=\\$$var_name\"\n");
                    script.push_str("        # Escape special characters for sed\n");
                    script.push_str(
                        "        escaped_value=$(printf '%s' \"$var_value\" | sed 's/[&/\\]/\\\\&/g')\n",
                    );
                    script.push_str("        # Replace ${var_name} pattern\n");
                    script.push_str("        SUBSTITUTED_COMMAND=$(echo \"$SUBSTITUTED_COMMAND\" | sed \"s/\\${$var_name}/$escaped_value/g\")\n");
                    script.push_str("    done\n");
                    script.push_str("fi\n");

                    // Write command output to temp file and tee to log file
                    script.push_str(
                        "{ eval \"$SUBSTITUTED_COMMAND\"; } 2>&1 | tee \"$LOG_FILE\" > \"$_CAPTURE_TMPFILE\"\n",
                    );
                } else {
                    // No substitution needed - but still need to properly quote the command
                    // to handle commands with special characters (quotes, etc.)
                    let escaped_command = normalized_command
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("$", "\\$");
                    script.push_str(&format!("ORIGINAL_COMMAND=\"{}\"\n", escaped_command));

                    // Special handling for here-documents: ensure EOF delimiter is on its own line
                    if is_heredoc_command {
                        // For here-documents, put closing brace on new line to keep EOF delimiter intact
                        script.push_str(&format!(
                            "{{\n{}\n}} 2>&1 | tee \"$LOG_FILE\" > \"$_CAPTURE_TMPFILE\"\n",
                            normalized_command
                        ));
                    } else {
                        // For regular commands, use compact form
                        // Note: newlines have already been removed from normalized_command above
                        script.push_str(&format!(
                            "{{ {}; }} 2>&1 | tee \"$LOG_FILE\" > \"$_CAPTURE_TMPFILE\"\n",
                            normalized_command
                        ));
                    }
                }

                script.push_str("EXIT_CODE=$?\n");
                script.push_str("COMMAND_OUTPUT=$(cat \"$_CAPTURE_TMPFILE\")\n");
                script.push_str("set -e\n\n");

                // Variable capture: extract values from COMMAND_OUTPUT using regex patterns or commands
                if let Some(ref capture_vars) = step.capture_vars {
                    if !capture_vars_is_empty(capture_vars) {
                        script.push_str("# Capture variables from output\n");
                        for (var_name, capture_pattern, command) in
                            capture_vars_to_vec(capture_vars)
                        {
                            if let Some(pattern) = capture_pattern {
                                // Capture from COMMAND_OUTPUT using regex pattern
                                let sed_pattern = convert_pcre_to_sed_pattern(&pattern);
                                script.push_str(&format!(
                                    "{}=$(echo \"$COMMAND_OUTPUT\" | sed -n {} | head -n 1 || echo \"\")\n",
                                    var_name,
                                    bash_escape(&sed_pattern)
                                ));
                            } else if let Some(cmd) = command {
                                // Command-based capture: Execute the capture command and store result in STEP_VAR_*
                                // The command is executed directly (not on COMMAND_OUTPUT)
                                // Both stdout and stderr are captured (2>&1)
                                // Fallback to empty string if command fails (|| echo "")
                                script.push_str(&format!(
                                    "{}=$({} 2>&1 || echo \"\")\n",
                                    var_name, cmd
                                ));
                            }
                            // Add to string-based list only if not already present (avoid duplicates)
                            script.push_str(&format!(
                                "if ! echo \" $CAPTURED_VAR_NAMES \" | grep -q \" {} \"; then\n",
                                var_name
                            ));
                            script.push_str(&format!(
                                "    CAPTURED_VAR_NAMES=\"$CAPTURED_VAR_NAMES {}\"\n",
                                var_name
                            ));
                            script.push_str("fi\n");
                        }
                        script.push('\n');
                    }
                }

                // Convert hydration placeholders in verification expressions
                let converted_result_expr =
                    convert_verification_expr_hydration(&step.verification.result);
                let converted_output_expr =
                    if let Some(ref output_file_verification) = step.verification.output_file {
                        convert_verification_expr_hydration(output_file_verification)
                    } else {
                        convert_verification_expr_hydration(&step.verification.output)
                    };

                // Add comment for verification
                script.push_str("# Verification result\n");
                script.push_str("VERIFICATION_RESULT_PASS=false\n");

                // Perform variable substitution on result verification if needed
                let result_verification_script = if uses_variables {
                    // For variable substitution support, need to extract and substitute in the verification
                    // Generate script that handles variable substitution before evaluation
                    generate_verification_with_var_subst(
                        &converted_result_expr,
                        "VERIFICATION_RESULT_PASS",
                    )
                } else {
                    // No variables, use direct generation
                    Self::generate_verification_script(
                        &converted_result_expr,
                        "VERIFICATION_RESULT_PASS",
                    )
                };
                script.push_str(&result_verification_script);
                script.push('\n');

                // Determine which output verification to use
                script.push_str("# Verification output\n");
                script.push_str("VERIFICATION_OUTPUT_PASS=false\n");

                let output_verification_script = if uses_variables {
                    generate_verification_with_var_subst(
                        &converted_output_expr,
                        "VERIFICATION_OUTPUT_PASS",
                    )
                } else {
                    Self::generate_verification_script(
                        &converted_output_expr,
                        "VERIFICATION_OUTPUT_PASS",
                    )
                };
                script.push_str(&output_verification_script);
                script.push('\n');

                // Generate general verification checks if present
                let mut general_verification_vars = Vec::new();
                if let Some(ref general_verifications) = step.verification.general {
                    if !general_verifications.is_empty() {
                        script.push_str("# General verifications\n");
                        for general_ver in general_verifications.iter() {
                            // Sanitize the name to create a valid bash variable name
                            let sanitized_name = general_ver
                                .name
                                .replace(" ", "_")
                                .replace("-", "_")
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == '_')
                                .collect::<String>();
                            let var_name = format!("GENERAL_VERIFY_PASS_{}", sanitized_name);
                            general_verification_vars.push(var_name.clone());

                            script.push_str(&format!("{}=false\n", var_name));

                            // Generate verification script with variable substitution support if needed
                            let general_verification_script = if uses_variables {
                                generate_verification_with_var_subst(
                                    &VerificationExpression::Simple(general_ver.condition.clone()),
                                    &var_name,
                                )
                            } else {
                                Self::generate_verification_script(
                                    &VerificationExpression::Simple(general_ver.condition.clone()),
                                    &var_name,
                                )
                            };
                            script.push_str(&general_verification_script);
                        }
                        script.push('\n');
                    }
                }

                // Escape command for JSON - handle all control characters
                // Note: Single quotes are converted to double quotes to avoid bash syntax issues
                let escaped_command = converted_command
                    .replace("\\", "\\\\")
                    .replace("'", "\"")
                    .replace("\"", "\\\"")
                    .replace("\n", "\\n")
                    .replace("\r", "\\r")
                    .replace("\t", "\\t")
                    .replace("\x08", "\\b") // backspace
                    .replace("\x0C", "\\f"); // form feed
                script.push_str(&self.generate_json_escaping_code());
                script.push('\n');

                script.push_str("if [ \"$FIRST_ENTRY\" = false ]; then\n");
                script.push_str("    echo ',' >> \"$JSON_LOG\"\n");
                script.push_str("fi\n");
                script.push_str("FIRST_ENTRY=false\n\n");

                script.push_str("# Write JSON entry\n");
                script.push_str("{\n");
                script.push_str("    echo '  {'\n");
                script.push_str(
                    "    echo \"    \\\"test_case_id\\\": \\\"$TEST_CASE_ID_ESCAPED\\\",\"\n",
                );
                script.push_str("    if [ -n \"$RUN_ID\" ]; then echo \"    $RUN_ID_JSON\"; fi\n");
                script.push_str(&format!(
                    "    echo '    \"test_sequence\": {},'\n",
                    sequence.id
                ));
                script.push_str(&format!("    echo '    \"step\": {},'\n", step.step));
                script.push_str(&format!(
                    "    echo '    \"command\": \"{}\",'\n",
                    escaped_command
                ));
                script.push_str("    echo \"    \\\"exit_code\\\": $EXIT_CODE,\"\n");
                script.push_str("    echo \"    \\\"output\\\": \\\"$OUTPUT_ESCAPED\\\",\"\n");
                script.push_str("    echo \"    \\\"timestamp\\\": \\\"$TIMESTAMP\\\",\"\n");
                if source_hash.is_some() {
                    script.push_str("    echo \"    \\\"source_yaml_sha256\\\": \\\"$SOURCE_YAML_SHA256\\\",\"\n");
                }
                script.push_str("    echo \"    \\\"result_verification_pass\\\": $VERIFICATION_RESULT_PASS,\"\n");
                script.push_str("    echo \"    \\\"output_verification_pass\\\": $VERIFICATION_OUTPUT_PASS\"\n");
                script.push_str("    echo '  }'\n");
                script.push_str("} >> \"$JSON_LOG\"\n\n");

                // Build the overall verification condition
                let mut verification_condition = String::from("\"$VERIFICATION_RESULT_PASS\" = true ] && [ \"$VERIFICATION_OUTPUT_PASS\" = true");
                for var_name in &general_verification_vars {
                    verification_condition.push_str(&format!(" ] && [ \"${}\" = true", var_name));
                }

                script.push_str(&format!("if [ {} ]; then\n", verification_condition));
                script.push_str(&format!(
                    "    echo \"[PASS] Step {}: {}\"\n",
                    step.step,
                    echo_desc.replace("\"", "\\\"")
                ));
                script.push_str("else\n");
                script.push_str(&format!(
                    "    echo \"[FAIL] Step {}: {}\"\n",
                    step.step,
                    echo_desc.replace("\"", "\\\"")
                ));
                script.push_str("    echo \"  Command: ");
                script.push_str(&converted_command.replace("\"", "\\\""));
                script.push_str("\"\n");
                script.push_str("    echo \"  Exit code: $EXIT_CODE\"\n");
                script.push_str("    echo \"  Output: $COMMAND_OUTPUT\"\n");
                script.push_str("    echo \"  Result verification: $VERIFICATION_RESULT_PASS\"\n");
                script.push_str("    echo \"  Output verification: $VERIFICATION_OUTPUT_PASS\"\n");

                // Add general verification results to error message
                for var_name in &general_verification_vars {
                    script.push_str(&format!("    echo \"  {}: ${}\"\n", var_name, var_name));
                }

                script.push_str("    exit 1\n");
                script.push_str("fi\n\n");

                // Execute after_step hook for automated step
                if let Some(ref hooks) = test_case.hooks {
                    if let Some(ref hook) = hooks.after_step {
                        // Set step context variables for after_step hook
                        script.push_str("STEP_EXIT_CODE=$EXIT_CODE\n");
                        script.push_str("export STEP_EXIT_CODE COMMAND_OUTPUT\n");
                        script.push_str(&Self::generate_hook_execution(
                            "after_step",
                            hook,
                            yaml_dir,
                        ));
                    }
                }
            }

            // Execute after_sequence hook
            if let Some(ref hooks) = test_case.hooks {
                if let Some(ref hook) = hooks.after_sequence {
                    script.push_str(&Self::generate_hook_execution(
                        "after_sequence",
                        hook,
                        yaml_dir,
                    ));
                }
            }
        }

        script.push_str("echo ']' >> \"$JSON_LOG\"\n\n");

        // Add JSON schema validation
        script.push_str("# Validate JSON against schema\n");
        script.push_str("if command -v jq >/dev/null 2>&1; then\n");
        script.push_str("    if ! jq empty \"$JSON_LOG\" >/dev/null 2>&1; then\n");
        script.push_str(
            "        echo \"500 - Internal Script Error: Generated JSON is not valid\"\n",
        );
        script.push_str("        exit 1\n");
        script.push_str("    fi\n");
        script.push_str("fi\n\n");

        // Execute teardown_test hook after all sequences
        if let Some(ref hooks) = test_case.hooks {
            if let Some(ref hook) = hooks.teardown_test {
                script.push_str(&Self::generate_hook_execution(
                    "teardown_test",
                    hook,
                    yaml_dir,
                ));
            }
        }

        script.push_str("echo \"All test sequences completed successfully\"\n");

        // Execute script_end hook before final exit
        if let Some(ref hooks) = test_case.hooks {
            if let Some(ref hook) = hooks.script_end {
                script.push_str(&Self::generate_hook_execution("script_end", hook, yaml_dir));
            }
        }

        script.push_str("exit 0\n");

        script.replace("\\r", "")
    }

    pub fn generate_test_script(&self, test_case: &TestCase) -> String {
        let json_path_str = format!("{}_execution_log.json", test_case.id);
        let json_path = Path::new(&json_path_str);
        self.generate_test_script_with_json_output(test_case, json_path, None, None)
    }

    pub fn generate_test_script_from_yaml(
        &self,
        test_case: &TestCase,
        yaml_bytes: &[u8],
    ) -> String {
        let json_path_str = format!("{}_execution_log.json", test_case.id);
        let json_path = Path::new(&json_path_str);
        let hash = compute_yaml_sha256(yaml_bytes);
        self.generate_test_script_with_json_output(test_case, json_path, Some(hash), None)
    }

    pub fn generate_test_script_from_yaml_with_path(
        &self,
        test_case: &TestCase,
        yaml_bytes: &[u8],
        yaml_file: &Path,
    ) -> String {
        let json_path_str = format!("{}_execution_log.json", test_case.id);
        let json_path = Path::new(&json_path_str);
        let hash = compute_yaml_sha256(yaml_bytes);
        let yaml_dir = yaml_file.parent();
        self.generate_test_script_with_json_output(test_case, json_path, Some(hash), yaml_dir)
    }

    pub fn execute_test_case(&self, test_case: &TestCase) -> Result<()> {
        let mut execution_entries = Vec::new();
        let mut execution_error = None;
        let mut verification_failed = false;

        // Handle prerequisites
        if let Some(ref prerequisites) = test_case.prerequisites {
            if !prerequisites.is_empty() {
                println!("Checking prerequisites...");

                for (idx, prereq) in prerequisites.iter().enumerate() {
                    match prereq.prerequisite_type {
                        testcase_models::PrerequisiteType::Manual => {
                            // Manual prerequisite: output skip message
                            println!(
                                "[SKIP] Manual prerequisite {}: {}",
                                idx + 1,
                                prereq.description
                            );
                        }
                        testcase_models::PrerequisiteType::Automatic => {
                            // Automatic prerequisite: execute verification_command
                            if let Some(ref verification_cmd) = prereq.verification_command {
                                println!(
                                    "[CHECK] Automatic prerequisite {}: {}",
                                    idx + 1,
                                    prereq.description
                                );

                                match Command::new("bash")
                                    .arg("-c")
                                    .arg(verification_cmd)
                                    .output()
                                {
                                    Ok(output) => {
                                        let exit_code = output.status.code().unwrap_or(-1);
                                        if exit_code != 0 {
                                            let prereq_output =
                                                String::from_utf8_lossy(&output.stdout)
                                                    .trim_end()
                                                    .to_string();
                                            let prereq_stderr =
                                                String::from_utf8_lossy(&output.stderr)
                                                    .trim_end()
                                                    .to_string();
                                            let combined_output = if !prereq_stderr.is_empty() {
                                                format!("{}\n{}", prereq_output, prereq_stderr)
                                            } else {
                                                prereq_output
                                            };

                                            return Err(anyhow::anyhow!(
                                                "Prerequisite {} failed: {}\nVerification command: {}\nExit code: {}\nOutput: {}",
                                                idx + 1,
                                                prereq.description,
                                                verification_cmd,
                                                exit_code,
                                                combined_output
                                            ));
                                        }
                                        println!("[PASS] Prerequisite {} verified", idx + 1);
                                    }
                                    Err(e) => {
                                        return Err(anyhow::anyhow!(
                                            "Failed to execute verification command for prerequisite {}: {}",
                                            idx + 1,
                                            e
                                        ));
                                    }
                                }
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Automatic prerequisite {} has no verification_command",
                                    idx + 1
                                ));
                            }
                        }
                    }
                }

                println!("All prerequisites satisfied\n");
            }
        }

        // Initialize variable storage for captured variables
        let mut step_vars: HashMap<String, String> = HashMap::new();

        'outer: for sequence in &test_case.test_sequences {
            for step in &sequence.steps {
                if step.manual == Some(true) {
                    // Check if manual step has verification fields (not just "true")
                    let has_result_verification = !matches!(&step.verification.result,
                        VerificationExpression::Simple(s) if s.trim() == "true");
                    let has_output_verification = !matches!(&step.verification.output,
                        VerificationExpression::Simple(s) if s.trim() == "true");
                    let has_verification = has_result_verification || has_output_verification;

                    if !has_verification {
                        // No verification - just skip
                        println!(
                            "[SKIP] Step {} (Sequence {}): {} - Manual step",
                            step.step, sequence.id, step.description
                        );
                        continue;
                    }

                    // Manual step with verification - prompt user for action
                    println!(
                        "[MANUAL] Step {} (Sequence {}): {}",
                        step.step, sequence.id, step.description
                    );
                    println!("  Command: {}", step.command);
                    println!(
                        "  INFO: This is a manual step. You must perform this action manually."
                    );

                    // Check if running in non-interactive mode
                    // We detect non-interactive mode via environment variables
                    let is_noninteractive = std::env::var("DEBIAN_FRONTEND")
                        .map(|v| v == "noninteractive")
                        .unwrap_or(false)
                        || std::env::var("CI").is_ok();

                    if is_noninteractive {
                        // Non-interactive mode: skip manual step with verification
                        println!(
                            "[SKIP] Step {} (Sequence {}): Non-interactive mode, skipping manual step",
                            step.step, sequence.id
                        );
                        continue;
                    }

                    // Prompt user to confirm they've completed the action
                    // If prompting fails (e.g., no TTY available), skip the step
                    let prompt = format!(
                        "Have you completed the manual action for Step {}?",
                        step.step
                    );
                    let prompter = DefaultConfirmPrompt;
                    let action_completed = match prompter.confirm(&prompt) {
                        Ok(confirmed) => confirmed,
                        Err(e) => {
                            println!(
                                "[SKIP] Step {} (Sequence {}): Failed to get user confirmation (likely no TTY): {}",
                                step.step, sequence.id, e
                            );
                            continue;
                        }
                    };

                    if !action_completed {
                        println!(
                            "[SKIP] Step {} (Sequence {}): User indicated action not completed",
                            step.step, sequence.id
                        );
                        continue;
                    }

                    // Evaluate verification expressions
                    // For manual steps, we don't have exit_code or command_output from execution
                    // But verification expressions might reference them or use step variables
                    let exit_code = 0; // Default for manual steps
                    let command_output = String::new(); // Empty for manual steps

                    let result_verification_passed = self.evaluate_verification(
                        &step.verification.result,
                        exit_code,
                        &command_output,
                        &step_vars,
                    )?;

                    let output_verification_passed = self.evaluate_verification(
                        &step.verification.output,
                        exit_code,
                        &command_output,
                        &step_vars,
                    )?;

                    // Evaluate general verifications if present
                    let mut general_verifications_passed = true;
                    if let Some(ref general_verifications) = step.verification.general {
                        for general_ver in general_verifications {
                            let general_expr =
                                VerificationExpression::Simple(general_ver.condition.clone());
                            let general_passed = self.evaluate_verification(
                                &general_expr,
                                exit_code,
                                &command_output,
                                &step_vars,
                            )?;

                            if !general_passed {
                                general_verifications_passed = false;
                                println!("  General verification '{}' failed", general_ver.name);
                            }
                        }
                    }

                    // Check if all verifications passed
                    if result_verification_passed
                        && output_verification_passed
                        && general_verifications_passed
                    {
                        println!(
                            "[PASS] Step {} (Sequence {}): {}",
                            step.step, sequence.id, step.description
                        );
                    } else {
                        println!(
                            "[FAIL] Step {} (Sequence {}): {}",
                            step.step, sequence.id, step.description
                        );
                        println!("  Result verification: {}", result_verification_passed);
                        println!("  Output verification: {}", output_verification_passed);
                        println!("  General verifications: {}", general_verifications_passed);
                        verification_failed = true;

                        if execution_error.is_none() {
                            execution_error = Some(anyhow::anyhow!(
                                "Manual step {} verification failed",
                                step.step
                            ));
                        }
                        break 'outer;
                    }

                    continue;
                }

                // Validate capture_vars mutual exclusivity at runtime
                if let Some(ref capture_vars) = step.capture_vars {
                    for (var_name, capture_pattern, command) in capture_vars_to_vec(capture_vars) {
                        if capture_pattern.is_some() && command.is_some() {
                            return Err(anyhow::anyhow!(
                                "Step {} (Sequence {}): Variable '{}' has both capture and command specified. They are mutually exclusive.",
                                step.step, sequence.id, var_name
                            ));
                        }
                        if capture_pattern.is_none() && command.is_none() {
                            return Err(anyhow::anyhow!(
                                "Step {} (Sequence {}): Variable '{}' must have either capture or command specified.",
                                step.step, sequence.id, var_name
                            ));
                        }
                    }
                }

                println!(
                    "[RUN] Step {} (Sequence {}): {}",
                    step.step, sequence.id, step.description
                );

                match Command::new("bash").arg("-c").arg(&step.command).output() {
                    Ok(output) => {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let command_output = String::from_utf8_lossy(&output.stdout)
                            .trim_end()
                            .to_string();

                        // Capture variables from output or command execution
                        if let Some(ref capture_vars) = step.capture_vars {
                            for (var_name, capture_pattern, command) in
                                capture_vars_to_vec(capture_vars)
                            {
                                if let Some(pattern) = capture_pattern {
                                    // Capture from COMMAND_OUTPUT using regex pattern
                                    let re = Regex::new(&pattern).with_context(|| {
                                        format!(
                                            "Invalid regex pattern for variable '{}': {}",
                                            var_name, pattern
                                        )
                                    })?;

                                    if let Some(captures) = re.captures(&command_output) {
                                        if let Some(captured_value) = captures.get(1) {
                                            step_vars.insert(
                                                var_name.clone(),
                                                captured_value.as_str().to_string(),
                                            );
                                        } else if let Some(captured_value) = captures.get(0) {
                                            // If no capture group, use the whole match
                                            step_vars.insert(
                                                var_name.clone(),
                                                captured_value.as_str().to_string(),
                                            );
                                        }
                                    }
                                } else if let Some(cmd) = command {
                                    // Command-based capture: Execute the capture command
                                    match Command::new("bash").arg("-c").arg(&cmd).output() {
                                        Ok(capture_output) => {
                                            let captured_value =
                                                String::from_utf8_lossy(&capture_output.stdout)
                                                    .trim_end()
                                                    .to_string();
                                            step_vars.insert(var_name.clone(), captured_value);
                                        }
                                        Err(e) => {
                                            // If command fails, store empty string
                                            eprintln!("Warning: Failed to execute capture command for variable '{}': {}", var_name, e);
                                            step_vars.insert(var_name.clone(), String::new());
                                        }
                                    }
                                }
                            }
                        }

                        // Perform verification
                        let result_verification_passed = self.evaluate_verification(
                            &step.verification.result,
                            exit_code,
                            &command_output,
                            &step_vars,
                        )?;
                        let output_verification_passed = self.evaluate_verification(
                            &step.verification.output,
                            exit_code,
                            &command_output,
                            &step_vars,
                        )?;

                        // Evaluate general verifications if present
                        let mut general_verifications_passed = true;
                        if let Some(ref general_verifications) = step.verification.general {
                            for general_ver in general_verifications {
                                let general_expr =
                                    VerificationExpression::Simple(general_ver.condition.clone());
                                let general_passed = self.evaluate_verification(
                                    &general_expr,
                                    exit_code,
                                    &command_output,
                                    &step_vars,
                                )?;

                                if !general_passed {
                                    general_verifications_passed = false;
                                    println!(
                                        "  General verification '{}' failed",
                                        general_ver.name
                                    );
                                }
                            }
                        }

                        // Create execution entry with verification results
                        let timestamp = Local::now().to_rfc3339();
                        let entry = TestStepExecutionEntry::with_timestamp(
                            sequence.id,
                            step.step,
                            step.command.clone(),
                            exit_code,
                            command_output.clone(),
                            timestamp,
                            result_verification_passed,
                            output_verification_passed,
                        );
                        execution_entries.push(entry);

                        if result_verification_passed
                            && output_verification_passed
                            && general_verifications_passed
                        {
                            println!(
                                "[PASS] Step {} (Sequence {}): {}",
                                step.step, sequence.id, step.description
                            );
                        } else {
                            println!(
                                "[FAIL] Step {} (Sequence {}): {}",
                                step.step, sequence.id, step.description
                            );
                            println!("  Command: {}", step.command);
                            println!("  EXIT_CODE: {}", exit_code);
                            println!("  COMMAND_OUTPUT: {}", command_output);
                            println!("  Result verification: {}", result_verification_passed);
                            println!("  Output verification: {}", output_verification_passed);
                            println!("  General verifications: {}", general_verifications_passed);
                            verification_failed = true;

                            if execution_error.is_none() {
                                execution_error = Some(anyhow::anyhow!(
                                    "Test execution failed: Step {} verification failed",
                                    step.step
                                ));
                            }
                            break 'outer;
                        }
                    }
                    Err(e) => {
                        // Store the error but continue to write the log
                        // Create an entry with exit code -1 to indicate execution failure
                        let timestamp = Local::now().to_rfc3339();
                        let entry = TestStepExecutionEntry::with_timestamp(
                            sequence.id,
                            step.step,
                            step.command.clone(),
                            -1,
                            format!("Failed to execute: {}", e),
                            timestamp,
                            false,
                            false,
                        );
                        execution_entries.push(entry);

                        if execution_error.is_none() {
                            execution_error = Some(anyhow::anyhow!(
                                "Failed to execute command '{}': {}",
                                step.command,
                                e
                            ));
                        }
                        break 'outer;
                    }
                }
            }
        }

        // Always write the execution log, even if there were errors
        self.write_execution_log(test_case, &execution_entries)?;

        // Now return any execution error after the log is written
        if let Some(err) = execution_error {
            return Err(err);
        }

        if !verification_failed {
            println!("All test sequences completed successfully");
        }

        Ok(())
    }

    fn evaluate_verification(
        &self,
        expression: &VerificationExpression,
        exit_code: i32,
        command_output: &str,
        step_vars: &HashMap<String, String>,
    ) -> Result<bool> {
        let bash_expr: bash_eval::BashExpression = expression.into();
        let context = bash_eval::BashEvalContext {
            exit_code,
            command_output: command_output.to_string(),
            variables: step_vars.clone(),
        };
        bash_eval::evaluate(&bash_expr, &context)
    }

    fn write_execution_log(
        &self,
        test_case: &TestCase,
        entries: &[TestStepExecutionEntry],
    ) -> Result<()> {
        let log_filename = format!("{}_execution_log.json", test_case.id);
        let log_path = if let Some(ref output_dir) = self.output_dir {
            output_dir.join(&log_filename)
        } else {
            PathBuf::from(&log_filename)
        };

        let json_content = serde_json::to_string_pretty(entries)
            .context("Failed to serialize execution entries to JSON")?;

        fs::write(&log_path, json_content)
            .context(format!("Failed to write log file: {}", log_path.display()))?;

        Ok(())
    }

    pub fn generate_execution_log_template(
        &self,
        test_case: &TestCase,
        script_path: &Path,
    ) -> Result<()> {
        // Determine the output path for the JSON log
        let json_log_path = if let Some(parent) = script_path.parent() {
            let stem = script_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&test_case.id);
            parent.join(format!("{}_execution_log.json", stem))
        } else {
            Path::new(&format!("{}_execution_log.json", test_case.id)).to_path_buf()
        };

        // Create execution entries template with all steps from the test case
        let mut template_entries: Vec<TestStepExecutionEntry> = Vec::new();

        // Base timestamp for template (arbitrary starting point in local timezone)
        let base_time = chrono::DateTime::parse_from_rfc3339("2026-01-22T10:30:00Z")
            .expect("Failed to parse hardcoded RFC3339 timestamp")
            .with_timezone(&Local);
        let mut step_index = 0;

        for sequence in &test_case.test_sequences {
            for step in &sequence.steps {
                // Skip manual steps
                if step.manual == Some(true) {
                    continue;
                }

                // Parse expected exit code from expected.result field
                let exit_code = step.expected.result.parse::<i32>().unwrap_or(0);

                // Use expected output, handling special cases
                let output = if step.expected.output == "true" || step.expected.output.is_empty() {
                    String::new()
                } else {
                    // Replace escaped newlines with actual newlines
                    step.expected.output.replace("\\n", "\n")
                };

                // Generate timestamp incrementing by 1 second per step
                let timestamp = base_time + chrono::Duration::seconds(step_index);

                // Create a template entry with expected values
                let entry = TestStepExecutionEntry {
                    doc_type: None,
                    schema: None,
                    test_sequence: sequence.id,
                    step: step.step,
                    command: step.command.clone(),
                    exit_code,
                    output,
                    timestamp: Some(timestamp.to_rfc3339()),
                    hook_type: None,
                    hook_path: None,
                    result_verification_pass: false,
                    output_verification_pass: false,
                    run_id: None,
                    test_case_id: None,
                };

                template_entries.push(entry);
                step_index += 1;
            }
        }

        // Serialize to JSON
        let json_content = serde_json::to_string_pretty(&template_entries)
            .context("Failed to serialize execution log template to JSON")?;

        // Write to file
        fs::write(&json_log_path, json_content).context(format!(
            "Failed to write execution log template: {}",
            json_log_path.display()
        ))?;

        println!(
            "Execution log template generated: {}",
            json_log_path.display()
        );

        Ok(())
    }
}

impl Default for TestExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a PCRE (Perl-compatible) regex pattern to a BSD-compatible sed pattern.
///
/// This function handles common PCRE constructs and converts them to sed patterns that work
/// on both BSD (macOS) and GNU (Linux) systems. The primary use case is converting patterns
/// used with `grep -oP` to patterns usable with `sed -n`.
///
/// # Conversions:
///
/// - `\K` (keep text before) is converted to a capturing group
/// - Lookbehind `(?<=...)` is converted to a match-and-capture pattern
/// - Lookahead patterns are simplified where possible
/// - PCRE character classes like `\d`, `\w` are converted to POSIX equivalents
///
/// # Arguments
///
/// * `pattern` - The PCRE regex pattern to convert
///
/// # Returns
///
/// A sed substitution pattern string (e.g., "s/regex/\\1/p")
///
/// # Examples
///
/// ```
/// use testcase_execution::executor::convert_pcre_to_sed_pattern;
///
/// // Convert \K pattern
/// let result = convert_pcre_to_sed_pattern("SESSION_ID=\\K\\d+");
/// assert_eq!(result, "s/.*SESSION_ID=\\([0-9][0-9]*\\).*/\\1/p");
///
/// // Convert lookbehind
/// let result = convert_pcre_to_sed_pattern("(?<=token=)[A-Z0-9]+");
/// assert_eq!(result, "s/.*token=\\([A-Z0-9][A-Z0-9]*\\).*/\\1/p");
/// ```
pub fn convert_pcre_to_sed_pattern(pattern: &str) -> String {
    // Handle common JSON/quoted string patterns: "field":"([^"]+)" -> capture quoted value
    if pattern.contains('"') && pattern.contains(r"\(") && pattern.contains("[^") {
        // Convert [^"]+ to [^"]* for basic sed (one-or-more to zero-or-more)
        let converted = pattern
            .replace(r#"[^"]+"#, "[^\"]*")
            .replace(r"[^']+", "[^']*")
            .replace(r"[^,]+", "[^,]*")
            .replace(r"[^}]+", "[^}]*")
            .replace(r"[^;]+", "[^;]*");
        return format!("s/.*{}.*/\\1/p", converted);
    }

    // Handle \K pattern (keep everything before, match after)
    if let Some(idx) = pattern.find(r"\K") {
        let prefix = &pattern[..idx];
        let suffix = &pattern[idx + 2..];

        // Convert PCRE parentheses to sed parentheses FIRST (before character class conversions)
        // This prevents double-escaping when parentheses are part of the replaced patterns
        let paren_converted_suffix = suffix.replace("(", r"\(").replace(")", r"\)");

        // Then convert PCRE character classes to POSIX for sed
        let converted_suffix = paren_converted_suffix
            .replace(r"\d+", r"\([0-9][0-9]*\)")
            .replace(r"\d", r"\([0-9]\)")
            .replace(r"\w+", r"\([a-zA-Z0-9_][a-zA-Z0-9_]*\)")
            .replace(r"\w", r"\([a-zA-Z0-9_]\)")
            .replace(r"\s+", r"\([[:space:]][[:space:]]*\)")
            .replace(r"\s", r"\([[:space:]]\)");

        // If suffix doesn't already have capture group, wrap it
        let capture_suffix = if converted_suffix.starts_with(r"\(") {
            converted_suffix
        } else {
            format!(
                r"\({}\)",
                converted_suffix.replace("+", r"[^[:space:]][^[:space:]]*")
            )
        };

        return format!("s/.*{}{}.*/\\1/p", prefix, capture_suffix);
    }

    // Handle positive lookbehind (?<=...)
    if pattern.starts_with("(?<=") {
        if let Some(end_idx) = pattern.find(')') {
            let lookbehind = &pattern[4..end_idx];
            let rest = &pattern[end_idx + 1..];

            // Convert PCRE parentheses to sed parentheses FIRST (before character class conversions)
            let paren_converted_rest = rest.replace("(", r"\(").replace(")", r"\)");

            // Then convert character classes
            let converted_rest = paren_converted_rest
                .replace(r"\d+", r"\([0-9][0-9]*\)")
                .replace(r"\d", r"\([0-9]\)")
                .replace(r"\w+", r"\([a-zA-Z0-9_][a-zA-Z0-9_]*\)")
                .replace(r"\w", r"\([a-zA-Z0-9_]\)")
                .replace("[a-f0-9]+", r"\([a-f0-9][a-f0-9]*\)")
                .replace("[A-Z0-9]+", r"\([A-Z0-9][A-Z0-9]*\)")
                .replace("[0-9]+", r"\([0-9][0-9]*\)");

            // Wrap in capture group if not already
            let capture_rest = if converted_rest.starts_with(r"\(") {
                converted_rest
            } else {
                format!(r"\({}\)", converted_rest)
            };

            return format!("s/.*{}{}.*/\\1/p", lookbehind, capture_rest);
        }
    }

    // Handle IP address pattern: \d+\.\d+\.\d+\.\d+
    if pattern.contains(r"\d+\.\d+\.\d+\.\d+") {
        return "s/[^0-9]*\\([0-9][0-9]*\\.[0-9][0-9]*\\.[0-9][0-9]*\\.[0-9][0-9]*\\).*/\\1/p"
            .to_string();
    }

    // Handle port pattern: :(\d+)
    if pattern == r":(\d+)" || pattern == r":\(\d+\)" {
        return "s/.*:\\([0-9][0-9]*\\).*/\\1/p".to_string();
    }

    // General case: convert PCRE classes and wrap in capture group
    let mut converted = pattern
        .replace(r"\d+", r"[0-9][0-9]*")
        .replace(r"\d", r"[0-9]")
        .replace(r"\w+", r"[a-zA-Z0-9_][a-zA-Z0-9_]*")
        .replace(r"\w", r"[a-zA-Z0-9_]")
        .replace(r"\s+", r"[[:space:]][[:space:]]*")
        .replace(r"\s", r"[[:space:]]");

    // Handle character class with + quantifier: [^x]+ -> [^x]*
    // This is necessary because basic sed doesn't support + quantifier
    let char_class_plus_re = Regex::new(r"\[\^([^\]]+)\]\+").unwrap();
    converted = char_class_plus_re
        .replace_all(&converted, |caps: &regex::Captures| {
            format!("[^{}]*", &caps[1])
        })
        .to_string();

    // Also handle positive character classes with +: [a-zA-Z]+ -> [a-zA-Z]*
    let char_class_plus_re2 = Regex::new(r"\[([^\[\]]+)\]\+").unwrap();
    converted = char_class_plus_re2
        .replace_all(&converted, |caps: &regex::Captures| {
            format!("[{}]*", &caps[1])
        })
        .to_string();

    // Handle {n,m} quantifiers: [0-9]{4} -> [0-9][0-9][0-9][0-9]
    // This converts exact repetition {n} to n copies of the preceding element
    let brace_quant_re = Regex::new(r"([^\[\]]*)\{(\d+)(?:,(\d+))?\}").unwrap();
    let mut prev_pos = 0;
    let mut result_str = String::new();
    for caps in brace_quant_re.captures_iter(&converted) {
        let match_start = caps.get(0).unwrap().start();
        let match_end = caps.get(0).unwrap().end();

        // Add the part before this match
        result_str.push_str(&converted[prev_pos..match_start]);

        // Get the element to repeat and the count
        let element = &caps[1];
        if let Ok(count) = caps[2].parse::<usize>() {
            // Repeat the element count times
            for _ in 0..count {
                result_str.push_str(element);
            }
        }

        prev_pos = match_end;
    }
    // Add remaining part
    result_str.push_str(&converted[prev_pos..]);
    converted = result_str;

    // Convert PCRE parentheses to sed parentheses
    converted = converted.replace("(", r"\(").replace(")", r"\)");

    // Check if pattern already has capture group
    if converted.contains(r"\(") {
        // Pattern already has a capture group, use it directly
        format!("s/.*{}.*/\\1/p", converted)
    } else {
        // No capture group in pattern, wrap entire pattern in one
        format!("s/.*\\({}\\).*/\\1/p", converted)
    }
}

/// Performs variable substitution in a command string using the STEP_VARS associative array.
///
/// This function substitutes patterns like `${STEP_VARS[var_name]}` or `$var_name` with their
/// corresponding values from the provided variables map. The substituted values are properly
/// escaped for safe use in bash commands.
///
/// # Arguments
///
/// * `command` - The command string containing variable references to substitute
/// * `step_vars` - A reference to a HashMap containing variable names and their values
///
/// # Returns
///
/// A new String with all variable references substituted and properly escaped
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use testcase_execution::executor::substitute_variables;
///
/// let mut vars = HashMap::new();
/// vars.insert("username".to_string(), "john.doe".to_string());
/// vars.insert("path".to_string(), "/home/user".to_string());
///
/// let cmd = "echo ${STEP_VARS[username]} lives at $path";
/// let result = substitute_variables(cmd, &vars);
/// assert_eq!(result, "echo 'john.doe' lives at '/home/user'");
/// ```
pub fn substitute_variables(command: &str, step_vars: &HashMap<String, String>) -> String {
    let mut result = command.to_string();

    // Pattern for ${STEP_VARS[var_name]} - using lazy matching and capturing the var name
    let array_pattern = Regex::new(r"\$\{STEP_VARS\[([^\]]+)\]\}").unwrap();

    // Pattern for $var_name (word characters, digits, underscores)
    let simple_pattern = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

    // First, substitute ${STEP_VARS[var_name]} patterns
    result = array_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if let Some(value) = step_vars.get(var_name) {
                bash_escape(value)
            } else {
                // If variable not found, leave the reference unchanged
                caps[0].to_string()
            }
        })
        .to_string();

    // Then, substitute $var_name patterns
    // We need to be careful not to substitute variables that are not in STEP_VARS
    // or are bash special variables
    result = simple_pattern
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if let Some(value) = step_vars.get(var_name) {
                bash_escape(value)
            } else {
                // If variable not found, leave the reference unchanged
                caps[0].to_string()
            }
        })
        .to_string();

    result
}

/// Escapes a string value for safe use in bash commands.
///
/// This function wraps the value in single quotes and escapes any single quotes
/// within the value by replacing them with '\'' (end quote, escaped quote, start quote).
///
/// # Arguments
///
/// * `value` - The string value to escape
///
/// # Returns
///
/// A bash-escaped string wrapped in single quotes
///
/// # Examples
///
/// ```
/// use testcase_execution::executor::bash_escape;
///
/// assert_eq!(bash_escape("hello"), "'hello'");
/// assert_eq!(bash_escape("it's"), "'it'\\''s'");
/// assert_eq!(bash_escape("a\"b"), "'a\"b'");
/// ```
pub fn bash_escape(value: &str) -> String {
    // In bash, the safest way to escape a string is to wrap it in single quotes
    // and escape any single quotes by ending the quote, adding an escaped quote, and starting again
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Helper function to format multi-line comments for bash scripts
///
/// Splits text on newlines, trims each line, filters out empty lines,
/// and prefixes each with the given string (e.g., '# Step 1: ')
///
/// # Examples
///
/// ```
/// use testcase_execution::executor::format_multiline_comment;
///
/// let text = "First line\nSecond line\n\nThird line";
/// let result = format_multiline_comment("# ", text);
/// assert_eq!(result, "# First line\n# Second line\n# Third line");
/// ```
pub fn format_multiline_comment(prefix: &str, text: &str) -> String {
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| format!("{}{}", prefix, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use testcase_models::{Expected, InitialConditions, Step, TestSequence, Verification};

    /// A failing hook must leave an entry that satisfies the execution-log schema and
    /// parses as `TestStepExecutionEntry`, even if its command has quotes, `$`,
    /// backslashes and newlines (inline hooks).
    #[test]
    fn test_hook_failure_log_entry_is_valid_execution_log_entry() {
        let hook = testcase_models::HookConfig {
            command: "echo \"q\" '\\\\' $HOME `true` > /dev/null\n(exit 3)".to_string(),
            on_error: None,
        };
        let hook_code = TestExecutor::generate_hook_execution("before_step", &hook, None);

        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log.json");
        let script = format!(
            "JSON_LOG='{}'\nFIRST_ENTRY=true\nTIMESTAMP=2026-10-28T12:00:00\n\
             TEST_SEQUENCE_ID=2\nTEST_STEP_NUMBER=5\necho '[' > \"$JSON_LOG\"\n(\n{}\n)\nRC=$?\n\
             echo ']' >> \"$JSON_LOG\"\nexit $RC\n",
            log.display(),
            hook_code
        );
        let status = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(3));

        let content = std::fs::read_to_string(&log).unwrap();
        let entries: Vec<testcase_models::TestStepExecutionEntry> =
            serde_json::from_str(&content).unwrap_or_else(|e| panic!("{e}: {content}"));
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(
            (entry.test_sequence, entry.step, entry.exit_code),
            (2, 5, 3)
        );
        assert_eq!(entry.hook_type, Some(testcase_models::HookType::BeforeStep));
        assert!(entry.command.contains("exit 3") && entry.command.contains("$HOME"));
        assert!(!entry.output.is_empty());
        assert!(!entry.result_verification_pass && !entry.output_verification_pass);
    }

    /// Run a generated script for a one-step test case and return the parsed log entries
    fn run_script_log(
        executor: &TestExecutor,
        test_case_id: &str,
        run_id_env: Option<&str>,
    ) -> Vec<testcase_models::TestStepExecutionEntry> {
        let mut test_case = TestCase::new(
            "REQ".to_string(),
            1,
            1,
            test_case_id.to_string(),
            "run ids".to_string(),
        );
        let mut sequence = TestSequence::new(1, "Seq".to_string(), "d".to_string());
        sequence.steps.push(Step {
            step: 1,
            manual: None,
            description: "echo".to_string(),
            command: "echo hello".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "hello".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        });
        test_case.test_sequences.push(sequence);

        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log.json");
        let script = executor.generate_test_script_with_json_output(&test_case, &log, None, None);
        let script_path = dir.path().join("run.sh");
        std::fs::write(&script_path, script).unwrap();

        let mut command = std::process::Command::new("bash");
        command.arg(&script_path).current_dir(dir.path());
        command.env_remove("TCMS_RUN_ID");
        if let Some(run_id) = run_id_env {
            command.env("TCMS_RUN_ID", run_id);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "script failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let content = std::fs::read_to_string(&log).unwrap();
        serde_json::from_str(&content).unwrap_or_else(|e| panic!("{e}: {content}"))
    }

    #[test]
    fn test_log_entries_carry_test_case_id_and_run_id() {
        // generation default
        let executor = TestExecutor::new().with_run_id(Some("GEN-1".to_string()));
        let entries = run_script_log(&executor, "TC_IDS_001", None);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].test_case_id.as_deref(), Some("TC_IDS_001"));
        assert_eq!(entries[0].run_id.as_deref(), Some("GEN-1"));

        // TCMS_RUN_ID at run time wins
        let entries = run_script_log(&executor, "TC_IDS_001", Some("RUN-42"));
        assert_eq!(entries[0].run_id.as_deref(), Some("RUN-42"));

        // no run ID anywhere: no run_id key, never an empty one
        let plain = TestExecutor::new();
        let entries = run_script_log(&plain, "TC_IDS_001", None);
        assert_eq!(entries[0].test_case_id.as_deref(), Some("TC_IDS_001"));
        assert_eq!(entries[0].run_id, None);
    }

    #[test]
    fn test_ids_with_quotes_and_backslashes_stay_valid_json() {
        use testcase_common::{JsonEscapingConfig, ScriptGenerationConfig};
        let mut methods = vec![JsonEscapingMethod::ShellFallback, JsonEscapingMethod::Auto];
        if std::process::Command::new("jq")
            .arg("--version")
            .output()
            .is_ok()
        {
            methods.push(JsonEscapingMethod::Jq);
        }
        for method in methods {
            let config = Config {
                script_generation: ScriptGenerationConfig {
                    json_escaping: JsonEscapingConfig {
                        method: method.clone(),
                        enabled: true,
                        binary_path: None,
                        jq_path: None,
                    },
                },
                ..Default::default()
            };
            let executor =
                TestExecutor::with_config(config).with_run_id(Some("a\"b\\c".to_string()));
            let entries = run_script_log(&executor, "TC 'odd' \"id\"", None);
            assert_eq!(
                entries[0].test_case_id.as_deref(),
                Some("TC 'odd' \"id\""),
                "{method:?}"
            );
            assert_eq!(entries[0].run_id.as_deref(), Some("a\"b\\c"), "{method:?}");
        }
    }

    #[test]
    fn test_helper_functions_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test helper functions".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify helper functions are present
        assert!(
            script.contains("read_true_false()"),
            "Script should contain read_true_false function"
        );
        assert!(
            script.contains("read_verification()"),
            "Script should contain read_verification function"
        );
        assert!(
            script.contains("# Bash helper functions for user prompts"),
            "Script should contain helper function comment"
        );
        assert!(
            script.contains("# Returns: 0 for yes, 1 for no (standard bash convention)"),
            "Script should contain return value documentation"
        );
        assert!(
            script.contains(
                "# Supports both interactive and non-interactive modes with TTY detection"
            ),
            "Script should contain mode documentation"
        );
        assert!(
            script.contains(
                "if [[ \"${DEBIAN_FRONTEND:-}\" == 'noninteractive' ]] || ! [ -t 0 ]; then"
            ),
            "Script should contain TTY detection"
        );
        assert!(
            script.contains("read -p \"$prompt [Y/n]: \" response"),
            "Script should contain Y/n prompt"
        );
        assert!(
            script.contains("read -p \"$prompt [y/N]: \" response"),
            "Script should contain y/N prompt"
        );
        assert!(
            script.contains("[Yy]|[Yy][Ee][Ss])"),
            "Script should validate yes responses"
        );
        assert!(
            script.contains("[Nn]|[Nn][Oo])"),
            "Script should validate no responses"
        );
    }

    #[test]
    fn test_generate_test_script_basic() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Basic test".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(script.contains("#!/bin/bash"));
        assert!(script.contains("TC001"));
        assert!(script.contains("Basic test"));
        assert!(script.contains("Echo test"));
        assert!(script.contains("echo 'hello'"));
        assert!(script.contains("COMMAND_OUTPUT"));
        assert!(script.contains("EXIT_CODE=$?"));
        assert!(script.contains("[ $EXIT_CODE -eq 0 ]"));
    }

    #[test]
    fn test_generate_test_script_with_manual_step() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with manual step".to_string(),
        );

        let mut sequence =
            TestSequence::new(1, "Seq1".to_string(), "Sequence with manual".to_string());
        let step = Step {
            step: 1,
            manual: Some(true),
            description: "Manual verification".to_string(),
            command: "ssh device".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "connected".to_string(),
                output: "success".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("true".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(script.contains("Manual verification"));
        assert!(script.contains("ssh device"));
        assert!(script.contains("echo \"Step 1: Manual verification\""));
        assert!(script.contains("echo \"Command: ssh device\""));
        assert!(script.contains(
            "echo \"INFO: This is a manual step. You must perform this action manually.\""
        ));
        assert!(script.contains("if read_true_false \"Have you completed the manual step?\""));
        assert!(!script.contains("MANUAL STEP - Skipping"));
    }

    #[test]
    fn test_generate_test_script_with_initial_conditions() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with conditions".to_string(),
        );

        let mut general_conditions = InitialConditions::default();
        general_conditions.devices.insert(
            "Device".to_string(),
            vec![testcase_models::InitialConditionItem::String(
                "Powered on".to_string(),
            )],
        );
        test_case.general_initial_conditions = general_conditions;

        let mut conditions = InitialConditions::default();
        conditions.devices.insert(
            "Connection".to_string(),
            vec![testcase_models::InitialConditionItem::String(
                "Established".to_string(),
            )],
        );
        test_case.initial_conditions = conditions;

        let sequence = TestSequence::new(1, "Seq1".to_string(), "Sequence".to_string());
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(script.contains("General Initial Conditions"));
        assert!(script.contains("Device: Powered on"));
        assert!(script.contains("Initial Conditions"));
        assert!(script.contains("Connection: Established"));
    }

    #[test]
    fn test_log_file_variable_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test for LOG_FILE".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
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
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(
            script.contains("LOG_FILE=\"TC001_sequence-1_step-1.actual.log\""),
            "Script should contain LOG_FILE variable declaration with .actual.log extension"
        );
    }

    #[test]
    fn test_tee_command_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test for tee command".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
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
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(
            script.contains("| tee \"$LOG_FILE\""),
            "Script should contain tee command piping to $LOG_FILE"
        );
    }

    #[test]
    fn test_log_file_and_tee_for_multiple_steps() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC002".to_string(),
            "Test with multiple steps".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());

        let step1 = Step {
            step: 1,
            manual: None,
            description: "First step".to_string(),
            command: "echo 'step1'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "step1".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };

        let step2 = Step {
            step: 2,
            manual: None,
            description: "Second step".to_string(),
            command: "echo 'step2'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "step2".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };

        sequence.steps.push(step1);
        sequence.steps.push(step2);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(
            script.contains("LOG_FILE=\"TC002_sequence-1_step-1.actual.log\""),
            "Script should contain LOG_FILE for step 1"
        );
        assert!(
            script.contains("LOG_FILE=\"TC002_sequence-1_step-2.actual.log\""),
            "Script should contain LOG_FILE for step 2"
        );

        let tee_count = script.matches("| tee \"$LOG_FILE\"").count();
        assert_eq!(
            tee_count, 2,
            "Script should contain tee command twice for two non-manual steps"
        );
    }

    #[test]
    fn test_log_file_and_tee_for_multiple_sequences() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC003".to_string(),
            "Test with multiple sequences".to_string(),
        );

        let mut sequence1 = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step1 = Step {
            step: 1,
            manual: None,
            description: "Step in sequence 1".to_string(),
            command: "echo 'seq1'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "seq1".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence1.steps.push(step1);

        let mut sequence2 = TestSequence::new(2, "Seq2".to_string(), "Second sequence".to_string());
        let step2 = Step {
            step: 1,
            manual: None,
            description: "Step in sequence 2".to_string(),
            command: "echo 'seq2'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "seq2".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence2.steps.push(step2);

        test_case.test_sequences.push(sequence1);
        test_case.test_sequences.push(sequence2);

        let script = executor.generate_test_script(&test_case);

        assert!(
            script.contains("LOG_FILE=\"TC003_sequence-1_step-1.actual.log\""),
            "Script should contain LOG_FILE for sequence 1"
        );
        assert!(
            script.contains("LOG_FILE=\"TC003_sequence-2_step-1.actual.log\""),
            "Script should contain LOG_FILE for sequence 2"
        );

        let tee_count = script.matches("| tee \"$LOG_FILE\"").count();
        assert_eq!(
            tee_count, 2,
            "Script should contain tee command twice for two sequences"
        );
    }

    #[test]
    fn test_manual_steps_skip_log_file_and_tee() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC004".to_string(),
            "Test with manual step".to_string(),
        );

        let mut sequence =
            TestSequence::new(1, "Seq1".to_string(), "Sequence with manual".to_string());

        let manual_step = Step {
            step: 1,
            manual: Some(true),
            description: "Manual step".to_string(),
            command: "manual command".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "output".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("true".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };

        let auto_step = Step {
            step: 2,
            manual: None,
            description: "Automated step".to_string(),
            command: "echo 'auto'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "auto".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };

        sequence.steps.push(manual_step);
        sequence.steps.push(auto_step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        let log_file_count = script.matches("LOG_FILE=").count();
        assert_eq!(
            log_file_count, 1,
            "Script should contain only one LOG_FILE declaration (manual step skipped)"
        );

        assert!(
            script.contains("LOG_FILE=\"TC004_sequence-1_step-2.actual.log\""),
            "Script should contain LOG_FILE only for non-manual step"
        );

        let tee_count = script.matches("| tee \"$LOG_FILE\"").count();
        assert_eq!(
            tee_count, 1,
            "Script should contain only one tee command (manual step skipped)"
        );
    }

    #[test]
    fn test_log_file_pattern_with_regex() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ999".to_string(),
            1,
            1,
            "TC999".to_string(),
            "Test LOG_FILE pattern".to_string(),
        );

        let mut sequence = TestSequence::new(5, "Seq5".to_string(), "Test sequence".to_string());
        let step = Step {
            step: 10,
            manual: None,
            description: "Test step".to_string(),
            command: "ls -la".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "files".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        use regex::Regex;
        let log_file_pattern = Regex::new(r#"LOG_FILE="[^"]*\.actual\.log""#).unwrap();
        assert!(
            log_file_pattern.is_match(&script),
            "Script should match LOG_FILE pattern with .actual.log extension"
        );

        let tee_pattern = Regex::new(r#"\| tee "\$LOG_FILE""#).unwrap();
        assert!(
            tee_pattern.is_match(&script),
            "Script should match tee command pattern with $LOG_FILE variable"
        );
    }

    #[test]
    fn test_stderr_capture_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test stderr capture".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
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
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(
            script.contains("2>&1 | tee"),
            "Script should contain '2>&1 | tee' pattern to capture stderr before tee command"
        );
    }

    #[test]
    fn test_bash_escape_basic() {
        assert_eq!(bash_escape("hello"), "'hello'");
        assert_eq!(bash_escape("hello world"), "'hello world'");
        assert_eq!(bash_escape("123"), "'123'");
    }

    #[test]
    fn test_bash_escape_with_single_quotes() {
        assert_eq!(bash_escape("it's"), "'it'\\''s'");
        assert_eq!(bash_escape("'quoted'"), "''\\''quoted'\\'''");
        assert_eq!(bash_escape("don't"), "'don'\\''t'");
    }

    #[test]
    fn test_bash_escape_with_special_chars() {
        assert_eq!(bash_escape("a\"b"), "'a\"b'");
        assert_eq!(bash_escape("a$b"), "'a$b'");
        assert_eq!(bash_escape("a`b"), "'a`b'");
        assert_eq!(bash_escape("a\\b"), "'a\\b'");
        assert_eq!(bash_escape("a!b"), "'a!b'");
        assert_eq!(bash_escape("a&b"), "'a&b'");
        assert_eq!(bash_escape("a|b"), "'a|b'");
        assert_eq!(bash_escape("a;b"), "'a;b'");
    }

    #[test]
    fn test_bash_escape_empty_string() {
        assert_eq!(bash_escape(""), "''");
    }

    #[test]
    fn test_substitute_variables_array_syntax() {
        let mut vars = HashMap::new();
        vars.insert("username".to_string(), "john.doe".to_string());
        vars.insert("server".to_string(), "example.com".to_string());

        let cmd = "ssh ${STEP_VARS[username]}@${STEP_VARS[server]}";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "ssh 'john.doe'@'example.com'");
    }

    #[test]
    fn test_substitute_variables_simple_syntax() {
        let mut vars = HashMap::new();
        vars.insert("username".to_string(), "jane".to_string());
        vars.insert("path".to_string(), "/home/user".to_string());

        let cmd = "echo $username lives at $path";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo 'jane' lives at '/home/user'");
    }

    #[test]
    fn test_substitute_variables_mixed_syntax() {
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "admin".to_string());
        vars.insert("host".to_string(), "server1".to_string());
        vars.insert("port".to_string(), "8080".to_string());

        let cmd = "curl http://$user:password@${STEP_VARS[host]}:$port/api";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "curl http://'admin':password@'server1':'8080'/api");
    }

    #[test]
    fn test_substitute_variables_undefined_var() {
        let mut vars = HashMap::new();
        vars.insert("defined".to_string(), "value".to_string());

        let cmd = "echo $defined $undefined ${STEP_VARS[missing]}";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo 'value' $undefined ${STEP_VARS[missing]}");
    }

    #[test]
    fn test_substitute_variables_with_special_chars_in_values() {
        let mut vars = HashMap::new();
        vars.insert("msg".to_string(), "hello 'world'!".to_string());
        vars.insert("cmd".to_string(), "ls -la | grep test".to_string());

        let cmd = "echo ${STEP_VARS[msg]} && $cmd";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(
            result,
            "echo 'hello '\\''world'\\''!' && 'ls -la | grep test'"
        );
    }

    #[test]
    fn test_substitute_variables_empty_command() {
        let vars = HashMap::new();
        let cmd = "";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "");
    }

    #[test]
    fn test_substitute_variables_no_vars_in_command() {
        let mut vars = HashMap::new();
        vars.insert("unused".to_string(), "value".to_string());

        let cmd = "echo hello world";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_substitute_variables_multiple_same_var() {
        let mut vars = HashMap::new();
        vars.insert("val".to_string(), "xyz".to_string());

        let cmd = "echo $val $val ${STEP_VARS[val]}";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo 'xyz' 'xyz' 'xyz'");
    }

    #[test]
    fn test_substitute_variables_var_with_numbers_and_underscores() {
        let mut vars = HashMap::new();
        vars.insert("var_name_1".to_string(), "value1".to_string());
        vars.insert("_var2".to_string(), "value2".to_string());
        vars.insert("VAR3".to_string(), "value3".to_string());

        let cmd = "echo $var_name_1 $_var2 $VAR3";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo 'value1' 'value2' 'value3'");
    }

    #[test]
    fn test_substitute_variables_preserves_bash_special_vars() {
        let vars = HashMap::new();

        let cmd = "echo $? $$ $! $# $@ $* $0 $1 $EXIT_CODE $COMMAND_OUTPUT";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(
            result,
            "echo $? $$ $! $# $@ $* $0 $1 $EXIT_CODE $COMMAND_OUTPUT"
        );
    }

    #[test]
    fn test_substitute_variables_complex_scenario() {
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "test_user".to_string());
        vars.insert("file".to_string(), "config.txt".to_string());
        vars.insert("dir".to_string(), "/tmp/test dir".to_string());

        let cmd = "scp $user@server:${STEP_VARS[dir]}/${STEP_VARS[file]} .";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(
            result,
            "scp 'test_user'@server:'/tmp/test dir'/'config.txt' ."
        );
    }

    #[test]
    fn test_substitute_variables_escaped_dollar_signs() {
        let mut vars = HashMap::new();
        vars.insert("price".to_string(), "100".to_string());

        // Note: In actual bash, \$ would prevent expansion.
        // This test shows current behavior - escaped $ is still processed
        let cmd = "echo The price is $price";
        let result = substitute_variables(cmd, &vars);
        assert_eq!(result, "echo The price is '100'");
    }

    #[test]
    fn test_generate_test_script_with_capture_vars() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with variable capture".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());

        let mut capture_vars = std::collections::BTreeMap::new();
        capture_vars.insert("user_id".to_string(), r#"(?<=user_id=)\d+"#.to_string());
        capture_vars.insert(
            "token".to_string(),
            r#"(?<=token=)[a-zA-Z0-9]+"#.to_string(),
        );

        let step = Step {
            step: 1,
            manual: None,
            description: "Extract variables".to_string(),
            command: "echo 'user_id=12345 token=abc123'".to_string(),
            capture_vars: Some(CaptureVarsFormat::Legacy(capture_vars)),
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(script.contains("# Capture variables from output"));
        assert!(script.contains("user_id=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("token=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("| head -n 1 || echo \"\")"));
    }

    #[test]
    fn test_variable_capture_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test variable capture initialization".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());

        let mut capture_vars = std::collections::BTreeMap::new();
        capture_vars.insert(
            "session_id".to_string(),
            r#"(?<=session=)[a-f0-9]+"#.to_string(),
        );

        let step = Step {
            step: 1,
            manual: None,
            description: "Capture session ID".to_string(),
            command: "echo 'session=abc123def'".to_string(),
            capture_vars: Some(CaptureVarsFormat::Legacy(capture_vars)),
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify variable storage initialization (bash 3.2+ compatible - uses string instead of array)
        assert!(script.contains("CAPTURED_VAR_NAMES=\"\""));
        assert!(script.contains("# Initialize variable storage for captured variables"));

        // Verify capture code generation
        assert!(script.contains("# Capture variables from output"));
        assert!(script.contains("session_id=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("| head -n 1 || echo \"\")"));
    }

    #[test]
    fn test_variable_substitution_in_commands() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test variable substitution in commands".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());

        let step = Step {
            step: 1,
            manual: None,
            description: "Use variable in command".to_string(),
            command: "echo ${user_name} works at ${STEP_VARS[company]}".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify substitution logic is present
        assert!(script.contains("ORIGINAL_COMMAND="));
        assert!(script.contains("SUBSTITUTED_COMMAND=\"$ORIGINAL_COMMAND\""));
        assert!(script.contains("for var_name in $CAPTURED_VAR_NAMES; do"));
        assert!(script.contains("eval \"var_value=\\$$var_name\""));
        assert!(script.contains("# Replace ${var_name} pattern"));
        assert!(script.contains("SUBSTITUTED_COMMAND=$(echo \"$SUBSTITUTED_COMMAND\" | sed \"s/\\${$var_name}/$escaped_value/g\")"));
        assert!(script.contains(
            "{ eval \"$SUBSTITUTED_COMMAND\"; } 2>&1 | tee \"$LOG_FILE\" > \"$_CAPTURE_TMPFILE\""
        ));
        assert!(script.contains("COMMAND_OUTPUT=$(cat \"$_CAPTURE_TMPFILE\")"));
    }

    #[test]
    fn test_variable_substitution_in_verification() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test variable substitution in verification".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());

        let step = Step {
            step: 1,
            manual: None,
            description: "Verify with variables".to_string(),
            command: "echo 'status: OK'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq ${expected_code} ]".to_string(),
                output: "[[ \"$COMMAND_OUTPUT\" == *\"${STEP_VARS[status]}\"* ]]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple(
                    "[ $EXIT_CODE -eq ${expected_code} ]".to_string(),
                ),
                output: VerificationExpression::Simple(
                    "[[ \"$COMMAND_OUTPUT\" == *\"${STEP_VARS[status]}\"* ]]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify substitution in result expression
        assert!(script.contains("EXPR="));
        assert!(script.contains("for var_name in $CAPTURED_VAR_NAMES; do"));
        assert!(script.contains("eval \"var_value=\\$$var_name\""));
        assert!(
            script.contains("EXPR=$(echo \"$EXPR\" | sed \"s/\\${$var_name}/$escaped_value/g\")")
        );

        // Verify evaluation of substituted expressions
        assert!(script.contains("if eval \"$EXPR\"; then"));
        assert!(script.contains("VERIFICATION_RESULT_PASS=true"));
        assert!(script.contains("VERIFICATION_OUTPUT_PASS=true"));
    }

    #[test]
    fn test_multiple_variables_from_single_step() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test multiple variable captures from single step".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());

        let mut capture_vars = std::collections::BTreeMap::new();
        capture_vars.insert("user_id".to_string(), r#"(?<=user_id=)\d+"#.to_string());
        capture_vars.insert(
            "session_token".to_string(),
            r#"(?<=token=)[A-Z0-9]+"#.to_string(),
        );
        capture_vars.insert("timestamp".to_string(), r#"(?<=ts=)\d{10}"#.to_string());

        let step = Step {
            step: 1,
            manual: None,
            description: "Capture multiple variables".to_string(),
            command: "echo 'user_id=12345 token=ABC123XYZ ts=1234567890'".to_string(),
            capture_vars: Some(CaptureVarsFormat::Legacy(capture_vars)),
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify all three variables are captured
        assert!(script.contains("# Capture variables from output"));
        assert!(script.contains("user_id=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("session_token=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("timestamp=$(echo \"$COMMAND_OUTPUT\" | sed -n"));

        // Verify the capture block appears exactly once for this step
        let capture_count = script.matches("# Capture variables from output").count();
        assert_eq!(
            capture_count, 1,
            "Should have exactly one capture block for the single step"
        );
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_with_k() {
        // Test \K pattern conversion
        let result = convert_pcre_to_sed_pattern(r"SESSION_ID=\K\d+");
        assert_eq!(result, r"s/.*SESSION_ID=\([0-9][0-9]*\).*/\1/p");
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_with_lookbehind() {
        // Test positive lookbehind conversion
        let result = convert_pcre_to_sed_pattern(r"(?<=token=)[A-Z0-9]+");
        assert_eq!(result, r"s/.*token=\([A-Z0-9][A-Z0-9]*\).*/\1/p");
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_with_word_class() {
        // Test \w+ pattern conversion
        let result = convert_pcre_to_sed_pattern(r"USER=\K\w+");
        assert_eq!(result, r"s/.*USER=\([a-zA-Z0-9_][a-zA-Z0-9_]*\).*/\1/p");
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_ip_address() {
        // Test IP address pattern (uses [^0-9]* instead of .* to avoid greedy matching issues)
        let result = convert_pcre_to_sed_pattern(r"\d+\.\d+\.\d+\.\d+");
        assert_eq!(
            result,
            r"s/[^0-9]*\([0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\).*/\1/p"
        );
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_port() {
        // Test port pattern
        let result = convert_pcre_to_sed_pattern(r":(\d+)");
        assert_eq!(result, r"s/.*:\([0-9][0-9]*\).*/\1/p");
    }

    #[test]
    fn test_convert_pcre_to_sed_pattern_hex_pattern() {
        // Test hex pattern with lookbehind
        let result = convert_pcre_to_sed_pattern(r"(?<=session=)[a-f0-9]+");
        assert_eq!(result, r"s/.*session=\([a-f0-9][a-f0-9]*\).*/\1/p");
    }

    #[test]
    fn test_convert_hydration_placeholder_to_bash() {
        // Test converting ${#VAR_NAME} to ${VAR_NAME}
        assert_eq!(
            convert_hydration_placeholder_to_bash("echo ${#SERVER_HOST}"),
            "echo ${SERVER_HOST}"
        );

        assert_eq!(
            convert_hydration_placeholder_to_bash("ssh user@${#HOST} -p ${#PORT}"),
            "ssh user@${HOST} -p ${PORT}"
        );

        // Multiple variables
        assert_eq!(
            convert_hydration_placeholder_to_bash("${#VAR1} ${#VAR2} ${#VAR3}"),
            "${VAR1} ${VAR2} ${VAR3}"
        );

        // No hydration placeholders - should remain unchanged
        assert_eq!(
            convert_hydration_placeholder_to_bash("echo ${STEP_VAR}"),
            "echo ${STEP_VAR}"
        );

        // Mixed hydration and regular variables
        assert_eq!(
            convert_hydration_placeholder_to_bash("echo ${#SERVER} and ${STEP_VAR}"),
            "echo ${SERVER} and ${STEP_VAR}"
        );
    }

    #[test]
    fn test_test_case_uses_hydration_vars() {
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with hydration vars".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());

        // Test case without hydration vars
        let step1 = Step {
            step: 1,
            manual: None,
            description: "Normal step".to_string(),
            command: "echo hello".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "hello".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step1);
        test_case.test_sequences.push(sequence.clone());

        assert!(!test_case_uses_hydration_vars(&test_case));

        // Add step with hydration var in command
        let mut test_case2 = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC002".to_string(),
            "Test with hydration vars".to_string(),
        );
        let mut sequence2 = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());
        let step2 = Step {
            step: 1,
            manual: None,
            description: "Step with hydration var".to_string(),
            command: "echo ${#SERVER_HOST}".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "example.com".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence2.steps.push(step2);
        test_case2.test_sequences.push(sequence2);

        assert!(test_case_uses_hydration_vars(&test_case2));
    }

    #[test]
    fn test_generate_script_with_hydration_vars() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC_HYDRATION_001".to_string(),
            "Test with hydration vars".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo server host".to_string(),
            command: "echo ${#SERVER_HOST}".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "example.com".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[[ \"$COMMAND_OUTPUT\" == *\"${#EXPECTED_OUTPUT}\"* ]]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Should source the export file
        assert!(script.contains("# Source environment variables for hydration"));
        assert!(script.contains("EXPORT_FILE=\"TC_HYDRATION_001.env\""));
        assert!(script.contains("if [ -f \"$EXPORT_FILE\" ]; then"));
        assert!(script.contains("    source \"$EXPORT_FILE\""));

        // Should convert ${#SERVER_HOST} to ${SERVER_HOST} (appears escaped in ORIGINAL_COMMAND)
        assert!(script.contains("ORIGINAL_COMMAND=\"echo \\${SERVER_HOST}\""));
        assert!(!script.contains("${#SERVER_HOST}"));

        // Should convert verification expression (hydration placeholder replaced)
        // The ${#EXPECTED_OUTPUT} should be converted to ${EXPECTED_OUTPUT}
        assert!(script.contains("${EXPECTED_OUTPUT}") || script.contains("\\${EXPECTED_OUTPUT}"));
        assert!(!script.contains("${#EXPECTED_OUTPUT}"));
    }

    #[test]
    fn test_generate_script_without_hydration_vars() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC_NO_HYDRATION_001".to_string(),
            "Test without hydration vars".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "Test sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo hello".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "0".to_string(),
                output: "hello".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Should NOT source the export file
        assert!(!script.contains("# Source environment variables for hydration"));
        assert!(!script.contains("EXPORT_FILE="));
        assert!(!script.contains("source \"$EXPORT_FILE\""));
    }

    #[test]
    fn test_generate_test_script_with_command_based_captures() {
        use testcase_models::CaptureVar;

        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with command-based capture".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());

        // Create capture_vars with both regex and command-based captures
        let capture_vars = vec![
            CaptureVar {
                name: "token".to_string(),
                capture: Some(r#"(?<=token=)[a-zA-Z0-9]+"#.to_string()),
                command: None,
            },
            CaptureVar {
                name: "file_size".to_string(),
                capture: None,
                command: Some("cat /tmp/output.txt | wc -c".to_string()),
            },
            CaptureVar {
                name: "timestamp".to_string(),
                capture: None,
                command: Some("date +%s".to_string()),
            },
        ];

        let step = Step {
            step: 1,
            manual: None,
            description: "Extract variables with commands".to_string(),
            command: "echo 'token=abc123' > /tmp/output.txt".to_string(),
            capture_vars: Some(CaptureVarsFormat::New(capture_vars)),
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify the script contains capture variables section
        assert!(script.contains("# Capture variables from output"));

        // Verify regex-based capture (token) uses sed
        assert!(script.contains("token=$(echo \"$COMMAND_OUTPUT\" | sed -n"));
        assert!(script.contains("| head -n 1 || echo \"\")"));

        // Verify command-based capture (file_size) executes the command
        assert!(script.contains("file_size=$(cat /tmp/output.txt | wc -c 2>&1 || echo \"\")"));

        // Verify command-based capture (timestamp) executes the command
        assert!(script.contains("timestamp=$(date +%s 2>&1 || echo \"\")"));

        // Verify all variables are added to CAPTURED_VAR_NAMES
        assert!(script.contains("if ! echo \" $CAPTURED_VAR_NAMES \" | grep -q \" token \"; then"));
        assert!(script.contains("CAPTURED_VAR_NAMES=\"$CAPTURED_VAR_NAMES token\""));
        assert!(
            script.contains("if ! echo \" $CAPTURED_VAR_NAMES \" | grep -q \" file_size \"; then")
        );
        assert!(script.contains("CAPTURED_VAR_NAMES=\"$CAPTURED_VAR_NAMES file_size\""));
        assert!(
            script.contains("if ! echo \" $CAPTURED_VAR_NAMES \" | grep -q \" timestamp \"; then")
        );
        assert!(script.contains("CAPTURED_VAR_NAMES=\"$CAPTURED_VAR_NAMES timestamp\""));
    }

    #[test]
    fn test_generate_test_script_with_general_verification() {
        use testcase_models::GeneralVerification;

        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with general verification".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: Some(vec![
                    GeneralVerification {
                        name: "check file exists".to_string(),
                        condition: "test -f /tmp/test.txt".to_string(),
                    },
                    GeneralVerification {
                        name: "verify-output".to_string(),
                        condition: "[ -n \"$COMMAND_OUTPUT\" ]".to_string(),
                    },
                ]),
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Verify general verification section is present
        assert!(script.contains("# General verifications"));

        // Verify variable declarations for general verifications
        assert!(script.contains("GENERAL_VERIFY_PASS_check_file_exists=false"));
        assert!(script.contains("GENERAL_VERIFY_PASS_verify_output=false"));

        // Verify condition checks are present
        assert!(script.contains("test -f /tmp/test.txt"));
        assert!(script.contains("[ -n \"$COMMAND_OUTPUT\" ]"));

        // Verify if statements set the variables
        assert!(script.contains("GENERAL_VERIFY_PASS_check_file_exists=true"));
        assert!(script.contains("GENERAL_VERIFY_PASS_verify_output=true"));

        // Verify the overall verification condition includes general verifications
        assert!(script.contains("\"$VERIFICATION_RESULT_PASS\" = true ] && [ \"$VERIFICATION_OUTPUT_PASS\" = true ] && [ \"$GENERAL_VERIFY_PASS_check_file_exists\" = true ] && [ \"$GENERAL_VERIFY_PASS_verify_output\" = true ]"));

        // Verify failure message includes general verification results
        assert!(script.contains("echo \"  GENERAL_VERIFY_PASS_check_file_exists: $GENERAL_VERIFY_PASS_check_file_exists\""));
        assert!(script.contains(
            "echo \"  GENERAL_VERIFY_PASS_verify_output: $GENERAL_VERIFY_PASS_verify_output\""
        ));
    }

    #[test]
    fn test_compute_yaml_sha256() {
        let yaml_content = b"id: TC001\ndescription: Test case\n";
        let hash = compute_yaml_sha256(yaml_content);

        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        let hash2 = compute_yaml_sha256(yaml_content);
        assert_eq!(hash, hash2);

        let different_content = b"id: TC002\ndescription: Different test\n";
        let hash3 = compute_yaml_sha256(different_content);
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_generate_test_script_from_yaml_includes_hash() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test with hash".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let yaml_content = b"id: TC001\ndescription: Test case\n";
        let expected_hash = compute_yaml_sha256(yaml_content);

        let script = executor.generate_test_script_from_yaml(&test_case, yaml_content);

        assert!(script.contains(&format!("# Source YAML SHA-256: {}", expected_hash)));
        assert!(script.contains(&format!("SOURCE_YAML_SHA256=\"{}\"", expected_hash)));
    }

    #[test]
    fn test_generate_test_script_without_hash() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test without hash".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(!script.contains("# Source YAML SHA-256:"));
        assert!(!script.contains("SOURCE_YAML_SHA256="));
    }

    #[test]
    fn test_hash_in_json_log_entries() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test JSON log hash".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'test'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let yaml_content = b"id: TC001\ndescription: Test case\n";
        let script = executor.generate_test_script_from_yaml(&test_case, yaml_content);

        assert!(script
            .contains("echo \"    \\\"source_yaml_sha256\\\": \\\"$SOURCE_YAML_SHA256\\\",\""));
    }

    #[test]
    fn test_hash_not_in_json_log_when_not_provided() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test no hash in JSON".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'test'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        assert!(!script.contains("source_yaml_sha256"));
    }

    #[test]
    fn test_json_escaping_jq_method() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Jq,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("# Escape output for JSON (BSD/GNU compatible)"));
        assert!(escaping_code.contains("printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs ."));
        assert!(escaping_code.contains("sed 's/^\"//;s/\"$//'"));
        assert!(escaping_code.contains("OUTPUT_ESCAPED="));
        assert!(!escaping_code.contains("if command -v"));
        assert!(!escaping_code.contains("json-escape"));
    }

    #[test]
    fn test_json_escaping_jq_method_custom_path() {
        use std::path::PathBuf;
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Jq,
                    enabled: true,
                    binary_path: None,
                    jq_path: Some(PathBuf::from("/usr/local/bin/jq")),
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("printf '%s' \"$COMMAND_OUTPUT\" | /usr/local/bin/jq -Rs ."));
    }

    #[test]
    fn test_json_escaping_rust_binary_method() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::RustBinary,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Config::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("# Escape output for JSON (BSD/GNU compatible)"));
        assert!(escaping_code.contains("printf '%s' \"$COMMAND_OUTPUT\" | json-escape"));
        assert!(escaping_code.contains("OUTPUT_ESCAPED="));
        assert!(!escaping_code.contains("if command -v"));
        assert!(!escaping_code.contains("jq -Rs"));
    }

    #[test]
    fn test_json_escaping_rust_binary_method_custom_path() {
        use std::path::PathBuf;
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::RustBinary,
                    enabled: true,
                    binary_path: Some(PathBuf::from("/opt/bin/json-escape")),
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("printf '%s' \"$COMMAND_OUTPUT\" | /opt/bin/json-escape"));
    }

    #[test]
    fn test_json_escaping_shell_fallback_method() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::ShellFallback,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("# Escape output for JSON (BSD/GNU compatible)"));
        assert!(escaping_code.contains(
            "# Shell fallback: escape backslashes, quotes, tabs, and convert newlines to \\n"
        ));
        assert!(escaping_code.contains(
            "printf '%s' \"$COMMAND_OUTPUT\" | sed 's/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\t/\\\\t/g'"
        ));
        assert!(escaping_code.contains("awk '{printf \"%s%s\", (NR>1?\"\\\\n\":\"\"), $0}'"));
        assert!(escaping_code.contains("OUTPUT_ESCAPED="));
        assert!(!escaping_code.contains("if command -v"));
        assert!(!escaping_code.contains("jq -Rs"));
        assert!(!escaping_code.contains("json-escape"));
    }

    #[test]
    fn test_json_escaping_auto_method() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Auto,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("# Escape output for JSON (BSD/GNU compatible)"));

        // Check for json-escape fallback chain (now comes first)
        assert!(escaping_code.contains("if command -v json-escape >/dev/null 2>&1; then"));
        assert!(escaping_code
            .contains("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | json-escape"));

        // Check for jq fallback
        assert!(escaping_code.contains("elif command -v jq >/dev/null 2>&1; then"));
        assert!(
            escaping_code.contains("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs .")
        );

        // Check for shell fallback
        assert!(escaping_code.contains("else"));
        assert!(escaping_code.contains(
            "# Shell fallback: escape backslashes, quotes, tabs, and convert newlines to \\n"
        ));
        assert!(escaping_code.contains("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | sed 's/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\t/\\\\t/g'"));
        assert!(escaping_code.contains("fi"));
    }

    #[test]
    fn test_json_escaping_auto_method_custom_paths() {
        use std::path::PathBuf;
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Auto,
                    enabled: true,
                    binary_path: Some(PathBuf::from("/custom/json-escape")),
                    jq_path: Some(PathBuf::from("/custom/jq")),
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let escaping_code = executor.generate_json_escaping_code();

        assert!(escaping_code.contains("if command -v /custom/json-escape >/dev/null 2>&1; then"));
        assert!(escaping_code
            .contains("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | /custom/json-escape"));
        assert!(escaping_code.contains("elif command -v /custom/jq >/dev/null 2>&1; then"));
        assert!(escaping_code
            .contains("OUTPUT_ESCAPED=$(printf '%s' \"$COMMAND_OUTPUT\" | /custom/jq -Rs ."));
    }

    #[test]
    fn test_cleanup_trap_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test cleanup trap".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Check cleanup function is defined
        assert!(script.contains("cleanup() {"));
        assert!(script.contains("# Clean up temp capture file if it exists"));
        assert!(script
            .contains("if [ -n \"${_CAPTURE_TMPFILE:-}\" ] && [ -f \"$_CAPTURE_TMPFILE\" ]; then"));
        assert!(script.contains("rm -f \"$_CAPTURE_TMPFILE\""));

        // Check trap is set
        assert!(script.contains("trap cleanup EXIT"));
    }

    #[test]
    fn test_temp_file_creation_in_generated_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test temp file creation".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Check temp file is created with mktemp
        assert!(script.contains("_CAPTURE_TMPFILE=$(mktemp)"));

        // Check command output is redirected to temp file
        assert!(script.contains("> \"$_CAPTURE_TMPFILE\""));

        // Check temp file content is read back
        assert!(script.contains("COMMAND_OUTPUT=$(cat \"$_CAPTURE_TMPFILE\")"));
    }

    #[test]
    fn test_temp_file_used_in_json_escaping() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Jq,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Config::default()
        };

        let executor = TestExecutor::with_config(config);
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test temp file in JSON escaping".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Check that JSON escaping code uses printf with COMMAND_OUTPUT
        assert!(script.contains("printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs ."));
    }

    #[test]
    fn test_edge_case_empty_output_handling() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test empty output".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Command with no output".to_string(),
            command: "true".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ -z \"$COMMAND_OUTPUT\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("[ -z \"$COMMAND_OUTPUT\" ]".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Verify JSON escaping code has fallback for empty output
        assert!(script.contains("|| echo \"\""));

        // Verify the script can handle empty COMMAND_OUTPUT
        assert!(script.contains("[ -z \"$COMMAND_OUTPUT\" ]"));
    }

    #[test]
    fn test_edge_case_binary_output_handling() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Jq,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Config::default()
        };

        let executor = TestExecutor::with_config(config);
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test binary output".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Command producing binary output".to_string(),
            command: "dd if=/dev/urandom bs=1 count=10".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "true".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple("true".to_string()),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Verify jq handles binary data with -Rs (raw input, slurp) and uses printf with COMMAND_OUTPUT
        assert!(script.contains("printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs ."));

        // Verify error handling with 2>/dev/null and fallback (jq uses piped sed before the || echo)
        assert!(script.contains("2>/dev/null"));
        assert!(script.contains("|| echo \"\""));
    }

    #[test]
    fn test_cleanup_trap_closes_json_file() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Test JSON file closure".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Check that cleanup function closes JSON file properly
        assert!(script.contains("cleanup() {"));
        assert!(script.contains("if [ -f \"$JSON_LOG\" ]; then"));

        // Verify cleanup checks last character
        assert!(script.contains("LAST_CHAR=$(tail -c 2 \"$JSON_LOG\" | head -c 1)"));

        // Verify cleanup adds closing bracket when needed
        assert!(script.contains("echo ']' >> \"$JSON_LOG\""));
    }

    #[test]
    fn test_json_escaping_in_full_script_with_jq() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Jq,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Full script test".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Verify all components are present
        assert!(script.contains("_CAPTURE_TMPFILE=$(mktemp)"));
        assert!(script.contains("printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs ."));
        assert!(script.contains("cleanup() {"));
        assert!(script.contains("rm -f \"$_CAPTURE_TMPFILE\""));
        assert!(script.contains("trap cleanup EXIT"));
    }

    #[test]
    fn test_json_escaping_in_full_script_with_auto() {
        use testcase_common::{
            Config, JsonEscapingConfig, JsonEscapingMethod, ScriptGenerationConfig,
        };

        let config = Config {
            script_generation: ScriptGenerationConfig {
                json_escaping: JsonEscapingConfig {
                    method: JsonEscapingMethod::Auto,
                    enabled: true,
                    binary_path: None,
                    jq_path: None,
                },
            },
            ..Default::default()
        };

        let executor = TestExecutor::with_config(config);
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            1,
            "TC001".to_string(),
            "Full script test with auto".to_string(),
        );

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Echo test".to_string(),
            command: "echo 'hello'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[ $EXIT_CODE -eq 0 ]".to_string(),
                output: "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[ $EXIT_CODE -eq 0 ]".to_string()),
                output: VerificationExpression::Simple(
                    "[ \"$COMMAND_OUTPUT\" = \"hello\" ]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let json_output_path = PathBuf::from("/tmp/test.json");
        let script = executor.generate_test_script_with_json_output(
            &test_case,
            &json_output_path,
            None,
            None,
        );

        // Verify all fallback levels are present (json-escape first, then jq)
        assert!(script.contains("if command -v json-escape >/dev/null 2>&1; then"));
        assert!(script.contains("printf '%s' \"$COMMAND_OUTPUT\" | json-escape"));
        assert!(script.contains("elif command -v jq >/dev/null 2>&1; then"));
        assert!(script.contains("printf '%s' \"$COMMAND_OUTPUT\" | jq -Rs ."));
        assert!(script.contains("else"));
        assert!(script.contains(
            "printf '%s' \"$COMMAND_OUTPUT\" | sed 's/\\\\/\\\\\\\\/g; s/\"/\\\\\"/g; s/\\t/\\\\t/g'"
        ));

        // Verify temp file and cleanup
        assert!(script.contains("_CAPTURE_TMPFILE=$(mktemp)"));
        assert!(script.contains("cleanup() {"));
        assert!(script.contains("rm -f \"$_CAPTURE_TMPFILE\""));
        assert!(script.contains("trap cleanup EXIT"));
    }

    #[test]
    fn test_format_as_comment_single_line() {
        let result = format_as_comment("Description: ", "Single line description");
        assert_eq!(result, "# Description: Single line description\n");
    }

    #[test]
    fn test_format_as_comment_multi_line() {
        let text =
            "First line of description\nSecond line of description\nThird line of description";
        let result = format_as_comment("Description: ", text);
        let expected = "# Description: First line of description\n# Second line of description\n# Third line of description\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_format_as_comment_multi_line_in_script() {
        let executor = TestExecutor::new();
        let mut test_case = TestCase::new(
            "REQ_DESC".to_string(),
            1,
            1,
            "TC_DESC_001".to_string(),
            "Multi-line test description".to_string(),
        );
        test_case.description = "This is the first line of the description.\nThis is the second line of the description.\nThis is the third line of the description.".to_string();

        let mut sequence = TestSequence::new(1, "Seq1".to_string(), "First sequence".to_string());
        let step = Step {
            step: 1,
            manual: None,
            description: "Simple echo step".to_string(),
            command: "echo 'test'".to_string(),
            capture_vars: None,
            expected: Expected {
                success: Some(true),
                result: "[[ $EXIT_CODE -eq 0 ]]".to_string(),
                output: "[[ \"$COMMAND_OUTPUT\" == *test* ]]".to_string(),
            },
            verification: Verification {
                result: VerificationExpression::Simple("[[ $EXIT_CODE -eq 0 ]]".to_string()),
                output: VerificationExpression::Simple(
                    "[[ \"$COMMAND_OUTPUT\" == *test* ]]".to_string(),
                ),
                output_file: None,
                general: None,
            },
            reference: None,
        };
        sequence.steps.push(step);
        test_case.test_sequences.push(sequence);

        let script = executor.generate_test_script(&test_case);

        // Check that all lines of the description are properly commented
        assert!(script.contains("# Description: This is the first line of the description."));
        assert!(script.contains("# This is the second line of the description."));
        assert!(script.contains("# This is the third line of the description."));
    }
}
