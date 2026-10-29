//! audit-trail-entry records for sync writes (`action: sync`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const AUDIT_SCHEMA_URI: &str = "tcms/audit-trail-entry.schema.json";

/// TCMS record a Jira write belongs to.
#[derive(Debug, Clone)]
pub struct WriteTarget {
    /// Schema name of the TCMS record (requirement, test-case, test-plan, problem-report, ...)
    pub entity_type: String,
    pub entity_id: String,
    pub version: Option<String>,
}

impl WriteTarget {
    pub fn new(entity_type: &str, entity_id: &str) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntity {
    pub entity_type: String,
    pub entity_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditTrailEntry {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub schema: String,
    pub id: String,
    /// The acting TCMS user
    pub who: String,
    pub when: String,
    pub action: String,
    pub entity: AuditEntity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl AuditTrailEntry {
    /// `after` holds the external_refs written and the Jira field ids touched.
    pub fn sync(
        id: String,
        who: &str,
        target: &WriteTarget,
        external_refs: Vec<Value>,
        written_fields: &[String],
        reason: &str,
    ) -> Self {
        Self {
            doc_type: "audit_trail_entry".into(),
            schema: AUDIT_SCHEMA_URI.into(),
            id,
            who: who.into(),
            when: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            action: "sync".into(),
            entity: AuditEntity {
                entity_type: target.entity_type.clone(),
                entity_id: target.entity_id.clone(),
                version: target.version.clone(),
            },
            before: None,
            after: Some(
                json!({ "external_refs": external_refs, "written_fields": written_fields }),
            ),
            reason: Some(reason.into()),
        }
    }

    pub fn to_yaml(&self) -> Result<String> {
        testcase_common::to_yaml_string(self)
    }
}

/// Write each entry as `<dir>/<id>.yaml`. Returns the paths.
pub fn write_audit_entries(dir: &Path, entries: &[AuditTrailEntry]) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut paths = Vec::new();
    for e in entries {
        let p = dir.join(format!("{}.yaml", e.id));
        std::fs::write(&p, e.to_yaml()?).with_context(|| format!("write {}", p.display()))?;
        paths.push(p);
    }
    Ok(paths)
}
