//! Regression scope from a baseline's change impact (TCMS-40.7).
//!
//! A tcms-schemas `baseline` may carry a `change_impact` (measured against the baseline's
//! `previous_baseline_id`). The regression scope is the test cases the impact analysis lists in
//! `regression_test_cases`, plus every test case that traces (through its `requirements`) to a
//! requirement in `impacted_requirements`. A `test-plan` with a `regression` block names the
//! baseline (`baseline_id`) and the baseline the changes are measured from
//! (`regression.previous_baseline_id`); the two must agree with the baseline file.

use anyhow::{bail, Context};
use serde::Serialize;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use testcase_models::{TestCase, TestCaseStatus};

/// The change impact analysis of a baseline
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeImpact {
    pub changed_items: Vec<String>,
    pub impacted_requirements: Vec<String>,
    pub regression_test_cases: Vec<String>,
    pub rationale: Option<String>,
}

/// The parts of a baseline regression selection needs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baseline {
    pub id: String,
    pub previous_baseline_id: Option<String>,
    pub change_impact: Option<ChangeImpact>,
}

/// The parts of a test plan regression selection needs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegressionPlan {
    pub id: Option<String>,
    pub baseline_id: String,
    pub previous_baseline_id: String,
}

fn string_list(value: &Value, key: &str, origin: &str) -> anyhow::Result<Vec<String>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Sequence(items)) => items
            .iter()
            .map(|item| {
                item.as_str().map(str::to_string).with_context(|| {
                    format!("{origin}: '{key}' must be a list of strings, found {item:?}")
                })
            })
            .collect(),
        Some(_) => bail!("{origin}: '{key}' must be a list of strings"),
    }
}

fn required_string(value: &Value, key: &str, origin: &str) -> anyhow::Result<String> {
    match value.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(_) => bail!("{origin}: '{key}' must be a non-empty string"),
        None => bail!("{origin}: required property '{key}' is missing"),
    }
}

fn parse_yaml(path: &Path, what: &str) -> anyhow::Result<Value> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read {what} {}", path.display()))?;
    let value: Value = serde_yaml::from_str(&content)
        .with_context(|| format!("{what} {} is not valid YAML", path.display()))?;
    if !value.is_mapping() {
        bail!("{what} {} is not a YAML mapping", path.display());
    }
    Ok(value)
}

/// Read and check a baseline file against the rules of the tcms-schemas `baseline` schema that
/// matter here: `id`, `build_id`, `created` and `items` are required, `change_impact` needs
/// `previous_baseline_id`, and the impact lists hold strings.
pub fn read_baseline(path: &Path) -> anyhow::Result<Baseline> {
    let origin = format!("baseline {}", path.display());
    let value = parse_yaml(path, "baseline")?;
    let id = required_string(&value, "id", &origin)?;
    required_string(&value, "build_id", &origin)?;
    required_string(&value, "created", &origin)?;
    if !matches!(value.get("items"), Some(Value::Sequence(_))) {
        bail!("{origin}: required property 'items' is missing or not a list");
    }
    let previous_baseline_id = match value.get("previous_baseline_id") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => bail!("{origin}: 'previous_baseline_id' must be a string"),
    };
    let change_impact = match value.get("change_impact") {
        None | Some(Value::Null) => None,
        Some(impact @ Value::Mapping(_)) => {
            if previous_baseline_id.is_none() {
                bail!("{origin}: 'change_impact' requires 'previous_baseline_id'");
            }
            Some(ChangeImpact {
                changed_items: string_list(impact, "changed_items", &origin)?,
                impacted_requirements: string_list(impact, "impacted_requirements", &origin)?,
                regression_test_cases: string_list(impact, "regression_test_cases", &origin)?,
                rationale: impact
                    .get("rationale")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            })
        }
        Some(_) => bail!("{origin}: 'change_impact' must be a mapping"),
    };
    Ok(Baseline {
        id,
        previous_baseline_id,
        change_impact,
    })
}

