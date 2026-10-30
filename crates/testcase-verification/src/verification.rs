use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use testcase_models::{ActualResult, Expected, Step, TestCase, TestSequence};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchStrategy {
    Exact,
    Regex,
    Contains,
    Precomputed,
    BashEvaluation,
}

/// Result of verifying a single step (struct-based, for backward compatibility with tests)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepVerificationResult {
    pub step_number: i64,
    pub passed: bool,
    pub result_match: bool,
    pub output_match: bool,
    pub success_match: bool,
    pub diff: VerificationDiff,
}

/// (De)serialization of step results as single-key maps (`Pass: {...}`), the form the
/// JSON schemas describe. Reading also accepts the `!Pass {...}` YAML tags that older
/// versions of the tools wrote.
mod step_results_serde {
    use super::StepVerificationResultEnum;
    use serde::{Deserialize, Deserializer, Serializer};

    // serde's `with` passes the field type, so this must take `&Vec`
    #[allow(clippy::ptr_arg)]
    pub fn serialize<S: Serializer>(
        steps: &Vec<StepVerificationResultEnum>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde_yaml::with::singleton_map_recursive::serialize(steps, serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<StepVerificationResultEnum>, D::Error> {
        let value = testcase_common::untag(serde_yaml::Value::deserialize(deserializer)?);
        serde_yaml::with::singleton_map_recursive::deserialize(value)
            .map_err(serde::de::Error::custom)
    }
}

/// Result of verifying a single step (enum-based, for batch verification)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepVerificationResultEnum {
    /// Step passed verification
    Pass {
        step: i64,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        requirement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        item: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tc: Option<i64>,
    },
    /// Step failed verification
    Fail {
        step: i64,
        description: String,
        expected: Expected,
        actual_result: String,
        actual_output: String,
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        requirement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        item: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tc: Option<i64>,
    },
    /// Step was not found in execution log
    NotExecuted {
        step: i64,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        requirement: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        item: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tc: Option<i64>,
    },
}

impl StepVerificationResultEnum {
    pub fn is_pass(&self) -> bool {
        matches!(self, StepVerificationResultEnum::Pass { .. })
    }

    pub fn step_number(&self) -> i64 {
        match self {
            StepVerificationResultEnum::Pass { step, .. } => *step,
            StepVerificationResultEnum::Fail { step, .. } => *step,
            StepVerificationResultEnum::NotExecuted { step, .. } => *step,
        }
    }

    pub fn requirement(&self) -> Option<&String> {
        match self {
            StepVerificationResultEnum::Pass { requirement, .. } => requirement.as_ref(),
            StepVerificationResultEnum::Fail { requirement, .. } => requirement.as_ref(),
            StepVerificationResultEnum::NotExecuted { requirement, .. } => requirement.as_ref(),
        }
    }

    pub fn item(&self) -> Option<i64> {
        match self {
            StepVerificationResultEnum::Pass { item, .. } => *item,
            StepVerificationResultEnum::Fail { item, .. } => *item,
            StepVerificationResultEnum::NotExecuted { item, .. } => *item,
        }
    }

    pub fn tc(&self) -> Option<i64> {
        match self {
            StepVerificationResultEnum::Pass { tc, .. } => *tc,
            StepVerificationResultEnum::Fail { tc, .. } => *tc,
            StepVerificationResultEnum::NotExecuted { tc, .. } => *tc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationDiff {
    pub result_diff: Option<DiffDetail>,
    pub output_diff: Option<DiffDetail>,
    pub success_diff: Option<DiffDetail>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffDetail {
    pub expected: String,
    pub actual: String,
    pub message: String,
}

/// Old-style execution verification result (for backward compatibility)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionVerificationResult {
    pub test_case_id: String,
    pub sequence_id: i64,
    pub overall_passed: bool,
    pub step_results: Vec<StepVerificationResult>,
    pub missing_steps: Vec<i64>,
    pub unexpected_steps: Vec<i64>,
}

/// Represents a parsed test execution log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecutionLog {
    /// Test case ID
    pub test_case_id: String,

    /// Test sequence ID
    pub sequence_id: i64,

    /// Step number
    pub step_number: i64,

    /// Whether the step succeeded
    pub success: Option<bool>,

    /// Actual result from execution
    pub actual_result: String,

    /// Actual output from execution
    pub actual_output: String,

    /// Timestamp of execution
    pub timestamp: Option<DateTime<Utc>>,

    /// Path to the log file
    pub log_file_path: PathBuf,

    /// Whether result verification passed (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_verification_pass: Option<bool>,

    /// Whether output verification passed (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_verification_pass: Option<bool>,

    /// SHA-256 hash of the source YAML test case file (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_yaml_sha256: Option<String>,
}

/// Run metadata of a verification result: what ran, on which build, where, by whom and when
/// (verification-result `run_id`, `build_id`, `environment`, `tester`, `date`).
/// Absent values are omitted, never written empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunInfo {
    /// ID of the test run that produced the result
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// Build/software identifier under test
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_id: Option<String>,

    /// Environment the test ran in
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Person or tool that executed the test
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tester: Option<String>,

    /// When the result was produced (RFC 3339)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

impl RunInfo {
    /// Drop empty strings so that they are omitted rather than written empty
    pub fn normalized(self) -> Self {
        fn keep(value: Option<String>) -> Option<String> {
            value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        }
        Self {
            run_id: keep(self.run_id),
            build_id: keep(self.build_id),
            environment: keep(self.environment),
            tester: keep(self.tester),
            date: keep(self.date),
        }
    }

    /// Fill the values that are not set yet from `defaults`
    pub fn fill_missing(&mut self, defaults: &RunInfo) {
        if self.run_id.is_none() {
            self.run_id = defaults.run_id.clone();
        }
        if self.build_id.is_none() {
            self.build_id = defaults.build_id.clone();
        }
        if self.environment.is_none() {
            self.environment = defaults.environment.clone();
        }
        if self.tester.is_none() {
            self.tester = defaults.tester.clone();
        }
        if self.date.is_none() {
            self.date = defaults.date.clone();
        }
    }
}

/// Result of verifying a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCaseVerificationResult {
    /// Test case ID
    pub test_case_id: String,

    /// Test case description
    pub description: String,

    /// Test sequence results
    pub sequences: Vec<SequenceVerificationResult>,

    /// Total number of steps
    pub total_steps: usize,

    /// Number of passed steps
    pub passed_steps: usize,

    /// Number of failed steps
    pub failed_steps: usize,

    /// Number of not executed steps
    pub not_executed_steps: usize,

    /// Overall pass/fail status
    pub overall_pass: bool,

    /// Requirement identifier (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,

    /// Item number (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<i64>,

    /// TC number (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tc: Option<i64>,

    /// SHA-256 hash of the source YAML test case file (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_yaml_sha256: Option<String>,

    /// Run metadata: run, build, environment, tester and date
    #[serde(flatten)]
    pub run: RunInfo,
}

/// Result of verifying a test sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequenceVerificationResult {
    /// Sequence ID
    pub sequence_id: i64,

    /// Sequence name
    pub name: String,

    /// Step results, written as `Pass: {...}` maps (the form the JSON schemas describe)
    #[serde(with = "step_results_serde")]
    pub step_results: Vec<StepVerificationResultEnum>,

    /// Whether all steps passed
    pub all_steps_passed: bool,

    /// Requirement identifier (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,

    /// Item number (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<i64>,

    /// TC number (optional, for reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tc: Option<i64>,
}

/// Aggregated batch verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchVerificationReport {
    /// All test case results
    pub test_cases: Vec<TestCaseVerificationResult>,

    /// Total number of test cases
    pub total_test_cases: usize,

    /// Number of test cases that passed
    pub passed_test_cases: usize,

    /// Number of test cases that failed
    pub failed_test_cases: usize,

    /// Total steps across all test cases
    pub total_steps: usize,

    /// Passed steps across all test cases
    pub passed_steps: usize,

    /// Failed steps across all test cases
    pub failed_steps: usize,

    /// Not executed steps across all test cases
    pub not_executed_steps: usize,

    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
}

impl BatchVerificationReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self {
            test_cases: Vec::new(),
            total_test_cases: 0,
            passed_test_cases: 0,
            failed_test_cases: 0,
            total_steps: 0,
            passed_steps: 0,
            failed_steps: 0,
            not_executed_steps: 0,
            generated_at: Local::now().with_timezone(&Utc),
        }
    }

    /// Add a test case result to the report
    pub fn add_test_case_result(&mut self, result: TestCaseVerificationResult) {
        self.total_test_cases += 1;
        if result.overall_pass {
            self.passed_test_cases += 1;
        } else {
            self.failed_test_cases += 1;
        }

        self.total_steps += result.total_steps;
        self.passed_steps += result.passed_steps;
        self.failed_steps += result.failed_steps;
        self.not_executed_steps += result.not_executed_steps;

        self.test_cases.push(result);
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Test Cases: {}/{} passed, Steps: {}/{} passed ({} failed, {} not executed)",
            self.passed_test_cases,
            self.total_test_cases,
            self.passed_steps,
            self.total_steps,
            self.failed_steps,
            self.not_executed_steps
        )
    }
}

impl Default for BatchVerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata for container reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerReportMetadata {
    /// Optional environment information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Optional platform information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    /// Optional executor information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,

    /// Execution duration in seconds
    pub execution_duration: f64,

    /// Total number of test cases
    pub total_test_cases: usize,

    /// Number of test cases that passed
    pub passed_test_cases: usize,

    /// Number of test cases that failed
    pub failed_test_cases: usize,

    /// SHA-256 hashes of source YAML test case files (test_case_id → sha256)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_hashes: Option<HashMap<String, String>>,
}

/// Configuration for generating container reports
#[derive(Debug, Clone)]
pub struct ContainerReportConfig {
    /// Report title
    pub title: String,

    /// Project name
    pub project: String,

    /// Optional environment information
    pub environment: Option<String>,

    /// Optional platform information
    pub platform: Option<String>,

