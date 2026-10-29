use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of prerequisite
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PrerequisiteType {
    /// Manual prerequisite requiring human verification
    Manual,
    /// Automatic prerequisite that can be verified programmatically
    Automatic,
}

/// Prerequisite that must be satisfied before running a test case
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Prerequisite {
    /// Type of prerequisite
    #[serde(rename = "type")]
    pub prerequisite_type: PrerequisiteType,

    /// Description of the prerequisite
    pub description: String,

    /// Command to verify the prerequisite (required for automatic type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_command: Option<String>,
}

/// Environment variable definition for hydration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EnvVariable {
    /// Name of the environment variable
    pub name: String,

    /// Description of the environment variable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Default value for the environment variable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,

    /// Whether this environment variable is required
    #[serde(default)]
    pub required: bool,
}

/// Configuration for environment variables (alias for EnvVariable)
pub type EnvVarConfig = EnvVariable;

/// General verification with name and condition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GeneralVerification {
    /// Name of the verification
    pub name: String,

    /// Condition to evaluate
    pub condition: String,
}

impl fmt::Display for GeneralVerification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.condition)
    }
}

/// Verification information for a test step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Verification {
    /// Verification result value
    pub result: VerificationExpression,

    /// Verification output (from output variable)
    pub output: VerificationExpression,

    /// Verification output from file (optional - reads from LOG_FILE instead of COMMAND_OUTPUT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<VerificationExpression>,

    /// General verifications (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general: Option<Vec<GeneralVerification>>,
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result_str = match &self.result {
            VerificationExpression::Simple(s) => s.clone(),
            VerificationExpression::Conditional { condition, .. } => {
                format!("conditional({})", condition)
            }
        };
        let output_str = match &self.output {
            VerificationExpression::Simple(s) => s.clone(),
            VerificationExpression::Conditional { condition, .. } => {
                format!("conditional({})", condition)
            }
        };
        write!(
            f,
            "result: {} | output: {}{}",
            result_str,
            output_str,
            if self.output_file.is_some() {
                " | output_file: enabled"
            } else {
                ""
            }
        )
    }
}

/// Verification expression that can be either a simple string or a conditional expression
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VerificationExpression {
    /// Conditional expression with condition and branches (try first for struct matching)
    Conditional {
        /// The condition to evaluate
        condition: String,

        /// Expressions to run if condition is true
        if_true: Option<Vec<String>>,

        /// Expressions to run if condition is false
        if_false: Option<Vec<String>>,

        /// Expressions to always run regardless of condition
        always: Option<Vec<String>>,
    },

    /// Simple string expression for backward compatibility (fallback for non-struct input)
    Simple(String),
}

impl VerificationExpression {
    /// Create a VerificationExpression from a boolean value
    /// true -> "true", false -> "false"
    pub fn from_bool(value: bool) -> Self {
        VerificationExpression::Simple(value.to_string())
    }
}

// Custom deserialization to support both string and boolean values
impl<'de> serde::Deserialize<'de> for VerificationExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;
        use std::fmt;

        struct VerificationExpressionVisitor;

        impl<'de> Visitor<'de> for VerificationExpressionVisitor {
            type Value = VerificationExpression;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, boolean, or conditional expression object")
            }

            fn visit_bool<E>(self, value: bool) -> Result<VerificationExpression, E>
            where
                E: serde::de::Error,
            {
                Ok(VerificationExpression::Simple(value.to_string()))
            }

            fn visit_str<E>(self, value: &str) -> Result<VerificationExpression, E>
            where
                E: serde::de::Error,
            {
                Ok(VerificationExpression::Simple(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<VerificationExpression, E>
            where
                E: serde::de::Error,
            {
                Ok(VerificationExpression::Simple(value))
            }

            fn visit_map<M>(self, mut map: M) -> Result<VerificationExpression, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                // Try to deserialize as Conditional expression
                let mut condition: Option<String> = None;
                let mut if_true: Option<Vec<String>> = None;
                let mut if_false: Option<Vec<String>> = None;
                let mut always: Option<Vec<String>> = None;

                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    match key.as_str() {
                        "condition" => condition = Some(value.as_str().unwrap_or("").to_string()),
                        "if_true" => {
                            if_true = value.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                        }
                        "if_false" => {
                            if_false = value.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                        }
                        "always" => {
                            always = value.as_array().map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                        }
                        _ => {}
                    }
                }

                if let Some(cond) = condition {
                    Ok(VerificationExpression::Conditional {
                        condition: cond,
                        if_true,
                        if_false,
                        always,
                    })
                } else {
                    Err(serde::de::Error::custom(
                        "Conditional variant requires 'condition' field",
                    ))
                }
            }
        }

        deserializer.deserialize_any(VerificationExpressionVisitor)
    }
}

// Custom serialization to match custom deserialization
impl serde::Serialize for VerificationExpression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            VerificationExpression::Simple(s) => {
                // Serialize Simple variant as just a plain string value, without the enum tag
                serializer.serialize_str(s)
            }
            VerificationExpression::Conditional {
                condition,
                if_true,
                if_false,
                always,
            } => {
                // Serialize Conditional variant as a map object
                // Count how many fields will be serialized (condition is always present, others are optional)
                let mut field_count = 1; // condition is always present
                if if_true.is_some() {
                    field_count += 1;
                }
                if if_false.is_some() {
                    field_count += 1;
                }
                if always.is_some() {
                    field_count += 1;
                }

                let mut map = serializer.serialize_map(Some(field_count))?;
                map.serialize_entry("condition", condition)?;
                if let Some(ref items) = if_true {
                    map.serialize_entry("if_true", items)?;
                }
                if let Some(ref items) = if_false {
                    map.serialize_entry("if_false", items)?;
                }
                if let Some(ref items) = always {
                    map.serialize_entry("always", items)?;
                }
                map.end()
            }
        }
    }
}

impl fmt::Display for VerificationExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationExpression::Simple(s) => write!(f, "{}", s),
            VerificationExpression::Conditional { condition, .. } => {
                write!(f, "conditional({})", condition)
            }
        }
    }
}

impl VerificationExpression {
    /// Check if the expression is empty (for Simple variant)
    pub fn is_empty(&self) -> bool {
        match self {
            VerificationExpression::Simple(s) => s.is_empty(),
            VerificationExpression::Conditional { .. } => false,
        }
    }
}

impl From<&VerificationExpression> for bash_eval::BashExpression {
    fn from(expr: &VerificationExpression) -> Self {
        match expr {
            VerificationExpression::Simple(s) => bash_eval::BashExpression::Simple(s.clone()),
            VerificationExpression::Conditional {
                condition,
                if_true,
                if_false,
                always,
            } => bash_eval::BashExpression::Conditional {
                condition: condition.clone(),
                if_true: if_true.clone(),
                if_false: if_false.clone(),
                always: always.clone(),
            },
        }
    }
}

/// Capture variable configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaptureVar {
    /// Name of the variable to capture
    pub name: String,

    /// Regex pattern to capture from COMMAND_OUTPUT (mutually exclusive with command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<String>,

    /// Command to execute to capture the value (mutually exclusive with capture)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl CaptureVar {
    /// Validate that capture and command are mutually exclusive
    pub fn validate(&self) -> Result<(), String> {
        match (&self.capture, &self.command) {
            (Some(_), Some(_)) => Err(format!(
                "Variable '{}': capture and command are mutually exclusive",
                self.name
            )),
            (None, None) => Err(format!(
                "Variable '{}': either capture or command must be specified",
                self.name
            )),
            _ => Ok(()),
        }
    }
}

/// Format for capture_vars field in Step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(untagged)]
pub enum CaptureVarsFormat {
    /// Legacy format: map of variable names to regex patterns
    Legacy(BTreeMap<String, String>),

    /// New format: list of CaptureVar objects with name and either capture or command
    New(Vec<CaptureVar>),
}

/// Expected outcome for a test step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Expected {
    /// Whether the step should succeed (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,

    /// Expected result value
    pub result: String,

    /// Expected output
    pub output: String,
}

impl fmt::Display for Expected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let success_str = match self.success {
            Some(true) => "true",
            Some(false) => "false",
            None => "None",
        };
        write!(
            f,
            "success: {} | result: {} | output: {}",
            success_str, self.result, self.output
        )
    }
}