/// Read baselines from files and directories (a directory contributes its `.yml`/`.yaml`
/// files, not recursively). Every file named on the command line must be a valid baseline;
/// files found in a directory that are not baselines fail too, since a silently skipped
/// baseline would change the scope.
pub fn read_baselines(paths: &[PathBuf]) -> anyhow::Result<Vec<(PathBuf, Baseline)>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut inner: Vec<PathBuf> = std::fs::read_dir(path)
                .with_context(|| format!("Cannot read baseline directory {}", path.display()))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    matches!(
                        p.extension().and_then(|e| e.to_str()),
                        Some("yml") | Some("yaml")
                    )
                })
                .collect();
            inner.sort();
            files.extend(inner);
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            bail!("Baseline not found: {}", path.display());
        }
    }
    files
        .into_iter()
        .map(|file| read_baseline(&file).map(|b| (file, b)))
        .collect()
}

/// Read the regression block of a test plan. A plan without one is not a regression plan.
pub fn read_regression_plan(path: &Path) -> anyhow::Result<RegressionPlan> {
    let origin = format!("test plan {}", path.display());
    let value = parse_yaml(path, "test plan")?;
    let baseline_id = required_string(&value, "baseline_id", &origin)?;
    let regression = match value.get("regression") {
        Some(r @ Value::Mapping(_)) => r,
        _ => bail!("{origin} has no 'regression' block, so it is not a regression plan"),
    };
    let previous_baseline_id = required_string(regression, "previous_baseline_id", &origin)
        .map_err(|e| e.context("invalid 'regression' block"))?;
    Ok(RegressionPlan {
        id: value.get("id").and_then(|v| v.as_str()).map(str::to_string),
        baseline_id,
        previous_baseline_id,
    })
}