    /// Optional executor information
    pub executor: Option<String>,

    /// Run metadata stamped on every result; set values replace what a result carries.
    /// Unset ones fall back to the result's own, then (`environment`, `date`) to the report's
    /// environment and generation time.
    pub run: RunInfo,
}

/// Container report for batch verification with enhanced metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerReport {
    /// Document type (envelope field)
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,

    /// Schema reference (envelope field)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Report title
    pub title: String,

    /// Project name
    pub project: String,

    /// Test execution date
    pub test_date: DateTime<Utc>,

    /// Test case results
    pub test_results: Vec<TestCaseVerificationResult>,

    /// Report metadata
    pub metadata: ContainerReportMetadata,
}

impl ContainerReport {
    /// Create a new ContainerReport from a BatchVerificationReport
    pub fn from_batch_report(
        batch_report: BatchVerificationReport,
        title: String,
        project: String,
        environment: Option<String>,
        platform: Option<String>,
        executor: Option<String>,
        execution_duration: f64,
    ) -> Self {
        let source_hashes: HashMap<String, String> = batch_report
            .test_cases
            .iter()
            .filter_map(|tc| {
                tc.source_yaml_sha256
                    .as_ref()
                    .map(|hash| (tc.test_case_id.clone(), hash.clone()))
            })
            .collect();

        let source_hashes = if source_hashes.is_empty() {
            None
        } else {
            Some(source_hashes)
        };

        Self {
            doc_type: Some("test_results_container".to_string()),
            schema: Some("tcms/test-results-container.schema.v1.json".to_string()),
            title,
            project,
            test_date: batch_report.generated_at,
            test_results: batch_report.test_cases,
            metadata: ContainerReportMetadata {
                environment,
                platform,
                executor,
                execution_duration,
                total_test_cases: batch_report.total_test_cases,
                passed_test_cases: batch_report.passed_test_cases,
                failed_test_cases: batch_report.failed_test_cases,
                source_hashes,
            },
        }
    }
}

/// Test verifier for comparing execution logs against test cases
pub struct TestVerifier {
    result_strategy: MatchStrategy,
    output_strategy: MatchStrategy,
}

impl Default for TestVerifier {
    fn default() -> Self {
        Self::with_exact_matching()
    }
}

/// A log that reached the text-format fallback and yielded no entry is not a log: it is not a JSON
/// array of entries and no line has the text log format. Reading it as "nothing was executed"
/// would make it a failed verdict that `--success-on-completion` forgives; it is unverifiable
/// instead (decision D-41f). A valid JSON array, even `[]`, never gets here.
fn reject_log_without_entries(logs: &[TestExecutionLog], log_path: &Path) -> Result<()> {
    if logs.is_empty() {
        anyhow::bail!(
            "{} is not a test execution log: it is not a JSON array of entries and has no line in the text log format",
            log_path.display()
        );
    }
    Ok(())
}

impl TestVerifier {
    pub fn new(result_strategy: MatchStrategy, output_strategy: MatchStrategy) -> Self {
        Self {
            result_strategy,
            output_strategy,
        }
    }

    pub fn with_strategies(result_strategy: MatchStrategy, output_strategy: MatchStrategy) -> Self {
        Self {
            result_strategy,
            output_strategy,
        }
    }

    /// Parse a test execution log file
    pub fn parse_log_file<P: AsRef<Path>>(&self, log_path: P) -> Result<Vec<TestExecutionLog>> {
        let log_path = log_path.as_ref();
        let content =
            fs::read_to_string(log_path).context("Failed to read test execution log file")?;

        self.parse_log_content(&content, log_path)
    }

    /// Parse a test execution log file with a specified test case ID
    /// This is useful when the log file doesn't follow the naming convention
    pub fn parse_log_file_with_test_case_id<P: AsRef<Path>>(
        &self,
        log_path: P,
        test_case_id: &str,
    ) -> Result<Vec<TestExecutionLog>> {
        let log_path = log_path.as_ref();
        let content =
            fs::read_to_string(log_path).context("Failed to read test execution log file")?;

        self.parse_log_content_with_test_case_id(&content, log_path, test_case_id)
    }

    /// Parse test execution log content with a specified test case ID
    pub fn parse_log_content_with_test_case_id(
        &self,
        content: &str,
        log_path: &Path,
        test_case_id: &str,
    ) -> Result<Vec<TestExecutionLog>> {
        let mut logs = Vec::new();

        log::debug!(
            "Parsing log file with test_case_id: {} from {} (length: {} bytes)",
            test_case_id,
            log_path.display(),
            content.len()
        );

        // Try to parse as JSON first
        let trimmed_content = content.trim();
        if trimmed_content.starts_with('[') {
            log::debug!("Log file appears to be JSON format (starts with '[')");
            // Attempt to parse as JSON array of TestStepExecutionEntry
            match self.parse_json_log_content_with_test_case_id(content, log_path, test_case_id) {
                Ok(json_logs) => {
                    log::debug!(
                        "Successfully parsed as JSON format: {} log entries",
                        json_logs.len()
                    );
                    return Ok(json_logs);
                }
                Err(e) => {
                    log::debug!(
                        "Failed to parse as JSON, falling back to text format: {}",
                        e
                    );
                }
            }
        } else {
            log::debug!("Log file appears to be text format (does not start with '[')");
        }

        // Fall back to text format parsing with the provided test_case_id
        let log_regex = Regex::new(
            r"(?x)
            (?:\[([^\]]+)\]\s+)?  # Optional timestamp
            TestCase:\s+([^,]+),\s+
            Sequence:\s+(\d+),\s+
            Step:\s+(\d+),\s+
            Success:\s+(true|false|null|none|-),\s+
            Result:\s+([^,]+),\s+
            Output:\s+(.+)
            ",
        )
        .context("Failed to compile log regex")?;

        for line in content.lines() {
            if let Some(caps) = log_regex.captures(line) {
                let timestamp = caps.get(1).and_then(|m| {
                    DateTime::parse_from_rfc3339(m.as_str())
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                // Safe to unwrap: regex capture groups 3-7 are guaranteed by successful match
                let sequence_id = caps
                    .get(3)
                    .expect("Regex capture group 3 should exist")
                    .as_str()
                    .parse::<i64>()
                    .context("Failed to parse sequence ID")?;
                let step_number = caps
                    .get(4)
                    .expect("Regex capture group 4 should exist")
                    .as_str()
                    .parse::<i64>()
                    .context("Failed to parse step number")?;

                let success_str = caps
                    .get(5)
                    .expect("Regex capture group 5 should exist")
                    .as_str()
                    .to_lowercase();
                let success = match success_str.as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };

                let actual_result = caps
                    .get(6)
                    .expect("Regex capture group 6 should exist")
                    .as_str()
                    .trim()
                    .to_string();
                let actual_output = caps
                    .get(7)
                    .expect("Regex capture group 7 should exist")
                    .as_str()
                    .trim()
                    .to_string();

                log::debug!(
                    "Parsed text log entry: TestCase={}, Seq={}, Step={}, Success={:?}, Result={}, Output={}",
                    test_case_id,
                    sequence_id,
                    step_number,
                    success,
                    actual_result,
                    actual_output
                );

                logs.push(TestExecutionLog {
                    test_case_id: test_case_id.to_string(),
                    sequence_id,
                    step_number,
                    success,
                    actual_result,
                    actual_output,
                    timestamp,
                    log_file_path: log_path.to_path_buf(),
                    result_verification_pass: None,
                    output_verification_pass: None,
                    source_yaml_sha256: None,
                });
            }
        }

        log::debug!("Parsed {} text format log entries", logs.len());

        reject_log_without_entries(&logs, log_path)?;
        Ok(logs)
    }

    /// Parse test execution log content
    pub fn parse_log_content(
        &self,
        content: &str,
        log_path: &Path,
    ) -> Result<Vec<TestExecutionLog>> {
        let mut logs = Vec::new();

        log::debug!(
            "Parsing log file: {} (length: {} bytes)",
            log_path.display(),
            content.len()
        );

        // Try to parse as JSON first
        let trimmed_content = content.trim();
        if trimmed_content.starts_with('[') {
            log::debug!("Log file appears to be JSON format (starts with '[')");
            // Attempt to parse as JSON array of TestStepExecutionEntry
            match self.parse_json_log_content(content, log_path) {
                Ok(json_logs) => {
                    log::debug!(
                        "Successfully parsed as JSON format: {} log entries",
                        json_logs.len()
                    );
                    return Ok(json_logs);
                }
                Err(e) => {
                    log::debug!(
                        "Failed to parse as JSON, falling back to text format: {}",
                        e
                    );
                }
            }
        } else {
            log::debug!("Log file appears to be text format (does not start with '[')");
        }

        // Fall back to text format parsing
        // Regular expressions for parsing log entries
        // Format: [TIMESTAMP] TestCase: <id>, Sequence: <seq_id>, Step: <step_num>, Success: <true/false>, Result: <result>, Output: <output>
        let log_regex = Regex::new(
            r"(?x)
            (?:\[([^\]]+)\]\s+)?  # Optional timestamp
            TestCase:\s+([^,]+),\s+
            Sequence:\s+(\d+),\s+
            Step:\s+(\d+),\s+
            Success:\s+(true|false|null|none|-),\s+
            Result:\s+([^,]+),\s+
            Output:\s+(.+)
            ",
        )
        .context("Failed to compile log regex")?;