/// Represents a single step in a test sequence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Step {
    /// Step number
    pub step: i64,

    /// Whether this is a manual step (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual: Option<bool>,

    /// Description of the step
    pub description: String,

    /// Command to execute
    pub command: String,

    /// Variable capture configuration (supports both legacy map format and new list format)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_vars: Option<CaptureVarsFormat>,

    /// Expected outcome
    pub expected: Expected,

    /// Verification information
    #[serde(default = "default_verification_from_expected")]
    pub verification: Verification,

    /// Reference to another test sequence or step (optional)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// Default verification based on expected values (for backward compatibility)
fn default_verification_from_expected() -> Verification {
    Verification {
        result: VerificationExpression::Simple("[[ $? -eq 0 ]]".to_string()),
        output: VerificationExpression::Simple(
            "cat $COMMAND_OUTPUT | grep -q \"${OUTPUT}\"".to_string(),
        ),
        output_file: None,
        general: None,
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}: {} ({})", self.step, self.description, self.command)
    }
}

/// Target for test_sequence reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TestSequenceRefTarget {
    /// ID of the test sequence
    pub id: i64,

    /// Step identifier
    pub step: String,
}

/// Item in initial conditions array (untagged enum)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(untagged)]
pub enum InitialConditionItem {
    /// Plain string condition
    String(String),

    /// Reference to another location
    RefItem {
        /// Reference identifier
        #[serde(rename = "ref")]
        reference: String,
    },

    /// Reference to a test sequence step
    TestSequenceRef {
        /// Test sequence reference
        test_sequence: TestSequenceRefTarget,
    },
}

/// Include reference for dependencies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IncludeRef {
    /// ID of the include
    pub id: String,

    /// Optional test sequence reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_sequence: Option<String>,
}

/// Initial conditions structure supporting both include array and device-keyed conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InitialConditions {
    /// Include array for dependencies (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<IncludeRef>>,

    /// Device-keyed condition arrays
    #[serde(flatten)]
    pub devices: HashMap<String, Vec<InitialConditionItem>>,
}

impl InitialConditions {
    /// Check if the initial conditions structure is empty
    pub fn is_empty(&self) -> bool {
        self.include.is_none() && self.devices.is_empty()
    }
}

/// Error handling behavior for hooks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum OnError {
    /// Fail the test execution if hook fails
    Fail,
    /// Continue test execution even if hook fails
    Continue,
}

/// Configuration for a specific hook
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HookConfig {
    /// Command or script to execute
    pub command: String,

    /// Error handling behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<OnError>,
}

/// Hooks configuration for test execution lifecycle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Hooks {
    /// Hook executed at the start of the script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_start: Option<HookConfig>,

    /// Hook executed before setting up a test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_test: Option<HookConfig>,

    /// Hook executed before a test sequence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_sequence: Option<HookConfig>,

    /// Hook executed after a test sequence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_sequence: Option<HookConfig>,

    /// Hook executed before each test step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_step: Option<HookConfig>,

    /// Hook executed after each test step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_step: Option<HookConfig>,

    /// Hook executed when tearing down a test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teardown_test: Option<HookConfig>,

    /// Hook executed at the end of the script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_end: Option<HookConfig>,
}

/// A sequence of test steps
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestSequence {
    /// Sequence identifier
    pub id: i64,

    /// Name of the test sequence
    pub name: String,

    /// Description of the test sequence
    pub description: String,

    /// Sequence-scoped variables that can be declared and used throughout the test sequence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, String>>,

    /// Initial conditions specific to this sequence
    pub initial_conditions: InitialConditions,

    /// List of steps in the sequence
    pub steps: Vec<Step>,

    /// Reference to another test sequence or step (optional)
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl fmt::Display for TestSequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}: {} - {}", self.id, self.name, self.description)
    }
}

/// Kind of test: which inputs and conditions the test case exercises
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    /// Inputs the software should handle
    NormalRange,
    /// Abnormal inputs and conditions
    Robustness,
    /// Attacks and misuse
    Security,
}

impl TestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestType::NormalRange => "normal_range",
            TestType::Robustness => "robustness",
            TestType::Security => "security",
        }
    }

    /// The categories that belong to this test type
    pub fn categories(&self) -> &'static [TestCategory] {
        match self {
            TestType::NormalRange => &[
                TestCategory::EquivalenceClass,
                TestCategory::BoundaryValue,
                TestCategory::StateTransition,
            ],
            TestType::Robustness => &[
                TestCategory::InvalidValue,
                TestCategory::OutOfRange,
                TestCategory::FailureMode,
                TestCategory::Overflow,
                TestCategory::TimingFault,
            ],
            TestType::Security => &[
                TestCategory::Fuzzing,
                TestCategory::Penetration,
                TestCategory::MisuseCase,
                TestCategory::VulnerabilityScan,
            ],
        }
    }
}

impl fmt::Display for TestType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Technique used to select the inputs of a test case; belongs to exactly one [`TestType`]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    EquivalenceClass,
    BoundaryValue,
    StateTransition,
    InvalidValue,
    OutOfRange,
    FailureMode,
    Overflow,
    TimingFault,
    Fuzzing,
    Penetration,
    MisuseCase,
    VulnerabilityScan,
}

impl TestCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestCategory::EquivalenceClass => "equivalence_class",
            TestCategory::BoundaryValue => "boundary_value",
            TestCategory::StateTransition => "state_transition",
            TestCategory::InvalidValue => "invalid_value",
            TestCategory::OutOfRange => "out_of_range",
            TestCategory::FailureMode => "failure_mode",
            TestCategory::Overflow => "overflow",
            TestCategory::TimingFault => "timing_fault",
            TestCategory::Fuzzing => "fuzzing",
            TestCategory::Penetration => "penetration",
            TestCategory::MisuseCase => "misuse_case",
            TestCategory::VulnerabilityScan => "vulnerability_scan",
        }
    }

    /// The test type this category belongs to
    pub fn test_type(&self) -> TestType {
        match self {
            TestCategory::EquivalenceClass
            | TestCategory::BoundaryValue
            | TestCategory::StateTransition => TestType::NormalRange,
            TestCategory::InvalidValue
            | TestCategory::OutOfRange
            | TestCategory::FailureMode
            | TestCategory::Overflow
            | TestCategory::TimingFault => TestType::Robustness,
            TestCategory::Fuzzing
            | TestCategory::Penetration
            | TestCategory::MisuseCase
            | TestCategory::VulnerabilityScan => TestType::Security,
        }
    }
}

impl fmt::Display for TestCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Test level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TestLevel {
    /// Hardware/software integration (high-level requirements, on target)
    HwSwIntegration,
    /// Software integration (requirement interaction, data and control coupling)
    SwIntegration,
    /// Low-level requirements
    LowLevel,
    /// System-level validation
    Validation,
}

impl TestLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestLevel::HwSwIntegration => "hw_sw_integration",
            TestLevel::SwIntegration => "sw_integration",
            TestLevel::LowLevel => "low_level",
            TestLevel::Validation => "validation",
        }
    }
}

impl fmt::Display for TestLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Life-cycle status of a test case; changes are gated by reviews
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TestCaseStatus {
    Draft,
    InReview,
    Approved,
    Obsolete,
}

/// External system a record is linked to
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSystem {
    Jira,
    Xray,
    ZephyrScale,
    ZephyrSquad,
    Strictdoc,
    Gitlab,
    Other,
}

/// Link from a record to an entity in an external system (Jira issue, Xray test, ...)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalRef {
    /// External system
    pub system: ExternalSystem,

    /// Identifier in that system (issue key, test key, UID)
    pub key: String,

    /// Resolved URL; derived by the resolver when absent
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Last successful sync of this record to that system (RFC 3339)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
}

