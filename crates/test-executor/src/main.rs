use anyhow::{Context, Result};
use audit_traceability::{AuditTraceabilityLog, StageInfo, TestCaseAudit};
use audit_verifier::audit_log::OperationStatus;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use testcase_execution::{AuditLogger, TestExecutor, VarHydrator};
use testcase_models::TestCase;
use testcase_storage::{TestCaseFilter, TestCaseFilterer, TestCaseStorage};
use testcase_validation::DependencyResolver;

#[derive(Parser)]
#[command(name = "test-executor")]
#[command(
    about = "Generate and execute test scripts from YAML test case files",
    version
)]
#[command(
    after_help = "ENVIRONMENT VARIABLES:\n    RUST_LOG    Set log level (trace, debug, info, warn, error). Overrides --log-level\n    AUDIT_LOG_FILE    Path to audit log file (default: audit.log.json)\n    AUDIT_LOG_ENABLED    Enable/disable audit logging (default: true)"
)]
struct Cli {
    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", default_value = "warn", global = true)]
    log_level: String,

    /// Enable verbose output (equivalent to --log-level=info)
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to audit log file (overrides AUDIT_LOG_FILE env var)
    #[arg(long, value_name = "PATH", global = true)]
    audit_log: Option<PathBuf>,

    /// Disable audit logging
    #[arg(long, global = true)]
    no_audit: bool,

    /// Path to a TOML configuration file (e.g. [script_generation.json_escaping]).
    /// When omitted, ~/.testcase-manager/config.toml is used
    #[arg(long, value_name = "TOML", global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a shell script from a test case YAML file
    Generate {
        /// Path to the test case YAML file
        #[arg(value_name = "YAML_FILE")]
        yaml_file: PathBuf,

        /// Optional output file path (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// Generate execution log JSON file alongside bash script
        #[arg(long)]
        json_log: bool,

        /// Force output even if shellcheck validation fails
        #[arg(short = 'f', long)]
        force: bool,

        /// Optional test case directory for dependency resolution (defaults to parent directory of YAML file)
        #[arg(long, value_name = "TEST_CASE_DIR")]
        test_case_dir: Option<PathBuf>,

        /// Update audit traceability log with generated artifacts
        #[arg(long, value_name = "AUDIT_LOG")]
        audit_log: Option<PathBuf>,

        /// Run ID written to the execution log entries; the TCMS_RUN_ID environment variable
        /// of the process that runs the script takes precedence
        #[arg(long, value_name = "ID")]
        run_id: Option<String>,
    },
    /// Execute a test case by generating and running the script
    Execute {
        /// Path to the test case YAML file
        #[arg(value_name = "YAML_FILE")]
        yaml_file: PathBuf,
    },
    /// Hydrate a test case YAML file with variable values from an export file
    Hydrate {
        /// Path to the test case YAML file
        #[arg(value_name = "YAML_FILE")]
        yaml_file: PathBuf,

        /// Path to the export file containing variable values
        #[arg(short, long, value_name = "EXPORT_FILE")]
        export_file: PathBuf,

        /// Optional output file path (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,
    },
    /// Generate an export file template from test case hydration_vars declarations
    GenerateExport {
        /// Path to the test case YAML file
        #[arg(value_name = "YAML_FILE")]
        yaml_file: PathBuf,

        /// Optional output file path (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,
    },
    /// Validate that an export file has all required variables from test case
    ValidateExport {
        /// Path to the test case YAML file
        #[arg(value_name = "YAML_FILE")]
        yaml_file: PathBuf,

        /// Path to the export file to validate
        #[arg(short, long, value_name = "EXPORT_FILE")]
        export_file: PathBuf,
    },
    /// List all test cases with optional filtering
    List {
        /// Optional base path to test cases directory (defaults to "testcases")
        #[arg(value_name = "BASE_PATH")]
        base_path: Option<PathBuf>,

        /// Show only test cases with manual steps
        #[arg(long, conflicts_with = "automated_only")]
        manual_only: bool,

        /// Show only test cases with automated steps
        #[arg(long, conflicts_with = "manual_only")]
        automated_only: bool,

        /// Show statistics about test cases
        #[arg(long)]
        show_stats: bool,
    },
    /// Create or check independence-assignment records (author and verifier of an artifact)
    #[command(subcommand)]
    IndependenceAssignment(IndependenceCommands),
    /// Generate the traceability matrix (requirement, test cases, runs) and check that every
    /// requirement has a normal-range and a robustness test case
    TraceMatrix {
        /// Directory of test cases (searched recursively)
        #[arg(value_name = "TEST_CASE_DIR")]
        test_case_dir: PathBuf,

        /// Requirements: an ID list (one ID, ID:HLR or ID:LLR per line), a requirement YAML
        /// file, a directory of requirement records, or a StrictDoc JSON export (a `.json` file).
        /// Only HLR and LLR count
        #[arg(long, value_name = "PATH")]
        requirements: PathBuf,

        /// Test run records (test-run YAML files or directories of them) to link results
        #[arg(long = "runs", value_name = "PATH")]
        runs: Vec<PathBuf>,

        /// Matrix ID (defaults to TM-<UTC date and time>)
        #[arg(long, value_name = "ID")]
        id: Option<String>,

        /// Baseline the matrix belongs to
        #[arg(long, value_name = "BASELINE_ID")]
        baseline_id: Option<String>,

        /// Output file (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// Do not fail when a requirement lacks a normal-range or robustness test case
        #[arg(long)]
        no_coverage_check: bool,
    },
    /// Load a trace matrix and test run records strictly, select one result per test case, and
    /// print the report. Every kind (traceability, summary, gaps, problems) is rendered as
    /// Markdown from one model, in that order.
    Report {
        /// Trace matrix YAML file
        #[arg(long, value_name = "FILE")]
        matrix: PathBuf,

        /// Test run records: YAML files or directories of them (repeatable)
        #[arg(long = "runs", value_name = "PATH", required = true)]
        runs: Vec<PathBuf>,

        /// Report kind to print (repeatable; defaults to all kinds)
        #[arg(long = "kind", value_enum, value_name = "KIND")]
        kinds: Vec<ReportKind>,

        /// Traceability layout: grid (default), a or b. a and b require `--kind traceability`
        #[arg(
            long = "shape",
            value_enum,
            value_name = "SHAPE",
            default_value = "grid"
        )]
        shape: ReportShape,

        /// Output file (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// Directory of test cases: supplies test types and obsolete status
        #[arg(long = "test-cases", value_name = "DIR")]
        test_cases: Option<PathBuf>,

        /// StrictDoc JSON export: supplies the source document of each requirement
        #[arg(long = "requirements-export", value_name = "FILE")]
        requirements_export: Option<PathBuf>,

        /// Generation time, RFC 3339 (defaults to the current UTC time)
        #[arg(long = "generated-at", value_name = "TIMESTAMP")]
        generated_at: Option<String>,
    },
    /// Select the regression test cases from a baseline's change impact: the test cases the
    /// impact analysis lists plus those tracing to the impacted requirements
    RegressionSelect {
        /// Directory of test cases (searched recursively)
        #[arg(value_name = "TEST_CASE_DIR")]
        test_case_dir: PathBuf,

        /// Baseline YAML file(s) or directories; with --plan, the one the plan names is used
        #[arg(long = "baseline", value_name = "PATH", required = true)]
        baselines: Vec<PathBuf>,

        /// Test plan with a `regression` block (baseline_id, regression.previous_baseline_id)
        #[arg(long, value_name = "PLAN_FILE")]
        plan: Option<PathBuf>,

        /// Output format: `ids` (one test case ID per line, for scripts), `json` or `yaml`
        #[arg(long, value_name = "FORMAT", default_value = "ids")]
        format: String,

        /// Output file (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// Do not fail when the baseline lists test case IDs that do not exist
        #[arg(long)]
        allow_unknown_test_cases: bool,
    },
    /// Import coverage tool output (LCOV, MC/DC side file, traceability matrix) into a
    /// coverage-report and apply gap annotations; unclassified gaps are reported and fail
    CoverageImport {
        /// LCOV tracefile (statement and decision coverage)
        #[arg(long, value_name = "FILE")]
        lcov: Option<PathBuf>,

        /// MC/DC side input (YAML: mcdc_percent, units, gaps)
        #[arg(long, value_name = "FILE")]
        mcdc: Option<PathBuf>,

        /// Traceability matrix YAML (from `trace-matrix`) for requirements-based coverage
        #[arg(long, value_name = "FILE")]
        trace_matrix: Option<PathBuf>,

        /// Gap annotations (YAML list: location, classification, rationale, approver, ...)
        #[arg(long, value_name = "FILE")]
        annotations: Option<PathBuf>,

        /// Baseline measured (required by the coverage-report schema)
        #[arg(long, value_name = "BASELINE_ID")]
        baseline_id: String,

        /// Report ID (defaults to COV-<UTC date and time>)
        #[arg(long, value_name = "ID")]
        id: Option<String>,

        /// Test run the coverage was measured on
        #[arg(long, value_name = "RUN_ID")]
        run_id: Option<String>,

        /// Coverage tool name
        #[arg(long, value_name = "NAME")]
        tool: Option<String>,

        /// Coverage tool version
        #[arg(long, value_name = "VERSION")]
        tool_version: Option<String>,

        /// Output file (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,

        /// Do not fail when gaps are left without classification and rationale
        #[arg(long)]
        allow_unclassified: bool,
    },
    /// Resolve dependencies in test case YAML files
    Resolve {
        /// Paths to YAML test case files
        #[arg(value_name = "YAML_FILES", required = true)]
        yaml_files: Vec<PathBuf>,

        /// Optional output directory (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_DIR")]
        output: Option<PathBuf>,
    },
    /// Create or update audit traceability log
    AuditLog {
        #[command(subcommand)]
        command: AuditLogCommands,
    },
}

