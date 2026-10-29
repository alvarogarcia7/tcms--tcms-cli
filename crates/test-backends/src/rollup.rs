//! Per-feature roll-up (TCMS-42.6): verification status, validation status and coverage of a
//! feature, computed from the requirements that trace to it, the latest verdict of each test
//! case that traces to them on the current baseline, and the open problem reports. The rules
//! are those of the mapping doc (jira-integration.md, section 2).

use anyhow::Result;
use id_resolver::SystemConfig;
use jira_client::WriteKind;
use serde_json::{json, Map, Value};

use crate::{status_text, Api, Ctx, StatusValue};

/// `requirement.validation_status`, with "not set" for a requirement that has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationState {
    NotSet,
    NotValidated,
    Validated,
    Rejected,
}

/// One requirement that traces to the feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReqState {
    /// Normal-range and robustness tests both cover it (`coverage-report.requirements_based`)
    pub covered: bool,
    pub validation: ValidationState,
}

/// Everything the roll-up reads for one feature.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RollupInput {
    pub requirements: Vec<ReqState>,
    /// Latest verdict of each test case that traces to the requirements, on the current
    /// baseline: [`StatusValue::NotStarted`] when it has no result there.
    pub test_statuses: Vec<StatusValue>,
    /// An open problem report links to the feature
    pub open_problem_reports: bool,
}

/// The feature's computed values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rollup {
    pub verification: StatusValue,
    pub validation: StatusValue,
    /// Percent of the feature's requirements that are covered, rounded half up
    pub coverage_percent: u32,
}

/// Verification status (mapping doc, section 2):
/// not_started when nothing traces or nothing has a result; failed when any latest verdict failed
/// (or errored); blocked when no failure and some case is not executed on a finished run;
/// passed when every requirement is covered, every latest verdict passed and no problem report is
/// open; in_progress otherwise.
pub fn verification(input: &RollupInput) -> StatusValue {
    let any = |v: StatusValue| input.test_statuses.contains(&v);
    if input.requirements.is_empty()
        || input
            .test_statuses
            .iter()
            .all(|s| *s == StatusValue::NotStarted)
    {
        return StatusValue::NotStarted;
    }
    if any(StatusValue::Failed) {
        return StatusValue::Failed;
    }
    if any(StatusValue::Blocked) {
        return StatusValue::Blocked;
    }
    let all_covered = input.requirements.iter().all(|r| r.covered);
    let all_passed = input
        .test_statuses
        .iter()
        .all(|s| *s == StatusValue::Passed);
    if all_covered && all_passed && !input.open_problem_reports {
        StatusValue::Passed
    } else {
        StatusValue::InProgress
    }
}

/// Validation status (mapping doc, section 2): failed when any requirement is rejected; passed
/// when all are validated; not_started when none has a validation status; in_progress otherwise.
pub fn validation(input: &RollupInput) -> StatusValue {
    let v = &input.requirements;
    if v.iter().any(|r| r.validation == ValidationState::Rejected) {
        StatusValue::Failed
    } else if !v.is_empty() && v.iter().all(|r| r.validation == ValidationState::Validated) {
        StatusValue::Passed
    } else if v.iter().all(|r| r.validation == ValidationState::NotSet) {
        StatusValue::NotStarted
    } else {
        StatusValue::InProgress
    }
}

/// Coverage: covered requirements over all requirements, as a rounded percent; `None` without
/// requirements.
pub fn coverage_percent(input: &RollupInput) -> Option<u32> {
    let total = input.requirements.len() as u32;
    if total == 0 {
        return None;
    }
    let covered = input.requirements.iter().filter(|r| r.covered).count() as u32;
    Some((covered * 100 + total / 2) / total)
}

pub fn rollup(input: &RollupInput) -> Option<Rollup> {
    Some(Rollup {
        verification: verification(input),
        validation: validation(input),
        coverage_percent: coverage_percent(input)?,
    })
}

/// One step of a feature write, reported in the dry run and in the sync log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannedWrite {
    Field {
        feature: String,
        field: String,
        value: Value,
    },
    Transition {
        feature: String,
        name: String,
    },
    /// Logged and skipped (AC #3): the workflow lacks the transition, or the policy is off
    Skipped {
        feature: String,
        reason: String,
    },
    /// Not written (AC #4): the feature has no traced requirement
    Unreported {
        feature: String,
        reason: String,
    },
}