/// A complete test case following the GSMA schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestCase {
    /// Document type (envelope field)
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,

    /// Schema reference (envelope field)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Requirement identifier
    pub requirement: String,

    /// Item number
    pub item: i64,

    /// TC number
    pub tc: i64,

    /// Unique identifier for the test case
    pub id: String,

    /// Description of the test case
    pub description: String,

    /// Prerequisites that must be satisfied before running this test case
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisites: Option<Vec<Prerequisite>>,

    /// General initial conditions
    pub general_initial_conditions: InitialConditions,

    /// Initial conditions
    pub initial_conditions: InitialConditions,

    /// Test sequences
    pub test_sequences: Vec<TestSequence>,

    /// Environment variables that need hydration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hydration_vars: Option<HashMap<String, EnvVariable>>,

    /// Hooks configuration for test execution lifecycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Hooks>,

    /// Requirement IDs (HLR/LLR, owned by the requirements tool) this test case verifies.
    /// Traceability and coverage use this list only, not the legacy `requirement` string.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<String>,

    /// Kind of test (normal range, robustness, security)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_type: Option<TestType>,

    /// Technique used to select inputs; must belong to `test_type`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_category: Option<TestCategory>,

    /// Test level
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<TestLevel>,

    /// The single objective this test case demonstrates
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,

    /// Test environment required to run the procedure
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Procedure-level pass/fail criteria, in addition to per-step `expected`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_fail_criteria: Option<String>,

    /// Person who wrote the test case (independence checks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// True if the review of this test case and its results must not be done by `author`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independence_required: Option<bool>,

    /// Test case version; runs and results record which version they executed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Life-cycle status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TestCaseStatus>,

    /// Entities in external systems created for this test case
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Collection of test cases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestSuite {
    /// Name of the test suite
    pub name: String,

    /// Description of the test suite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Version of the test suite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// List of test cases
    pub test_cases: Vec<TestCase>,
}

impl TestCase {
    /// Create a new test case with required fields
    pub fn new(requirement: String, item: i64, tc: i64, id: String, description: String) -> Self {
        Self {
            doc_type: None,
            schema: None,
            requirement,
            item,
            tc,
            id,
            description,
            prerequisites: None,
            general_initial_conditions: InitialConditions::default(),
            initial_conditions: InitialConditions::default(),
            test_sequences: Vec::new(),
            hydration_vars: None,
            hooks: None,
            requirements: Vec::new(),
            test_type: None,
            test_category: None,
            level: None,
            objective: None,
            environment: None,
            pass_fail_criteria: None,
            author: None,
            independence_required: None,
            version: None,
            status: None,
            external_refs: Vec::new(),
        }
    }

    /// Check that `test_category` belongs to `test_type` when both are set.
    ///
    /// The error names the offending field and lists the categories the type allows.
    pub fn validate_classification(&self) -> Result<(), String> {
        if let (Some(test_type), Some(category)) = (self.test_type, self.test_category) {
            if category.test_type() != test_type {
                let allowed: Vec<&str> =
                    test_type.categories().iter().map(|c| c.as_str()).collect();
                return Err(format!(
                    "test_category '{}' does not belong to test_type '{}' in test case '{}' (allowed: {})",
                    category,
                    test_type,
                    self.id,
                    allowed.join(", ")
                ));
            }
        }
        Ok(())
    }

    /// Check if the test case has any manual steps
    pub fn has_manual_steps(&self) -> bool {
        self.test_sequences
            .iter()
            .any(|sequence| sequence.steps.iter().any(|step| step.manual == Some(true)))
    }

    /// Check if the test case has any automated (non-manual) steps
    pub fn has_automated_steps(&self) -> bool {
        self.test_sequences
            .iter()
            .any(|sequence| sequence.steps.iter().any(|step| step.manual != Some(true)))
    }

    /// Get the total count of manual steps across all test sequences
    pub fn get_manual_step_count(&self) -> usize {
        self.test_sequences
            .iter()
            .flat_map(|sequence| &sequence.steps)
            .filter(|step| step.manual == Some(true))
            .count()
    }
}

impl TestSequence {
    /// Create a new test sequence
    pub fn new(id: i64, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            variables: None,
            initial_conditions: InitialConditions::default(),
            steps: Vec::new(),
            reference: None,
        }
    }
}

impl Step {
    /// Create a new step
    pub fn new(
        step: i64,
        description: String,
        command: String,
        result: String,
        output: String,
    ) -> Self {
        Self {
            step,
            manual: None,
            description,
            command,
            capture_vars: None,
            expected: Expected {
                success: None,
                result,
                output,
            },
            verification: default_verification_from_expected(),
            reference: None,
        }
    }
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(name: String) -> Self {
        Self {
            name,
            description: None,
            version: None,
            test_cases: Vec::new(),
        }
    }
}

/// Detailed validation error information
#[derive(Debug, Clone)]
pub struct ValidationErrorDetail {
    /// JSON path where the error occurred (e.g., "/test_sequences/0/steps/1/expected")
    pub path: String,

    /// The specific constraint that failed
    pub constraint: String,

    /// The actual value that was found
    pub found_value: String,

    /// The expected value or constraint
    pub expected_constraint: String,
}

/// Result of validating a file
#[derive(Debug, Clone)]
pub enum FileValidationStatus {
    /// File is valid
    Valid,

    /// File failed to parse as YAML
    ParseError { message: String },

    /// File failed schema validation
    ValidationError { errors: Vec<ValidationErrorDetail> },
}

/// Information about a test case file and its validation status
#[derive(Debug, Clone)]
pub struct TestCaseFileInfo {
    /// Path to the file
    pub path: PathBuf,

    /// Validation status
    pub status: FileValidationStatus,

    /// Test case data if successfully loaded
    pub test_case: Option<TestCase>,
}

/// Test execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestRunStatus {
    /// Test passed
    Pass,
    /// Test failed
    Fail,
    /// Test was skipped
    Skip,
}

impl fmt::Display for TestRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestRunStatus::Pass => write!(f, "Pass"),
            TestRunStatus::Fail => write!(f, "Fail"),
            TestRunStatus::Skip => write!(f, "Skip"),
        }
    }
}

/// Test run execution result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestRun {
    /// Test case ID
    pub test_case_id: String,

    /// Test execution status
    pub status: TestRunStatus,

    /// Timestamp when the test was executed
    pub timestamp: DateTime<Utc>,

    /// Duration in seconds
    pub duration: f64,

    /// Execution log capturing output and events
    pub execution_log: String,
    /// Optional error message (for failed or skipped tests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Optional test name/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl TestRun {
    /// Create a new test run
    pub fn new(
        test_case_id: String,
        status: TestRunStatus,
        timestamp: DateTime<Utc>,
        duration: f64,
        execution_log: String,
    ) -> Self {
        Self {
            test_case_id,
            status,
            timestamp,
            duration,
            error_message: None,
            name: None,
            execution_log,
        }
    }

    /// Create a test run with error message
    pub fn with_error(
        test_case_id: String,
        status: TestRunStatus,
        timestamp: DateTime<Utc>,
        duration: f64,
        error_message: String,
    ) -> Self {
        Self {
            test_case_id,
            status,
            timestamp,
            duration,
            error_message: Some(error_message),
            name: None,
            execution_log: "".to_string(),
        }
    }
}

/// Metadata for a test run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestRunMetadata {
    /// Identifier of the test case that was run
    pub test_case_id: String,

    /// Timestamp when the test was executed
    pub timestamp: DateTime<Utc>,

    /// Duration of the test run in milliseconds
    pub duration: u64,
}

/// Actual result of a test step execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActualResult {
    /// Actual result value
    pub result: String,

    /// Actual output
    pub output: String,

    /// Whether the step succeeded
    pub success: bool,
}

/// Result of executing a single test step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepExecutionResult {
    /// Step number
    pub step_number: i64,

    /// Timestamp when the step was executed
    pub timestamp: String,

    /// Actual result of the step
    pub actual_result: ActualResult,

    /// Duration of the step execution in milliseconds
    pub duration_ms: u64,

    /// Optional error message if the step failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Execution log for a test case
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestExecutionLog {
    /// Test case identifier
    pub test_case_id: String,

    /// Sequence identifier
    pub sequence_id: i64,

    /// Timestamp when the execution started
    pub timestamp: String,

    /// Actual output of the execution
    pub actual_output: String,

    /// Whether the overall execution was successful
    pub actual_success: bool,

    /// Total duration of the execution in milliseconds
    pub duration_ms: u64,

    /// Optional error message if the execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// Field-level difference between expected and actual values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDiff {
    /// Name of the field
    pub field_name: String,

    /// Expected value
    pub expected: String,

    /// Actual value
    pub actual: String,

    /// Whether the field matched
    pub matched: bool,
}

impl fmt::Display for FieldDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.matched {
            "\x1b[32m✓\x1b[0m" // Green checkmark
        } else {
            "\x1b[31m✗\x1b[0m" // Red X
        };
        write!(
            f,
            "    {} {}: expected '{}', actual '{}'",
            status, self.field_name, self.expected, self.actual
        )
    }
}