#[derive(Subcommand)]
enum AuditLogCommands {
    /// Create a new audit traceability log
    Create {
        /// Output path for the audit log JSON file
        #[arg(short, long, default_value = "audit-traceability-log.json")]
        output: PathBuf,

        /// Witness key for the audit log
        #[arg(short, long, default_value = "default-witness")]
        witness_key: String,
    },
    /// Add a test case to the audit log
    Add {
        /// Path to the audit log JSON file
        #[arg(short, long, default_value = "audit-traceability-log.json")]
        log_file: PathBuf,

        /// Test case ID
        #[arg(long)]
        test_case_id: String,

        /// Path to initial YAML file
        #[arg(long)]
        initial: PathBuf,

        /// Path to generated shell script (stage: 05_shell_script)
        #[arg(long)]
        shell_script: Option<PathBuf>,
    },
    /// Verify all files in the audit log
    Verify {
        /// Path to the audit log JSON file
        #[arg(short, long, default_value = "audit-traceability-log.json")]
        log_file: PathBuf,

        /// Optional test case ID to verify (verifies all if not specified)
        #[arg(long)]
        test_case_id: Option<String>,
    },
}

fn load_test_case(yaml_file: &PathBuf) -> Result<TestCase> {
    testcase_reporting::legacy::load_test_case(yaml_file)
}

#[derive(Subcommand)]
enum IndependenceCommands {
    /// Create an independence-assignment record; the verifier counts as independent only when
    /// it is not the author
    Create {
        /// Record ID
        #[arg(long)]
        id: String,

        /// The objective that asks for independence
        #[arg(long)]
        objective: String,

        /// Objective ID (for example A-7.4)
        #[arg(long)]
        objective_id: Option<String>,

        /// What was checked (a run ID, a test case ID, a document)
        #[arg(long)]
        artifact: String,

        /// Who wrote the artifact
        #[arg(long)]
        author: String,

        /// Who checked it
        #[arg(long)]
        verifier: String,

        /// Date of the check (defaults to today, UTC)
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,

        /// Output file (defaults to stdout)
        #[arg(short, long, value_name = "OUTPUT_FILE")]
        output: Option<PathBuf>,
    },
    /// Validate independence-assignment records against their schema and the rule that a
    /// verifier who is the author cannot be independent
    Check {
        /// Record files
        #[arg(value_name = "FILES", required = true)]
        files: Vec<PathBuf>,
    },
}