        for line in content.lines() {
            if let Some(caps) = log_regex.captures(line) {
                let timestamp = caps.get(1).and_then(|m| {
                    DateTime::parse_from_rfc3339(m.as_str())
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                });

                // Safe to unwrap: regex capture groups 2-7 are guaranteed by successful match
                let test_case_id = caps
                    .get(2)
                    .expect("Regex capture group 2 should exist")
                    .as_str()
                    .trim()
                    .to_string();
                let sequence_id = caps
                    .get(3)
                    .expect("Regex capture group 3 should exist")
                    .as_str()
                    .parse::<i64>()
                    .context("Failed to parse sequence ID")?;
                let step_number = caps
                    .get(4)
                    .expect("Regex capture group 4 should exist")
                    .as_str()
                    .parse::<i64>()
                    .context("Failed to parse step number")?;

                let success_str = caps
                    .get(5)
                    .expect("Regex capture group 5 should exist")
                    .as_str()
                    .to_lowercase();
                let success = match success_str.as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };

                let actual_result = caps
                    .get(6)
                    .expect("Regex capture group 6 should exist")
                    .as_str()
                    .trim()
                    .to_string();
                let actual_output = caps
                    .get(7)
                    .expect("Regex capture group 7 should exist")
                    .as_str()
                    .trim()
                    .to_string();

                log::debug!(
                    "Parsed text log entry: TestCase={}, Seq={}, Step={}, Success={:?}, Result={}, Output={}",
                    test_case_id,
                    sequence_id,
                    step_number,
                    success,
                    actual_result,
                    actual_output
                );

                logs.push(TestExecutionLog {
                    test_case_id,
                    sequence_id,
                    step_number,
                    success,
                    actual_result,
                    actual_output,
                    timestamp,
                    log_file_path: log_path.to_path_buf(),
                    result_verification_pass: None,
                    output_verification_pass: None,
                    source_yaml_sha256: None,
                });
            }
        }

        log::debug!("Parsed {} text format log entries", logs.len());

        reject_log_without_entries(&logs, log_path)?;
        Ok(logs)
    }

    /// Parse JSON-formatted execution log content with a specified test case ID
    fn parse_json_log_content_with_test_case_id(
        &self,
        content: &str,
        log_path: &Path,
        test_case_id: &str,
    ) -> Result<Vec<TestExecutionLog>> {
        use testcase_models::TestStepExecutionEntry;

        log::debug!(
            "Parsing JSON log content with test_case_id: {}",
            test_case_id
        );

        let entries: Vec<TestStepExecutionEntry> =
            serde_json::from_str(content).context("Failed to parse JSON execution log")?;

        log::debug!("Successfully parsed {} JSON entries", entries.len());

        // Check for type/schema fields in the first entry (if any)
        if let Some(first_entry) = entries.first() {
            if first_entry.doc_type.is_none() {
                log::warn!(
                    "Missing 'type' field in execution log: {}",
                    log_path.display()
                );
            }
            if first_entry.schema.is_none() {
                log::warn!(
                    "Missing 'schema' field in execution log: {}",
                    log_path.display()
                );
            } else if let Some(ref schema) = first_entry.schema {
                log::debug!("Execution log schema: {}", schema);
            }
        }

        let raw_hashes = raw_source_hashes(content);
        let mut logs = Vec::new();
        for (index, entry) in entries.into_iter().enumerate() {
            // An entry that names another test case is not evidence for this one
            if let Some(other) = entry.test_case_id.as_deref().filter(|id| !id.is_empty()) {
                if other != test_case_id {
                    log::warn!(
                        "Ignoring entry for test case '{}' in the log of '{}': {}",
                        other,
                        test_case_id,
                        log_path.display()
                    );
                    continue;
                }
            }
            // Derive success from exit_code (0 = success, non-zero = failure)
            let success = Some(entry.exit_code == 0);

            // Derive actual_result from exit_code
            let actual_result = entry.exit_code.to_string();

            // Parse timestamp if present
            let timestamp = entry.timestamp.as_ref().and_then(|ts| {
                DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            log::debug!(
                "Parsed JSON entry: TestCase={}, Seq={}, Step={}, ExitCode={}, Success={:?}, Output={}",
                test_case_id,
                entry.test_sequence,
                entry.step,
                entry.exit_code,
                success,
                entry.output
            );

            logs.push(TestExecutionLog {
                test_case_id: test_case_id.to_string(),
                sequence_id: entry.test_sequence,
                step_number: entry.step,
                success,
                actual_result,
                actual_output: entry.output.clone(),
                timestamp,
                log_file_path: log_path.to_path_buf(),
                result_verification_pass: Some(entry.result_verification_pass),
                output_verification_pass: Some(entry.output_verification_pass),
                source_yaml_sha256: raw_hashes.get(index).cloned().flatten(),
            });
        }

        Ok(logs)
    }

    /// Parse JSON-formatted execution log content
    fn parse_json_log_content(
        &self,
        content: &str,
        log_path: &Path,
    ) -> Result<Vec<TestExecutionLog>> {
        use testcase_models::TestStepExecutionEntry;

        let entries: Vec<TestStepExecutionEntry> =
            serde_json::from_str(content).context("Failed to parse JSON execution log")?;

        // Extract test_case_id from the log file path
        // Expected format: {test_case_id}_execution_log.json
        let test_case_id = log_path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_suffix("_execution_log"))
            .unwrap_or("UNKNOWN")
            .to_string();

        log::debug!(
            "Parsing JSON log content, extracted test_case_id: {} from path: {}",
            test_case_id,
            log_path.display()
        );
        log::debug!("Successfully parsed {} JSON entries", entries.len());

        // Check for type/schema fields in the first entry (if any)
        if let Some(first_entry) = entries.first() {
            if first_entry.doc_type.is_none() {
                log::warn!(
                    "Missing 'type' field in execution log: {}",
                    log_path.display()
                );
            }
            if first_entry.schema.is_none() {
                log::warn!(
                    "Missing 'schema' field in execution log: {}",
                    log_path.display()
                );
            } else if let Some(ref schema) = first_entry.schema {
                log::debug!("Execution log schema: {}", schema);
            }
        }

        let raw_hashes = raw_source_hashes(content);
        let mut logs = Vec::new();
        for (index, entry) in entries.into_iter().enumerate() {
            // An entry that names another test case is not evidence for this one
            if let Some(other) = entry.test_case_id.as_deref().filter(|id| !id.is_empty()) {
                if other != test_case_id {
                    log::warn!(
                        "Ignoring entry for test case '{}' in the log of '{}': {}",
                        other,
                        test_case_id,
                        log_path.display()
                    );
                    continue;
                }
            }
            // Derive success from exit_code (0 = success, non-zero = failure)
            let success = Some(entry.exit_code == 0);

            // Derive actual_result from exit_code
            let actual_result = entry.exit_code.to_string();

            // Parse timestamp if present
            let timestamp = entry.timestamp.as_ref().and_then(|ts| {
                DateTime::parse_from_rfc3339(ts)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            log::debug!(
                "Parsed JSON entry: TestCase={}, Seq={}, Step={}, ExitCode={}, Success={:?}, Output={}",
                test_case_id,
                entry.test_sequence,
                entry.step,
                entry.exit_code,
                success,
                entry.output
            );

            logs.push(TestExecutionLog {
                test_case_id: test_case_id.clone(),
                sequence_id: entry.test_sequence,
                step_number: entry.step,
                success,
                actual_result,
                actual_output: entry.output.clone(),
                timestamp,
                log_file_path: log_path.to_path_buf(),
                result_verification_pass: Some(entry.result_verification_pass),
                output_verification_pass: Some(entry.output_verification_pass),
                source_yaml_sha256: raw_hashes.get(index).cloned().flatten(),
            });
        }

        Ok(logs)
    }

    /// Verify a single test case against execution logs
    pub fn verify_test_case(
        &self,
        test_case: &TestCase,
        execution_logs: &[TestExecutionLog],
    ) -> TestCaseVerificationResult {
        log::debug!(
            "Verifying test case: {} ({}) with {} execution logs",
            test_case.id,
            test_case.description,
            execution_logs.len()
        );

        let mut sequences = Vec::new();
        let mut total_steps = 0;
        let mut passed_steps = 0;
        let mut failed_steps = 0;
        let mut not_executed_steps = 0;

        // Create a lookup map for execution logs
        let mut log_map: HashMap<(i64, i64), &TestExecutionLog> = HashMap::new();
        for log in execution_logs {
            if log.test_case_id == test_case.id {
                log_map.insert((log.sequence_id, log.step_number), log);
            }
        }

        log::debug!(
            "Built log map with {} entries for test case {}",
            log_map.len(),
            test_case.id
        );

        // Extract requirement, item, and tc from test case
        let requirement = Some(test_case.requirement.clone());
        let item = Some(test_case.item);
        let tc = Some(test_case.tc);

        for sequence in &test_case.test_sequences {
            log::debug!(
                "Verifying sequence {} ({}) with {} steps",
                sequence.id,
                sequence.name,
                sequence.steps.len()
            );

            let step_results =
                self.verify_sequence(sequence, &log_map, requirement.as_ref(), item, tc);

            let all_steps_passed = step_results.iter().all(|r| r.is_pass());

            for result in &step_results {
                total_steps += 1;
                match result {
                    StepVerificationResultEnum::Pass { .. } => passed_steps += 1,
                    StepVerificationResultEnum::Fail { .. } => failed_steps += 1,
                    StepVerificationResultEnum::NotExecuted { .. } => not_executed_steps += 1,
                }
            }

            log::debug!(
                "Sequence {} verification complete: all_passed={}",
                sequence.id,
                all_steps_passed
            );

            sequences.push(SequenceVerificationResult {
                sequence_id: sequence.id,
                name: sequence.name.clone(),
                step_results,
                all_steps_passed,
                requirement: requirement.clone(),
                item,
                tc,
            });
        }

        // A test case with no steps verifies nothing, so it cannot pass
        let overall_pass = total_steps > 0 && failed_steps == 0 && not_executed_steps == 0;

        log::debug!(
            "Test case {} verification complete: passed={}/{}, failed={}, not_executed={}, overall_pass={}",
            test_case.id,
            passed_steps,
            total_steps,
            failed_steps,
            not_executed_steps,
            overall_pass
        );

        TestCaseVerificationResult {
            test_case_id: test_case.id.clone(),
            description: test_case.description.clone(),
            sequences,
            total_steps,
            passed_steps,
            failed_steps,
            not_executed_steps,
            overall_pass,
            requirement: requirement.clone(),
            item,
            tc,
            source_yaml_sha256: None,
            run: Default::default(),
        }
    }

    /// Verify a single sequence against execution logs
    fn verify_sequence(
        &self,
        sequence: &TestSequence,
        log_map: &HashMap<(i64, i64), &TestExecutionLog>,
        requirement: Option<&String>,
        item: Option<i64>,
        tc: Option<i64>,
    ) -> Vec<StepVerificationResultEnum> {
        let mut results = Vec::new();
        let mut captured_vars: HashMap<String, String> = HashMap::new();

        for step in &sequence.steps {
            let result = if let Some(log) = log_map.get(&(sequence.id, step.step)) {
                log::debug!(
                    "Found execution log for sequence {} step {}, verifying...",
                    sequence.id,
                    step.step
                );
                self.verify_step_new(step, log, requirement, item, tc, &captured_vars)
            } else {
                log::debug!(
                    "No execution log found for sequence {} step {} - marking as NotExecuted",
                    sequence.id,
                    step.step
                );
                StepVerificationResultEnum::NotExecuted {
                    step: step.step,
                    description: step.description.clone(),
                    requirement: requirement.cloned(),
                    item,
                    tc,
                }
            };

            // After step verification, extract captured variables for BashEvaluation if step passed
            if result.is_pass() {
                if let Some(log) = log_map.get(&(sequence.id, step.step)) {
                    if self.result_strategy == MatchStrategy::BashEvaluation
                        || self.output_strategy == MatchStrategy::BashEvaluation
                    {
                        self.extract_captured_vars_from_step(step, log, &mut captured_vars);
                    }
                }
            }

            results.push(result);
        }

        results
    }

    /// Verify a single step against its execution log (new API)
    fn verify_step_new(
        &self,
        step: &Step,
        log: &TestExecutionLog,
        requirement: Option<&String>,
        item: Option<i64>,
        tc: Option<i64>,
        captured_vars: &HashMap<String, String>,
    ) -> StepVerificationResultEnum {
        let expected = &step.expected;

        // BashEvaluation mode: evaluate verification expressions via bash
        if self.result_strategy == MatchStrategy::BashEvaluation
            || self.output_strategy == MatchStrategy::BashEvaluation
        {
            // Parse exit_code from log.actual_result (it's stored as a string like "0")
            let exit_code: i32 = log.actual_result.parse().unwrap_or(-1);

            let context = bash_eval::BashEvalContext {
                exit_code,
                command_output: log.actual_output.clone(),
                variables: captured_vars.clone(),
            };

            // Evaluate result verification if using BashEvaluation
            if self.result_strategy == MatchStrategy::BashEvaluation {
                let expr: bash_eval::BashExpression = (&step.verification.result).into();
                match bash_eval::evaluate(&expr, &context) {
                    Ok(true) => {
                        log::debug!("Result verification passed (bash evaluation)");
                    }
                    _ => {
                        return StepVerificationResultEnum::Fail {
                            step: step.step,
                            description: step.description.clone(),
                            expected: expected.clone(),
                            actual_result: log.actual_result.clone(),
                            actual_output: log.actual_output.clone(),
                            reason: "Result verification failed (bash evaluation)".to_string(),
                            requirement: requirement.cloned(),
                            item,
                            tc,
                        };
                    }
                }
            }

            // Evaluate output verification if using BashEvaluation
            if self.output_strategy == MatchStrategy::BashEvaluation {
                let expr: bash_eval::BashExpression = (&step.verification.output).into();
                match bash_eval::evaluate(&expr, &context) {
                    Ok(true) => {
                        log::debug!("Output verification passed (bash evaluation)");
                    }
                    _ => {
                        return StepVerificationResultEnum::Fail {
                            step: step.step,
                            description: step.description.clone(),
                            expected: expected.clone(),
                            actual_result: log.actual_result.clone(),
                            actual_output: log.actual_output.clone(),
                            reason: "Output verification failed (bash evaluation)".to_string(),
                            requirement: requirement.cloned(),
                            item,
                            tc,
                        };
                    }
                }
            }

            // In BashEvaluation mode, skip success field check and pass
            return StepVerificationResultEnum::Pass {
                step: step.step,
                description: step.description.clone(),
                requirement: requirement.cloned(),
                item,
                tc,
            };
        }

        // Precomputed mode: check precomputed verification fields
        if self.result_strategy == MatchStrategy::Precomputed
            || self.output_strategy == MatchStrategy::Precomputed
        {
            // Check result verification if in Precomputed mode
            if self.result_strategy == MatchStrategy::Precomputed
                && log.result_verification_pass != Some(true)
            {
                return StepVerificationResultEnum::Fail {
                    step: step.step,
                    description: step.description.clone(),
                    expected: expected.clone(),
                    actual_result: log.actual_result.clone(),
                    actual_output: log.actual_output.clone(),
                    reason: "Result verification failed (precomputed)".to_string(),
                    requirement: requirement.cloned(),
                    item,
                    tc,
                };
            }

            // Check output verification if in Precomputed mode
            if self.output_strategy == MatchStrategy::Precomputed
                && log.output_verification_pass != Some(true)
            {
                return StepVerificationResultEnum::Fail {
                    step: step.step,
                    description: step.description.clone(),
                    expected: expected.clone(),
                    actual_result: log.actual_result.clone(),
                    actual_output: log.actual_output.clone(),
                    reason: "Output verification failed (precomputed)".to_string(),
                    requirement: requirement.cloned(),
                    item,
                    tc,
                };
            }

            // In Precomputed mode, skip success field check and pass
            return StepVerificationResultEnum::Pass {
                step: step.step,
                description: step.description.clone(),
                requirement: requirement.cloned(),
                item,
                tc,
            };
        }

        // Non-Precomputed mode: standard verification logic
        log::debug!(
            "Verifying step {} ({}): Expected[result='{}', output='{}', success={:?}]",
            step.step,
            step.description,
            expected.result,
            expected.output,
            expected.success
        );
        log::debug!(
            "  Actual from log: result='{}', output='{}', success={:?}",
            log.actual_result,
            log.actual_output,
            log.success
        );

        // Check success field if it's defined
        if let Some(expected_success) = expected.success {
            if let Some(actual_success) = log.success {
                log::debug!(
                    "  Checking success: expected={}, actual={}",
                    expected_success,
                    actual_success
                );
                if expected_success != actual_success {
                    log::debug!("  SUCCESS CHECK FAILED: mismatch detected");
                    return StepVerificationResultEnum::Fail {
                        step: step.step,
                        description: step.description.clone(),
                        expected: expected.clone(),
                        actual_result: log.actual_result.clone(),
                        actual_output: log.actual_output.clone(),
                        reason: format!(
                            "Success mismatch: expected {}, got {}",
                            expected_success, actual_success
                        ),
                        requirement: requirement.cloned(),
                        item,
                        tc,
                    };
                }
                log::debug!("  Success check passed");
            }
        }

        // Check result
        log::debug!(
            "  Checking result: expected='{}', actual='{}', strategy={:?}",
            expected.result,
            log.actual_result,
            self.result_strategy
        );
        if !self.matches(&expected.result, &log.actual_result, self.result_strategy) {
            log::debug!(
                "  RESULT CHECK FAILED: no match with strategy {:?}",
                self.result_strategy
            );
            return StepVerificationResultEnum::Fail {
                step: step.step,
                description: step.description.clone(),
                expected: expected.clone(),
                actual_result: log.actual_result.clone(),
                actual_output: log.actual_output.clone(),
                reason: format!(
                    "Result mismatch: expected '{}', got '{}'",
                    expected.result, log.actual_result
                ),
                requirement: requirement.cloned(),
                item,
                tc,
            };
        }
        log::debug!("  Result check passed");

        // Check output
        log::debug!(
            "  Checking output: expected='{}', actual='{}', strategy={:?}",
            expected.output,
            log.actual_output,
            self.output_strategy
        );
        if !self.matches(&expected.output, &log.actual_output, self.output_strategy) {
            log::debug!(
                "  OUTPUT CHECK FAILED: no match with strategy {:?}",
                self.output_strategy
            );
            return StepVerificationResultEnum::Fail {
                step: step.step,
                description: step.description.clone(),
                expected: expected.clone(),
                actual_result: log.actual_result.clone(),
                actual_output: log.actual_output.clone(),
                reason: format!(
                    "Output mismatch: expected '{}', got '{}'",
                    expected.output, log.actual_output
                ),
                requirement: requirement.cloned(),
                item,
                tc,
            };
        }
        log::debug!("  Output check passed");

        log::debug!("  Step {} PASSED all checks", step.step);

        StepVerificationResultEnum::Pass {
            step: step.step,
            description: step.description.clone(),
            requirement: requirement.cloned(),
            item,
            tc,
        }
    }

    /// Extract captured variables from a step's execution log
    /// Uses regex-based capture patterns to extract values from the output
    /// Command-based captures are skipped with a warning
    fn extract_captured_vars_from_step(
        &self,
        step: &Step,
        log: &TestExecutionLog,
        captured_vars: &mut HashMap<String, String>,
    ) {
        if let Some(ref capture_vars) = step.capture_vars {
            let capture_list = match capture_vars {
                testcase_models::CaptureVarsFormat::Legacy(map) => map
                    .iter()
                    .map(|(name, pattern)| (name.clone(), Some(pattern.clone()), None))
                    .collect::<Vec<_>>(),
                testcase_models::CaptureVarsFormat::New(vec) => vec
                    .iter()
                    .map(|cv| (cv.name.clone(), cv.capture.clone(), cv.command.clone()))
                    .collect::<Vec<_>>(),
            };

            for (var_name, capture_pattern, command) in capture_list {
                if let Some(pattern) = capture_pattern {
                    // Regex-based capture
                    if let Some(value) = bash_eval::extract_capture(&log.actual_output, &pattern) {
                        log::debug!("Captured variable {}: {}", var_name, value);
                        captured_vars.insert(var_name, value);
                    } else {
                        log::debug!(
                            "Failed to capture variable {} with pattern {}",
                            var_name,
                            pattern
                        );
                    }
                } else if command.is_some() {
                    // Command-based capture - cannot replay at verify time
                    log::warn!("Skipping command-based capture of variable {} (cannot replay at verify-time with BashEvaluation strategy)", var_name);
                }
            }
        }
    }

    fn matches(&self, expected: &str, actual: &str, strategy: MatchStrategy) -> bool {
        let result = match strategy {
            MatchStrategy::Exact => {
                let matched = expected == actual;
                log::debug!(
                    "    Match check (Exact): expected='{}', actual='{}' => {}",
                    expected,
                    actual,
                    matched
                );
                matched
            }
            MatchStrategy::Contains => {
                let matched = actual.contains(expected);
                log::debug!(
                    "    Match check (Contains): expected='{}', actual='{}' => {}",
                    expected,
                    actual,
                    matched
                );
                matched
            }
            MatchStrategy::Regex => {
                if let Ok(regex) = Regex::new(expected) {
                    let matched = regex.is_match(actual);
                    log::debug!(
                        "    Match check (Regex): pattern='{}', actual='{}' => {}",
                        expected,
                        actual,
                        matched
                    );
                    matched
                } else {
                    log::debug!(
                        "    Match check (Regex): INVALID pattern='{}' - compilation failed",
                        expected
                    );
                    false
                }
            }
            MatchStrategy::Precomputed | MatchStrategy::BashEvaluation => {
                // These strategies should not be used with matches()
                // They are handled separately in verify_step_new
                false
            }
        };
        result
    }

    /// Process multiple log files and verify against provided test cases
    pub fn batch_verify<P: AsRef<Path>>(
        &self,
        log_paths: &[P],
        test_cases: &[TestCase],
    ) -> Result<BatchVerificationReport> {
        let mut report = BatchVerificationReport::new();
        let mut all_logs: HashMap<String, Vec<TestExecutionLog>> = HashMap::new();

        // Parse all log files
        for log_path in log_paths {
            let logs = self.parse_log_file(log_path)?;
            for log in logs {
                all_logs
                    .entry(log.test_case_id.clone())
                    .or_default()
                    .push(log);
            }
        }

        // Create a test case lookup map
        let test_case_map: HashMap<String, &TestCase> =
            test_cases.iter().map(|tc| (tc.id.clone(), tc)).collect();

        // Verify each test case
        for (test_case_id, logs) in all_logs.iter() {
            // Try to find test case
            match test_case_map.get(test_case_id) {
                Some(test_case) => {
                    let result = self.verify_test_case(test_case, logs);
                    report.add_test_case_result(result);
                }
                None => {
                    log::warn!(
                        "Test case '{}' not found in provided test cases. Skipping verification.",
                        test_case_id
                    );
                    // Create a failed result for missing test case
                    let failed_result = TestCaseVerificationResult {
                        test_case_id: test_case_id.clone(),
                        description: "Test case not found".to_string(),
                        sequences: Vec::new(),
                        total_steps: logs.len(),
                        passed_steps: 0,
                        failed_steps: logs.len(),
                        not_executed_steps: 0,
                        overall_pass: false,
                        requirement: None,
                        item: None,
                        tc: None,
                        source_yaml_sha256: None,
                        run: Default::default(),
                    };
                    report.add_test_case_result(failed_result);
                }
            }
        }

        Ok(report)
    }

    // ========================================================================
    // Old-style API methods (for backward compatibility with existing tests)
    // ========================================================================

    /// Create verifier with exact matching (old API)
    pub fn with_exact_matching() -> Self {
        Self::new(MatchStrategy::Exact, MatchStrategy::Exact)
    }

    /// Create verifier from storage (for backward compatibility with existing tests)
    ///
    /// This method accepts a TestCaseStorage parameter for API consistency but does not
    /// actually use it, as TestVerifier does not require storage access for verification.
    /// The verifier is created with default exact matching strategies.
    pub fn from_storage<S>(_storage: S) -> Self {
        Self::with_exact_matching()
    }

    /// Verify a single step against an execution log (new two-stage workflow)
    pub fn verify_step_from_log(
        &self,
        step: &Step,
        log: &TestExecutionLog,
    ) -> StepVerificationResult {
        // Handle Precomputed strategy
        let result_match = if self.result_strategy == MatchStrategy::Precomputed {
            log.result_verification_pass == Some(true)
        } else {
            self.matches(
                &step.expected.result,
                &log.actual_result,
                self.result_strategy,
            )
        };

        let output_match = if self.output_strategy == MatchStrategy::Precomputed {
            log.output_verification_pass == Some(true)
        } else {
            self.matches(
                &step.expected.output,
                &log.actual_output,
                self.output_strategy,
            )
        };

        let mut success_match = true;
        let mut success_diff = None;

        // Skip success field check if either strategy is Precomputed
        if self.result_strategy != MatchStrategy::Precomputed
            && self.output_strategy != MatchStrategy::Precomputed
        {
            if let Some(expected_success) = step.expected.success {
                if let Some(actual_success) = log.success {
                    success_match = expected_success == actual_success;
                    if !success_match {
                        success_diff = Some(DiffDetail {
                            expected: expected_success.to_string(),
                            actual: actual_success.to_string(),
                            message: "Success flag mismatch".to_string(),
                        });
                    }
                }
            }
        }

        let passed = result_match && output_match && success_match;

        let result_diff = if !result_match {
            Some(DiffDetail {
                expected: step.expected.result.clone(),
                actual: log.actual_result.clone(),
                message: format!(
                    "Result mismatch ({})",
                    self.strategy_name(self.result_strategy)
                ),
            })
        } else {
            None
        };

        let output_diff = if !output_match {
            Some(DiffDetail {
                expected: step.expected.output.clone(),
                actual: log.actual_output.clone(),
                message: format!(
                    "Output mismatch ({})",
                    self.strategy_name(self.output_strategy)
                ),
            })
        } else {
            None
        };

        StepVerificationResult {
            step_number: step.step,
            passed,
            result_match,
            output_match,
            success_match,
            diff: VerificationDiff {
                result_diff,
                output_diff,
                success_diff,
            },
        }
    }

    fn strategy_name(&self, strategy: MatchStrategy) -> &'static str {
        match strategy {
            MatchStrategy::Exact => "Exact",
            MatchStrategy::Regex => "Regex",
            MatchStrategy::Contains => "Contains",
            MatchStrategy::Precomputed => "Precomputed",
            MatchStrategy::BashEvaluation => "BashEvaluation",
        }
    }

    /// Verify a single step with old-style result (struct-based)
    pub fn verify_step(&self, step: &Step, actual: &ActualResult) -> StepVerificationResult {
        let result_match =
            self.matches(&step.expected.result, &actual.result, self.result_strategy);
        let output_match =
            self.matches(&step.expected.output, &actual.output, self.output_strategy);

        let success_match = match step.expected.success {
            Some(expected_success) => expected_success == actual.success,
            None => true,
        };

        let passed = result_match && output_match && success_match;

        let diff = VerificationDiff {
            result_diff: if !result_match {
                Some(DiffDetail {
                    expected: step.expected.result.clone(),
                    actual: actual.result.clone(),
                    message: format!(
                        "Result mismatch (strategy: {:?}): expected '{}' but got '{}'",
                        self.result_strategy, step.expected.result, actual.result
                    ),
                })
            } else {
                None
            },
            output_diff: if !output_match {
                Some(DiffDetail {
                    expected: step.expected.output.clone(),
                    actual: actual.output.clone(),
                    message: format!(
                        "Output mismatch (strategy: {:?}): expected '{}' but got '{}'",
                        self.output_strategy, step.expected.output, actual.output
                    ),
                })
            } else {
                None
            },
            success_diff: if !success_match {
                Some(DiffDetail {
                    expected: step
                        .expected
                        .success
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "None".to_string()),
                    actual: actual.success.to_string(),
                    message: format!(
                        "Success flag mismatch: expected {:?} but got {}",
                        step.expected.success, actual.success
                    ),
                })
            } else {
                None
            },
        };

        StepVerificationResult {
            step_number: step.step,
            passed,
            result_match,
            output_match,
            success_match,
            diff,
        }
    }

    /// Verify execution log with old-style result
    pub fn verify_execution_log(
        &self,
        test_case: &TestCase,
        execution_log: &testcase_models::TestExecutionLog,
    ) -> ExecutionVerificationResult {
        let sequence = test_case
            .test_sequences
            .iter()
            .find(|seq| seq.id == execution_log.sequence_id);

        let Some(sequence) = sequence else {
            return ExecutionVerificationResult {
                test_case_id: test_case.id.clone(),
                sequence_id: execution_log.sequence_id,
                overall_passed: false,
                step_results: vec![],
                missing_steps: vec![],
                unexpected_steps: vec![],
            };
        };

        let mut step_results = Vec::new();
        for step in &sequence.steps {
            let actual_result = ActualResult {
                result: execution_log.actual_output.clone(),
                output: execution_log.actual_output.clone(),
                success: execution_log.actual_success,
            };
            let verification_result = self.verify_step(step, &actual_result);
            step_results.push(verification_result);
        }

        let overall_passed = step_results.iter().all(|r| r.passed);

        ExecutionVerificationResult {
            test_case_id: test_case.id.clone(),
            sequence_id: execution_log.sequence_id,
            overall_passed,
            step_results,
            missing_steps: vec![],
            unexpected_steps: vec![],
        }
    }

    // ========================================================================
    // Report Generation Methods
    // ========================================================================

    /// Generate YAML report for a single test case verification result
    pub fn generate_report_yaml(&self, result: &TestCaseVerificationResult) -> Result<String> {
        testcase_common::to_yaml_string(result)
            .context("Failed to serialize verification result to YAML")
    }

    /// Generate JSON report for a single test case verification result
    pub fn generate_report_json(&self, result: &TestCaseVerificationResult) -> Result<String> {
        serde_json::to_string_pretty(result)
            .context("Failed to serialize verification result to JSON")
    }

    /// Generate container report with enhanced metadata
    /// This method creates a ContainerReport with custom title, project, and metadata fields
    ///
    /// If multiple reports are provided, calculates execution_duration as the time difference
    /// between the earliest and latest generated_at timestamps. Otherwise uses 0.0.
    pub fn generate_report(
        &self,
        reports: &[BatchVerificationReport],
        format: &str,
        config: ContainerReportConfig,
    ) -> Result<String> {
        // Merge all reports into a single batch report
        let mut merged_report = BatchVerificationReport::new();

        // Track timestamps for duration calculation
        let mut earliest_timestamp: Option<DateTime<Utc>> = None;
        let mut latest_timestamp: Option<DateTime<Utc>> = None;

        for report in reports {
            // Update timestamp tracking
            let timestamp = report.generated_at;
            earliest_timestamp = match earliest_timestamp {
                None => Some(timestamp),
                Some(current) => Some(current.min(timestamp)),
            };
            latest_timestamp = match latest_timestamp {
                None => Some(timestamp),
                Some(current) => Some(current.max(timestamp)),
            };

            // Add all test case results from this report
            for test_case_result in &report.test_cases {
                merged_report.add_test_case_result(test_case_result.clone());
            }
        }

        // Stamp run metadata. Precedence: the configured values (flag, environment, config
        // file), then what the result already carries (the run ID found in its log), then the
        // report's environment and generation time
        let report_defaults = RunInfo {
            environment: config.environment.clone(),
            date: Some(
                merged_report
                    .generated_at
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            ),
            ..Default::default()
        }
        .normalized();
        let configured = config.run.clone().normalized();
        for result in &mut merged_report.test_cases {
            let own = std::mem::take(&mut result.run).normalized();
            let mut stamped = configured.clone();
            stamped.fill_missing(&own);
            stamped.fill_missing(&report_defaults);
            result.run = stamped;
        }

        // Calculate execution duration
        let execution_duration = match (earliest_timestamp, latest_timestamp) {
            (Some(earliest), Some(latest)) if reports.len() > 1 => {
                // Calculate difference in seconds as f64
                let duration = latest.signed_duration_since(earliest);
                duration.num_milliseconds() as f64 / 1000.0
            }
            _ => 0.0,
        };

        let container_report = ContainerReport::from_batch_report(
            merged_report,
            config.title,
            config.project,
            config.environment,
            config.platform,
            config.executor,
            execution_duration,
        );

        match format.to_lowercase().as_str() {
            "yaml" => testcase_common::to_yaml_string(&container_report)
                .context("Failed to serialize container report to YAML"),
            "json" => serde_json::to_string_pretty(&container_report)
                .context("Failed to serialize container report to JSON"),
            _ => anyhow::bail!("Unsupported format: {}. Use 'yaml' or 'json'.", format),
        }
    }
}