/// Hook type for test step execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Hook executed at the start of the script
    ScriptStart,
    /// Hook executed before setting up a test
    SetupTest,
    /// Hook executed before a test sequence
    BeforeSequence,
    /// Hook executed after a test sequence
    AfterSequence,
    /// Hook executed before each test step
    BeforeStep,
    /// Hook executed after each test step
    AfterStep,
    /// Hook executed when tearing down a test
    TeardownTest,
    /// Hook executed at the end of the script
    ScriptEnd,
}

impl fmt::Display for HookType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookType::ScriptStart => write!(f, "script_start"),
            HookType::SetupTest => write!(f, "setup_test"),
            HookType::BeforeSequence => write!(f, "before_sequence"),
            HookType::AfterSequence => write!(f, "after_sequence"),
            HookType::BeforeStep => write!(f, "before_step"),
            HookType::AfterStep => write!(f, "after_step"),
            HookType::TeardownTest => write!(f, "teardown_test"),
            HookType::ScriptEnd => write!(f, "script_end"),
        }
    }
}

/// Test step execution entry following the test execution log schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestStepExecutionEntry {
    /// Document type (envelope field)
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,

    /// Schema reference (envelope field)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// ID of the test run this entry belongs to (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,

    /// ID of the test case being executed (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_case_id: Option<String>,

    /// Sequence number of the test
    pub test_sequence: i64,

    /// Step number within the test sequence
    pub step: i64,

    /// The command that was executed
    pub command: String,

    /// Exit code of the command execution
    pub exit_code: i32,

    /// Output from the command execution
    pub output: String,

    /// Timestamp of the execution (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,

    /// Hook type if this entry represents a hook execution (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_type: Option<HookType>,

    /// Path to the hook script if this entry represents a hook execution (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook_path: Option<String>,

    /// Whether result verification passed
    pub result_verification_pass: bool,

    /// Whether output verification passed
    pub output_verification_pass: bool,
}

impl TestStepExecutionEntry {
    /// Create a new test step execution entry
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        test_sequence: i64,
        step: i64,
        command: String,
        exit_code: i32,
        output: String,
        result_verification_pass: bool,
        output_verification_pass: bool,
    ) -> Self {
        Self {
            doc_type: None,
            schema: None,
            run_id: None,
            test_case_id: None,
            test_sequence,
            step,
            command,
            exit_code,
            output,
            timestamp: None,
            hook_type: None,
            hook_path: None,
            result_verification_pass,
            output_verification_pass,
        }
    }

    /// Create a new test step execution entry with timestamp
    #[allow(clippy::too_many_arguments)]
    pub fn with_timestamp(
        test_sequence: i64,
        step: i64,
        command: String,
        exit_code: i32,
        output: String,
        timestamp: String,
        result_verification_pass: bool,
        output_verification_pass: bool,
    ) -> Self {
        Self {
            doc_type: None,
            schema: None,
            run_id: None,
            test_case_id: None,
            test_sequence,
            step,
            command,
            exit_code,
            output,
            timestamp: Some(timestamp),
            hook_type: None,
            hook_path: None,
            result_verification_pass,
            output_verification_pass,
        }
    }

    /// Check if the execution was successful (exit code 0)
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if the execution failed (non-zero exit code)
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Validate that the entry has all required fields with non-empty values
    pub fn validate(&self) -> Result<(), String> {
        if self.test_sequence < 0 {
            return Err("test_sequence must be non-negative".to_string());
        }

        if self.step < 0 {
            return Err("step must be non-negative".to_string());
        }

        if self.command.is_empty() {
            return Err("command must not be empty".to_string());
        }

        Ok(())
    }

    /// Check if this entry matches a given test sequence and step
    pub fn matches(&self, test_sequence: i64, step: i64) -> bool {
        self.test_sequence == test_sequence && self.step == step
    }

    /// Get a formatted string representation of the execution result
    pub fn result_summary(&self) -> String {
        let status = if self.is_success() {
            "SUCCESS"
        } else {
            "FAILURE"
        };
        format!(
            "Sequence {}, Step {}: {} (exit code: {})",
            self.test_sequence, self.step, status, self.exit_code
        )
    }

    /// Parse timestamp if present and valid ISO 8601 format
    pub fn parse_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp.as_ref().and_then(|ts| {
            DateTime::parse_from_rfc3339(ts)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        })
    }
}

impl fmt::Display for TestStepExecutionEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref hook_type) = self.hook_type {
            write!(
                f,
                "Sequence {}, Step {}: {} (exit code: {}) [Hook: {}",
                self.test_sequence, self.step, self.command, self.exit_code, hook_type
            )?;
            if let Some(ref hook_path) = self.hook_path {
                write!(f, ", path: {}", hook_path)?;
            }
            write!(f, "]")
        } else {
            write!(
                f,
                "Sequence {}, Step {}: {} (exit code: {})",
                self.test_sequence, self.step, self.command, self.exit_code
            )
        }
    }
}

/// Overall status of verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VerificationStatus {
    /// All steps passed
    Passed,

    /// Some steps failed
    Failed,

    /// Some steps were skipped
    PartiallySkipped,

    /// All steps were skipped
    AllSkipped,
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerificationStatus::Passed => write!(f, "\x1b[32mPASSED\x1b[0m"),
            VerificationStatus::Failed => write!(f, "\x1b[31mFAILED\x1b[0m"),
            VerificationStatus::PartiallySkipped => write!(f, "\x1b[33mPARTIALLY SKIPPED\x1b[0m"),
            VerificationStatus::AllSkipped => write!(f, "\x1b[33mALL SKIPPED\x1b[0m"),
        }
    }
}

/// Result of verifying a single step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StepVerificationResult {
    /// Step number
    pub step_number: i64,

    /// Whether the step passed verification
    pub passed: bool,

    /// Whether the step was skipped
    pub skipped: bool,

    /// Field-level differences between expected and actual
    pub field_diffs: Vec<FieldDiff>,

    /// Optional error or failure message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

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

impl fmt::Display for StepVerificationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.skipped {
            "\x1b[33mSKIPPED\x1b[0m"
        } else if self.passed {
            "\x1b[32mPASS\x1b[0m"
        } else {
            "\x1b[31mFAIL\x1b[0m"
        };

        writeln!(f, "  Step {}: {}", self.step_number, status)?;

        if let Some(ref msg) = self.message {
            writeln!(f, "    Message: {}", msg)?;
        }

        if !self.field_diffs.is_empty() {
            writeln!(f, "    Field Differences:")?;
            for diff in &self.field_diffs {
                writeln!(f, "{}", diff)?;
            }
        }

        Ok(())
    }
}

/// Complete verification report for a test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationReport {
    /// Total number of steps
    pub total_steps: usize,

    /// Number of steps that passed
    pub passed_steps: usize,

    /// Number of steps that failed
    pub failed_steps: usize,

    /// Number of steps that were skipped
    pub skipped_steps: usize,

    /// Detailed results for each step
    pub step_results: Vec<StepVerificationResult>,

    /// Overall verification status
    pub overall_status: VerificationStatus,

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

impl fmt::Display for VerificationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Verification Report")?;
        writeln!(f, "===================")?;
        writeln!(f, "Overall Status: {}", self.overall_status)?;
        writeln!(f)?;
        writeln!(
            f,
            "Summary: {} total, {} passed, {} failed, {} skipped",
            self.total_steps, self.passed_steps, self.failed_steps, self.skipped_steps
        )?;
        writeln!(f)?;

        if !self.step_results.is_empty() {
            writeln!(f, "Step Results:")?;
            for result in &self.step_results {
                write!(f, "{}", result)?;
            }
        }

        Ok(())
    }
}

/// Container-level report format supporting both single test case and multi-test case reports
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestReportOutput {
    /// Report title (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Project name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,

    /// Test date (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_date: Option<String>,

    /// Environment name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,

    /// Platform information (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,

    /// Executor information (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,

    /// Execution duration in seconds (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_duration: Option<f64>,

    /// Test results - supports either single or multiple test cases
    pub results: TestReportResults,
}

/// Test results can be either a single test case or multiple test cases
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum TestReportResults {
    /// Single test case result
    Single(TestRun),

    /// Multiple test case results
    Multiple(Vec<TestRun>),
}

impl TestReportOutput {
    /// Create a new test report output for a single test case
    pub fn single(test_run: TestRun) -> Self {
        Self {
            title: None,
            project: None,
            test_date: None,
            environment: None,
            platform: None,
            executor: None,
            execution_duration: None,
            results: TestReportResults::Single(test_run),
        }
    }