fn independence_assignment(command: IndependenceCommands) -> Result<()> {
    use testcase_verification::IndependenceAssignment;

    match command {
        IndependenceCommands::Create {
            id,
            objective,
            objective_id,
            artifact,
            author,
            verifier,
            date,
            output,
        } => {
            let mut record =
                IndependenceAssignment::new(&id, &objective, &artifact, &author, &verifier);
            record.objective_id = objective_id;
            record.date =
                Some(date.unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()));
            record.validate().map_err(|e| anyhow::anyhow!(e))?;
            let yaml = testcase_common::to_yaml_string(&record)?;
            match output {
                Some(path) => fs::write(&path, &yaml)
                    .context(format!("Failed to write record: {}", path.display()))?,
                None => print!("{yaml}"),
            }
            if !record.verifier_is_independent {
                eprintln!(
                    "Note: the verifier is the author, so the record says the check is not independent"
                );
            }
            Ok(())
        }
        IndependenceCommands::Check { files } => {
            let mut failed = 0;
            for file in &files {
                let result = (|| -> Result<()> {
                    let mut content = fs::read_to_string(file)
                        .context(format!("Failed to read {}", file.display()))?;
                    // the tcms-schemas samples carry no envelope; add it as a user would
                    if !content.lines().any(|l| l.starts_with("schema:")) {
                        content = format!(
                            "type: independence_assignment\nschema: tcms/independence-assignment.schema.json\n{content}"
                        );
                    }
                    let record: IndependenceAssignment =
                        testcase_common::parse_and_validate_yaml_string(
                            &content,
                            "schemas/",
                            &file.display().to_string(),
                        )?;
                    record.validate().map_err(|e| anyhow::anyhow!(e))
                })();
                match result {
                    Ok(()) => println!("✓ {}", file.display()),
                    Err(e) => {
                        failed += 1;
                        eprintln!("✗ {}: {:#}", file.display(), e);
                    }
                }
            }
            if failed > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

fn load_all_yaml_files_from_dir(dir: &PathBuf) -> Result<Vec<(PathBuf, TestCase)>> {
    let mut test_cases = Vec::new();

    if !dir.is_dir() {
        return Ok(test_cases);
    }

    for entry in
        fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "yaml" || ext_str == "yml" {
                    match load_test_case(&path) {
                        Ok(test_case) => {
                            test_cases.push((path, test_case));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }

    Ok(test_cases)
}

fn build_dependency_resolver(yaml_file: &Path) -> Result<DependencyResolver> {
    let dir = yaml_file
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?
        .to_path_buf();

    let test_cases = load_all_yaml_files_from_dir(&dir)?;

    let mut index = HashMap::new();
    for (_, test_case) in test_cases {
        index.insert(test_case.id.clone(), test_case);
    }

    Ok(DependencyResolver::new(index))
}

fn build_dependency_resolver_from_dir(test_case_dir: &Path) -> Result<DependencyResolver> {
    let test_cases = load_all_yaml_files_from_dir_recursive(test_case_dir)?;

    let mut index = HashMap::new();
    for (_, test_case) in test_cases {
        index.insert(test_case.id.clone(), test_case);
    }

    Ok(DependencyResolver::new(index))
}

/// Requirements of the matrix. A `.json` file is a StrictDoc export (its layout is an assumption,
/// see `requirement_import::strictdoc`); anything else is read by `testcase_verification`.
fn read_matrix_requirements(path: &Path) -> Result<Vec<testcase_verification::Requirement>> {
    let is_json = path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    if !is_json {
        return testcase_verification::read_requirements(path);
    }

    let text =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let items = requirement_import::strictdoc::export_levels(&text)
        .with_context(|| format!("{} is not a StrictDoc export", path.display()))?;
    let mut seen = std::collections::BTreeSet::new();
    let mut requirements = Vec::new();
    for (id, level) in items {
        if !seen.insert(id.clone()) {
            anyhow::bail!(
                "{}: requirement '{id}' appears twice in the StrictDoc export",
                path.display()
            );
        }
        let level = match level.as_str() {
            "HLR" => testcase_verification::RequirementLevel::Hlr,
            "LLR" => testcase_verification::RequirementLevel::Llr,
            _ => continue, // only HLR and LLR count
        };
        requirements.push(testcase_verification::Requirement { id, level });
    }
    Ok(requirements)
}

/// Report kinds accepted by `report --kind` (D-19). Every kind is rendered as Markdown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
enum ReportKind {
    Traceability,
    Summary,
    Gaps,
    Problems,
}

/// Traceability layouts accepted by `report --shape` (D-02-6-2). Each is a view over one model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum ReportShape {
    #[value(name = "grid")]
    Grid,
    #[value(name = "a")]
    A,
    #[value(name = "b")]
    B,
}

impl ReportKind {
    const ALL: [ReportKind; 4] = [
        ReportKind::Traceability,
        ReportKind::Summary,
        ReportKind::Gaps,
        ReportKind::Problems,
    ];

    fn as_str(self) -> &'static str {
        match self {
            ReportKind::Traceability => "traceability",
            ReportKind::Summary => "summary",
            ReportKind::Gaps => "gaps",
            ReportKind::Problems => "problems",
        }
    }
}

/// Source documents of the requirements, from a StrictDoc JSON export. Every item's document is
/// listed; HLR and LLR items map to their document. A UID that appears twice is an error, as in
/// `read_matrix_requirements`.
fn read_requirement_documents(
    path: &Path,
) -> Result<testcase_reporting::model::RequirementDocuments> {
    let text =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let items = requirement_import::strictdoc::export_items(&text)
        .with_context(|| format!("{} is not a StrictDoc export", path.display()))?;
    let mut seen = std::collections::BTreeSet::new();
    let mut documents = std::collections::BTreeSet::new();
    let mut requirement_document = std::collections::BTreeMap::new();
    for item in items {
        if !seen.insert(item.uid.clone()) {
            anyhow::bail!(
                "{}: requirement '{}' appears twice in the StrictDoc export",
                path.display(),
                item.uid
            );
        }
        documents.insert(item.path.clone());
        if matches!(item.level.as_str(), "HLR" | "LLR") {
            requirement_document.insert(item.uid, item.path);
        }
    }
    Ok(testcase_reporting::model::RequirementDocuments {
        documents: documents.into_iter().collect(),
        requirement_document,
    })
}

/// Validate `--generated-at` and return the timestamp as the user wrote it. Without the flag the
/// current UTC time is used: the only clock read in the report pipeline.
fn generated_at_value(given: Option<&str>) -> Result<String> {
    match given {
        Some(value) => {
            chrono::DateTime::parse_from_rfc3339(value).with_context(|| {
                format!("--generated-at '{value}' is not an RFC 3339 timestamp")
            })?;
            Ok(value.to_string())
        }
        None => Ok(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
    }
}

/// Optional report inputs and the generation time (`report --test-cases`, `--requirements-export`,
/// `--generated-at`).
#[derive(Clone, Copy, Default)]
struct ReportOptions<'a> {
    test_cases: Option<&'a Path>,
    requirements_export: Option<&'a Path>,
    generated_at: Option<&'a str>,
}

fn report(
    matrix: &Path,
    run_paths: &[PathBuf],
    kinds: &[ReportKind],
    shape: ReportShape,
    output: Option<PathBuf>,
    options: ReportOptions<'_>,
) -> Result<()> {
    // Usage error (exit 2) before any input is read or output is written (D-02-6-2, D-02-6-4)
    let only_traceability = {
        let mut requested = kinds.to_vec();
        requested.sort();
        requested.dedup();
        requested == [ReportKind::Traceability]
    };
    if shape != ReportShape::Grid && !only_traceability {
        clap::Error::raw(
            clap::error::ErrorKind::ArgumentConflict,
            "--shape a|b requires --kind traceability as the only --kind (without --kind, all kinds are shown)",
        )
        .exit();
    }

    // Any input error returns here, before anything is written (T-01-14)
    let inputs = testcase_reporting::load::load_inputs(matrix, run_paths, "schemas/")?;
    let selection = testcase_reporting::select::select_results(&inputs.runs);
    let generated_at = generated_at_value(options.generated_at)?;
    let test_cases = match options.test_cases {
        Some(dir) => Some(testcase_reporting::legacy::load_test_cases_strict(dir)?),
        None => None,
    };
    let documents = match options.requirements_export {
        Some(path) => Some(read_requirement_documents(path)?),
        None => None,
    };

    let selected: Vec<ReportKind> = if kinds.is_empty() {
        ReportKind::ALL.to_vec()
    } else {
        let mut sorted: Vec<ReportKind> = kinds.to_vec();
        sorted.sort();
        sorted.dedup();
        sorted
    };

    // One model for every kind, so the numbers agree across the report (D-06)
    let model = testcase_reporting::model::build_model(&testcase_reporting::model::ReportInputs {
        inputs: &inputs,
        selection: &selection,
        test_cases: test_cases.as_deref(),
        documents: documents.as_ref(),
        generated_at: &generated_at,
    });
    let view = testcase_reporting::view::ViewInput {
        model: &model,
        selection: &selection,
    };
    let body = |kind: ReportKind| -> String {
        match kind {
            ReportKind::Traceability => match shape {
                ReportShape::Grid => testcase_reporting::render_markdown::render_markdown(&model),
                ReportShape::A => testcase_reporting::render_shapes::render_shape_a(&model),
                ReportShape::B => testcase_reporting::render_shapes::render_shape_b(&model),
            },
            ReportKind::Summary => testcase_reporting::summary::render_summary_markdown(
                &testcase_reporting::summary::build_summary(view),
            ),
            ReportKind::Gaps => testcase_reporting::gaps::render_gaps_markdown(
                &testcase_reporting::gaps::build_gaps(view),
            ),
            ReportKind::Problems => testcase_reporting::problems::render_problems_markdown(
                &testcase_reporting::problems::build_problems(view),
            ),
        }
    };

    let text = if selected.contains(&ReportKind::Traceability) {
        // The traceability document comes first and keeps its own heading; the other kinds follow
        let mut text = body(ReportKind::Traceability);
        for kind in selected.iter().filter(|k| **k != ReportKind::Traceability) {
            text.push_str(&format!(
                "\nReport kind {}: rendered below\n\n",
                kind.as_str()
            ));
            text.push_str(&body(*kind));
        }
        text
    } else {
        let mut text = testcase_reporting::input_summary(&inputs, &selection);
        text.push_str("\nReport kinds\n");
        for kind in &selected {
            text.push_str(&format!("  {}: rendered below\n", kind.as_str()));
        }
        for kind in &selected {
            text.push('\n');
            text.push_str(&body(*kind));
        }
        text
    };

    match output {
        Some(path) => fs::write(&path, &text)
            .context(format!("Failed to write report: {}", path.display()))?,
        None => print!("{text}"),
    }
    Ok(())
}

fn trace_matrix(
    test_case_dir: &Path,
    requirements_path: &Path,
    run_paths: &[PathBuf],
    id: Option<String>,
    baseline_id: Option<String>,
    output: Option<PathBuf>,
    no_coverage_check: bool,
) -> Result<()> {
    let requirements = read_matrix_requirements(requirements_path)?;
    if requirements.is_empty() {
        anyhow::bail!("No HLR or LLR found in {}", requirements_path.display());
    }
    let test_cases = testcase_reporting::legacy::load_test_cases_strict(test_case_dir)?;
    let runs = testcase_reporting::legacy::read_test_runs(run_paths)?;

    let id = id.unwrap_or_else(|| format!("TM-{}", chrono::Utc::now().format("%Y%m%dT%H%M%SZ")));
    let (matrix, findings) = testcase_verification::build_matrix(
        &id,
        baseline_id.as_deref(),
        &requirements,
        &test_cases,
        &runs,
    );

    let yaml = testcase_common::to_yaml_string(&matrix)?;
    match &output {
        Some(path) => {
            fs::write(path, &yaml)
                .context(format!("Failed to write matrix: {}", path.display()))?;
            eprintln!(
                "Trace matrix {} written to {} ({} requirements, {} test cases, {} runs)",
                matrix.id,
                path.display(),
                requirements.len(),
                test_cases.len(),
                runs.len()
            );
        }
        None => print!("{yaml}"),
    }

    if !findings.untraced.is_empty() {
        eprintln!(
            "Requirements without test cases: {}",
            findings.untraced.join(", ")
        );
    }
    for (test_case_id, unknown) in &findings.unknown_references {
        eprintln!(
            "Warning: test case {} names requirements that are not in the requirement set: {}",
            test_case_id,
            unknown.join(", ")
        );
    }

    if !findings.complete() && !no_coverage_check {
        eprintln!(
            "Coverage check failed: every HLR and LLR needs a normal-range and a robustness test \
             (use --no-coverage-check to skip):\n{}",
            findings.gap_report()
        );
        std::process::exit(1);
    }
    Ok(())
}

fn regression_select(
    test_case_dir: &Path,
    baseline_paths: &[PathBuf],
    plan_path: Option<&Path>,
    format: &str,
    output: Option<PathBuf>,
    allow_unknown: bool,
) -> Result<()> {
    if !matches!(format, "ids" | "json" | "yaml") {
        anyhow::bail!("Unknown --format '{format}': use ids, json or yaml");
    }
    let baselines = testcase_verification::read_baselines(baseline_paths)?;
    let plan = plan_path
        .map(testcase_verification::read_regression_plan)
        .transpose()?;
    let baseline = testcase_verification::resolve_baseline(&baselines, plan.as_ref())?;
    let test_cases = testcase_reporting::legacy::load_test_cases_strict(test_case_dir)?;
    let selection = testcase_verification::select_regression(baseline, &test_cases);

    let text = match format {
        "json" => serde_json::to_string_pretty(&selection)? + "\n",
        "yaml" => testcase_common::to_yaml_string(&selection)?,
        _ => selection
            .test_case_ids()
            .iter()
            .map(|id| format!("{id}\n"))
            .collect(),
    };
    match &output {
        Some(path) => fs::write(path, &text)
            .context(format!("Failed to write selection: {}", path.display()))?,
        None => print!("{text}"),
    }
    eprintln!(
        "Regression scope of baseline {} (from {}): {} of {} test cases",
        selection.baseline_id,
        selection.previous_baseline_id,
        selection.selected.len(),
        test_cases.len()
    );
    if !selection.untested_requirements.is_empty() {
        eprintln!(
            "Warning: impacted requirements without test cases: {}",
            selection.untested_requirements.join(", ")
        );
    }
    if !selection.unknown_listed.is_empty() {
        eprintln!(
            "Baseline lists test cases that do not exist (or are obsolete): {}",
            selection.unknown_listed.join(", ")
        );
        if !allow_unknown {
            eprintln!("(use --allow-unknown-test-cases to accept this)");
            std::process::exit(1);
        }
    }
    Ok(())
}

struct CoverageImportArgs {
    lcov: Option<PathBuf>,
    mcdc: Option<PathBuf>,
    trace_matrix: Option<PathBuf>,
    annotations: Option<PathBuf>,
    baseline_id: String,
    id: Option<String>,
    run_id: Option<String>,
    tool: Option<String>,
    tool_version: Option<String>,
    output: Option<PathBuf>,
    allow_unclassified: bool,
}

fn coverage_import(args: CoverageImportArgs) -> Result<()> {
    use testcase_verification as tv;
    let matrix = match &args.trace_matrix {
        Some(path) => {
            let text = fs::read_to_string(path).context(format!(
                "Cannot read traceability matrix {}",
                path.display()
            ))?;
            let matrix: tv::TraceMatrix = serde_yaml::from_str(&text)
                .context(format!("{} is not a traceability matrix", path.display()))?;
            Some(matrix)
        }
        None => None,
    };
    let input = tv::CoverageInput {
        id: args
            .id
            .unwrap_or_else(|| format!("COV-{}", chrono::Utc::now().format("%Y%m%dT%H%M%SZ"))),
        baseline_id: args.baseline_id,
        run_id: args.run_id,
        tool: args.tool,
        tool_version: args.tool_version,
        lcov: args.lcov.as_deref().map(tv::Lcov::read).transpose()?,
        mcdc: args.mcdc.as_deref().map(tv::McdcInput::read).transpose()?,
        matrix,
        annotations: match &args.annotations {
            Some(path) => tv::read_annotations(path)?,
            None => Vec::new(),
        },
    };
    let (report, findings) = tv::build_coverage_report(input)?;

    let yaml = testcase_common::to_yaml_string(&report)?;
    match &args.output {
        Some(path) => {
            fs::write(path, &yaml).context(format!(
                "Failed to write coverage report: {}",
                path.display()
            ))?;
            eprintln!(
                "Coverage report {} written to {} ({} classified gaps)",
                report.id,
                path.display(),
                report.gaps.len()
            );
        }
        None => print!("{yaml}"),
    }
    for (classification, count) in tv::classification_counts(&report) {
        eprintln!("  {count} gap(s) classified {classification}");
    }
    for location in &findings.unused_annotations {
        eprintln!("Warning: annotation for '{location}' matches no gap");
    }
    if !findings.unclassified.is_empty() {
        eprintln!(
            "{} unclassified gap(s); each needs a classification and a rationale in the \
             annotations (they are not in the report):",
            findings.unclassified.len()
        );
        for gap in &findings.unclassified {
            eprintln!("  {} ({})", gap.location, gap.coverage_type);
        }
        if !args.allow_unclassified {
            eprintln!("(use --allow-unclassified to accept this)");
            std::process::exit(1);
        }
    }
    Ok(())
}

fn load_all_yaml_files_from_dir_recursive(dir: &Path) -> Result<Vec<(PathBuf, TestCase)>> {
    let mut test_cases = Vec::new();

    if !dir.is_dir() {
        return Ok(test_cases);
    }

    // Load YAML files from this directory
    for entry in
        fs::read_dir(dir).context(format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "yaml" || ext_str == "yml" {
                    match load_test_case(&path) {
                        Ok(test_case) => {
                            test_cases.push((path, test_case));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to load {}: {}", path.display(), e);
                        }
                    }
                }
            }
        } else if path.is_dir() {
            // Recursively load from subdirectories
            match load_all_yaml_files_from_dir_recursive(&path) {
                Ok(mut sub_test_cases) => {
                    test_cases.append(&mut sub_test_cases);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to load from {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(test_cases)
}

fn build_resolver_from_files(yaml_files: &[PathBuf]) -> Result<DependencyResolver> {
    let mut index = HashMap::new();

    for yaml_file in yaml_files {
        let test_case = load_test_case(yaml_file)?;
        index.insert(test_case.id.clone(), test_case);
    }

    Ok(DependencyResolver::new(index))
}

fn list_test_cases(
    base_path: Option<PathBuf>,
    manual_only: bool,
    automated_only: bool,
    show_stats: bool,
) -> Result<()> {
    let path = base_path.unwrap_or_else(|| PathBuf::from("testcases"));
    let storage = TestCaseStorage::new(&path)?;

    let test_cases = storage.load_all_test_cases()?;

    let filter = if manual_only {
        TestCaseFilter::ManualOnly
    } else if automated_only {
        TestCaseFilter::AutomatedOnly
    } else {
        TestCaseFilter::All
    };

    let filterer = TestCaseFilterer::new();
    let filtered_cases = filterer.filter_test_cases(test_cases.clone(), filter);

    if filtered_cases.is_empty() {
        println!("No test cases found.");
        return Ok(());
    }

    println!("Test Cases:");
    println!();

    for test_case in &filtered_cases {
        let manual_step_count = test_case.get_manual_step_count();
        let manual_indicator = if manual_step_count > 0 {
            format!(" [M:{}]", manual_step_count)
        } else {
            String::new()
        };

        println!(
            "  {}{} - {}",
            test_case.id, manual_indicator, test_case.description
        );
    }

    if show_stats {
        println!();
        println!("Statistics:");

        let total_count = filtered_cases.len();
        let manual_count = filtered_cases
            .iter()
            .filter(|tc| tc.has_manual_steps())
            .count();
        let automated_count = filtered_cases
            .iter()
            .filter(|tc| tc.has_automated_steps())
            .count();
        let total_manual_steps: usize = filtered_cases
            .iter()
            .map(|tc| tc.get_manual_step_count())
            .sum();

        println!("  Total test cases: {}", total_count);
        println!("  Test cases with manual steps: {}", manual_count);
        println!("  Test cases with automated steps: {}", automated_count);
        println!("  Total manual steps: {}", total_manual_steps);
    }

    Ok(())
}

/// Run shellcheck validation on generated script content.
/// Returns true if validation passed (or shellcheck not available), false if errors found.
fn run_shellcheck_validation(script_content: &str, force: bool) -> Result<bool> {
    let shellcheck_available = std::process::Command::new("shellcheck")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok();

    if !shellcheck_available {
        eprintln!("Warning: shellcheck not found on PATH, skipping validation");
        return Ok(true);
    }

    let temp_file = tempfile::Builder::new()
        .suffix(".sh")
        .tempfile()
        .context("Failed to create temporary file for shellcheck")?;

    fs::write(temp_file.path(), script_content)
        .context("Failed to write script to temporary file")?;

    let output = std::process::Command::new("shellcheck")
        .arg("-S")
        .arg("error")
        .arg(temp_file.path())
        .output()
        .context("Failed to run shellcheck")?;

    if output.status.success() {
        eprintln!("shellcheck: validation passed");
        Ok(true)
    } else {
        let findings = String::from_utf8_lossy(&output.stdout);
        if force {
            eprintln!(
                "Warning: shellcheck found errors (--force specified, continuing):\n{}",
                findings
            );
            Ok(true)
        } else {
            eprintln!("Error: shellcheck validation failed:\n{}", findings);
            eprintln!("Use -f/--force to skip shellcheck validation");
            Ok(false)
        }
    }
}

/// Build the executor from an explicit `--config` file when given, otherwise from the
/// default user configuration (`TestExecutor::new`).
fn build_executor(config: Option<&testcase_common::Config>) -> TestExecutor {
    match config {
        Some(cfg) => TestExecutor::with_config(cfg.clone()),
        None => TestExecutor::new(),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "info" } else { &cli.log_level };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    let explicit_config = match cli.config.as_deref() {
        Some(path) => Some(
            testcase_common::Config::load_from_path(path)
                .with_context(|| format!("Failed to load --config file: {}", path.display()))?,
        ),
        None => None,
    };

    let audit_log_path = cli
        .audit_log
        .clone()
        .or_else(|| std::env::var("AUDIT_LOG_FILE").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("audit.log.json"));

    let mut audit_logger = AuditLogger::with_file(&audit_log_path);
    if cli.no_audit {
        audit_logger.disable();
    }

    match cli.command {
        Commands::Generate {
            yaml_file,
            output,
            json_log,
            force,
            test_case_dir,
            audit_log,
            run_id,
        } => {
            let result = (|| -> Result<()> {
                let yaml_bytes = fs::read(&yaml_file)
                    .context(format!("Failed to read YAML file: {}", yaml_file.display()))?;
                let mut test_case = load_test_case(&yaml_file)?;

                let resolver = if let Some(dir) = test_case_dir {
                    build_dependency_resolver_from_dir(&dir)?
                } else {
                    build_dependency_resolver(&yaml_file)?
                };
                test_case = resolver
                    .resolve(&test_case)
                    .context("Failed to resolve dependencies")?;

                let executor = build_executor(explicit_config.as_ref()).with_run_id(run_id.clone());
                let script = executor.generate_test_script_from_yaml_with_path(
                    &test_case,
                    &yaml_bytes,
                    &yaml_file,
                );

                if let Some(output_path) = &output {
                    fs::write(output_path, &script).context(format!(
                        "Failed to write script to file: {}",
                        output_path.display()
                    ))?;
                    println!(
                        "Test script generated successfully: {}",
                        output_path.display()
                    );

                    if json_log {
                        executor.generate_execution_log_template(&test_case, output_path)?;
                    }

                    if let Some(audit_log_path) = audit_log {
                        let mut log = if audit_log_path.exists() {
                            AuditTraceabilityLog::load_from_file(&audit_log_path)?
                        } else {
                            AuditTraceabilityLog::new("default-witness".to_string())
                        };

                        let mut audit = TestCaseAudit::new();

                        let initial_stage = StageInfo::from_file(&yaml_file).context(format!(
                            "Failed to create stage info for initial file: {}",
                            yaml_file.display()
                        ))?;
                        audit.add_stage("initial", initial_stage);

                        let script_stage = StageInfo::from_file(output_path).context(format!(
                            "Failed to create stage info for shell script: {}",
                            output_path.display()
                        ))?;
                        audit.add_stage("05_shell_script", script_stage);

                        log.add_test_case(&test_case.id, audit);
                        log.save_to_file(&audit_log_path)?;

                        println!(
                            "✓ Test case '{}' added to audit log: {}",
                            test_case.id,
                            audit_log_path.display()
                        );
                    }
                } else {
                    print!("{}", script);
                    if json_log {
                        eprintln!("Warning: --json-log requires --output to be specified");
                    }
                    if audit_log.is_some() {
                        eprintln!("Warning: --audit-log requires --output to be specified");
                    }
                }

                let shellcheck_passed = run_shellcheck_validation(&script, force)?;
                if !shellcheck_passed {
                    std::process::exit(1);
                }

                Ok(())
            })();

            let status = if result.is_ok() {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            };
            let error = result.as_ref().err().map(|e| e.to_string());
            audit_logger.log_generate_script(
                &yaml_file,
                output.as_deref(),
                status,
                error.clone(),
            )?;

            result
        }
        Commands::Execute { yaml_file } => {
            let result = (|| -> Result<()> {
                let mut test_case = load_test_case(&yaml_file)?;

                let resolver = build_dependency_resolver(&yaml_file)?;
                test_case = resolver
                    .resolve(&test_case)
                    .context("Failed to resolve dependencies")?;

                let executor = build_executor(explicit_config.as_ref());
                executor.execute_test_case(&test_case)?;

                Ok(())
            })();

            let status = if result.is_ok() {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            };
            let error = result.as_ref().err().map(|e| e.to_string());
            audit_logger.log_execute_script(&yaml_file, status, error.clone())?;

            result
        }
        Commands::Hydrate {
            yaml_file,
            export_file,
            output,
        } => {
            let result = (|| -> Result<()> {
                let yaml_content = fs::read_to_string(&yaml_file)
                    .context(format!("Failed to read YAML file: {}", yaml_file.display()))?;

                let mut hydrator = VarHydrator::new();
                hydrator.load_from_export_file(&export_file)?;

                let hydrated_content = hydrator.hydrate_yaml_content(&yaml_content);

                if let Some(output_path) = &output {
                    fs::write(output_path, &hydrated_content).context(format!(
                        "Failed to write hydrated YAML to file: {}",
                        output_path.display()
                    ))?;
                    println!("Hydrated YAML written to: {}", output_path.display());
                } else {
                    print!("{}", hydrated_content);
                }

                Ok(())
            })();

            let status = if result.is_ok() {
                OperationStatus::Success
            } else {
                OperationStatus::Failed
            };
            let error = result.as_ref().err().map(|e| e.to_string());
            audit_logger.log_hydrate_yaml(
                &yaml_file,
                &export_file,
                output.as_deref(),
                status,
                error.clone(),
            )?;

            result
        }
        Commands::GenerateExport { yaml_file, output } => {
            let test_case = load_test_case(&yaml_file)?;

            let mut hydrator = VarHydrator::new();

            if let Some(hydration_vars) = &test_case.hydration_vars {
                for (var_name, env_var) in hydration_vars {
                    let value = env_var.default_value.as_deref().unwrap_or("");
                    hydrator.set(var_name.clone(), value.to_string());
                }
            }

            if let Some(output_path) = output {
                hydrator.generate_export_file(&output_path)?;
                println!("Export file template generated: {}", output_path.display());
            } else {
                let temp_file =
                    tempfile::NamedTempFile::new().context("Failed to create temporary file")?;
                hydrator.generate_export_file(temp_file.path())?;
                let content = fs::read_to_string(temp_file.path())
                    .context("Failed to read temporary file")?;
                print!("{}", content);
            }

            Ok(())
        }
        Commands::ValidateExport {
            yaml_file,
            export_file,
        } => {
            let test_case = load_test_case(&yaml_file)?;

            let mut hydrator = VarHydrator::new();
            hydrator.load_from_export_file(&export_file)?;

            let mut missing_vars = Vec::new();
            let mut missing_required_vars = Vec::new();

            if let Some(hydration_vars) = &test_case.hydration_vars {
                for (var_name, env_var) in hydration_vars {
                    if !hydrator.contains(var_name) {
                        missing_vars.push(var_name.clone());
                        if env_var.required {
                            missing_required_vars.push(var_name.clone());
                        }
                    }
                }
            }

            if missing_required_vars.is_empty() {
                if missing_vars.is_empty() {
                    println!("✓ Export file is valid: all variables are present");
                } else {
                    println!("✓ Export file is valid: all required variables are present");
                    println!("  Optional variables missing: {}", missing_vars.join(", "));
                }
                Ok(())
            } else {
                eprintln!("✗ Export file validation failed");
                eprintln!(
                    "  Required variables missing: {}",
                    missing_required_vars.join(", ")
                );
                if !missing_vars.is_empty() {
                    let optional_missing: Vec<String> = missing_vars
                        .into_iter()
                        .filter(|v| !missing_required_vars.contains(v))
                        .collect();
                    if !optional_missing.is_empty() {
                        eprintln!(
                            "  Optional variables missing: {}",
                            optional_missing.join(", ")
                        );
                    }
                }
                std::process::exit(1);
            }
        }
        Commands::List {
            base_path,
            manual_only,
            automated_only,
            show_stats,
        } => list_test_cases(base_path, manual_only, automated_only, show_stats),
        Commands::IndependenceAssignment(command) => independence_assignment(command),
        Commands::TraceMatrix {
            test_case_dir,
            requirements,
            runs,
            id,
            baseline_id,
            output,
            no_coverage_check,
        } => trace_matrix(
            &test_case_dir,
            &requirements,
            &runs,
            id,
            baseline_id,
            output,
            no_coverage_check,
        ),
        Commands::Report {
            matrix,
            runs,
            kinds,
            shape,
            output,
            test_cases,
            requirements_export,
            generated_at,
        } => report(
            &matrix,
            &runs,
            &kinds,
            shape,
            output,
            ReportOptions {
                test_cases: test_cases.as_deref(),
                requirements_export: requirements_export.as_deref(),
                generated_at: generated_at.as_deref(),
            },
        ),
        Commands::RegressionSelect {
            test_case_dir,
            baselines,
            plan,
            format,
            output,
            allow_unknown_test_cases,
        } => regression_select(
            &test_case_dir,
            &baselines,
            plan.as_deref(),
            &format,
            output,
            allow_unknown_test_cases,
        ),
        Commands::CoverageImport {
            lcov,
            mcdc,
            trace_matrix,
            annotations,
            baseline_id,
            id,
            run_id,
            tool,
            tool_version,
            output,
            allow_unclassified,
        } => coverage_import(CoverageImportArgs {
            lcov,
            mcdc,
            trace_matrix,
            annotations,
            baseline_id,
            id,
            run_id,
            tool,
            tool_version,
            output,
            allow_unclassified,
        }),
        Commands::Resolve { yaml_files, output } => {
            let resolver = build_resolver_from_files(&yaml_files)?;

            if let Some(output_dir) = output {
                if !output_dir.exists() {
                    fs::create_dir_all(&output_dir).context(format!(
                        "Failed to create output directory: {}",
                        output_dir.display()
                    ))?;
                }

                for yaml_file in &yaml_files {
                    let test_case = load_test_case(yaml_file)?;
                    let resolved = resolver.resolve(&test_case)?;

                    let output_filename = format!("{}_resolved.yaml", resolved.id);
                    let output_path = output_dir.join(&output_filename);

                    let yaml_content = serde_yaml::to_string(&resolved)
                        .context("Failed to serialize resolved test case")?;

                    fs::write(&output_path, yaml_content).context(format!(
                        "Failed to write resolved YAML to: {}",
                        output_path.display()
                    ))?;

                    println!("Resolved test case written to: {}", output_path.display());
                }
            } else {
                for yaml_file in &yaml_files {
                    let test_case = load_test_case(yaml_file)?;
                    let resolved = resolver.resolve(&test_case)?;

                    let yaml_content = serde_yaml::to_string(&resolved)
                        .context("Failed to serialize resolved test case")?;

                    println!("---");
                    println!("# Resolved: {}", yaml_file.display());
                    print!("{}", yaml_content);
                }
            }

            Ok(())
        }
        Commands::AuditLog { command } => match command {
            AuditLogCommands::Create {
                output,
                witness_key,
            } => {
                let log = AuditTraceabilityLog::new(witness_key);
                log.save_to_file(&output)?;
                println!("✓ Audit traceability log created: {}", output.display());
                Ok(())
            }
            AuditLogCommands::Add {
                log_file,
                test_case_id,
                initial,
                shell_script,
            } => {
                let mut log = if log_file.exists() {
                    AuditTraceabilityLog::load_from_file(&log_file)?
                } else {
                    AuditTraceabilityLog::new("default-witness".to_string())
                };

                let mut audit = TestCaseAudit::new();

                let initial_stage = StageInfo::from_file(&initial).context(format!(
                    "Failed to create stage info for initial file: {}",
                    initial.display()
                ))?;
                audit.add_stage("initial", initial_stage);

                if let Some(script_path) = shell_script {
                    let script_stage = StageInfo::from_file(&script_path).context(format!(
                        "Failed to create stage info for shell script: {}",
                        script_path.display()
                    ))?;
                    audit.add_stage("05_shell_script", script_stage);
                }

                log.add_test_case(&test_case_id, audit);
                log.save_to_file(&log_file)?;

                println!(
                    "✓ Test case '{}' added to audit log: {}",
                    test_case_id,
                    log_file.display()
                );
                Ok(())
            }
            AuditLogCommands::Verify {
                log_file,
                test_case_id,
            } => {
                let log = AuditTraceabilityLog::load_from_file(&log_file)?;

                if let Some(tc_id) = test_case_id {
                    let result = log.verify_test_case(&tc_id)?;
                    result.print_summary();

                    if !result.all_passed {
                        std::process::exit(1);
                    }
                } else {
                    let results = log.verify_all()?;

                    if results.is_empty() {
                        println!("No test cases found in audit log.");
                        return Ok(());
                    }

                    println!("\n=== Audit Verification Results ===\n");

                    let mut all_passed = true;
                    for result in &results {
                        result.print_summary();
                        println!();
                        if !result.all_passed {
                            all_passed = false;
                        }
                    }

                    println!("=== Summary ===");
                    let passed_count = results.iter().filter(|r| r.all_passed).count();
                    let failed_count = results.len() - passed_count;
                    println!("Total test cases: {}", results.len());
                    println!("Passed: {}", passed_count);
                    println!("Failed: {}", failed_count);

                    if !all_passed {
                        std::process::exit(1);
                    }
                }

                Ok(())
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shellcheck_validation_valid_script() {
        if std::process::Command::new("shellcheck")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
        {
            eprintln!("shellcheck not available, skipping test");
            return;
        }

        let script = "#!/bin/bash\nset -euo pipefail\necho 'hello world'\n";
        let result = run_shellcheck_validation(script, false).unwrap();
        assert!(result, "Valid script should pass shellcheck");
    }

    #[test]
    fn test_shellcheck_validation_force_flag() {
        if std::process::Command::new("shellcheck")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
        {
            eprintln!("shellcheck not available, skipping test");
            return;
        }

        // Even if shellcheck finds issues, force=true should return Ok(true)
        let script = "#!/bin/bash\necho 'hello world'\n";
        let result = run_shellcheck_validation(script, true).unwrap();
        assert!(result, "Force flag should always return true");
    }

    #[test]
    fn test_shellcheck_validation_generated_script() {
        if std::process::Command::new("shellcheck")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_err()
        {
            eprintln!("shellcheck not available, skipping test");
            return;
        }

        // Load a real test case YAML file and generate a script from it
        let yaml_path = PathBuf::from("testcases/SELF_VALIDATED_EXAMPLE_002.yml");
        if !yaml_path.exists() {
            eprintln!("Test YAML file not found, skipping test");
            return;
        }

        let test_case = load_test_case(&yaml_path).expect("Failed to load test case YAML");
        let executor = TestExecutor::new();
        let script = executor.generate_test_script(&test_case);
        let result = run_shellcheck_validation(&script, false).unwrap();
        assert!(
            result,
            "Generated test script should pass shellcheck validation"
        );
    }

    #[test]
    fn strictdoc_export_gives_hlr_and_llr_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.json");
        std::fs::write(
            &path,
            r#"{"documents":[{"path":"docs/srs.sdoc","requirements":[
                {"uid":"SYS-1","level":"system"},
                {"uid":"HLR-1","level":"HLR"},
                {"uid":"LLR-2","level":"LLR"},
                {"uid":"SFR-3","level":"SFR"}]}]}"#,
        )
        .unwrap();
        let reqs = read_matrix_requirements(&path).unwrap();
        let ids: Vec<(&str, testcase_verification::RequirementLevel)> =
            reqs.iter().map(|r| (r.id.as_str(), r.level)).collect();
        assert_eq!(
            ids,
            vec![
                ("HLR-1", testcase_verification::RequirementLevel::Hlr),
                ("LLR-2", testcase_verification::RequirementLevel::Llr),
            ]
        );
    }

    #[test]
    fn strictdoc_export_refuses_unknown_levels_duplicates_and_bad_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.json");
        let cases = [
            (
                r#"{"documents":[{"path":"a.sdoc","requirements":[{"uid":"X-1","level":"Epic"}]}]}"#,
                "not a requirement type",
            ),
            (
                r#"{"documents":[{"path":"a.sdoc","requirements":[{"uid":"HLR-1","level":"HLR"},{"uid":"HLR-1","level":"HLR"}]}]}"#,
                "appears twice",
            ),
            ("{ not json", "not a StrictDoc export"),
        ];
        for (text, expected) in cases {
            std::fs::write(&path, text).unwrap();
            let err = format!("{:#}", read_matrix_requirements(&path).unwrap_err());
            assert!(err.contains(expected), "{err}");
        }
    }

    #[test]
    fn id_lists_are_still_read_as_before() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ids.txt");
        std::fs::write(&path, "HLR-1\nLLR-2:LLR\n").unwrap();
        assert_eq!(read_matrix_requirements(&path).unwrap().len(), 2);
    }
}
