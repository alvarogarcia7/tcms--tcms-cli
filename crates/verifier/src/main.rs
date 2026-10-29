use anyhow::{Context, Result};
use clap::Parser;
use jsonschema::JSONSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use testcase_common::load_and_validate_yaml;
use testcase_models::TestCase;
use testcase_storage::TestCaseStorage;
use testcase_verification::{
    check_run_independence, BatchVerificationReport, CaseIndependence, ContainerReportConfig,
    MatchStrategy, RunInfo, StepVerificationResultEnum, TestCaseVerificationResult,
    TestExecutionLog, TestRunInput, TestRunRecord, TestVerifier,
};

/// Default schemas root directory
const SCHEMAS_ROOT: &str = "schemas";

/// Storage-aware TestVerifier that wraps the core TestVerifier from testcase-verification
/// and provides storage integration
struct StorageTestVerifier {
    storage: TestCaseStorage,
    verifier: TestVerifier,
}

impl StorageTestVerifier {
    /// Create a new verifier with storage and custom match strategies
    fn with_strategies(
        storage: TestCaseStorage,
        result_strategy: MatchStrategy,
        output_strategy: MatchStrategy,
    ) -> Self {
        Self {
            storage,
            verifier: TestVerifier::new(result_strategy, output_strategy),
        }
    }

    /// Get a reference to the test case storage
    fn storage(&self) -> &TestCaseStorage {
        &self.storage
    }

    /// Generate a container report from multiple batch reports
    fn generate_report(
        &self,
        reports: &[BatchVerificationReport],
        format: &str,
        config: ContainerReportConfig,
    ) -> Result<String> {
        self.verifier.generate_report(reports, format, config)
    }

    /// Parse a test execution log file
    fn parse_log_file<P: AsRef<Path>>(&self, log_path: P) -> Result<Vec<TestExecutionLog>> {
        self.verifier.parse_log_file(log_path)
    }

    /// Parse a test execution log file with a specified test case ID
    fn parse_log_file_with_test_case_id<P: AsRef<Path>>(
        &self,
        log_path: P,
        test_case_id: &str,
    ) -> Result<Vec<TestExecutionLog>> {
        self.verifier
            .parse_log_file_with_test_case_id(log_path, test_case_id)
    }

    /// Verify a single test case against execution logs
    fn verify_test_case(
        &self,
        test_case: &TestCase,
        execution_logs: &[TestExecutionLog],
    ) -> TestCaseVerificationResult {
        self.verifier.verify_test_case(test_case, execution_logs)
    }
}

/// Configuration for the verifier report output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct VerifierConfig {
    /// Schema reference for envelope support
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<String>,

    /// Report title
    title: String,

    /// Project name
    project: String,

    /// Environment information
    environment: Option<String>,

    /// Platform information
    platform: Option<String>,

    /// Executor information
    executor: Option<String>,

    /// Test run the results belong to
    run_id: Option<String>,

    /// Build/software identifier under test
    build_id: Option<String>,

    /// Person or tool that executed the tests
    tester: Option<String>,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            schema: None,
            title: "Test Execution Results".to_string(),
            project: "Test Case Manager - Verification Results".to_string(),
            environment: None,
            platform: None,
            executor: None,
            run_id: None,
            build_id: None,
            tester: None,
        }
    }
}

impl VerifierConfig {
    /// Load configuration from a YAML file with schema validation
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        load_and_validate_yaml(path, SCHEMAS_ROOT)
            .context("Failed to load and validate configuration file")
    }

    /// Apply CLI overrides to the configuration
    fn apply_cli_overrides(
        &mut self,
        title: Option<String>,
        project: Option<String>,
        environment: Option<String>,
        platform: Option<String>,
        executor: Option<String>,
    ) {
        if let Some(t) = title {
            self.title = t;
        }
        if let Some(p) = project {
            self.project = p;
        }
        // Empty and blank values count as absent (TCMS-40.3): a blank flag keeps the config
        // value, and a blank config value is dropped rather than written empty.
        if let Some(e) = environment
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
        {
            self.environment = Some(e);
        }
        self.environment = self.environment.take().filter(|v| !v.trim().is_empty());
        if platform.is_some() {
            self.platform = platform;
        }
        if executor.is_some() {
            self.executor = executor;
        }
    }

    /// Apply the run metadata. Precedence: flag, then `TCMS_*` environment variable, then the
    /// configuration file. Empty values count as absent.
    fn apply_run_metadata(
        &mut self,
        run_id: Option<String>,
        build_id: Option<String>,
        tester: Option<String>,
    ) {
        fn pick(flag: Option<String>, env_name: &str, config: &Option<String>) -> Option<String> {
            let non_empty = |v: String| Some(v.trim().to_string()).filter(|v| !v.is_empty());
            flag.and_then(non_empty)
                .or_else(|| std::env::var(env_name).ok().and_then(non_empty))
                .or_else(|| config.clone().and_then(non_empty))
        }
        self.run_id = pick(run_id, "TCMS_RUN_ID", &self.run_id);
        self.build_id = pick(build_id, "TCMS_BUILD_ID", &self.build_id);
        self.tester = pick(tester, "TCMS_TESTER", &self.tester);
    }
}

#[derive(Parser)]
#[command(name = "verifier")]
#[command(version)]
#[command(about = "Verify test execution logs against test case definitions")]
#[command(
    after_help = "ENVIRONMENT VARIABLES:\n    RUST_LOG    Set log level (trace, debug, info, warn, error). Overrides --log-level"
)]
struct Cli {
    /// Single-file mode: path to log file
    #[arg(short, long, value_name = "PATH")]
    log: Option<PathBuf>,