/// The `source_yaml_sha256` of each entry of a JSON execution log, by position. The key is not
/// part of the typed entry, so it is read from the raw JSON.
fn raw_source_hashes(content: &str) -> Vec<Option<String>> {
    serde_json::from_str::<Vec<serde_json::Value>>(content)
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    entry
                        .get("source_yaml_sha256")
                        .and_then(|hash| hash.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_content() {
        let verifier = TestVerifier::with_exact_matching();

        let log_content = r#"
[2024-01-15T10:30:00Z] TestCase: TC001, Sequence: 1, Step: 1, Success: true, Result: SW=0x9000, Output: Success
[2024-01-15T10:30:01Z] TestCase: TC001, Sequence: 1, Step: 2, Success: true, Result: OK, Output: Done
"#;

        let logs = verifier
            .parse_log_content(log_content, Path::new("test.log"))
            .unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].test_case_id, "TC001");
        assert_eq!(logs[0].sequence_id, 1);
        assert_eq!(logs[0].step_number, 1);
        assert_eq!(logs[0].success, Some(true));
    }

    // Note: Internal verification tests are covered by tests/verification_test.rs

    #[test]
    fn test_generate_report_yaml() {
        let verifier = TestVerifier::with_exact_matching();

        let result = TestCaseVerificationResult {
            test_case_id: "TC001".to_string(),
            description: "Test Case 1".to_string(),
            sequences: vec![],
            total_steps: 5,
            passed_steps: 4,
            failed_steps: 1,
            not_executed_steps: 0,
            overall_pass: false,
            requirement: Some("REQ001".to_string()),
            item: Some(1),
            tc: Some(1),
            source_yaml_sha256: None,
            run: Default::default(),
        };

        let yaml = verifier.generate_report_yaml(&result).unwrap();
        assert!(yaml.contains("test_case_id: TC001"));
        assert!(yaml.contains("description: Test Case 1"));
        assert!(yaml.contains("total_steps: 5"));
        assert!(yaml.contains("passed_steps: 4"));
        assert!(yaml.contains("failed_steps: 1"));
        assert!(yaml.contains("overall_pass: false"));
        assert!(yaml.contains("requirement: REQ001"));
    }

    #[test]
    fn test_generate_report_json() {
        let verifier = TestVerifier::with_exact_matching();

        let result = TestCaseVerificationResult {
            test_case_id: "TC002".to_string(),
            description: "Test Case 2".to_string(),
            sequences: vec![],
            total_steps: 3,
            passed_steps: 3,
            failed_steps: 0,
            not_executed_steps: 0,
            overall_pass: true,
            requirement: Some("REQ002".to_string()),
            item: Some(2),
            tc: Some(2),
            source_yaml_sha256: None,
            run: Default::default(),
        };

        let json = verifier.generate_report_json(&result).unwrap();
        assert!(json.contains("\"test_case_id\": \"TC002\""));
        assert!(json.contains("\"description\": \"Test Case 2\""));
        assert!(json.contains("\"total_steps\": 3"));
        assert!(json.contains("\"passed_steps\": 3"));
        assert!(json.contains("\"failed_steps\": 0"));
        assert!(json.contains("\"overall_pass\": true"));
        assert!(json.contains("\"requirement\": \"REQ002\""));

        // Verify it can be deserialized
        let parsed: TestCaseVerificationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.test_case_id, "TC002");
        assert_eq!(parsed.total_steps, 3);
        assert!(parsed.overall_pass);
    }

    #[test]
    fn test_generate_report_single_report() {
        let verifier = TestVerifier::with_exact_matching();

        let mut report = BatchVerificationReport::new();
        report.add_test_case_result(TestCaseVerificationResult {
            test_case_id: "TC001".to_string(),
            description: "Test 1".to_string(),
            sequences: vec![],
            total_steps: 2,
            passed_steps: 2,
            failed_steps: 0,
            not_executed_steps: 0,
            overall_pass: true,
            requirement: Some("REQ001".to_string()),
            item: Some(1),
            tc: Some(1),
            source_yaml_sha256: None,
            run: Default::default(),
        });

        let config = ContainerReportConfig {
            title: "Test Report".to_string(),
            project: "Test Project".to_string(),
            environment: Some("Test Env".to_string()),
            platform: Some("Test Platform".to_string()),
            executor: Some("Test Executor".to_string()),
            run: Default::default(),
        };
        let yaml = verifier.generate_report(&[report], "yaml", config).unwrap();

        // Verify YAML structure
        assert!(yaml.contains("type: test_results_container"));
        assert!(yaml.contains("schema: tcms/test-results-container.schema.v1.json"));
        assert!(yaml.contains("title: Test Report"));
        assert!(yaml.contains("project: Test Project"));
        assert!(yaml.contains("test_date:"));
        assert!(yaml.contains("test_results:"));
        assert!(yaml.contains("metadata:"));
        assert!(yaml.contains("environment: Test Env"));
        assert!(yaml.contains("platform: Test Platform"));
        assert!(yaml.contains("executor: Test Executor"));
        assert!(yaml.contains("execution_duration: 0"));
        assert!(yaml.contains("total_test_cases: 1"));
        assert!(yaml.contains("passed_test_cases: 1"));

        // Verify it can be deserialized
        let parsed: ContainerReport = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.doc_type, Some("test_results_container".to_string()));
        assert_eq!(
            parsed.schema,
            Some("tcms/test-results-container.schema.v1.json".to_string())
        );
        assert_eq!(parsed.title, "Test Report");
        assert_eq!(parsed.project, "Test Project");
        assert_eq!(parsed.metadata.execution_duration, 0.0);
        assert_eq!(parsed.metadata.total_test_cases, 1);
    }

    #[test]
    fn test_generate_report_multiple_reports() {
        use chrono::Duration;

        let verifier = TestVerifier::with_exact_matching();

        let base_time = Local::now().with_timezone(&Utc);

        let mut report1 = BatchVerificationReport::new();
        report1.generated_at = base_time;
        report1.add_test_case_result(TestCaseVerificationResult {
            test_case_id: "TC001".to_string(),
            description: "Test 1".to_string(),
            sequences: vec![],
            total_steps: 2,
            passed_steps: 2,
            failed_steps: 0,
            not_executed_steps: 0,
            overall_pass: true,
            requirement: Some("REQ001".to_string()),
            item: Some(1),
            tc: Some(1),
            source_yaml_sha256: None,
            run: Default::default(),
        });

        let mut report2 = BatchVerificationReport::new();
        report2.generated_at = base_time + Duration::seconds(100);
        report2.add_test_case_result(TestCaseVerificationResult {
            test_case_id: "TC002".to_string(),
            description: "Test 2".to_string(),
            sequences: vec![],
            total_steps: 3,
            passed_steps: 3,
            failed_steps: 0,
            not_executed_steps: 0,
            overall_pass: true,
            requirement: Some("REQ002".to_string()),
            item: Some(2),
            tc: Some(2),
            source_yaml_sha256: None,
            run: Default::default(),
        });

        let config = ContainerReportConfig {
            title: "Multi-Report Test".to_string(),
            project: "Test Project".to_string(),
            environment: None,
            platform: None,
            executor: None,
            run: Default::default(),
        };
        let yaml = verifier
            .generate_report(&[report1, report2], "yaml", config)
            .unwrap();

        // Verify envelope fields are present
        assert!(yaml.contains("type: test_results_container"));
        assert!(yaml.contains("schema: tcms/test-results-container.schema.v1.json"));

        // Verify execution duration is calculated (100 seconds)
        assert!(yaml.contains("execution_duration: 100"));
        assert!(yaml.contains("total_test_cases: 2"));
        assert!(yaml.contains("passed_test_cases: 2"));

        // Verify it can be deserialized
        let parsed: ContainerReport = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.doc_type, Some("test_results_container".to_string()));
        assert_eq!(
            parsed.schema,
            Some("tcms/test-results-container.schema.v1.json".to_string())
        );
        assert_eq!(parsed.title, "Multi-Report Test");
        assert_eq!(parsed.metadata.execution_duration, 100.0);
        assert_eq!(parsed.metadata.total_test_cases, 2);
        assert_eq!(parsed.test_results.len(), 2);
    }

    // ── verify_test_case unit tests ───────────────────────────────────────
    //
    // These tests exercise the core verification logic through four scenarios:
    //   1. All execution log steps are present and every value matches.
    //   2. Some steps have no log entry (NotExecuted).
    //   3. Some steps have log entries whose values do not match (Fail).
    //   4. The same step appears more than once in the log (repeated entries).
    //
    // All data is built in-memory — no files, no YAML fixtures.
    // Each test uses `TestVerifier::with_exact_matching()` so the comparison
    // rule is simple: expected string must equal actual string character-for-character.

    /// Returns a Step whose expected result and output are `result` and `output`.
    /// Uses `Exact` matching in all tests below (set on the verifier, not the step).
    fn make_step(step_num: i64, result: &str, output: &str) -> Step {
        Step::new(
            step_num,
            format!("Step {step_num}"),
            "echo test".to_string(),
            result.to_string(),
            output.to_string(),
        )
    }

    /// Returns a TestSequence containing `steps`.
    fn make_sequence(id: i64, steps: Vec<Step>) -> TestSequence {
        let mut seq =
            TestSequence::new(id, format!("Sequence {id}"), format!("Test sequence {id}"));
        seq.steps = steps;
        seq
    }

    /// Returns a minimal TestCase with the given `id` and `sequences`.
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

    /// Returns a TestExecutionLog entry for a step that ran and produced
    /// `actual_result` and `actual_output`.  The `success` flag is derived
    /// from whether `actual_result` equals `"0"` (the conventional exit code
    /// used in all tests below).
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

    // ── Scenario 1: all steps present and passed ──────────────────────────

    /// Three steps are defined in the test case.
    /// The execution log contains matching entries for all three steps, and
    /// every actual value is identical to the expected value.
    ///
    /// Verification outcome
    ///   overall_pass        = true
    ///   total / passed / failed / not_executed = 3 / 3 / 0 / 0
    ///   step 1, 2, 3        → Pass
    #[test]
    fn test_verify_test_case_all_steps_present_and_passed() {
        let tc = make_test_case(
            "TC_ALL_PASS",
            vec![make_sequence(
                1,
                vec![
                    make_step(1, "0", "hello"),
                    make_step(2, "0", "world"),
                    make_step(3, "0", "done"),
                ],
            )],
        );

        let logs = vec![
            make_log("TC_ALL_PASS", 1, 1, "0", "hello"),
            make_log("TC_ALL_PASS", 1, 2, "0", "world"),
            make_log("TC_ALL_PASS", 1, 3, "0", "done"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        assert!(
            result.overall_pass,
            "all steps matched — should be an overall pass"
        );
        assert_eq!(result.total_steps, 3);
        assert_eq!(result.passed_steps, 3);
        assert_eq!(result.failed_steps, 0);
        assert_eq!(result.not_executed_steps, 0);

        let steps = &result.sequences[0].step_results;
        assert!(
            matches!(steps[0], StepVerificationResultEnum::Pass { .. }),
            "step 1 should be Pass"
        );
        assert!(
            matches!(steps[1], StepVerificationResultEnum::Pass { .. }),
            "step 2 should be Pass"
        );
        assert!(
            matches!(steps[2], StepVerificationResultEnum::Pass { .. }),
            "step 3 should be Pass"
        );
    }

    // ── Scenario 2: some steps have no log entry (NotExecuted) ───────────

    /// Three steps are defined in the test case.
    /// The execution log only contains entries for steps 1 and 3 — step 2
    /// was never executed (e.g. the run was interrupted before reaching it).
    ///
    /// Because a missing step could hide a real failure, the verifier treats
    /// it as a hard error: overall_pass = false even though no step *failed*.
    ///
    /// Verification outcome
    ///   overall_pass        = false   (missing steps prevent a pass)
    ///   total / passed / failed / not_executed = 3 / 2 / 0 / 1
    ///   step 1 → Pass, step 2 → NotExecuted, step 3 → Pass
    #[test]
    fn test_verify_test_case_some_steps_missing() {
        let tc = make_test_case(
            "TC_MISSING",
            vec![make_sequence(
                1,
                vec![
                    make_step(1, "0", "first"),
                    make_step(2, "0", "second"), // no log entry for this step
                    make_step(3, "0", "third"),
                ],
            )],
        );

        // Deliberately omit the log entry for step 2.
        let logs = vec![
            make_log("TC_MISSING", 1, 1, "0", "first"),
            // no entry for step 2
            make_log("TC_MISSING", 1, 3, "0", "third"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        assert!(
            !result.overall_pass,
            "a missing step must cause an overall failure"
        );
        assert_eq!(result.total_steps, 3);
        assert_eq!(result.passed_steps, 2);
        assert_eq!(result.failed_steps, 0);
        assert_eq!(result.not_executed_steps, 1);

        let steps = &result.sequences[0].step_results;
        assert!(
            matches!(steps[0], StepVerificationResultEnum::Pass { .. }),
            "step 1 should be Pass"
        );
        assert!(
            matches!(steps[1], StepVerificationResultEnum::NotExecuted { .. }),
            "step 2 should be NotExecuted — it has no log entry"
        );
        assert!(
            matches!(steps[2], StepVerificationResultEnum::Pass { .. }),
            "step 3 should be Pass"
        );
    }

    // ── Scenario 3: some steps failed ────────────────────────────────────

    /// All 3 steps ran and have log entries, but step 2's actual_result is
    /// "1" while the expected result is "0" — the exit codes disagree.
    ///
    /// Verification outcome
    ///   overall_pass        = false
    ///   total / passed / failed / not_executed = 3 / 2 / 1 / 0
    ///   step 1 → Pass, step 2 → Fail (result mismatch), step 3 → Pass
    #[test]
    fn test_verify_test_case_some_steps_failed_result_mismatch() {
        let tc = make_test_case(
            "TC_FAIL_RESULT",
            vec![make_sequence(
                1,
                vec![
                    make_step(1, "0", "ok"),
                    make_step(2, "0", "ok"), // expects result "0"
                    make_step(3, "0", "ok"),
                ],
            )],
        );

        let logs = vec![
            make_log("TC_FAIL_RESULT", 1, 1, "0", "ok"),
            make_log("TC_FAIL_RESULT", 1, 2, "1", "ok"), // actual result "1" ≠ expected "0"
            make_log("TC_FAIL_RESULT", 1, 3, "0", "ok"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        assert!(
            !result.overall_pass,
            "a failing step must cause an overall failure"
        );
        assert_eq!(result.total_steps, 3);
        assert_eq!(result.passed_steps, 2);
        assert_eq!(result.failed_steps, 1);
        assert_eq!(result.not_executed_steps, 0);

        let steps = &result.sequences[0].step_results;
        assert!(
            matches!(steps[0], StepVerificationResultEnum::Pass { .. }),
            "step 1 should be Pass"
        );
        assert!(
            matches!(steps[1], StepVerificationResultEnum::Fail { .. }),
            "step 2 should be Fail — actual result \"1\" does not match expected \"0\""
        );
        assert!(
            matches!(steps[2], StepVerificationResultEnum::Pass { .. }),
            "step 3 should be Pass"
        );
    }

    /// All 3 steps ran and have log entries, but step 2's actual_output is
    /// "unexpected" while the expected output is "expected output".
    ///
    /// Verification outcome
    ///   overall_pass        = false
    ///   total / passed / failed / not_executed = 3 / 2 / 1 / 0
    ///   step 1 → Pass, step 2 → Fail (output mismatch), step 3 → Pass
    #[test]
    fn test_verify_test_case_some_steps_failed_output_mismatch() {
        let tc = make_test_case(
            "TC_FAIL_OUTPUT",
            vec![make_sequence(
                1,
                vec![
                    make_step(1, "0", "expected output"),
                    make_step(2, "0", "expected output"), // expects this exact string
                    make_step(3, "0", "expected output"),
                ],
            )],
        );

        let logs = vec![
            make_log("TC_FAIL_OUTPUT", 1, 1, "0", "expected output"),
            make_log("TC_FAIL_OUTPUT", 1, 2, "0", "unexpected"), // output differs
            make_log("TC_FAIL_OUTPUT", 1, 3, "0", "expected output"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        assert!(!result.overall_pass);
        assert_eq!(result.total_steps, 3);
        assert_eq!(result.passed_steps, 2);
        assert_eq!(result.failed_steps, 1);
        assert_eq!(result.not_executed_steps, 0);

        let steps = &result.sequences[0].step_results;
        assert!(
            matches!(steps[0], StepVerificationResultEnum::Pass { .. }),
            "step 1 should be Pass"
        );
        assert!(
            matches!(steps[1], StepVerificationResultEnum::Fail { .. }),
            "step 2 should be Fail — actual output does not match expected"
        );
        assert!(
            matches!(steps[2], StepVerificationResultEnum::Pass { .. }),
            "step 3 should be Pass"
        );
    }

    // ── Scenario 4: repeated step entries ────────────────────────────────
    //
    // The verifier builds a HashMap keyed by (sequence_id, step_number).
    // When the execution log contains multiple entries for the same step,
    // each insertion overwrites the previous one, so the LAST entry wins.

    /// The log has two entries for (sequence=1, step=1).
    /// The first entry would pass; the second entry has wrong values and fails.
    /// Because the second entry overwrites the first in the HashMap, step 1
    /// ends up being evaluated against the failing values.
    ///
    /// Verification outcome
    ///   overall_pass        = false
    ///   total / passed / failed / not_executed = 2 / 1 / 1 / 0
    ///   step 1 → Fail (second duplicate overwrote the first)
    ///   step 2 → Pass
    #[test]
    fn test_verify_test_case_repeated_step_last_entry_wins_and_it_fails() {
        let tc = make_test_case(
            "TC_DUP_FAIL",
            vec![make_sequence(
                1,
                vec![make_step(1, "0", "correct"), make_step(2, "0", "ok")],
            )],
        );

        let logs = vec![
            make_log("TC_DUP_FAIL", 1, 1, "0", "correct"), // first entry — would pass
            make_log("TC_DUP_FAIL", 1, 1, "1", "wrong"),   // second entry — overwrites first
            make_log("TC_DUP_FAIL", 1, 2, "0", "ok"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        // Step 1 is verified against the LAST log entry: result="1", output="wrong".
        // Both differ from expected ("0", "correct"), so it is a Fail.
        assert!(!result.overall_pass);
        assert_eq!(result.total_steps, 2);
        assert_eq!(result.passed_steps, 1);
        assert_eq!(result.failed_steps, 1);
        assert_eq!(result.not_executed_steps, 0);

        let steps = &result.sequences[0].step_results;
        assert!(
            matches!(steps[0], StepVerificationResultEnum::Fail { .. }),
            "step 1 should be Fail — the second (failing) duplicate overwrote the first"
        );
        assert!(
            matches!(steps[1], StepVerificationResultEnum::Pass { .. }),
            "step 2 should be Pass"
        );
    }

    /// The log has two entries for (sequence=1, step=1), and both have values
    /// that match the expected outcome.  Even though one is redundant, the
    /// last entry still wins — and since it also matches, the step passes.
    ///
    /// Verification outcome
    ///   overall_pass        = true
    ///   total / passed / failed / not_executed = 2 / 2 / 0 / 0
    #[test]
    fn test_verify_test_case_repeated_step_last_entry_wins_and_it_passes() {
        let tc = make_test_case(
            "TC_DUP_PASS",
            vec![make_sequence(
                1,
                vec![make_step(1, "0", "correct"), make_step(2, "0", "ok")],
            )],
        );

        let logs = vec![
            make_log("TC_DUP_PASS", 1, 1, "0", "correct"), // first duplicate — passes
            make_log("TC_DUP_PASS", 1, 1, "0", "correct"), // second duplicate — also passes
            make_log("TC_DUP_PASS", 1, 2, "0", "ok"),
        ];

        let verifier = TestVerifier::with_exact_matching();
        let result = verifier.verify_test_case(&tc, &logs);

        assert!(
            result.overall_pass,
            "when both duplicates match the expected values the test should still pass"
        );
        assert_eq!(result.total_steps, 2);
        assert_eq!(result.passed_steps, 2);
        assert_eq!(result.failed_steps, 0);
        assert_eq!(result.not_executed_steps, 0);
    }

    // ── end of verify_test_case unit tests ───────────────────────────────

    #[test]
    fn test_generate_report_json_format() {
        let verifier = TestVerifier::with_exact_matching();

        let mut report = BatchVerificationReport::new();
        report.add_test_case_result(TestCaseVerificationResult {
            test_case_id: "TC001".to_string(),
            description: "Test 1".to_string(),
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
        });

        let config = ContainerReportConfig {
            title: "JSON Test".to_string(),
            project: "JSON Project".to_string(),
            environment: Some("JSON Env".to_string()),
            platform: None,
            executor: None,
            run: Default::default(),
        };
        let json = verifier.generate_report(&[report], "json", config).unwrap();

        // Verify JSON structure
        assert!(json.contains("\"type\": \"test_results_container\""));
        assert!(json.contains("\"schema\": \"tcms/test-results-container.schema.v1.json\""));
        assert!(json.contains("\"title\": \"JSON Test\""));
        assert!(json.contains("\"project\": \"JSON Project\""));
        assert!(json.contains("\"test_date\""));
        assert!(json.contains("\"test_results\""));
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("\"environment\": \"JSON Env\""));
        assert!(json.contains("\"execution_duration\": 0"));

        // Verify it can be deserialized
        let parsed: ContainerReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.doc_type, Some("test_results_container".to_string()));
        assert_eq!(
            parsed.schema,
            Some("tcms/test-results-container.schema.v1.json".to_string())
        );
        assert_eq!(parsed.title, "JSON Test");
        assert_eq!(parsed.project, "JSON Project");
        assert_eq!(parsed.metadata.execution_duration, 0.0);
    }

    #[test]
    fn test_step_results_are_written_as_maps_and_read_in_both_forms() {
        let map_form = "\
sequence_id: 1
name: seq
step_results:
- Pass:
    step: 1
    description: first
- NotExecuted:
    step: 2
    description: second
all_steps_passed: false
";
        let tag_form = "\
sequence_id: 1
name: seq
step_results:
- !Pass
  step: 1
  description: first
- !NotExecuted
  step: 2
  description: second
all_steps_passed: false
";
        let from_map: SequenceVerificationResult = serde_yaml::from_str(map_form).unwrap();
        let from_tags: SequenceVerificationResult = serde_yaml::from_str(tag_form).unwrap();
        assert_eq!(from_map.step_results, from_tags.step_results);
        assert!(matches!(
            from_map.step_results[0],
            StepVerificationResultEnum::Pass { step: 1, .. }
        ));

        // typed serialization writes the map form, also without the output helper
        let written = serde_yaml::to_string(&from_tags).unwrap();
        assert!(!written.contains('!'), "tags written: {written}");
        assert!(written.contains("- Pass:"), "{written}");

        // and JSON keeps the externally tagged form
        let json = serde_json::to_value(&from_map).unwrap();
        assert!(json["step_results"][0].get("Pass").is_some(), "{json}");
        let back: SequenceVerificationResult = serde_json::from_value(json).unwrap();
        assert_eq!(back.step_results, from_map.step_results);
    }

    fn result_with_run(id: &str, run: RunInfo) -> TestCaseVerificationResult {
        TestCaseVerificationResult {
            test_case_id: id.to_string(),
            description: "d".to_string(),
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
            run,
        }
    }

    fn stamped_results(
        results: Vec<TestCaseVerificationResult>,
        environment: Option<&str>,
        run: RunInfo,
    ) -> Vec<TestCaseVerificationResult> {
        let verifier = TestVerifier::with_exact_matching();
        let mut report = BatchVerificationReport::new();
        for result in results {
            report.add_test_case_result(result);
        }
        let config = ContainerReportConfig {
            title: "T".to_string(),
            project: "P".to_string(),
            environment: environment.map(str::to_string),
            platform: None,
            executor: None,
            run,
        };
        let yaml = verifier.generate_report(&[report], "yaml", config).unwrap();
        serde_yaml::from_str::<ContainerReport>(&yaml)
            .unwrap()
            .test_results
    }

    #[test]
    fn test_run_info_is_stamped_on_every_result() {
        let results = stamped_results(
            vec![
                result_with_run("TC1", RunInfo::default()),
                result_with_run("TC2", RunInfo::default()),
            ],
            Some("Staging"),
            RunInfo {
                run_id: Some("RUN-1".to_string()),
                build_id: Some("b-9".to_string()),
                tester: Some("alice".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(results.len(), 2);
        for result in &results {
            assert_eq!(result.run.run_id.as_deref(), Some("RUN-1"));
            assert_eq!(result.run.build_id.as_deref(), Some("b-9"));
            assert_eq!(result.run.tester.as_deref(), Some("alice"));
            // environment and date default to the report's
            assert_eq!(result.run.environment.as_deref(), Some("Staging"));
            let date = result.run.date.as_deref().unwrap();
            assert!(
                chrono::DateTime::parse_from_rfc3339(date).is_ok(),
                "date is not RFC 3339: {date}"
            );
        }
    }

    #[test]
    fn test_run_info_precedence_configured_then_result_then_report() {
        let own = RunInfo {
            run_id: Some("FROM-LOG".to_string()),
            build_id: Some("own-build".to_string()),
            ..Default::default()
        };
        let results = stamped_results(
            vec![result_with_run("TC1", own)],
            None,
            RunInfo {
                run_id: Some("CONFIGURED".to_string()),
                ..Default::default()
            },
        );
        // configured wins over the result's own value; the result's own survives otherwise
        assert_eq!(results[0].run.run_id.as_deref(), Some("CONFIGURED"));
        assert_eq!(results[0].run.build_id.as_deref(), Some("own-build"));
        assert_eq!(results[0].run.environment, None);
    }

    #[test]
    fn test_absent_and_empty_run_info_is_omitted_not_written_empty() {
        let verifier = TestVerifier::with_exact_matching();
        let mut report = BatchVerificationReport::new();
        report.add_test_case_result(result_with_run("TC1", RunInfo::default()));
        let config = ContainerReportConfig {
            title: "T".to_string(),
            project: "P".to_string(),
            environment: Some("  ".to_string()),
            platform: None,
            executor: None,
            run: RunInfo {
                run_id: Some(String::new()),
                build_id: Some("   ".to_string()),
                ..Default::default()
            },
        };
        let yaml = verifier.generate_report(&[report], "yaml", config).unwrap();
        let results = serde_yaml::from_str::<ContainerReport>(&yaml)
            .unwrap()
            .test_results;
        let run = &results[0].run;
        assert_eq!(
            (&run.run_id, &run.build_id, &run.tester, &run.environment),
            (&None, &None, &None, &None)
        );
        for key in ["run_id:", "build_id:", "tester:"] {
            assert!(!yaml.contains(key), "{key} written: {yaml}");
        }
        // the date is always known
        assert!(run.date.is_some());
    }
}
