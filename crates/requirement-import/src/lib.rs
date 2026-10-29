//! Requirement source adapters (TCMS-42.3).
//!
//! A product reads requirements from exactly one source (`integration-config.requirement_source`).
//! Both adapters produce the same [`RequirementRecord`], written as requirement YAML that
//! validates against `tcms-schemas/tcms/requirement.schema.json`, so coverage, the trace matrix
//! and reviews read them unchanged.
//!
//! * [`jira`]: issues of `feature_issue_types` in `project_keys` (Epic = system, Story/Feature = HLR).
//! * [`strictdoc`]: items of a StrictDoc JSON export taken at a commit. The export layout is an
//!   ASSUMPTION (see the module docs).
//!
//! [`write_requirements`] is idempotent: a record whose YAML is unchanged is not rewritten, so a
//! re-import produces no diff.

pub mod jira;
pub mod strictdoc;

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

/// Schema URI written into every requirement record.
pub const REQUIREMENT_SCHEMA_URI: &str = "tcms/requirement.schema.json";

/// Allowed `type` values (requirement schema).
pub const REQUIREMENT_TYPES: &[&str] = &[
    "system",
    "HLR",
    "LLR",
    "derived",
    "SFR",
    "TSFI",
    "threat",
    "objective",
    "assumption",
];

/// Allowed `status` values (requirement schema).
pub const REQUIREMENT_STATUSES: &[&str] = &["draft", "in_review", "approved", "obsolete"];

/// A pointer to the external system a requirement comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalRef {
    pub system: String,
    pub key: String,
    /// Jira status of the issue at import time (not a requirement status)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// One requirement, as written to `<id>.yaml`. Field order is the file order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RequirementRecord {
    pub schema: String,
    pub id: String,
    /// `jira` or `strictdoc`
    pub source_system: String,
    /// Requirement type (`system`, `HLR`, ...)
    #[serde(rename = "type")]
    pub req_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub text: String,
    /// Jira: the issue's `updated` timestamp. StrictDoc: the commit of the export.
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_related: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_related: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parent_ids: Vec<String>,
    /// Jira: the issue key. StrictDoc: `<document path>#<UID>`.
    pub source: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

impl RequirementRecord {
    /// A record with the fields every adapter sets; the adapter fills the rest.
    pub fn new(id: &str, source_system: &str, req_type: &str, version: &str, source: &str) -> Self {
        Self {
            schema: REQUIREMENT_SCHEMA_URI.into(),
            id: id.into(),
            source_system: source_system.into(),
            req_type: req_type.into(),
            title: None,
            text: String::new(),
            version: version.into(),
            status: None,
            safety_related: None,
            security_related: None,
            parent_ids: Vec::new(),
            source: source.into(),
            external_refs: Vec::new(),
        }
    }

    /// The file content: requirement YAML in the form the schemas read back.
    pub fn to_yaml(&self) -> Result<String> {
        testcase_common::to_yaml_string(self)
    }
}

/// What [`write_requirements`] did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportReport {
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub unchanged: Vec<String>,
    /// Records of this source already on disk that the source no longer has. Kept, not deleted.
    pub stale: Vec<String>,
}

/// Validate every record, then write `<dir>/<id>.yaml` for each one that is new or changed.
///
/// * Records are written in id order; the YAML is deterministic, so unchanged records are skipped.
/// * Refused, before anything is written, when the directory holds records of another source
///   (a product reads requirements from one source only) or when two records share an id.
/// * Files of this source that the new import does not contain are reported as `stale`.
pub fn write_requirements(
    dir: &Path,
    records: &[RequirementRecord],
    schemas_root: &str,
) -> Result<ImportReport> {
    let mut sorted: Vec<&RequirementRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));

    let source = sorted.first().map(|r| r.source_system.clone());
    let mut seen = BTreeSet::new();
    for r in &sorted {
        validate_id(&r.id)?;
        if r.source_system != source.as_deref().unwrap_or_default() {
            bail!(
                "records of several sources in one import ({} and {})",
                r.source_system,
                source.clone().unwrap_or_default()
            );
        }
        if !seen.insert(r.id.as_str()) {
            bail!("requirement '{}' appears twice in the import", r.id);
        }
    }

    // What is on disk already
    let mut on_disk: Vec<(String, String)> = Vec::new(); // (id, source_system)
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
            let path = entry?.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let value: serde_yaml::Value = match serde_yaml::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let (Some(id), Some(src)) = (
                value.get("id").and_then(|v| v.as_str()),
                value.get("source_system").and_then(|v| v.as_str()),
            ) else {
                continue; // not a requirement record
            };
            on_disk.push((id.to_string(), src.to_string()));
        }
    }
    if let Some(src) = &source {
        if let Some((id, other)) = on_disk.iter().find(|(_, s)| s != src) {
            bail!(
                "{} holds requirement '{id}' from source '{other}', but this import reads '{src}': \
                 a product reads requirements from exactly one source",
                dir.display()
            );
        }
    }

    // Validate every record before writing any of them
    let mut texts = Vec::with_capacity(sorted.len());
    for r in &sorted {
        let text = r.to_yaml()?;
        testcase_common::parse_and_validate_yaml_string::<serde_yaml::Value, _>(
            &text,
            schemas_root,
            &format!("requirement {}", r.id),
        )?;
        texts.push(text);
    }

    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut report = ImportReport::default();
    for (r, text) in sorted.iter().zip(texts) {
        let path = dir.join(format!("{}.yaml", r.id));
        match std::fs::read_to_string(&path) {
            Ok(old) if old == text => report.unchanged.push(r.id.clone()),
            Ok(_) => {
                std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
                report.updated.push(r.id.clone());
            }
            Err(_) => {
                std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
                report.created.push(r.id.clone());
            }
        }
    }

    let ids: BTreeSet<&str> = sorted.iter().map(|r| r.id.as_str()).collect();
    let mut stale: Vec<String> = on_disk
        .into_iter()
        .filter(|(id, src)| Some(src.as_str()) == source.as_deref() && !ids.contains(id.as_str()))
        .map(|(id, _)| id)
        .collect();
    stale.sort();
    report.stale = stale;
    Ok(report)
}

/// Ids become file names and external keys: no path separators, no whitespace.
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.starts_with('.')
        || id
            .chars()
            .any(|c| c == '/' || c == '\\' || c.is_whitespace())
    {
        bail!("requirement id '{id}' cannot be used as a file name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_that_are_not_file_names_are_refused() {
        assert!(validate_id("PAY-2231").is_ok());
        assert!(validate_id("4.2.2").is_ok());
        assert!(validate_id("../x").is_err());
        assert!(validate_id("a b").is_err());
        assert!(validate_id("").is_err());
    }
}
