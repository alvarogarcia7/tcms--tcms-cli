use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "testcase-manager")]
#[command(about = "A tool for managing test cases in YAML format", long_about = None)]
#[command(version)]
#[command(
    after_help = "ENVIRONMENT VARIABLES:\n    RUST_LOG    Set log level (trace, debug, info, warn, error). Overrides --log-level"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the test cases directory
    #[arg(short, long, default_value = "./testcases", global = true)]
    pub path: String,

    /// Set log level (trace, debug, info, warn, error)
    #[arg(long, value_name = "LEVEL", default_value = "warn", global = true)]
    pub log_level: String,

    /// Enable verbose output (equivalent to --log-level=info)
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new test case
    Create {
        /// ID for the test case
        #[arg(short, long)]
        id: Option<String>,
    },

    CreateGeneralInitialConditions {
        #[arg(short, long)]
        id: Option<String>,
    },

    /// Edit an existing test case
    Edit {
        /// ID of the test case to edit
        id: Option<String>,

        /// Use fuzzy finder if ID not provided
        #[arg(short, long)]
        fuzzy: bool,
    },

    /// List all test cases
    List {
        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by priority
        #[arg(short = 'r', long)]
        priority: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// View a test case
    View {
        /// ID of the test case to view
        id: Option<String>,

        /// Use fuzzy finder if ID not provided
        #[arg(short, long)]
        fuzzy: bool,
    },

    /// Delete a test case
    Delete {
        /// ID of the test case to delete
        id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Validate test case files
    Validate {
        /// Specific file to validate
        #[arg(short, long)]
        file: Option<String>,

        /// Validate all files
        #[arg(short, long)]
        all: bool,
    },

    /// Search test cases using fuzzy finder
    Search {
        /// Initial query
        query: Option<String>,
    },

    /// Export test cases to a test suite file
    Export {
        /// Output file name
        #[arg(short, long, default_value = "test-suite.yaml")]
        output: String,

        /// Filter by tags
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Import test cases from a test suite file
    Import {
        /// Input file to import
        file: String,

        /// Skip validation
        #[arg(short, long)]
        skip_validation: bool,
    },

    /// Git operations
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Initialize a new test case repository
    Init {
        /// Path to initialize
        path: Option<String>,

        /// Initialize git repository
        #[arg(short, long)]
        git: bool,
    },

    /// Create test case interactively with metadata prompts
    CreateInteractive {
        /// Path to the test cases directory
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Build test sequences interactively with git commits
    BuildSequences {
        /// Path to the test cases directory
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Add steps to a test sequence with git commits
    AddSteps {
        /// Path to the test cases directory
        #[arg(short, long)]
        path: Option<String>,

        /// Sequence ID to add steps to
        #[arg(short, long)]
        sequence_id: Option<i64>,
    },

    /// Build test sequences with step collection loops and commits
    BuildSequencesWithSteps {
        /// Path to the test cases directory
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Complete interactive workflow: metadata, conditions, sequences, steps with git commits
    Complete {
        /// Output file path (default: testcases/GSMA_4_4_2_3_TC_001.yml)
        #[arg(short, long, default_value = "testcases/GSMA_4_4_2_3_TC_001.yml")]
        output: String,

        /// Commit message prefix (default: empty)
        #[arg(short, long)]
        commit_prefix: Option<String>,

        /// Use sample data for prompts (default answers that can be modified)
        #[arg(long)]
        sample: bool,
    },

    /// Parse general initial conditions from database with fuzzy search
    ParseGeneralConditions {
        /// Path to the database file
        #[arg(short, long, default_value = "data")]
        database: String,

        /// Path to the test cases directory
        #[arg(short = 'p', long)]
        path: Option<String>,
    },

    /// Parse initial conditions from database with fuzzy search
    ParseInitialConditions {
        /// Path to the database file
        #[arg(short, long, default_value = "data")]
        database: String,

        /// Path to the test cases directory
        #[arg(short = 'p', long)]
        path: Option<String>,
    },

    /// Parse initial conditions from database with fuzzy search
    ParseInitialConditionsComplex {
        /// Path to the database file
        #[arg(short, long, default_value = "data")]
        database: String,

        /// Path to the test cases directory
        #[arg(short = 'p', long)]
        path: Option<String>,
    },

    /// Validate a YAML payload against a JSON schema
    ValidateYaml {
        /// Path to the YAML payload file
        yaml_file: String,

        /// Path to the JSON schema file
        schema_file: String,
    },
}

#[derive(Subcommand)]
pub enum GitCommands {
    /// Add test cases to git staging
    Add {
        /// Test case IDs to add
        ids: Vec<String>,

        /// Add all test cases
        #[arg(short, long)]
        all: bool,
    },

    /// Commit changes
    Commit {
        /// Commit message
        #[arg(short, long)]
        message: String,
    },

    /// Show git status
    Status,

    /// Show commit log
    Log {
        /// Number of commits to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}