    /// Single-file mode: test case ID to verify against
    #[arg(short = 'c', long = "test-case", value_name = "ID")]
    test_case_id: Option<String>,

    /// Folder discovery mode: path to folder containing log files
    #[arg(short = 'f', long, value_name = "PATH")]
    folder: Option<PathBuf>,

    /// Output format (yaml or json)
    #[arg(short = 'F', long, default_value = "yaml", value_name = "FORMAT")]
    format: String,

    /// Output file path (optional, defaults to stdout)
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Path to test case storage directory
    #[arg(short = 'd', long, default_value = "testcases", value_name = "DIR")]
    test_case_dir: PathBuf,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: String,

    /// Enable verbose output (equivalent to --log-level=debug)
    #[arg(short, long)]
    verbose: bool,

    /// Path to configuration file (YAML format)
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Report title (overrides config file)
    #[arg(long)]
    title: Option<String>,

    /// Project name (overrides config file)
    #[arg(long)]
    project: Option<String>,

    /// Environment information (overrides config file)
    #[arg(long)]
    environment: Option<String>,

    /// Platform information (overrides config file)
    #[arg(long)]
    platform: Option<String>,

    /// Executor information (overrides config file)
    #[arg(long)]
    executor: Option<String>,

    /// Test run ID stamped on every result (overrides TCMS_RUN_ID and the config file)
    #[arg(long, value_name = "ID")]
    run_id: Option<String>,

    /// Build identifier stamped on every result (overrides TCMS_BUILD_ID and the config file)
    #[arg(long, value_name = "ID")]
    build_id: Option<String>,

    /// Tester stamped on every result (overrides TCMS_TESTER and the config file)
    #[arg(long, value_name = "NAME")]
    tester: Option<String>,

    /// Write the test run record (tcms-schemas test-run) to PATH. By default it is written
    /// whenever the run ID, build ID, environment and tester are known, next to --output as
    /// <name>.test-run.yaml (test-run.yaml in the current directory without --output); giving
    /// PATH makes those four required
    #[arg(long, value_name = "PATH")]
    test_run: Option<PathBuf>,

    /// Do not write the test run record
    #[arg(long, conflicts_with = "test_run")]
    no_test_run: bool,

    /// Who checked the results, recorded in the test run (overrides TCMS_REVIEWER)
    #[arg(long, value_name = "NAME")]
    reviewer: Option<String>,

    /// Attach a problem report to a failed or errored test case of the test run, as
    /// TEST_CASE_ID=PROBLEM_REPORT_ID (repeatable)
    #[arg(long, value_name = "TC=PR")]
    problem_report: Vec<String>,