/// Pick the baseline the scope comes from.
///
/// With a plan: the baseline whose `id` is the plan's `baseline_id`, which must follow the
/// plan's `regression.previous_baseline_id`. Without a plan exactly one baseline must be given.
pub fn resolve_baseline<'a>(
    baselines: &'a [(PathBuf, Baseline)],
    plan: Option<&RegressionPlan>,
) -> anyhow::Result<&'a Baseline> {
    if baselines.is_empty() {
        bail!("No baseline given: pass --baseline <baseline.yaml>");
    }
    let baseline = match plan {
        Some(plan) => {
            let found = baselines
                .iter()
                .map(|(_, b)| b)
                .find(|b| b.id == plan.baseline_id)
                .with_context(|| {
                    format!(
                        "The plan names baseline '{}' but no given baseline has that id (given: {})",
                        plan.baseline_id,
                        baselines
                            .iter()
                            .map(|(_, b)| b.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            if found.previous_baseline_id.as_deref() != Some(plan.previous_baseline_id.as_str()) {
                bail!(
                    "The plan measures baseline '{}' from '{}' but the baseline's \
                     previous_baseline_id is {}",
                    found.id,
                    plan.previous_baseline_id,
                    found
                        .previous_baseline_id
                        .as_deref()
                        .map(|p| format!("'{p}'"))
                        .unwrap_or_else(|| "not set".to_string())
                );
            }
            found
        }
        None => {
            if baselines.len() > 1 {
                bail!(
                    "{} baselines given and no --plan to choose between them",
                    baselines.len()
                );
            }
            &baselines[0].1
        }
    };
    if baseline.change_impact.is_none() {
        bail!(
            "Baseline '{}' has no change_impact, so no regression scope can be derived",
            baseline.id
        );
    }
    Ok(baseline)
}

/// A selected test case and why it is in scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectedTestCase {
    pub test_case_id: String,
    /// `listed` (named in regression_test_cases) and/or the impacted requirement IDs it traces to
    pub reasons: Vec<String>,
}

/// The regression scope
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegressionSelection {
    pub baseline_id: String,
    pub previous_baseline_id: String,
    pub selected: Vec<SelectedTestCase>,
    /// Listed test case IDs that no (non-obsolete) test case has
    pub unknown_listed: Vec<String>,
    /// Impacted requirements no test case traces to
    pub untested_requirements: Vec<String>,
}

impl RegressionSelection {
    pub fn test_case_ids(&self) -> Vec<&str> {
        self.selected
            .iter()
            .map(|s| s.test_case_id.as_str())
            .collect()
    }
}

/// Select the regression test cases of `baseline` (which must have a change impact) from
/// `test_cases`. Obsolete test cases are never selected. The result is sorted by ID.
pub fn select_regression(baseline: &Baseline, test_cases: &[TestCase]) -> RegressionSelection {
    let impact = baseline.change_impact.clone().unwrap_or_default();
    let live: Vec<&TestCase> = test_cases
        .iter()
        .filter(|tc| tc.status != Some(TestCaseStatus::Obsolete))
        .collect();

    let mut reasons: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unknown_listed = Vec::new();
    for listed in &impact.regression_test_cases {
        if live.iter().any(|tc| &tc.id == listed) {
            let entry = reasons.entry(listed.clone()).or_default();
            if !entry.iter().any(|r| r == "listed") {
                entry.push("listed".to_string());
            }
        } else if !unknown_listed.contains(listed) {
            unknown_listed.push(listed.clone());
        }
    }

    let mut untested_requirements = Vec::new();
    let mut seen = BTreeSet::new();
    for requirement in &impact.impacted_requirements {
        if !seen.insert(requirement.clone()) {
            continue;
        }
        let mut traced = false;
        for tc in &live {
            if tc.requirements.iter().any(|r| r == requirement) {
                traced = true;
                let entry = reasons.entry(tc.id.clone()).or_default();
                entry.push(requirement.clone());
            }
        }
        if !traced {
            untested_requirements.push(requirement.clone());
        }
    }

    RegressionSelection {
        baseline_id: baseline.id.clone(),
        previous_baseline_id: baseline.previous_baseline_id.clone().unwrap_or_default(),
        selected: reasons
            .into_iter()
            .map(|(test_case_id, reasons)| SelectedTestCase {
                test_case_id,
                reasons,
            })
            .collect(),
        unknown_listed,
        untested_requirements,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(id: &str, requirements: &[&str]) -> TestCase {
        let mut tc = TestCase::new("L".into(), 1, 1, id.into(), "d".into());
        tc.requirements = requirements.iter().map(|s| s.to_string()).collect();
        tc
    }

    fn baseline(listed: &[&str], requirements: &[&str]) -> Baseline {
        Baseline {
            id: "BL-1".into(),
            previous_baseline_id: Some("BL-0".into()),
            change_impact: Some(ChangeImpact {
                regression_test_cases: listed.iter().map(|s| s.to_string()).collect(),
                impacted_requirements: requirements.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
        }
    }

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    const BASELINE: &str =
        "id: BL-1\nbuild_id: b1\ncreated: '2026-10-27T08:00:00Z'\nitems: [a@1]\n\
        previous_baseline_id: BL-0\nchange_impact:\n  impacted_requirements: [HLR-1]\n  \
        regression_test_cases: [T9]\n";

    #[test]
    fn selects_listed_cases_and_cases_tracing_to_impacted_requirements_only() {
        let cases = [
            case("T1", &["HLR-1"]),
            case("T2", &["HLR-2"]),
            case("T9", &[]),
        ];
        let selection = select_regression(&baseline(&["T9"], &["HLR-1"]), &cases);
        assert_eq!(selection.test_case_ids(), ["T1", "T9"]);
        assert_eq!(selection.selected[0].reasons, ["HLR-1"]);
        assert_eq!(selection.selected[1].reasons, ["listed"]);
    }

    #[test]
    fn a_case_both_listed_and_tracing_has_both_reasons_once() {
        let cases = [case("T1", &["HLR-1"])];
        let selection = select_regression(&baseline(&["T1", "T1"], &["HLR-1"]), &cases);
        assert_eq!(selection.selected.len(), 1);
        assert_eq!(selection.selected[0].reasons, ["listed", "HLR-1"]);
    }

    #[test]
    fn unknown_listed_ids_and_untested_requirements_are_reported_and_obsolete_cases_skipped() {
        let mut obsolete = case("T3", &["HLR-1"]);
        obsolete.status = Some(TestCaseStatus::Obsolete);
        let cases = [obsolete, case("T1", &["HLR-5"])];
        let selection = select_regression(&baseline(&["NOPE", "T3"], &["HLR-1"]), &cases);
        assert!(selection.selected.is_empty());
        assert_eq!(selection.unknown_listed, ["NOPE", "T3"]);
        assert_eq!(selection.untested_requirements, ["HLR-1"]);
    }

    #[test]
    fn reads_the_vendored_sample_baseline() {
        let sample = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/tcms-schemas/samples/baseline/bl_001.yml");
        let b = read_baseline(&sample).unwrap();
        assert_eq!(b.id, "BL-001");
        assert_eq!(b.previous_baseline_id.as_deref(), Some("BL-000"));
        assert_eq!(b.change_impact.unwrap().impacted_requirements, ["HLR-12"]);
    }

    #[test]
    fn invalid_baselines_give_clear_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = read_baseline(&dir.path().join("none.yaml")).unwrap_err();
        assert!(format!("{missing:#}").contains("Cannot read baseline"));

        let garbage = write(dir.path(), "g.yaml", "id: [unclosed\n");
        assert!(format!("{:#}", read_baseline(&garbage).unwrap_err()).contains("not valid YAML"));

        let no_build = write(dir.path(), "n.yaml", "id: B\ncreated: x\nitems: []\n");
        assert!(format!("{:#}", read_baseline(&no_build).unwrap_err())
            .contains("required property 'build_id' is missing"));

        let no_prev = write(
            dir.path(),
            "p.yaml",
            "id: B\nbuild_id: b\ncreated: x\nitems: []\nchange_impact: {}\n",
        );
        assert!(format!("{:#}", read_baseline(&no_prev).unwrap_err())
            .contains("requires 'previous_baseline_id'"));

        let bad_list = write(
            dir.path(),
            "l.yaml",
            "id: B\nbuild_id: b\ncreated: x\nitems: []\nprevious_baseline_id: A\n\
             change_impact:\n  impacted_requirements: HLR-1\n",
        );
        assert!(format!("{:#}", read_baseline(&bad_list).unwrap_err())
            .contains("'impacted_requirements' must be a list of strings"));
    }

    #[test]
    fn a_baseline_without_change_impact_cannot_give_a_scope() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(
            dir.path(),
            "b.yaml",
            "id: B\nbuild_id: b\ncreated: x\nitems: []\n",
        );
        let baselines = read_baselines(&[file]).unwrap();
        let err = resolve_baseline(&baselines, None).unwrap_err();
        assert!(err.to_string().contains("has no change_impact"));
    }

    #[test]
    fn the_plan_picks_the_baseline_and_must_agree_on_the_previous_one() {
        let dir = tempfile::tempdir().unwrap();
        let other = BASELINE.replace("BL-1", "BL-2").replace("BL-0", "BL-1");
        let baselines = read_baselines(&[
            write(dir.path(), "a.yaml", BASELINE),
            write(dir.path(), "b.yaml", &other),
        ])
        .unwrap();

        let plan = |baseline: &str, previous: &str| {
            let path = write(
                dir.path(),
                "plan.yaml",
                &format!(
                    "id: TP-1\nbaseline_id: {baseline}\nregression:\n  previous_baseline_id: {previous}\n"
                ),
            );
            read_regression_plan(&path).unwrap()
        };

        let chosen = resolve_baseline(&baselines, Some(&plan("BL-2", "BL-1"))).unwrap();
        assert_eq!(chosen.id, "BL-2");

        let err = resolve_baseline(&baselines, Some(&plan("BL-2", "BL-0"))).unwrap_err();
        assert!(err
            .to_string()
            .contains("measures baseline 'BL-2' from 'BL-0'"));

        let err = resolve_baseline(&baselines, Some(&plan("BL-7", "BL-1"))).unwrap_err();
        assert!(err.to_string().contains("no given baseline has that id"));

        let err = resolve_baseline(&baselines, None).unwrap_err();
        assert!(err.to_string().contains("no --plan"));
    }

    #[test]
    fn a_plan_without_regression_block_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write(dir.path(), "p.yaml", "id: TP\nbaseline_id: BL-1\n");
        let err = read_regression_plan(&plan).unwrap_err();
        assert!(err.to_string().contains("not a regression plan"));
        let plan = write(
            dir.path(),
            "q.yaml",
            "id: TP\nbaseline_id: BL-1\nregression: {}\n",
        );
        assert!(format!("{:#}", read_regression_plan(&plan).unwrap_err())
            .contains("previous_baseline_id"));
    }
}
