//! Write guard: decides, from the integration config alone, whether a write is allowed.
//! It runs before any HTTP request is built, so a refused write never reaches the network.

use anyhow::{bail, Result};
use id_resolver::SystemConfig;

/// Fields that identify or describe a feature. TCMS never writes them on a feature issue.
pub const PROTECTED_FEATURE_FIELDS: &[&str] = &["summary", "description"];

/// System fields allowed when creating a TCMS-owned issue or a defect.
pub const CREATE_SYSTEM_FIELDS: &[&str] = &[
    "project",
    "issuetype",
    "summary",
    "description",
    "priority",
    "labels",
];

/// System field the milestone sets on a feature (the Fix Version of the test plan).
pub const FIX_VERSIONS_FIELD: &str = "fixVersions";

/// True when test plans are milestones carried by Jira Fix Versions.
pub fn fix_version_target(system: &SystemConfig) -> bool {
    system
        .milestone
        .as_ref()
        .is_some_and(|m| m.target == "fix_version")
}

/// What is being written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteKind {
    /// Custom fields on a feature issue (a requirement from Jira)
    FeatureFields,
    /// Workflow transition on a feature issue, by transition name
    FeatureTransition { name: String },
    /// Create or update a TCMS-owned issue (test, run, fix version holder)
    DedicatedIssue,
    /// Comment on an issue
    Comment,
    /// Create a defect issue
    Defect,
    /// Create or update a Fix Version (the test plan as milestone); needs milestone.target
    /// fix_version
    Milestone,
}

#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub kind: WriteKind,
    /// Issue key (empty for a create)
    pub issue_key: String,
    /// Field ids that would be written
    pub field_ids: Vec<String>,
}

/// Refuse a write the config does not allow. `Ok` means: allowed.
pub fn check_write(system: &SystemConfig, req: &WriteRequest) -> Result<()> {
    let p = &system.write_policy;
    let what = req.issue_key.clone();
    let who = if what.is_empty() {
        String::new()
    } else {
        format!("issue {what}: ")
    };

    // Project scope: never touch an issue outside project_keys
    if !req.issue_key.is_empty() && !system.project_keys.is_empty() {
        let project = req.issue_key.rsplit_once('-').map(|(p, _)| p).unwrap_or("");
        if !system.project_keys.iter().any(|k| k == project) {
            bail!(
                "{who}write refused: project '{project}' is not in project_keys {:?}",
                system.project_keys
            );
        }
    }

    match &req.kind {
        WriteKind::Milestone => {
            if !fix_version_target(system) {
                bail!(
                    "{who}write refused: milestones need milestone.target fix_version in system '{}'",
                    system.name
                );
            }
        }
        WriteKind::FeatureFields => {
            if !p.feature_fields {
                bail!("{who}write refused by write_policy: feature_fields is off");
            }
            for f in &req.field_ids {
                if PROTECTED_FEATURE_FIELDS.contains(&f.as_str()) {
                    bail!("{who}write refused: '{f}' of a feature is never written by TCMS");
                }
                // the Fix Version of a feature is written only for the fix_version milestone
                if f == FIX_VERSIONS_FIELD && fix_version_target(system) {
                    continue;
                }
                if !system.fields.configured_ids().contains(&f.as_str()) {
                    bail!(
                        "{who}write refused: field '{f}' is not listed in fields of system '{}'",
                        system.name
                    );
                }
            }
        }
        WriteKind::FeatureTransition { name } => {
            if !p.feature_transitions {
                bail!("{who}write refused by write_policy: feature_transitions is off");
            }
            if !system.transitions.values().any(|t| t == name) {
                bail!("{who}write refused: transition '{name}' is not listed in transitions of system '{}'", system.name);
            }
        }
        WriteKind::DedicatedIssue | WriteKind::Defect => {
            let (allowed, label) = if req.kind == WriteKind::Defect {
                (p.defects, "defects")
            } else {
                (p.dedicated_issues, "dedicated_issues")
            };
            if !allowed {
                bail!("{who}write refused by write_policy: {label} is off");
            }
            for f in &req.field_ids {
                // summary and description are set when an issue is created, never on an update
                if !req.issue_key.is_empty() && PROTECTED_FEATURE_FIELDS.contains(&f.as_str()) {
                    bail!(
                        "{who}write refused: '{f}' of an existing issue is never written by TCMS"
                    );
                }
                let ok = CREATE_SYSTEM_FIELDS.contains(&f.as_str())
                    || system.fields.configured_ids().contains(&f.as_str());
                if !ok {
                    bail!(
                        "{who}write refused: field '{f}' is not listed in fields of system '{}'",
                        system.name
                    );
                }
            }
        }
        WriteKind::Comment => {
            if !p.comments {
                bail!("{who}write refused by write_policy: comments is off");
            }
        }
    }
    Ok(())
}