    /// Match strategy for verification (exact, regex, contains, or precomputed)
    #[arg(
        short = 'm',
        long = "match-strategy",
        default_value = "exact",
        value_name = "STRATEGY"
    )]
    match_strategy: String,

    /// Path to JSON schema file for validating output
    #[arg(
        long = "schema",
        default_value = "data/testcase_results_container/schema.json",
        value_name = "PATH"
    )]
    schema_path: PathBuf,

    /// Exit with 0 even if test cases failed (useful for acceptance testing)
    #[arg(long = "success-on-completion")]
    success_on_completion: bool,

    /// Verify source YAML hash from execution logs
    #[arg(long = "verify-source-hash")]
    verify_source_hash: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { &cli.log_level };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    // Validate CLI arguments
    let (mode, log_path, test_case_id, folder_path) = validate_args(&cli)?;

    // Parse match strategy
    let match_strategy = parse_match_strategy(&cli.match_strategy)?;

    // Load configuration
    let config = load_configuration(&cli)?;

    // Initialize storage and verifier
    let storage = TestCaseStorage::new(&cli.test_case_dir)
        .context("Failed to initialize test case storage")?;
    let verifier = StorageTestVerifier::with_strategies(storage, match_strategy, match_strategy);

    // The logs behind the report, for the test run record
    let log_sources: Vec<(String, PathBuf)> = match (&mode, &log_path, &test_case_id, &folder_path)
    {
        (Mode::SingleFile, Some(log), Some(id), _) => vec![(id.clone(), log.clone())],
        (Mode::FolderDiscovery, _, _, Some(folder)) => discover_log_files(folder)?
            .into_iter()
            .map(|log| (extract_test_case_id_from_filename(&log), log))
            .collect(),
        _ => Vec::new(),
    };

    // Execute appropriate mode
    let (report, skipped_logs) = match mode {
        Mode::SingleFile => {
            // Safe unwraps: validate_args guarantees these are Some when mode is SingleFile
            let log_path = log_path.expect("log_path must be Some for SingleFile mode");
            let test_case_id = test_case_id.expect("test_case_id must be Some for SingleFile mode");
            (
                handle_single_file_mode(
                    &verifier,
                    &log_path,
                    &test_case_id,
                    cli.verify_source_hash,
                )?,
                0,
            )
        }
        Mode::FolderDiscovery => {
            // Safe unwrap: validate_args guarantees this is Some when mode is FolderDiscovery
            let folder_path =
                folder_path.expect("folder_path must be Some for FolderDiscovery mode");
            handle_folder_mode(&verifier, &folder_path, cli.verify_source_hash)?
        }
    };

    // Log summary of failures
    log_verification_errors(&report);

    // Generate output in requested format
    let output_content =
        generate_output(&verifier, &report, &cli.format, &config, &cli.schema_path)?;

    // Write to file or stdout
    write_output(&output_content, cli.output.as_ref())?;

    // The test run record is written by default; --test-run, --reviewer and --problem-report
    // ask for it, so for them missing run metadata is an error and not a reason to skip
    if !cli.no_test_run {
        let requested =
            cli.test_run.is_some() || cli.reviewer.is_some() || !cli.problem_report.is_empty();
        let test_run_path = cli
            .test_run
            .clone()
            .unwrap_or_else(|| default_test_run_path(cli.output.as_deref()));
        write_test_run(
            &cli,
            &config,
            &verifier,
            &report,
            &log_sources,
            &test_run_path,
            requested,
        )?;
    }

    // A log that could not be verified is an error, whatever --success-on-completion says: the
    // report does not contain its test case, so a clean exit would hide it
    if skipped_logs > 0 {
        anyhow::bail!(
            "{skipped_logs} execution log(s) could not be verified and are missing from the report"
        );
    }

    // A case whose hook failed errored: its steps may all have passed, but the run did not finish
    // cleanly, so it is a failure of the run (decision D-41e)
    let errored = collect_log_facts(&log_sources).errored;
    if !errored.is_empty() {
        log::error!(
            "Test case(s) errored, a hook failed: {}",
            errored.iter().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    // Exit with non-zero code if any tests failed or errored (unless --success-on-completion is set)
    if !cli.success_on_completion && (report.failed_test_cases > 0 || !errored.is_empty()) {
        std::process::exit(1);
    }

    Ok(())
}

#[derive(Debug)]
enum Mode {
    SingleFile,
    FolderDiscovery,
}

type ValidationResult = (Mode, Option<PathBuf>, Option<String>, Option<PathBuf>);

fn validate_args(cli: &Cli) -> Result<ValidationResult> {
    // Check for single-file mode
    let single_file_mode = cli.log.is_some() && cli.test_case_id.is_some();
    // Check for folder discovery mode
    let folder_mode = cli.folder.is_some();

    // Validate mutually exclusive modes
    if !single_file_mode && !folder_mode {
        anyhow::bail!("Must specify either single-file mode (--log and --test-case) or folder discovery mode (--folder)");
    }

    if single_file_mode && folder_mode {
        anyhow::bail!("Cannot use both single-file mode (--log/--test-case) and folder mode (--folder) simultaneously");
    }

    // Validate format
    let format = cli.format.to_lowercase();
    if format != "yaml" && format != "json" {
        anyhow::bail!("Format must be 'yaml' or 'json', got: {}", format);
    }

    if single_file_mode {
        // Safe to unwrap: single_file_mode is true only if both log and test_case_id are Some
        let log_path = cli
            .log
            .as_ref()
            .expect("log should be Some in single-file mode");
        let test_case_id = cli
            .test_case_id
            .as_ref()
            .expect("test_case_id should be Some in single-file mode");

        if !log_path.exists() {
            anyhow::bail!("Log file does not exist: {}", log_path.display());
        }

        Ok((
            Mode::SingleFile,
            Some(log_path.clone()),
            Some(test_case_id.clone()),
            None,
        ))
    } else {
        // Safe to unwrap: folder_mode is true only if folder is Some
        let folder_path = cli
            .folder
            .as_ref()
            .expect("folder should be Some in folder mode");

        if !folder_path.exists() {
            anyhow::bail!("Folder does not exist: {}", folder_path.display());
        }

        if !folder_path.is_dir() {
            anyhow::bail!("Path is not a directory: {}", folder_path.display());
        }

        Ok((Mode::FolderDiscovery, None, None, Some(folder_path.clone())))
    }
}

fn handle_single_file_mode(
    verifier: &StorageTestVerifier,
    log_path: &PathBuf,
    test_case_id: &str,
    verify_source_hash: bool,
) -> Result<BatchVerificationReport> {
    log::info!("Processing log file: {}", log_path.display());
    log::debug!("Single-file mode: test case ID = '{}'", test_case_id);

    // Parse log file with specified test case ID
    log::debug!("Parsing log file: {}", log_path.display());
    let logs = verifier
        .parse_log_file_with_test_case_id(log_path, test_case_id)
        .context("Failed to parse test execution log")?;

    log::debug!(
        "Successfully parsed log file with {} log entries",
        logs.len()
    );

    // Load test case
    log::debug!("Loading test case definition for ID: '{}'", test_case_id);
    let test_case = verifier
        .storage()
        .load_test_case_by_id(test_case_id)
        .context(format!("Failed to load test case: {}", test_case_id))?;

    log::debug!(
        "Successfully loaded test case '{}' with {} test sequences",
        test_case_id,
        test_case.test_sequences.len()
    );

    // Compute source hash and verify if requested
    let source_hash = compute_test_case_source_hash(verifier.storage(), test_case_id)?;
    if verify_source_hash {
        verify_hash_against_logs(&logs, test_case_id, &source_hash)?;
    }

    // Verify
    log::debug!("Verifying test case '{}' against logs", test_case_id);
    let mut result = verifier.verify_test_case(&test_case, &logs);
    result.source_yaml_sha256 = Some(source_hash);
    result.run.run_id = run_id_from_log(log_path);

    log::debug!(
        "Verification result: pass={}, steps={}/{}",
        result.overall_pass,
        result.passed_steps,
        result.total_steps
    );

    // Create batch report with single test case
    log::debug!("Creating batch report with single test case result");
    let mut report = BatchVerificationReport::new();
    report.add_test_case_result(result);

    log::debug!(
        "Batch report created: total={}, passed={}, failed={}",
        report.total_test_cases,
        report.passed_test_cases,
        report.failed_test_cases
    );

    Ok(report)
}

fn handle_folder_mode(
    verifier: &StorageTestVerifier,
    folder_path: &PathBuf,
    verify_source_hash: bool,
) -> Result<(BatchVerificationReport, usize)> {
    log::debug!(
        "Starting folder discovery mode for: {}",
        folder_path.display()
    );

    // Discover all log files in the folder recursively
    let log_files = discover_log_files(folder_path)?;

    if log_files.is_empty() {
        // Nothing was verified, so there is no verdict: an exit code of 0 would read as a pass
        anyhow::bail!(
            "No execution log files (*_execution_log.json) found in folder: {}",
            folder_path.display()
        );
    }

    log::info!(
        "Found {} execution log file(s) in {}",
        log_files.len(),
        folder_path.display()
    );

    log::debug!("Discovered log files:");
    for log_file in &log_files {
        log::debug!("  - {}", log_file.display());
    }

    // Process each log file individually for better logging
    // Logs that could not be verified at all (unreadable, unknown test case, wrong source hash)
    let mut skipped = 0usize;
    let mut report = BatchVerificationReport::new();
    log::debug!("Initialized empty batch verification report");

    for (idx, log_file) in log_files.iter().enumerate() {
        log::info!(
            "Processing log file {}/{}: {}",
            idx + 1,
            log_files.len(),
            log_file.display()
        );

        // Extract test case ID from filename
        let test_case_id = extract_test_case_id_from_filename(log_file);
        log::debug!("Extracted test case ID: '{}' from filename", test_case_id);

        log::debug!("Parsing log file: {}", log_file.display());
        match verifier.parse_log_file(log_file) {
            Ok(logs) => {
                log::debug!(
                    "Successfully parsed log file with {} log entries",
                    logs.len()
                );

                // Load test case
                log::debug!("Loading test case definition for ID: '{}'", test_case_id);
                match verifier.storage().load_test_case_by_id(&test_case_id) {
                    Ok(test_case) => {
                        log::debug!(
                            "Successfully loaded test case '{}' with {} test sequences",
                            test_case_id,
                            test_case.test_sequences.len()
                        );

                        // Compute source hash and verify if requested
                        let source_hash_result =
                            compute_test_case_source_hash(verifier.storage(), &test_case_id);
                        let source_hash = match source_hash_result {
                            Ok(hash) => {
                                if verify_source_hash {
                                    if let Err(e) =
                                        verify_hash_against_logs(&logs, &test_case_id, &hash)
                                    {
                                        log::error!(
                                            "Source hash verification failed for test case '{}': {}",
                                            test_case_id,
                                            e
                                        );
                                        skipped += 1;
                                        continue;
                                    }
                                }
                                Some(hash)
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to compute source hash for test case '{}': {}",
                                    test_case_id,
                                    e
                                );
                                None
                            }
                        };

                        log::debug!("Verifying test case '{}' against logs", test_case_id);

                        let mut result = verifier.verify_test_case(&test_case, &logs);
                        result.source_yaml_sha256 = source_hash;
                        result.run.run_id = logs
                            .first()
                            .and_then(|log| run_id_from_log(&log.log_file_path));

                        log::debug!(
                            "Verification result for '{}': pass={}, steps={}/{}",
                            test_case_id,
                            result.overall_pass,
                            result.passed_steps,
                            result.total_steps
                        );

                        report.add_test_case_result(result);
                        log::debug!("Added test case result to batch report. Current report stats: total={}, passed={}, failed={}",
                                   report.total_test_cases, report.passed_test_cases, report.failed_test_cases);
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to load test case '{}' for log file '{}': {}",
                            test_case_id,
                            log_file.display(),
                            e
                        );
                        skipped += 1;
                        log::debug!(
                            "Test case loading failed, skipping verification for this log file"
                        );
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to parse log file '{}': {}", log_file.display(), e);
                skipped += 1;
                log::debug!("Log file parsing failed, skipping this file");
            }
        }
    }

    log::debug!("Folder mode processing complete. Final batch report stats:");
    log::debug!("  Total test cases: {}", report.total_test_cases);
    log::debug!("  Passed test cases: {}", report.passed_test_cases);
    log::debug!("  Failed test cases: {}", report.failed_test_cases);
    log::debug!("  Total steps: {}", report.total_steps);
    log::debug!("  Passed steps: {}", report.passed_steps);
    log::debug!("  Failed steps: {}", report.failed_steps);

    Ok((report, skipped))
}

fn discover_log_files(folder_path: &PathBuf) -> Result<Vec<PathBuf>> {
    log::debug!(
        "Beginning recursive log file discovery in: {}",
        folder_path.display()
    );
    let mut log_files = Vec::new();
    discover_log_files_recursive(folder_path, &mut log_files)?;
    log::debug!(
        "Log file discovery complete. Found {} file(s)",
        log_files.len()
    );
    Ok(log_files)
}

fn discover_log_files_recursive(dir: &PathBuf, log_files: &mut Vec<PathBuf>) -> Result<()> {
    log::debug!("Scanning directory: {}", dir.display());

    let entries =
        fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?;

    let mut file_count = 0;
    let mut dir_count = 0;
    let mut skipped_count = 0;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Get metadata to check file type without following symlinks
        let metadata = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to read metadata for '{}': {}", path.display(), e);
                log::debug!("Skipping file due to metadata read error");
                skipped_count += 1;
                continue;
            }
        };

        // Skip symlinks to avoid potential infinite loops
        if metadata.is_symlink() {
            log::debug!("Skipping symlink: {}", path.display());
            skipped_count += 1;
            continue;
        }

        if metadata.is_dir() {
            log::debug!("Found subdirectory: {}", path.display());
            dir_count += 1;
            // Recursively search subdirectories
            discover_log_files_recursive(&path, log_files)?;
        } else if metadata.is_file() {
            file_count += 1;
            // Check if filename matches the pattern *_execution_log.json
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.ends_with("_execution_log.json") {
                    log::debug!("Found matching log file: {}", path.display());
                    log_files.push(path);
                } else {
                    log::debug!("Skipping non-matching file: {}", path.display());
                }
            }
        }
    }

    log::debug!(
        "Directory scan complete for {}: {} files, {} subdirs, {} skipped",
        dir.display(),
        file_count,
        dir_count,
        skipped_count
    );

    Ok(())
}

