//! Typed view of the vendored `integration-config` schema, and its loader.
//!
//! A config is validated against `schemas/tcms-schemas/tcms/integration-config.schema.json`
//! before it is deserialized, and then checked for rules JSON Schema cannot express
//! (compilable regexes, no overlapping `id_pattern`s, one requirement source).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::overlap::find_overlap;

/// Schema URI written in config files.
pub const CONFIG_SCHEMA_URI: &str = "tcms/integration-config.schema.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementSource {
    Jira,
    Strictdoc,
}

impl RequirementSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequirementSource::Jira => "jira",
            RequirementSource::Strictdoc => "strictdoc",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemType {
    JiraCloud,
    JiraDatacenter,
    StrictdocHtml,
    StrictdocServer,
    Gitlab,
}

impl SystemType {
    pub fn is_jira(&self) -> bool {
        matches!(self, SystemType::JiraCloud | SystemType::JiraDatacenter)
    }
    pub fn is_strictdoc(&self) -> bool {
        matches!(
            self,
            SystemType::StrictdocHtml | SystemType::StrictdocServer
        )
    }
    /// The `external_refs[].system` family this adapter belongs to.
    pub fn family(&self) -> testcase_models::ExternalSystem {
        use testcase_models::ExternalSystem as E;
        match self {
            SystemType::JiraCloud | SystemType::JiraDatacenter => E::Jira,
            SystemType::StrictdocHtml | SystemType::StrictdocServer => E::Strictdoc,
            SystemType::Gitlab => E::Gitlab,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Auth {
    #[serde(default)]
    pub method: String,
    /// Name of an environment variable (or secret store key); never the secret itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TcmsIssueTypes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_run: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
}

/// Custom field ids written by the sync. Unset means: never written.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Fields {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_lifecycle_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_verdict: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcms_id: Option<String>,
}

impl Fields {
    /// All configured custom field ids.
    pub fn configured_ids(&self) -> Vec<&str> {
        [
            &self.verification_status,
            &self.validation_status,
            &self.coverage,
            &self.test_lifecycle_status,
            &self.latest_verdict,
            &self.baseline,
            &self.tcms_id,
        ]
        .into_iter()
        .filter_map(|f| f.as_deref())
        .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Milestone {
    #[serde(default)]
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_on_plan_complete: Option<bool>,
}

/// What the sync may write. Everything not enabled is read-only.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WritePolicy {
    #[serde(default)]
    pub feature_fields: bool,
    #[serde(default)]
    pub feature_transitions: bool,
    #[serde(default)]
    pub dedicated_issues: bool,
    #[serde(default)]
    pub comments: bool,
    #[serde(default)]
    pub defects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub system_type: SystemType,
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    pub id_pattern: String,
    pub url_template: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub document_map: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_issue_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defect_issue_type: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub severity_to_priority: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcms_issue_types: Option<TcmsIssueTypes>,
    #[serde(default)]
    pub fields: Fields,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub status_values: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<Milestone>,
    #[serde(default)]
    pub write_policy: WritePolicy,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub transitions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SyncConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_actor: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrationConfig {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub product_id: String,
    pub requirement_source: RequirementSource,
    pub systems: Vec<SystemConfig>,
    #[serde(default)]
    pub sync: SyncConfig,
}

impl IntegrationConfig {
    /// Load, schema-validate and check a config file. `schemas_root` is the directory
    /// holding `tcms-schemas/` (normally `schemas/`).
    pub fn load(path: impl AsRef<Path>, schemas_root: &str) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read integration config {}", path.display()))?;
        Self::from_yaml_str(&text, schemas_root, &path.display().to_string())
    }

    /// Same as [`load`](Self::load) for YAML text. A missing `schema:` line is added, so the
    /// upstream samples (which carry no envelope) load too.
    pub fn from_yaml_str(text: &str, schemas_root: &str, source_name: &str) -> Result<Self> {
        let mut value: serde_yaml::Value = serde_yaml::from_str(text)
            .with_context(|| format!("Failed to parse YAML from {source_name}"))?;
        let map = value
            .as_mapping_mut()
            .ok_or_else(|| anyhow!("{source_name}: integration config must be a YAML mapping"))?;

        reject_both_sources(map, source_name)?;

        let schema_key = serde_yaml::Value::from("schema");
        if !map.contains_key(&schema_key) {
            map.insert(schema_key, serde_yaml::Value::from(CONFIG_SCHEMA_URI));
        }
        let normalized = serde_yaml::to_string(&value)?;
        let config: IntegrationConfig = testcase_common::parse_and_validate_yaml_string(
            &normalized,
            schemas_root,
            source_name,
        )?;
        config
            .check()
            .with_context(|| format!("Invalid integration config {source_name}"))?;
        Ok(config)
    }

    /// Rules beyond JSON Schema.
    pub fn check(&self) -> Result<()> {
        let mut names = std::collections::BTreeSet::new();
        for s in &self.systems {
            if !names.insert(s.name.as_str()) {
                bail!("system name '{}' is used twice", s.name);
            }
            regex::Regex::new(&s.id_pattern).with_context(|| {
                format!(
                    "system '{}': id_pattern '{}' is not a valid regex",
                    s.name, s.id_pattern
                )
            })?;
            if let Some(up) = &s.url_pattern {
                let re = regex::Regex::new(up).with_context(|| {
                    format!(
                        "system '{}': url_pattern '{}' is not a valid regex",
                        s.name, up
                    )
                })?;
                if !re.capture_names().flatten().any(|n| n == "id") {
                    bail!(
                        "system '{}': url_pattern must define the named group 'id'",
                        s.name
                    );
                }
            }
            if s.url_template.contains("{doc}") && s.document_map.is_empty() {
                bail!(
                    "system '{}': url_template uses {{doc}} but document_map is empty",
                    s.name
                );
            }
        }
        if let Some((a, b, witness)) = find_overlap(
            &self
                .systems
                .iter()
                .map(|s| (s.name.as_str(), s.id_pattern.as_str()))
                .collect::<Vec<_>>(),
        ) {
            bail!(
                "id_pattern of system '{a}' and system '{b}' overlap (for example '{witness}' matches both); \
                 id_patterns must be disjoint so an id belongs to exactly one system"
            );
        }
        // Exactly one requirement source: the selected source needs a system of its family.
        let has = |pred: fn(&SystemType) -> bool| self.systems.iter().any(|s| pred(&s.system_type));
        match self.requirement_source {
            RequirementSource::Jira if !has(SystemType::is_jira) => {
                bail!("requirement_source is jira but no jira_cloud or jira_datacenter system is configured")
            }
            RequirementSource::Strictdoc if !has(SystemType::is_strictdoc) => {
                bail!("requirement_source is strictdoc but no strictdoc_html or strictdoc_server system is configured")
            }
            _ => {}
        }
        Ok(())
    }

    /// The system that provides requirements.
    pub fn source_system(&self) -> Result<&SystemConfig> {
        let want: fn(&SystemType) -> bool = match self.requirement_source {
            RequirementSource::Jira => SystemType::is_jira,
            RequirementSource::Strictdoc => SystemType::is_strictdoc,
        };
        let found: Vec<&SystemConfig> = self
            .systems
            .iter()
            .filter(|s| want(&s.system_type))
            .collect();
        match found.as_slice() {
            [one] => Ok(one),
            [] => bail!(
                "no system for requirement_source {}",
                self.requirement_source.as_str()
            ),
            many => bail!(
                "requirement_source {} matches several systems ({}); keep one",
                self.requirement_source.as_str(),
                many.iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// `requirement_source: [jira, strictdoc]` or `jira,strictdoc` names both sources: say so
/// plainly instead of a bare schema "is not one of" error.
fn reject_both_sources(map: &serde_yaml::Mapping, source_name: &str) -> Result<()> {
    let Some(v) = map.get(serde_yaml::Value::from("requirement_source")) else {
        return Ok(());
    };
    let text = match v {
        serde_yaml::Value::Sequence(items) => items
            .iter()
            .filter_map(|i| i.as_str())
            .collect::<Vec<_>>()
            .join(","),
        serde_yaml::Value::String(s) => s.clone(),
        _ => return Ok(()),
    };
    if text.contains("jira") && text.contains("strictdoc") {
        bail!(
            "{source_name}: requirement_source names both jira and strictdoc; \
             a product reads requirements from exactly one source"
        );
    }
    Ok(())
}
