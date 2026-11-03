//! The metrics JSON written by `tcms vv metrics` (camelCase, as documented in docs/VV_METRICS.md
//! "Output"). Fields are read as written; nothing is computed here.
//!
//! A nullable field must be present, with a value or `null`. An absent key is a missing field.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::InputError;

/// The whole document.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub generated_at: String,
    pub parameters: Parameters,
    pub run: Run,
    pub gate: Gate,
    pub vv_run: VvRun,
    pub requirements: Requirements,
    pub stability: Stability,
    pub load_summary: LoadSummary,
}

/// Where each parameter came from: `file`, `flag` or `default`.
#[derive(Debug, Deserialize)]
pub struct Parameters {
    #[serde(rename = "gateThresholdPercent")]
    pub gate_threshold_percent: Param,
    #[serde(rename = "flakyThreshold")]
    pub flaky_threshold: Param,
}

#[derive(Debug, Deserialize)]
pub struct Param {
    pub value: u64,
    pub source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub run_id: String,
    pub started_at: String,
    #[serde(deserialize_with = "nullable")]
    pub finished_at: Option<String>,
    pub provisional: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gate {
    pub gate_threshold_percent: u64,
    pub denominator: u64,
    pub counts: Counts,
    #[serde(deserialize_with = "nullable")]
    pub pass_rate: Option<Percent>,
    pub verdict: String,
    pub provisional: bool,
    #[serde(deserialize_with = "nullable")]
    pub delta_from_previous: Option<i64>,
    #[serde(deserialize_with = "nullable")]
    pub previous_run_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Counts {
    pub passed: u64,
    pub failed: u64,
    pub blocked: u64,
    pub skipped: u64,
    pub untested: u64,
}

/// A percentage: `display` is the rounded figure, `exact` the unrounded one.
#[derive(Debug, Deserialize)]
pub struct Percent {
    pub display: u64,
    pub exact: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VvRun {
    pub total_test_cases: u64,
    pub passed_test_cases: u64,
    #[serde(deserialize_with = "nullable")]
    pub vv_pass_rate: Option<Percent>,
    pub total_steps: u64,
    pub passed_steps: u64,
    #[serde(deserialize_with = "nullable")]
    pub step_coverage: Option<Percent>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    pub summary: RequirementSummary,
    pub items: Vec<RequirementItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementSummary {
    pub total: u64,
    pub covered: u64,
    pub failing: u64,
    pub not_run: u64,
    pub no_cases: u64,
    pub with_defect: u64,
    pub open_defects: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementItem {
    pub id: String,
    pub coverage: String,
    pub executed: u64,
    pub total: u64,
    pub cases: Vec<CaseStatus>,
    pub problem_reports: Vec<String>,
    pub open_problem_reports: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseStatus {
    pub case_id: String,
    #[serde(deserialize_with = "nullable")]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stability {
    pub flaky_threshold: u64,
    pub flaky_count: u64,
    pub cases: Vec<CaseStability>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseStability {
    pub case_id: String,
    pub stability: String,
    pub flips: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadSummary {
    pub distinct_cases: u64,
    pub schema_error_count: u64,
    pub schema_errors: Vec<SchemaError>,
    pub runs_recorded: u64,
}

#[derive(Debug, Deserialize)]
pub struct SchemaError {
    pub file: String,
    pub error: String,
}

/// Accepts `null` or a value; the field itself must be present (a missing key is an error).
fn nullable<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = Value::deserialize(de)?;
    if value.is_null() {
        Ok(None)
    } else {
        serde_json::from_value(value)
            .map(Some)
            .map_err(D::Error::custom)
    }
}

/// Parses the metrics JSON. A missing or mistyped field is an error whose message names it.
pub fn parse(text: &str) -> Result<Metrics, InputError> {
    serde_json::from_str(text).map_err(|e| InputError::new(format!("invalid metrics JSON: {e}")))
}