    /// Create a new test report output for multiple test cases
    pub fn multiple(test_runs: Vec<TestRun>) -> Self {
        Self {
            title: None,
            project: None,
            test_date: None,
            environment: None,
            platform: None,
            executor: None,
            execution_duration: None,
            results: TestReportResults::Multiple(test_runs),
        }
    }

    /// Set the report title
    pub fn with_title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the project name
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// Set the test date
    pub fn with_test_date(mut self, test_date: String) -> Self {
        self.test_date = Some(test_date);
        self
    }

    /// Set the environment
    pub fn with_environment(mut self, environment: String) -> Self {
        self.environment = Some(environment);
        self
    }

    /// Set the platform
    pub fn with_platform(mut self, platform: String) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Set the executor
    pub fn with_executor(mut self, executor: String) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Set the execution duration
    pub fn with_execution_duration(mut self, duration: f64) -> Self {
        self.execution_duration = Some(duration);
        self
    }

    /// Get the number of test cases in the report
    pub fn test_case_count(&self) -> usize {
        match &self.results {
            TestReportResults::Single(_) => 1,
            TestReportResults::Multiple(runs) => runs.len(),
        }
    }

    /// Get all test runs as a slice
    pub fn test_runs(&self) -> Vec<&TestRun> {
        match &self.results {
            TestReportResults::Single(run) => vec![run],
            TestReportResults::Multiple(runs) => runs.iter().collect(),
        }
    }

    /// Calculate summary statistics
    pub fn summary_stats(&self) -> TestReportSummary {
        let runs = self.test_runs();
        let total = runs.len();
        let passed = runs
            .iter()
            .filter(|r| r.status == TestRunStatus::Pass)
            .count();
        let failed = runs
            .iter()
            .filter(|r| r.status == TestRunStatus::Fail)
            .count();
        let skipped = runs
            .iter()
            .filter(|r| r.status == TestRunStatus::Skip)
            .count();
        let total_duration = runs.iter().map(|r| r.duration).sum();

        TestReportSummary {
            total,
            passed,
            failed,
            skipped,
            total_duration,
        }
    }
}