fn extract_test_case_id_from_filename(log_path: &Path) -> String {
    // Expected format: {test_case_id}_execution_log.json
    let test_case_id = log_path
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_suffix("_execution_log"))
        .unwrap_or("UNKNOWN")
        .to_string();

    log::debug!(
        "Extracting test case ID from filename '{}': '{}'",
        log_path.display(),
        test_case_id
    );

    test_case_id
}

fn log_verification_errors(report: &BatchVerificationReport) {
    for test_case in &report.test_cases {
        if !test_case.overall_pass {
            log::error!(
                "Test case '{}' FAILED ({}/{} steps passed)",
                test_case.test_case_id,
                test_case.passed_steps,
                test_case.total_steps
            );

            for sequence in &test_case.sequences {
                if !sequence.all_steps_passed {
                    log::error!(
                        "  Sequence {} '{}' FAILED",
                        sequence.sequence_id,
                        sequence.name
                    );

                    for step_result in &sequence.step_results {
                        if !step_result.is_pass() {
                            match step_result {
                                StepVerificationResultEnum::Fail {
                                    step,
                                    description,
                                    reason,
                                    ..
                                } => {
                                    log::error!(
                                        "    Step {} '{}' FAILED: {}",
                                        step,
                                        description,
                                        reason
                                    );
                                }
                                StepVerificationResultEnum::NotExecuted {
                                    step,
                                    description,
                                    ..
                                } => {
                                    log::error!("    Step {} '{}' NOT EXECUTED", step, description);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// What the execution logs say about a run: when it started and which cases were aborted
#[derive(Debug, Default)]
struct LogFacts {
    earliest: Option<chrono::DateTime<chrono::Utc>>,
    latest: Option<chrono::DateTime<chrono::Utc>>,
    run_id: Option<String>,
    errored: std::collections::BTreeSet<String>,
}

fn collect_log_facts(sources: &[(String, PathBuf)]) -> LogFacts {
    use chrono::TimeZone;
    let mut facts = LogFacts::default();
    for (test_case_id, path) in sources {
        let entries: Vec<serde_json::Value> = std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default();
        for entry in &entries {
            // test-executor writes local time without an offset
            let timestamp = entry
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(|t| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S").ok())
                .and_then(|naive| chrono::Local.from_local_datetime(&naive).earliest())
                .map(|local| local.with_timezone(&chrono::Utc));
            if let Some(timestamp) = timestamp {
                facts.earliest = Some(facts.earliest.map_or(timestamp, |e| e.min(timestamp)));
                facts.latest = Some(facts.latest.map_or(timestamp, |l| l.max(timestamp)));
            }
            // a failed hook ends the script: the case has no result, it errored
            let hook_failed = entry.get("hook_type").is_some_and(|h| !h.is_null())
                && entry.get("exit_code").and_then(|c| c.as_i64()) != Some(0);
            if hook_failed {
                facts.errored.insert(test_case_id.clone());
            }
        }
        if facts.run_id.is_none() {
            facts.run_id = run_id_from_log(path);
        }
    }
    facts
}

/// Where the test run record goes when --test-run is not given: next to the report as
/// `<name>.test-run.yaml`, else `test-run.yaml` in the current directory
fn default_test_run_path(output: Option<&Path>) -> PathBuf {
    match output {
        Some(output) => {
            let stem = output
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "report".to_string());
            output.with_file_name(format!("{stem}.test-run.yaml"))
        }
        None => PathBuf::from("test-run.yaml"),
    }
}

fn write_test_run(
    cli: &Cli,
    config: &VerifierConfig,
    verifier: &StorageTestVerifier,
    report: &BatchVerificationReport,
    log_sources: &[(String, PathBuf)],
    path: &PathBuf,
    requested: bool,
) -> Result<()> {
    let facts = collect_log_facts(log_sources);

    // By default a run without the metadata a test run needs simply has no record
    if !requested {
        let known = [
            ("run_id", config.run_id.is_some() || facts.run_id.is_some()),
            ("build_id", config.build_id.is_some()),
            ("environment", config.environment.is_some()),
            ("tester", config.tester.is_some()),
        ];
        let missing: Vec<&str> = known
            .iter()
            .filter(|(_, present)| !present)
            .map(|(name, _)| *name)
            .collect();
        if !missing.is_empty() {
            log::warn!(
                "No test run record written: missing {} (set them with --run-id, --build-id, \
                 --environment, --tester or the TCMS_* variables; --no-test-run silences this)",
                missing.join(", ")
            );
            return Ok(());
        }
    }

    let mut problem_reports = std::collections::BTreeMap::new();
    for assignment in &cli.problem_report {
        let (test_case_id, problem_report) = assignment.split_once('=').with_context(|| {
            format!("--problem-report expects TEST_CASE_ID=PROBLEM_REPORT_ID, got '{assignment}'")
        })?;
        problem_reports.insert(test_case_id.trim().to_string(), problem_report.to_string());
    }

    let reviewer = cli
        .reviewer
        .clone()
        .or_else(|| std::env::var("TCMS_REVIEWER").ok());
    let result_ref = cli.output.as_ref().map(|p| p.display().to_string());
    let input = TestRunInput {
        run: RunInfo {
            run_id: config.run_id.clone().or(facts.run_id),
            build_id: config.build_id.clone(),
            environment: config.environment.clone(),
            tester: config.tester.clone(),
            date: None,
        },
        reviewer: reviewer.as_deref(),
        started_at: facts.earliest,
        finished_at: facts.latest,
        problem_reports,
        errored: facts.errored,
        result_ref: result_ref.as_deref(),
    };

    let record = TestRunRecord::build(report, &input).map_err(|e| anyhow::anyhow!(e))?;

    // Independence: the reviewer of a test case that requires it must not be its author
    let mut cases = std::collections::BTreeMap::new();
    for result in &report.test_cases {
        if let Ok(test_case) = verifier
            .storage()
            .load_test_case_by_id(&result.test_case_id)
        {
            cases.insert(
                result.test_case_id.clone(),
                CaseIndependence {
                    required: test_case.independence_required == Some(true),
                    author: test_case.author.clone(),
                },
            );
        }
    }
    check_run_independence(&record, &cases)
        .map_err(|violations| anyhow::anyhow!("independence check failed:\n{violations}"))?;

    write_output(&record.to_yaml()?, Some(path))?;
    log::info!("Test run record written to {}", path.display());
    Ok(())
}

/// The run ID test-executor recorded in an execution log (first entry that has one)
fn run_id_from_log(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&content).ok()?;
    entries
        .iter()
        .find_map(|entry| entry.get("run_id")?.as_str())
        .map(|run_id| run_id.trim().to_string())
        .filter(|run_id| !run_id.is_empty())
}

fn load_configuration(cli: &Cli) -> Result<VerifierConfig> {
    // Start with defaults or load from config file if specified
    let mut config = if let Some(config_path) = &cli.config {
        log::info!("Loading configuration from: {}", config_path.display());
        VerifierConfig::load_from_file(config_path)?
    } else {
        VerifierConfig::default()
    };

    // Apply CLI overrides
    config.apply_cli_overrides(
        cli.title.clone(),
        cli.project.clone(),
        cli.environment.clone(),
        cli.platform.clone(),
        cli.executor.clone(),
    );
    config.apply_run_metadata(cli.run_id.clone(), cli.build_id.clone(), cli.tester.clone());

    log::debug!(
        "Final configuration: title='{}', project='{}'",
        config.title,
        config.project
    );

    Ok(config)
}

fn generate_output(
    verifier: &StorageTestVerifier,
    report: &BatchVerificationReport,
    format: &str,
    config: &VerifierConfig,
    schema_path: &PathBuf,
) -> Result<String> {
    // Always use container format with metadata
    let container_config = ContainerReportConfig {
        title: config.title.clone(),
        project: config.project.clone(),
        environment: config.environment.clone(),
        platform: config.platform.clone(),
        executor: config.executor.clone(),
        run: RunInfo {
            run_id: config.run_id.clone(),
            build_id: config.build_id.clone(),
            tester: config.tester.clone(),
            ..Default::default()
        },
    };
    let output = verifier
        .generate_report(std::slice::from_ref(report), format, container_config)
        .context("Failed to generate container report")?;

    // Validate the output against the schema
    validate_output_against_schema(&output, format, schema_path)?;

    Ok(output)
}

/// Validate generated output against the JSON schema
fn validate_output_against_schema(output: &str, format: &str, schema_path: &PathBuf) -> Result<()> {
    // Check if schema file exists
    if !schema_path.exists() {
        anyhow::bail!(
            "Schema file not found at {}. Please ensure the schema file exists or specify a valid path using --schema",
            schema_path.display()
        );
    }

    log::debug!("Loading schema from {}", schema_path.display());
    let schema_content = fs::read_to_string(schema_path).context(format!(
        "Failed to read schema file: {}",
        schema_path.display()
    ))?;

    let schema_json: serde_json::Value =
        serde_json::from_str(&schema_content).context("Failed to parse schema file as JSON")?;

    // Compile the schema
    let compiled_schema = JSONSchema::options()
        .compile(&schema_json)
        .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema: {}", e))?;

    // Parse the output based on format
    log::debug!("Validating {} output against schema", format);
    let output_json: serde_json::Value = match format.to_lowercase().as_str() {
        "yaml" => {
            // The YAML output writes enum variants as single-key maps (not YAML tags),
            // so it converts to the JSON the schema describes without a typed round trip
            serde_yaml::from_str(output).context("Failed to parse YAML output for validation")?
        }
        "json" => {
            // Parse JSON directly
            serde_json::from_str(output).context("Failed to parse JSON output for validation")?
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported format for validation: {}",
                format
            ));
        }
    };

    // Validate against schema
    let validation_result = compiled_schema.validate(&output_json);

    match validation_result {
        Ok(_) => {
            log::info!("✓ Output validation passed: conforms to schema");
            Ok(())
        }
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .map(|e| format!("  - {} at {}", e, e.instance_path))
                .collect();

            log::error!("✗ Output validation failed:");
            for msg in &error_messages {
                log::error!("{}", msg);
            }

            Err(anyhow::anyhow!(
                "Output does not conform to schema:\n{}",
                error_messages.join("\n")
            ))
        }
    }
}

fn write_output(content: &str, output_path: Option<&PathBuf>) -> Result<()> {
    if let Some(path) = output_path {
        fs::write(path, content)
            .context(format!("Failed to write output to {}", path.display()))?;
        log::info!("Report written to {}", path.display());
    } else {
        // The report is the program's output, not a log message: it must reach stdout whatever
        // the log level is
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(content.as_bytes())?;
        if !content.ends_with('\n') {
            stdout.write_all(b"\n")?;
        }
    }

    Ok(())
}
fn parse_match_strategy(strategy: &str) -> Result<MatchStrategy> {
    match strategy.to_lowercase().as_str() {
        "exact" => Ok(MatchStrategy::Exact),
        "regex" => Ok(MatchStrategy::Regex),
        "contains" => Ok(MatchStrategy::Contains),
        "precomputed" => Ok(MatchStrategy::Precomputed),
        "bash-evaluation" | "bash" => Ok(MatchStrategy::BashEvaluation),
        _ => anyhow::bail!(
            "Invalid match strategy '{}'. Must be one of: exact, regex, contains, precomputed, bash-evaluation",
            strategy
        ),
    }
}

fn compute_test_case_source_hash(storage: &TestCaseStorage, test_case_id: &str) -> Result<String> {
    let yaml_path = find_test_case_file_path(storage, test_case_id).context(format!(
        "Failed to find test case file for ID: {}",
        test_case_id
    ))?;

    log::debug!(
        "Computing SHA-256 hash for test case file: {}",
        yaml_path.display()
    );

    let file_bytes = fs::read(&yaml_path).context(format!(
        "Failed to read test case file: {}",
        yaml_path.display()
    ))?;

    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let hash_result = hasher.finalize();
    let hash_hex = format!("{:x}", hash_result);

    log::debug!("Computed SHA-256 hash for '{}': {}", test_case_id, hash_hex);

    Ok(hash_hex)
}

fn find_test_case_file_path(storage: &TestCaseStorage, test_case_id: &str) -> Result<PathBuf> {
    const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];

    let base_path = storage.base_path();
    let id_path = Path::new(test_case_id);

    if let Some(ext) = id_path.extension() {
        if YAML_EXTENSIONS.contains(&ext.to_string_lossy().as_ref()) {
            let file_path = base_path.join(test_case_id);
            if file_path.exists() {
                return Ok(file_path);
            }
            if id_path.exists() {
                return Ok(id_path.to_path_buf());
            }
        }
    }

    for ext in YAML_EXTENSIONS {
        let file_name = format!("{}.{}", test_case_id, ext);
        let file_path = base_path.join(&file_name);

        if file_path.exists() {
            return Ok(file_path);
        }
    }

    for ext in YAML_EXTENSIONS {
        let file_name = format!("{}.{}", test_case_id, ext);
        if let Ok(entries) = fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = search_recursive(&path, &file_name) {
                        return Ok(found);
                    }
                }
            }
        }
    }

    anyhow::bail!("Test case file not found for ID: {}", test_case_id)
}

fn search_recursive(dir: &Path, file_name: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name()?.to_str()? == file_name {
                return Some(path);
            } else if path.is_dir() {
                if let Some(found) = search_recursive(&path, file_name) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn verify_hash_against_logs(
    logs: &[TestExecutionLog],
    test_case_id: &str,
    expected_hash: &str,
) -> Result<()> {
    log::debug!(
        "Verifying source hash for test case '{}' against {} log entries",
        test_case_id,
        logs.len()
    );

    let mut hash_map: HashMap<String, usize> = HashMap::new();
    let mut missing_count = 0;

    for log in logs {
        if let Some(log_hash) = &log.source_yaml_sha256 {
            *hash_map.entry(log_hash.clone()).or_insert(0) += 1;
        } else {
            missing_count += 1;
        }
    }

    if missing_count > 0 {
        anyhow::bail!(
            "Source hash verification failed for test case '{}': {} log entries are missing source_yaml_sha256 field",
            test_case_id,
            missing_count
        );
    }

    for (log_hash, count) in &hash_map {
        if log_hash != expected_hash {
            anyhow::bail!(
                "Source hash verification failed for test case '{}': expected hash '{}' but {} log entries have hash '{}'",
                test_case_id,
                expected_hash,
                count,
                log_hash
            );
        }
    }

    log::info!(
        "✓ Source hash verification passed for test case '{}': all {} log entries match expected hash",
        test_case_id,
        logs.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One test owns the TCMS_* variables: the environment is process-wide, and tests run in
    /// parallel threads.
    #[test]
    fn test_run_metadata_precedence_flag_then_env_then_config() {
        for name in ["TCMS_RUN_ID", "TCMS_BUILD_ID", "TCMS_TESTER"] {
            std::env::remove_var(name);
        }
        let config_with = |run: &str, build: &str, tester: &str| VerifierConfig {
            run_id: Some(run.to_string()),
            build_id: Some(build.to_string()),
            tester: Some(tester.to_string()),
            ..Default::default()
        };

        // config only
        let mut config = config_with("cfg-run", "cfg-build", "cfg-tester");
        config.apply_run_metadata(None, None, None);
        assert_eq!(config.run_id.as_deref(), Some("cfg-run"));
        assert_eq!(config.build_id.as_deref(), Some("cfg-build"));

        // environment beats config
        std::env::set_var("TCMS_RUN_ID", "env-run");
        std::env::set_var("TCMS_TESTER", "env-tester");
        let mut config = config_with("cfg-run", "cfg-build", "cfg-tester");
        config.apply_run_metadata(None, None, None);
        assert_eq!(config.run_id.as_deref(), Some("env-run"));
        assert_eq!(config.build_id.as_deref(), Some("cfg-build"));
        assert_eq!(config.tester.as_deref(), Some("env-tester"));

        // flag beats environment
        let mut config = config_with("cfg-run", "cfg-build", "cfg-tester");
        config.apply_run_metadata(Some("flag-run".to_string()), None, None);
        assert_eq!(config.run_id.as_deref(), Some("flag-run"));

        // empty and blank values count as absent at every level
        std::env::set_var("TCMS_RUN_ID", "   ");
        let mut config = VerifierConfig::default();
        config.apply_run_metadata(Some(String::new()), None, Some("  ".to_string()));
        assert_eq!(config.run_id, None);
        assert_eq!(config.tester.as_deref(), Some("env-tester"));
        assert_eq!(config.build_id, None);

        for name in ["TCMS_RUN_ID", "TCMS_BUILD_ID", "TCMS_TESTER"] {
            std::env::remove_var(name);
        }
    }

    #[test]
    fn test_blank_environment_is_absent_not_written_empty() {
        // blank flag keeps the config value
        let mut config = VerifierConfig {
            environment: Some("cfg-env".to_string()),
            ..Default::default()
        };
        config.apply_cli_overrides(None, None, Some("  ".to_string()), None, None);
        assert_eq!(config.environment.as_deref(), Some("cfg-env"));

        // blank config value and no flag: absent
        let mut config = VerifierConfig {
            environment: Some(String::new()),
            ..Default::default()
        };
        config.apply_cli_overrides(None, None, None, None, None);
        assert_eq!(config.environment, None);

        // flag value is trimmed
        let mut config = VerifierConfig::default();
        config.apply_cli_overrides(None, None, Some(" Staging ".to_string()), None, None);
        assert_eq!(config.environment.as_deref(), Some("Staging"));
    }

    #[test]
    fn test_run_id_is_read_back_from_the_execution_log() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("log.json");

        std::fs::write(&log, r#"[{"step": 1}, {"step": 2, "run_id": " RUN-7 "}]"#).unwrap();
        assert_eq!(run_id_from_log(&log).as_deref(), Some("RUN-7"));

        std::fs::write(&log, r#"[{"step": 1, "run_id": ""}]"#).unwrap();
        assert_eq!(run_id_from_log(&log), None);

        std::fs::write(&log, "not json").unwrap();
        assert_eq!(run_id_from_log(&log), None);
        assert_eq!(run_id_from_log(&dir.path().join("missing.json")), None);
    }

    #[test]
    fn test_log_facts_timestamps_hook_failures_and_run_id() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("ok.json");
        let hook = dir.path().join("hook.json");
        std::fs::write(
            &ok,
            r#"[{"timestamp": "2026-10-28T10:00:05", "exit_code": 0, "run_id": "RUN-9"},
                {"timestamp": "2026-10-28T10:00:01", "exit_code": 1},
                {"timestamp": "2026-10-28T10:00:09", "exit_code": 0, "hook_type": "after_step"}]"#,
        )
        .unwrap();
        std::fs::write(
            &hook,
            r#"[{"timestamp": "2026-10-28T10:00:03", "exit_code": 127, "hook_type": "script_start"}]"#,
        )
        .unwrap();

        let facts = collect_log_facts(&[
            ("TC_OK".to_string(), ok),
            ("TC_HOOK".to_string(), hook),
            ("TC_GONE".to_string(), dir.path().join("missing.json")),
        ]);

        // a failing step is not an error; a failed hook is, and a passing hook is neither
        assert_eq!(facts.errored.iter().collect::<Vec<_>>(), ["TC_HOOK"]);
        assert_eq!(facts.run_id.as_deref(), Some("RUN-9"));
        let (earliest, latest) = (facts.earliest.unwrap(), facts.latest.unwrap());
        assert_eq!((latest - earliest).num_seconds(), 8);
    }

    #[test]
    fn test_default_test_run_path_follows_the_report() {
        assert_eq!(
            default_test_run_path(Some(Path::new("out/results.yaml"))),
            PathBuf::from("out/results.test-run.yaml")
        );
        assert_eq!(
            default_test_run_path(Some(Path::new("report.json"))),
            PathBuf::from("report.test-run.yaml")
        );
        assert_eq!(default_test_run_path(None), PathBuf::from("test-run.yaml"));
    }
}