/// Writes the roll-up of features to Jira. Every write goes through [`Ctx::write`].
pub struct FeatureWriter {
    ctx: Ctx,
}

impl FeatureWriter {
    pub fn new(system: &SystemConfig, api: Box<dyn Api>) -> Self {
        Self {
            ctx: Ctx {
                api,
                system: system.clone(),
                overwrites: Vec::new(),
            },
        }
    }

    /// Roll up one feature and write it. `dry_run` sends nothing (reads still run) and returns
    /// what would have been written.
    pub fn write(
        &mut self,
        feature: &str,
        input: &RollupInput,
        dry_run: bool,
    ) -> Result<Vec<PlannedWrite>> {
        let Some(r) = rollup(input) else {
            let reason = "no requirement traces to this feature".to_string();
            log::warn!("feature {feature}: {reason}; not written");
            return Ok(vec![PlannedWrite::Unreported {
                feature: feature.into(),
                reason,
            }]);
        };
        let system = self.ctx.system.clone();
        let mut planned = Vec::new();
        let mut fields = Map::new();
        let mut put = |field: &Option<String>, value: Value, planned: &mut Vec<PlannedWrite>| {
            if let Some(id) = field {
                fields.insert(id.clone(), value.clone());
                planned.push(PlannedWrite::Field {
                    feature: feature.into(),
                    field: id.clone(),
                    value,
                });
            }
        };
        put(
            &system.fields.verification_status,
            json!(status_text(&system, r.verification)),
            &mut planned,
        );
        put(
            &system.fields.validation_status,
            json!(status_text(&system, r.validation)),
            &mut planned,
        );
        put(
            &system.fields.coverage,
            json!(r.coverage_percent),
            &mut planned,
        );

        if !fields.is_empty() && !dry_run {
            let ids: Vec<&str> = fields.keys().map(String::as_str).collect();
            let prefix = self.ctx.api.rest_prefix();
            self.ctx.write(
                WriteKind::FeatureFields,
                feature,
                &ids,
                "PUT",
                &format!("{prefix}/issue/{feature}"),
                &json!({ "fields": fields }),
            )?;
        }

        // workflow transition for the verification status, when configured
        if let Some(name) = system.transitions.get(r.verification.as_str()).cloned() {
            if !system.write_policy.feature_transitions {
                planned.push(PlannedWrite::Skipped {
                    feature: feature.into(),
                    reason: "feature_transitions is off".into(),
                });
            } else {
                let prefix = self.ctx.api.rest_prefix();
                let avail = self
                    .ctx
                    .api
                    .get(&format!("{prefix}/issue/{feature}/transitions"), &[])?;
                let id = avail["transitions"]
                    .as_array()
                    .and_then(|a| a.iter().find(|t| t["name"].as_str() == Some(name.as_str())))
                    .and_then(|t| t["id"].as_str().map(str::to_string));
                match id {
                    None => {
                        let reason =
                            format!("transition '{name}' is not in the workflow of {feature}");
                        log::warn!("{reason}; skipped");
                        planned.push(PlannedWrite::Skipped {
                            feature: feature.into(),
                            reason,
                        });
                    }
                    Some(id) => {
                        planned.push(PlannedWrite::Transition {
                            feature: feature.into(),
                            name: name.clone(),
                        });
                        if !dry_run {
                            self.ctx.write(
                                WriteKind::FeatureTransition { name: name.clone() },
                                feature,
                                &[],
                                "POST",
                                &format!("{prefix}/issue/{feature}/transitions"),
                                &json!({ "transition": { "id": id } }),
                            )?;
                        }
                    }
                }
            }
        }
        Ok(planned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(covered: bool, validation: ValidationState) -> ReqState {
        ReqState {
            covered,
            validation,
        }
    }

    fn input(reqs: Vec<ReqState>, tests: Vec<StatusValue>, pr: bool) -> RollupInput {
        RollupInput {
            requirements: reqs,
            test_statuses: tests,
            open_problem_reports: pr,
        }
    }

    use StatusValue::*;

    #[test]
    fn not_started_without_traced_requirement_or_without_results() {
        assert_eq!(
            verification(&input(vec![], vec![Passed], false)),
            NotStarted
        );
        assert_eq!(
            verification(&input(
                vec![req(true, ValidationState::NotSet)],
                vec![],
                false
            )),
            NotStarted
        );
        assert_eq!(
            verification(&input(
                vec![req(false, ValidationState::NotSet)],
                vec![NotStarted, NotStarted],
                false
            )),
            NotStarted
        );
    }

    #[test]
    fn failed_when_any_latest_verdict_failed_or_errored() {
        let r = vec![req(true, ValidationState::NotSet)];
        assert_eq!(
            verification(&input(r.clone(), vec![Passed, Failed], false)),
            Failed
        );
        // an error is recorded as failed (verdict mapping)
        assert_eq!(
            verification(&input(r.clone(), vec![Failed, Blocked], false)),
            Failed
        );
        // a failure wins even when a problem report is open
        assert_eq!(verification(&input(r, vec![Failed], true)), Failed);
    }

    #[test]
    fn blocked_when_no_failure_but_a_case_is_not_executed_on_a_finished_run() {
        let r = vec![req(true, ValidationState::NotSet)];
        assert_eq!(
            verification(&input(r, vec![Passed, Blocked], false)),
            Blocked
        );
    }

    #[test]
    fn passed_only_when_all_covered_all_pass_and_no_open_problem_report() {
        let r = vec![
            req(true, ValidationState::NotSet),
            req(true, ValidationState::NotSet),
        ];
        assert_eq!(
            verification(&input(r.clone(), vec![Passed, Passed], false)),
            Passed
        );
        // an open problem report keeps it from passed
        assert_eq!(
            verification(&input(r.clone(), vec![Passed, Passed], true)),
            InProgress
        );
        // a requirement not covered (missing robustness test) keeps it from passed
        let partial = vec![
            req(true, ValidationState::NotSet),
            req(false, ValidationState::NotSet),
        ];
        assert_eq!(
            verification(&input(partial, vec![Passed, Passed], false)),
            InProgress
        );
    }

    #[test]
    fn in_progress_otherwise() {
        let r = vec![req(true, ValidationState::NotSet)];
        // some case has no result on the current baseline
        assert_eq!(
            verification(&input(r, vec![Passed, NotStarted], false)),
            InProgress
        );
        // a case still running
        assert_eq!(
            verification(&input(
                vec![req(true, ValidationState::NotSet)],
                vec![InProgress],
                false
            )),
            InProgress
        );
    }

    #[test]
    fn validation_follows_requirement_validation_status() {
        use ValidationState::*;
        assert_eq!(
            validation(&input(
                vec![req(true, Rejected), req(true, Validated)],
                vec![],
                false
            )),
            Failed
        );
        assert_eq!(
            validation(&input(
                vec![req(true, Validated), req(true, Validated)],
                vec![],
                false
            )),
            Passed
        );
        assert_eq!(
            validation(&input(
                vec![req(true, NotSet), req(true, NotSet)],
                vec![],
                false
            )),
            NotStarted
        );
        assert_eq!(
            validation(&input(
                vec![req(true, Validated), req(true, NotValidated)],
                vec![],
                false
            )),
            InProgress
        );
        assert_eq!(
            validation(&input(
                vec![req(true, Validated), req(true, NotSet)],
                vec![],
                false
            )),
            InProgress
        );
        assert_eq!(validation(&input(vec![], vec![], false)), NotStarted);
    }

    #[test]
    fn coverage_is_the_rounded_percent_of_covered_requirements() {
        use ValidationState::NotSet;
        let r = |c: &[bool]| input(c.iter().map(|&b| req(b, NotSet)).collect(), vec![], false);
        assert_eq!(coverage_percent(&r(&[])), None);
        assert_eq!(coverage_percent(&r(&[true, false, false])), Some(33));
        assert_eq!(coverage_percent(&r(&[true, true, false])), Some(67));
        assert_eq!(coverage_percent(&r(&[true, true, true, true])), Some(100));
        assert_eq!(coverage_percent(&r(&[false, false])), Some(0));
    }
}
