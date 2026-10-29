//! Milestones (TCMS-42.7): a test plan is a Jira Fix Version. The sync creates the version
//! (once), keeps its description current, assigns it to the features in scope and releases it
//! when the plan completes. A reopened plan never un-releases the version; it is reported.
//! Only `milestone.target: fix_version` is implemented; `epic` and `issue` fail clearly.

use anyhow::{anyhow, bail, Result};
use id_resolver::SystemConfig;
use jira_client::{WriteKind, FIX_VERSIONS_FIELD};
use serde_json::json;

use crate::backends::id_text;
use crate::{Api, Ctx};

/// Progress of a test plan, as the sync reads it from its runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanState {
    pub name: String,
    pub baseline_id: String,
    /// Test cases with a result on the current baseline
    pub executed: u32,
    pub total: u32,
    pub passed: u32,
    /// Every case of the plan has a finished result
    pub complete: bool,
    /// TCMS link to the plan
    pub tcms_link: String,
}

impl PlanState {
    pub fn state(&self) -> &'static str {
        if self.complete {
            "complete"
        } else if self.executed > 0 {
            "in progress"
        } else {
            "not started"
        }
    }

    /// Passed over executed, as a rounded percent (0 with nothing executed).
    pub fn pass_rate(&self) -> u32 {
        if self.executed == 0 {
            return 0;
        }
        (self.passed * 100 + self.executed / 2) / self.executed
    }

    /// The Fix Version description: plan state, progress, pass rate, baseline and the TCMS link.
    pub fn description(&self) -> String {
        format!(
            "Plan state: {}\nExecuted: {}/{}\nPass rate: {}%\nBaseline: {}\nTCMS: {}",
            self.state(),
            self.executed,
            self.total,
            self.pass_rate(),
            self.baseline_id,
            self.tcms_link
        )
    }
}

/// What one milestone sync did (or, in a dry run, would do).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MilestoneReport {
    /// Fix Version id; kept in the test plan's external_refs
    pub version_id: String,
    pub created: bool,
    pub released_now: bool,
    /// Features that got the Fix Version
    pub features_assigned: Vec<String>,
    /// Notes for the sync log (reopened plan, features skipped, ...)
    pub notes: Vec<String>,
}

pub struct MilestoneSync {
    ctx: Ctx,
}

impl MilestoneSync {
    pub fn new(system: &SystemConfig, api: Box<dyn Api>) -> Result<Self> {
        match &system.milestone {
            None => bail!("system '{}': milestone is not configured", system.name),
            Some(m) if m.target == "fix_version" => {}
            Some(m) => bail!(
                "system '{}': milestone.target '{}' is not implemented yet; only fix_version works",
                system.name,
                m.target
            ),
        }
        Ok(Self {
            ctx: Ctx {
                api,
                system: system.clone(),
                overwrites: Vec::new(),
            },
        })
    }

    /// Sync one plan. `known_version` is the Fix Version id from the plan's external_refs;
    /// `release_date` (YYYY-MM-DD) is used when the plan completes now.
    pub fn sync(
        &mut self,
        plan: &PlanState,
        known_version: Option<&str>,
        features: &[String],
        release_date: &str,
        dry_run: bool,
    ) -> Result<MilestoneReport> {
        let prefix = self.ctx.api.rest_prefix();
        let release_on_complete = self
            .ctx
            .system
            .milestone
            .as_ref()
            .and_then(|m| m.release_on_plan_complete)
            .unwrap_or(false);
        let mut report = MilestoneReport::default();

        // 1. the Fix Version: created once, its id kept
        let version_id = match known_version {
            Some(id) => id.to_string(),
            None => {
                let p = self
                    .ctx
                    .system
                    .project_keys
                    .first()
                    .cloned()
                    .unwrap_or_default();
                let pinfo = self.ctx.api.get(&format!("{prefix}/project/{p}"), &[])?;
                let pid = id_text(&pinfo["id"]).ok_or_else(|| anyhow!("project {p} has no id"))?;
                report.created = true;
                if dry_run {
                    "(new)".to_string()
                } else {
                    let v = self.ctx.write(
                        WriteKind::Milestone,
                        "",
                        &[],
                        "POST",
                        &format!("{prefix}/version"),
                        &json!({
                            "name": plan.name,
                            "projectId": pid,
                            "description": plan.description()
                        }),
                    )?;
                    id_text(&v["id"]).ok_or_else(|| anyhow!("version response has no id"))?
                }
            }
        };
        report.version_id = version_id.clone();

        // 2. description: progress after every sync
        if !report.created && !dry_run {
            self.ctx.write(
                WriteKind::Milestone,
                "",
                &[],
                "PUT",
                &format!("{prefix}/version/{version_id}"),
                &json!({ "description": plan.description() }),
            )?;
        }

        // 3. release on completion; a reopened plan is reported, never un-released
        let released = if dry_run || report.created {
            false
        } else {
            self.ctx
                .api
                .get(&format!("{prefix}/version/{version_id}"), &[])?["released"]
                .as_bool()
                .unwrap_or(false)
        };
        if plan.complete && release_on_complete && !released {
            report.released_now = true;
            if !dry_run {
                self.ctx.write(
                    WriteKind::Milestone,
                    "",
                    &[],
                    "PUT",
                    &format!("{prefix}/version/{version_id}"),
                    &json!({ "released": true, "releaseDate": release_date }),
                )?;
            }
        }
        if !plan.complete && released {
            report.notes.push(format!(
                "plan '{}' is reopened; Fix Version {version_id} stays released (never un-released automatically)",
                plan.name
            ));
        }

        // 4. the Fix Version on the features in scope, when feature_fields allows it
        if !self.ctx.system.write_policy.feature_fields {
            if !features.is_empty() {
                report
                    .notes
                    .push("features not assigned: write_policy.feature_fields is off".into());
            }
        } else {
            for f in features {
                if !dry_run {
                    self.ctx.write(
                        WriteKind::FeatureFields,
                        f,
                        &[FIX_VERSIONS_FIELD],
                        "PUT",
                        &format!("{prefix}/issue/{f}"),
                        &json!({ "fields": { FIX_VERSIONS_FIELD: [{ "id": version_id }] } }),
                    )?;
                }
                report.features_assigned.push(f.clone());
            }
        }
        Ok(report)
    }
}