/// Summary statistics for a test report
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestReportSummary {
    /// Total number of test cases
    pub total: usize,

    /// Number of passed test cases
    pub passed: usize,

    /// Number of failed test cases
    pub failed: usize,

    /// Number of skipped test cases
    pub skipped: usize,

    /// Total execution duration
    pub total_duration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expected_creation() {
        let expected = Expected {
            success: Some(true),
            result: "SW=0x9000".to_string(),
            output: "Success".to_string(),
        };
        assert_eq!(expected.success, Some(true));
        assert_eq!(expected.result, "SW=0x9000");
        assert_eq!(expected.output, "Success");
    }

    #[test]
    fn test_expected_without_success() {
        let expected = Expected {
            success: None,
            result: "SW=0x9000".to_string(),
            output: "Success".to_string(),
        };
        assert_eq!(expected.success, None);
    }

    #[test]
    fn test_step_creation() {
        let step = Step::new(
            1,
            "Test step".to_string(),
            "ssh".to_string(),
            "SW=0x9000".to_string(),
            "Success".to_string(),
        );
        assert_eq!(step.step, 1);
        assert_eq!(step.description, "Test step");
        assert_eq!(step.command, "ssh");
        assert_eq!(step.manual, None);
        assert_eq!(step.expected.result, "SW=0x9000");
    }

    #[test]
    fn test_step_with_manual() {
        let mut step = Step::new(
            1,
            "Manual step".to_string(),
            "ssh".to_string(),
            "result".to_string(),
            "output".to_string(),
        );
        step.manual = Some(true);
        assert_eq!(step.manual, Some(true));
    }

    #[test]
    fn test_test_sequence_creation() {
        let sequence = TestSequence::new(1, "Test Sequence".to_string(), "Description".to_string());
        assert_eq!(sequence.id, 1);
        assert_eq!(sequence.name, "Test Sequence");
        assert_eq!(sequence.description, "Description");
        assert_eq!(sequence.initial_conditions.devices.len(), 0);
        assert_eq!(sequence.steps.len(), 0);
    }

    #[test]
    fn test_test_sequence_with_steps() {
        let mut sequence =
            TestSequence::new(1, "Test Sequence".to_string(), "Description".to_string());
        let step = Step::new(
            1,
            "Step 1".to_string(),
            "ssh".to_string(),
            "result".to_string(),
            "output".to_string(),
        );
        sequence.steps.push(step);
        assert_eq!(sequence.steps.len(), 1);
    }

    #[test]
    fn test_test_case_creation() {
        let test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            2,
            "TC001".to_string(),
            "Test description".to_string(),
        );
        assert_eq!(test_case.requirement, "REQ001");
        assert_eq!(test_case.item, 1);
        assert_eq!(test_case.tc, 2);
        assert_eq!(test_case.id, "TC001");
        assert_eq!(test_case.description, "Test description");
        assert_eq!(test_case.general_initial_conditions.devices.len(), 0);
        assert!(test_case.initial_conditions.devices.is_empty());
        assert_eq!(test_case.test_sequences.len(), 0);
    }

    #[test]
    fn test_test_case_with_sequences() {
        let mut test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            2,
            "TC001".to_string(),
            "Test description".to_string(),
        );
        let sequence = TestSequence::new(1, "Seq 1".to_string(), "Desc".to_string());
        test_case.test_sequences.push(sequence);
        assert_eq!(test_case.test_sequences.len(), 1);
    }

    #[test]
    fn test_test_suite_creation() {
        let suite = TestSuite::new("Test Suite".to_string());
        assert_eq!(suite.name, "Test Suite");
        assert_eq!(suite.description, None);
        assert_eq!(suite.version, None);
        assert_eq!(suite.test_cases.len(), 0);
    }

    #[test]
    fn test_test_suite_with_description() {
        let mut suite = TestSuite::new("Test Suite".to_string());
        suite.description = Some("Suite description".to_string());
        suite.version = Some("1.0.0".to_string());
        assert_eq!(suite.description, Some("Suite description".to_string()));
        assert_eq!(suite.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_test_suite_with_test_cases() {
        let mut suite = TestSuite::new("Test Suite".to_string());
        let test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            2,
            "TC001".to_string(),
            "Test".to_string(),
        );
        suite.test_cases.push(test_case);
        assert_eq!(suite.test_cases.len(), 1);
    }

    #[test]
    fn test_serialization_and_deserialization() {
        let test_case = TestCase::new(
            "REQ001".to_string(),
            1,
            2,
            "TC001".to_string(),
            "Test description".to_string(),
        );

        let yaml = serde_yaml::to_string(&test_case).unwrap();
        let deserialized: TestCase = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(test_case, deserialized);
    }

    #[test]
    fn test_step_serialization_with_optional_fields() {
        let mut step = Step::new(
            1,
            "Step".to_string(),
            "ssh".to_string(),
            "result".to_string(),
            "output".to_string(),
        );
        step.manual = Some(true);
        step.expected.success = Some(false);

        let yaml = serde_yaml::to_string(&step).unwrap();
        assert!(yaml.contains("manual: true"));
        assert!(yaml.contains("success: false"));
    }

    #[test]
    fn test_step_serialization_without_optional_fields() {
        let step = Step::new(
            1,
            "Step".to_string(),
            "ssh".to_string(),
            "result".to_string(),
            "output".to_string(),
        );

        let yaml = serde_yaml::to_string(&step).unwrap();
        assert!(!yaml.contains("manual:"));
        assert!(!yaml.contains("success:"));
    }

    #[test]
    fn test_test_run_creation() {
        let timestamp = Utc::now();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.500,
            "".to_string(),
        );

        assert_eq!(test_run.test_case_id, "TC001");
        assert_eq!(test_run.status, TestRunStatus::Pass);
        assert_eq!(test_run.duration, 1.500);
        assert_eq!(test_run.error_message, None);
    }

    #[test]
    fn test_test_run_with_error() {
        let timestamp = Utc::now();
        let test_run = TestRun::with_error(
            "TC002".to_string(),
            TestRunStatus::Fail,
            timestamp,
            2.500,
            "Test failed due to assertion error".to_string(),
        );

        assert_eq!(test_run.test_case_id, "TC002");
        assert_eq!(test_run.status, TestRunStatus::Fail);
        assert_eq!(test_run.duration, 2.500);
        assert_eq!(
            test_run.error_message,
            Some("Test failed due to assertion error".to_string())
        );
    }

    #[test]
    fn test_test_run_status_display() {
        assert_eq!(format!("{}", TestRunStatus::Pass), "Pass");
        assert_eq!(format!("{}", TestRunStatus::Fail), "Fail");
        assert_eq!(format!("{}", TestRunStatus::Skip), "Skip");
    }

    #[test]
    fn test_test_step_execution_entry_creation() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            true,
            true,
        );

        assert_eq!(entry.test_sequence, 1);
        assert_eq!(entry.step, 2);
        assert_eq!(entry.command, "echo test");
        assert_eq!(entry.exit_code, 0);
        assert_eq!(entry.output, "test output");
        assert_eq!(entry.timestamp, None);
        assert!(entry.result_verification_pass);
        assert!(entry.output_verification_pass);
    }

    #[test]
    fn test_test_step_execution_entry_with_timestamp() {
        let entry = TestStepExecutionEntry::with_timestamp(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            true,
            true,
        );

        assert_eq!(entry.test_sequence, 1);
        assert_eq!(entry.step, 2);
        assert_eq!(entry.command, "echo test");
        assert_eq!(entry.exit_code, 0);
        assert_eq!(entry.output, "test output");
        assert_eq!(entry.timestamp, Some("2024-01-15T10:30:00Z".to_string()));
        assert!(entry.result_verification_pass);
        assert!(entry.output_verification_pass);
    }

    #[test]
    fn test_test_step_execution_entry_is_success() {
        let success_entry = TestStepExecutionEntry::new(
            1,
            1,
            "cmd".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        assert!(success_entry.is_success());
        assert!(!success_entry.is_failure());

        let failure_entry = TestStepExecutionEntry::new(
            1,
            1,
            "cmd".to_string(),
            1,
            "error".to_string(),
            false,
            false,
        );
        assert!(!failure_entry.is_success());
        assert!(failure_entry.is_failure());
    }

    #[test]
    fn test_test_step_execution_entry_validate_success() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_test_step_execution_entry_validate_negative_sequence() {
        let entry = TestStepExecutionEntry::new(
            -1,
            2,
            "echo test".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "test_sequence must be non-negative");
    }

    #[test]
    fn test_test_step_execution_entry_validate_negative_step() {
        let entry = TestStepExecutionEntry::new(
            1,
            -1,
            "echo test".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "step must be non-negative");
    }

    #[test]
    fn test_test_step_execution_entry_validate_empty_command() {
        let entry =
            TestStepExecutionEntry::new(1, 2, "".to_string(), 0, "output".to_string(), true, true);
        let result = entry.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "command must not be empty");
    }

    #[test]
    fn test_test_step_execution_entry_matches() {
        let entry = TestStepExecutionEntry::new(
            3,
            5,
            "cmd".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );

        assert!(entry.matches(3, 5));
        assert!(!entry.matches(3, 6));
        assert!(!entry.matches(4, 5));
        assert!(!entry.matches(1, 1));
    }

    #[test]
    fn test_test_step_execution_entry_result_summary() {
        let success_entry = TestStepExecutionEntry::new(
            1,
            2,
            "cmd".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        assert_eq!(
            success_entry.result_summary(),
            "Sequence 1, Step 2: SUCCESS (exit code: 0)"
        );

        let failure_entry = TestStepExecutionEntry::new(
            3,
            4,
            "cmd".to_string(),
            127,
            "error".to_string(),
            false,
            false,
        );
        assert_eq!(
            failure_entry.result_summary(),
            "Sequence 3, Step 4: FAILURE (exit code: 127)"
        );
    }

    #[test]
    fn test_test_step_execution_entry_parse_timestamp() {
        let entry_with_timestamp = TestStepExecutionEntry::with_timestamp(
            1,
            1,
            "cmd".to_string(),
            0,
            "output".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            true,
            true,
        );
        let parsed = entry_with_timestamp.parse_timestamp();
        assert!(parsed.is_some());
        let timestamp = parsed.unwrap();
        assert_eq!(timestamp.to_rfc3339(), "2024-01-15T10:30:00+00:00");

        let entry_without_timestamp = TestStepExecutionEntry::new(
            1,
            1,
            "cmd".to_string(),
            0,
            "output".to_string(),
            true,
            true,
        );
        assert!(entry_without_timestamp.parse_timestamp().is_none());

        let entry_with_invalid_timestamp = TestStepExecutionEntry::with_timestamp(
            1,
            1,
            "cmd".to_string(),
            0,
            "output".to_string(),
            "invalid timestamp".to_string(),
            true,
            true,
        );
        assert!(entry_with_invalid_timestamp.parse_timestamp().is_none());
    }

    #[test]
    fn test_test_step_execution_entry_display() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            true,
            true,
        );
        assert_eq!(
            format!("{}", entry),
            "Sequence 1, Step 2: echo test (exit code: 0)"
        );
    }

    #[test]
    fn test_test_step_execution_entry_serialization() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            true,
            true,
        );

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: TestStepExecutionEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_test_step_execution_entry_serialization_with_timestamp() {
        let entry = TestStepExecutionEntry::with_timestamp(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            "2024-01-15T10:30:00Z".to_string(),
            true,
            true,
        );

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"timestamp\":\"2024-01-15T10:30:00Z\""));

        let deserialized: TestStepExecutionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_test_step_execution_entry_skip_none_timestamp() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            true,
            true,
        );

        let json = serde_json::to_string(&entry).unwrap();
        assert!(!json.contains("timestamp"));
    }

    #[test]
    fn test_test_step_execution_entry_yaml_serialization() {
        let entry = TestStepExecutionEntry::new(
            1,
            2,
            "echo test".to_string(),
            0,
            "test output".to_string(),
            true,
            true,
        );

        let yaml = serde_yaml::to_string(&entry).unwrap();
        let deserialized: TestStepExecutionEntry = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_verification_simple_string() {
        let yaml = r#""[[ $? -eq 0 ]]""#;
        let result: VerificationExpression = serde_yaml::from_str(yaml).unwrap();

        match result {
            VerificationExpression::Simple(s) => {
                assert_eq!(s, "[[ $? -eq 0 ]]");
            }
            _ => panic!("Expected Simple variant"),
        }
    }

    #[test]
    fn test_verification_conditional_full() {
        let yaml = r#"
condition: "test -f /tmp/file"
if_true:
  - "echo true"
  - "echo success"
if_false:
  - "echo false"
  - "echo failure"
always:
  - "echo always"
  - "echo cleanup"
"#;
        let result: VerificationExpression = serde_yaml::from_str(yaml).unwrap();

        match result {
            VerificationExpression::Conditional {
                condition,
                if_true,
                if_false,
                always,
            } => {
                assert_eq!(condition, "test -f /tmp/file");
                assert_eq!(
                    if_true,
                    Some(vec!["echo true".to_string(), "echo success".to_string()])
                );
                assert_eq!(
                    if_false,
                    Some(vec!["echo false".to_string(), "echo failure".to_string()])
                );
                assert_eq!(
                    always,
                    Some(vec!["echo always".to_string(), "echo cleanup".to_string()])
                );
            }
            _ => panic!("Expected Conditional variant"),
        }
    }

    #[test]
    fn test_verification_conditional_minimal() {
        let yaml = r#"
condition: "test -d /var/log"
"#;
        let result: VerificationExpression = serde_yaml::from_str(yaml).unwrap();

        match result {
            VerificationExpression::Conditional {
                condition,
                if_true,
                if_false,
                always,
            } => {
                assert_eq!(condition, "test -d /var/log");
                assert_eq!(if_true, None);
                assert_eq!(if_false, None);
                assert_eq!(always, None);
            }
            _ => panic!("Expected Conditional variant"),
        }
    }

    #[test]
    fn test_verification_conditional_missing_optional() {
        let yaml = r#"
condition: "[[ -n $OUTPUT ]]"
if_true:
  - "echo output is not empty"
always:
  - "echo done"
"#;
        let result: VerificationExpression = serde_yaml::from_str(yaml).unwrap();

        match result {
            VerificationExpression::Conditional {
                condition,
                if_true,
                if_false,
                always,
            } => {
                assert_eq!(condition, "[[ -n $OUTPUT ]]");
                assert_eq!(if_true, Some(vec!["echo output is not empty".to_string()]));
                assert_eq!(if_false, None);
                assert_eq!(always, Some(vec!["echo done".to_string()]));
            }
            _ => panic!("Expected Conditional variant"),
        }
    }

    #[test]
    fn test_capture_var_with_capture() {
        let yaml = r#"
name: token
capture: '"token":"([^"]+)"'
"#;
        let result: CaptureVar = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.name, "token");
        assert_eq!(result.capture, Some("\"token\":\"([^\"]+)\"".to_string()));
        assert_eq!(result.command, None);
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_capture_var_with_command() {
        let yaml = r#"
name: output_len
command: "cat /tmp/hello.txt | wc -c"
"#;
        let result: CaptureVar = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.name, "output_len");
        assert_eq!(result.capture, None);
        assert_eq!(
            result.command,
            Some("cat /tmp/hello.txt | wc -c".to_string())
        );
        assert!(result.validate().is_ok());
    }

    #[test]
    fn test_capture_var_both_fields_error() {
        let capture_var = CaptureVar {
            name: "test".to_string(),
            capture: Some("pattern".to_string()),
            command: Some("command".to_string()),
        };
        let result = capture_var.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mutually exclusive"));
    }

    #[test]
    fn test_capture_var_neither_field_error() {
        let capture_var = CaptureVar {
            name: "test".to_string(),
            capture: None,
            command: None,
        };
        let result = capture_var.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("either capture or command must be specified"));
    }

    #[test]
    fn test_capture_vars_format_legacy() {
        let yaml = r#"
token: '"token":"([^"]+)"'
session_id: '"session_id":"([^"]+)"'
"#;
        let result: CaptureVarsFormat = serde_yaml::from_str(yaml).unwrap();
        match result {
            CaptureVarsFormat::Legacy(map) => {
                assert_eq!(map.len(), 2);
                assert_eq!(
                    map.get("token"),
                    Some(&"\"token\":\"([^\"]+)\"".to_string())
                );
                assert_eq!(
                    map.get("session_id"),
                    Some(&"\"session_id\":\"([^\"]+)\"".to_string())
                );
            }
            _ => panic!("Expected Legacy variant"),
        }
    }

    #[test]
    fn test_capture_vars_format_new() {
        let yaml = r#"
- name: token
  capture: '"token":"([^"]+)"'
- name: output_len
  command: "cat /tmp/hello.txt | wc -c"
"#;
        let result: CaptureVarsFormat = serde_yaml::from_str(yaml).unwrap();
        match result {
            CaptureVarsFormat::New(vec) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[0].name, "token");
                assert_eq!(vec[0].capture, Some("\"token\":\"([^\"]+)\"".to_string()));
                assert_eq!(vec[0].command, None);
                assert_eq!(vec[1].name, "output_len");
                assert_eq!(vec[1].capture, None);
                assert_eq!(
                    vec[1].command,
                    Some("cat /tmp/hello.txt | wc -c".to_string())
                );
            }
            _ => panic!("Expected New variant"),
        }
    }

    #[test]
    fn test_step_with_legacy_capture_vars() {
        let yaml = r#"
step: 1
description: "Test step"
command: "echo test"
capture_vars:
  token: '"token":"([^"]+)"'
  session_id: '"session_id":"([^"]+)"'
expected:
  result: "0"
  output: "test"
"#;
        let result: Step = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.step, 1);
        assert!(result.capture_vars.is_some());
        match result.capture_vars.unwrap() {
            CaptureVarsFormat::Legacy(map) => {
                assert_eq!(map.len(), 2);
            }
            _ => panic!("Expected Legacy variant"),
        }
    }

    #[test]
    fn test_step_with_new_capture_vars() {
        let yaml = r#"
step: 1
description: "Test step"
command: "echo test"
capture_vars:
  - name: token
    capture: '"token":"([^"]+)"'
  - name: output_len
    command: "cat /tmp/hello.txt | wc -c"
expected:
  result: "0"
  output: "test"
"#;
        let result: Step = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.step, 1);
        assert!(result.capture_vars.is_some());
        match result.capture_vars.unwrap() {
            CaptureVarsFormat::New(vec) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[0].name, "token");
                assert_eq!(vec[1].name, "output_len");
            }
            _ => panic!("Expected New variant"),
        }
    }

    #[test]
    fn test_step_without_capture_vars() {
        let yaml = r#"
step: 1
description: "Test step"
command: "echo test"
expected:
  result: "0"
  output: "test"
"#;
        let result: Step = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.step, 1);
        assert!(result.capture_vars.is_none());
    }

    #[test]
    fn test_general_verification_creation() {
        let general_verification = GeneralVerification {
            name: "check_file_exists".to_string(),
            condition: "test -f /tmp/output.txt".to_string(),
        };
        assert_eq!(general_verification.name, "check_file_exists");
        assert_eq!(general_verification.condition, "test -f /tmp/output.txt");
    }

    #[test]
    fn test_general_verification_display() {
        let general_verification = GeneralVerification {
            name: "check_file_exists".to_string(),
            condition: "test -f /tmp/output.txt".to_string(),
        };
        assert_eq!(
            format!("{}", general_verification),
            "check_file_exists: test -f /tmp/output.txt"
        );
    }

    #[test]
    fn test_general_verification_serialization() {
        let general_verification = GeneralVerification {
            name: "check_file_exists".to_string(),
            condition: "test -f /tmp/output.txt".to_string(),
        };

        let yaml = serde_yaml::to_string(&general_verification).unwrap();
        let deserialized: GeneralVerification = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(general_verification, deserialized);
    }

    #[test]
    fn test_general_verification_deserialization() {
        let yaml = r#"
name: check_directory
condition: "test -d /var/log"
"#;
        let result: GeneralVerification = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(result.name, "check_directory");
        assert_eq!(result.condition, "test -d /var/log");
    }

    #[test]
    fn test_verification_with_general_field() {
        let yaml = r#"
result: "[[ $? -eq 0 ]]"
output: "cat $COMMAND_OUTPUT | grep -q \"${OUTPUT}\""
general:
  - name: check_file_exists
    condition: "test -f /tmp/output.txt"
  - name: check_permissions
    condition: "test -r /tmp/output.txt"
"#;
        let result: Verification = serde_yaml::from_str(yaml).unwrap();

        assert!(result.general.is_some());
        let general = result.general.unwrap();
        assert_eq!(general.len(), 2);
        assert_eq!(general[0].name, "check_file_exists");
        assert_eq!(general[0].condition, "test -f /tmp/output.txt");
        assert_eq!(general[1].name, "check_permissions");
        assert_eq!(general[1].condition, "test -r /tmp/output.txt");
    }

    #[test]
    fn test_verification_without_general_field() {
        let yaml = r#"
result: "[[ $? -eq 0 ]]"
output: "cat $COMMAND_OUTPUT | grep -q \"${OUTPUT}\""
"#;
        let result: Verification = serde_yaml::from_str(yaml).unwrap();
        assert!(result.general.is_none());
    }

    #[test]
    fn test_verification_serialization_with_general() {
        let verification = Verification {
            result: VerificationExpression::Simple("[[ $? -eq 0 ]]".to_string()),
            output: VerificationExpression::Simple(
                "cat $COMMAND_OUTPUT | grep -q \"${OUTPUT}\"".to_string(),
            ),
            output_file: None,
            general: Some(vec![GeneralVerification {
                name: "check_file_exists".to_string(),
                condition: "test -f /tmp/output.txt".to_string(),
            }]),
        };

        let yaml = serde_yaml::to_string(&verification).unwrap();
        assert!(yaml.contains("general:"));
        assert!(yaml.contains("check_file_exists"));
    }

    #[test]
    fn test_test_report_output_single() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let report = TestReportOutput::single(test_run.clone());

        assert_eq!(report.test_case_count(), 1);
        assert_eq!(report.test_runs().len(), 1);
        assert!(matches!(report.results, TestReportResults::Single(_)));

        match &report.results {
            TestReportResults::Single(run) => {
                assert_eq!(run.test_case_id, "TC001");
                assert_eq!(run.status, TestRunStatus::Pass);
            }
            _ => panic!("Expected Single variant"),
        }
    }

    #[test]
    fn test_test_report_output_multiple() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run1 = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log1".to_string(),
        );
        let test_run2 = TestRun::new(
            "TC002".to_string(),
            TestRunStatus::Fail,
            timestamp,
            2.5,
            "log2".to_string(),
        );

        let report = TestReportOutput::multiple(vec![test_run1, test_run2]);

        assert_eq!(report.test_case_count(), 2);
        assert_eq!(report.test_runs().len(), 2);
        assert!(matches!(report.results, TestReportResults::Multiple(_)));
    }

    #[test]
    fn test_test_report_output_with_metadata() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let report = TestReportOutput::single(test_run)
            .with_title("Test Report".to_string())
            .with_project("MyProject".to_string())
            .with_test_date("2024-01-15".to_string())
            .with_environment("production".to_string())
            .with_platform("linux-x64".to_string())
            .with_executor("jenkins".to_string())
            .with_execution_duration(3.5);

        assert_eq!(report.title, Some("Test Report".to_string()));
        assert_eq!(report.project, Some("MyProject".to_string()));
        assert_eq!(report.test_date, Some("2024-01-15".to_string()));
        assert_eq!(report.environment, Some("production".to_string()));
        assert_eq!(report.platform, Some("linux-x64".to_string()));
        assert_eq!(report.executor, Some("jenkins".to_string()));
        assert_eq!(report.execution_duration, Some(3.5));
    }

    #[test]
    fn test_test_report_output_summary_stats() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run1 = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log1".to_string(),
        );
        let test_run2 = TestRun::new(
            "TC002".to_string(),
            TestRunStatus::Fail,
            timestamp,
            2.5,
            "log2".to_string(),
        );
        let test_run3 = TestRun::new(
            "TC003".to_string(),
            TestRunStatus::Skip,
            timestamp,
            0.0,
            "log3".to_string(),
        );

        let report = TestReportOutput::multiple(vec![test_run1, test_run2, test_run3]);
        let summary = report.summary_stats();

        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.total_duration, 4.0);
    }

    #[test]
    fn test_test_report_output_json_serialization() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let report = TestReportOutput::single(test_run)
            .with_title("Test Report".to_string())
            .with_project("MyProject".to_string());

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"title\":\"Test Report\""));
        assert!(json.contains("\"project\":\"MyProject\""));
        assert!(json.contains("\"TC001\""));

        let deserialized: TestReportOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(report, deserialized);
    }

    #[test]
    fn test_test_report_output_yaml_serialization() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let report = TestReportOutput::single(test_run)
            .with_title("Test Report".to_string())
            .with_environment("staging".to_string());

        let yaml = serde_yaml::to_string(&report).unwrap();
        assert!(yaml.contains("title: Test Report"));
        assert!(yaml.contains("environment: staging"));
        assert!(yaml.contains("TC001"));

        let deserialized: TestReportOutput = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(report, deserialized);
    }

    #[test]
    fn test_test_report_output_skip_none_fields() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let report = TestReportOutput::single(test_run);

        let json = serde_json::to_string(&report).unwrap();
        assert!(!json.contains("\"title\""));
        assert!(!json.contains("\"project\""));
        assert!(!json.contains("\"test_date\""));
        assert!(!json.contains("\"environment\""));
        assert!(!json.contains("\"platform\""));
        assert!(!json.contains("\"executor\""));
        assert!(!json.contains("\"execution_duration\""));
    }

    #[test]
    fn test_test_report_summary() {
        let summary = TestReportSummary {
            total: 10,
            passed: 7,
            failed: 2,
            skipped: 1,
            total_duration: 45.5,
        };

        assert_eq!(summary.total, 10);
        assert_eq!(summary.passed, 7);
        assert_eq!(summary.failed, 2);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.total_duration, 45.5);
    }

    #[test]
    fn test_test_report_results_untagged_single() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log".to_string(),
        );

        let json = serde_json::to_string(&TestReportResults::Single(test_run)).unwrap();
        let deserialized: TestReportResults = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, TestReportResults::Single(_)));
    }

    #[test]
    fn test_test_report_results_untagged_multiple() {
        let timestamp = "2024-01-15T10:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let test_run1 = TestRun::new(
            "TC001".to_string(),
            TestRunStatus::Pass,
            timestamp,
            1.5,
            "log1".to_string(),
        );
        let test_run2 = TestRun::new(
            "TC002".to_string(),
            TestRunStatus::Fail,
            timestamp,
            2.5,
            "log2".to_string(),
        );

        let json = serde_json::to_string(&TestReportResults::Multiple(vec![test_run1, test_run2]))
            .unwrap();
        let deserialized: TestReportResults = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, TestReportResults::Multiple(_)));
    }

    const MINIMAL_TEST_CASE: &str = "\
requirement: REQ1
item: 1
tc: 1
id: TC_CLASS_001
description: classification
general_initial_conditions: {}
initial_conditions: {}
test_sequences: []
";

    fn test_case_with(extra: &str) -> Result<TestCase, serde_yaml::Error> {
        serde_yaml::from_str(&format!("{MINIMAL_TEST_CASE}{extra}"))
    }

    #[test]
    fn test_classification_fields_absent_behave_as_before() {
        let test_case = test_case_with("").unwrap();
        assert!(test_case.requirements.is_empty());
        assert!(test_case.test_type.is_none() && test_case.test_category.is_none());
        assert!(test_case.level.is_none() && test_case.author.is_none());
        assert!(test_case.validate_classification().is_ok());

        // nothing new is written for a test case that does not use the fields
        let yaml = serde_yaml::to_string(&test_case).unwrap();
        for key in [
            "requirements",
            "test_type",
            "test_category",
            "level",
            "objective",
            "environment",
            "pass_fail_criteria",
            "author",
            "independence_required",
            "version",
            "status",
            "external_refs",
        ] {
            assert!(!yaml.contains(&format!("{key}:")), "{key} written: {yaml}");
        }
    }

    #[test]
    fn test_classification_fields_round_trip() {
        let test_case = test_case_with(
            "\
requirements: [HLR-12, LLR-7]
test_type: robustness
test_category: out_of_range
level: sw_integration
objective: Reject out-of-range metadata
environment: simulator v2
pass_fail_criteria: No crash and error 6A80
author: alvaro
independence_required: true
version: '3'
status: in_review
external_refs:
  - system: zephyr_scale
    key: PRJ-T42
    synced_at: '2026-10-28T12:00:00Z'
",
        )
        .unwrap();

        assert_eq!(test_case.requirements, vec!["HLR-12", "LLR-7"]);
        assert_eq!(test_case.test_type, Some(TestType::Robustness));
        assert_eq!(test_case.test_category, Some(TestCategory::OutOfRange));
        assert_eq!(test_case.level, Some(TestLevel::SwIntegration));
        assert_eq!(test_case.independence_required, Some(true));
        assert_eq!(test_case.status, Some(TestCaseStatus::InReview));
        assert_eq!(
            test_case.external_refs[0].system,
            ExternalSystem::ZephyrScale
        );
        assert!(test_case.validate_classification().is_ok());

        let yaml = serde_yaml::to_string(&test_case).unwrap();
        let back: TestCase = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, test_case, "lost data on save and load: {yaml}");
    }

    #[test]
    fn test_invalid_classification_values_name_the_field() {
        for (extra, field) in [
            ("test_type: smoke\n", "test_type"),
            ("test_category: sampling\n", "test_category"),
            ("level: unit\n", "level"),
            ("status: done\n", "status"),
        ] {
            let err = test_case_with(extra).unwrap_err().to_string();
            assert!(err.contains(field), "{field} not named in: {err}");
            assert!(err.contains("unknown variant"), "{err}");
        }
    }

    #[test]
    fn test_category_must_belong_to_test_type() {
        let test_case =
            test_case_with("test_type: normal_range\ntest_category: fuzzing\n").unwrap();
        let err = test_case.validate_classification().unwrap_err();
        assert!(err.contains("test_category 'fuzzing'"), "{err}");
        assert!(err.contains("test_type 'normal_range'"), "{err}");
        assert!(
            err.contains("equivalence_class"),
            "lists the allowed ones: {err}"
        );

        // a category alone, or a type alone, is not a conflict
        assert!(test_case_with("test_category: fuzzing\n")
            .unwrap()
            .validate_classification()
            .is_ok());
        assert!(test_case_with("test_type: security\n")
            .unwrap()
            .validate_classification()
            .is_ok());
    }

    #[test]
    fn test_every_category_belongs_to_exactly_the_type_that_lists_it() {
        for test_type in [
            TestType::NormalRange,
            TestType::Robustness,
            TestType::Security,
        ] {
            for category in test_type.categories() {
                assert_eq!(category.test_type(), test_type);
            }
        }
        let listed: usize = [
            TestType::NormalRange,
            TestType::Robustness,
            TestType::Security,
        ]
        .iter()
        .map(|t| t.categories().len())
        .sum();
        assert_eq!(listed, 12);
    }
}
