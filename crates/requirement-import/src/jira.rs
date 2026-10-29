//! Jira adapter: issues of `feature_issue_types` in `project_keys` become requirement records.
//!
//! Mapping: Epic = `system`, Story and Feature = `HLR`; any other issue type is refused rather
//! than guessed. `id` and `source` = issue key; `title` = summary; `text` = description as
//! plain text; `version` = the issue's `updated` timestamp; `status` (Jira's own status name) is
//! kept in `external_refs[].status`; `parent_ids` = the parent issue (epic link) when there is one.
//! Safety and security flags come from labels, and only when a label name is given.
//!
//! Verified against the mock Jira (both layouts) in the tests; never against a real site.

use anyhow::{anyhow, bail, Context, Result};
use id_resolver::SystemConfig;
use jira_client::JiraClient;
use serde_json::Value;

use crate::{ExternalRef, RequirementRecord};

/// Fields read from each issue.
pub const FIELDS: &[&str] = &[
    "summary",
    "description",
    "issuetype",
    "status",
    "updated",
    "labels",
    "parent",
];

/// Label names that mark safety or security features. `None`: the flag is not set at all.
#[derive(Debug, Clone, Copy, Default)]
pub struct LabelFlags<'a> {
    pub safety: Option<&'a str>,
    pub security: Option<&'a str>,
}

/// JQL selecting the feature issues of the system's projects.
pub fn jql(system: &SystemConfig) -> Result<String> {
    if system.project_keys.is_empty() {
        bail!(
            "system '{}': project_keys is empty, nothing to import",
            system.name
        );
    }
    if system.feature_issue_types.is_empty() {
        bail!(
            "system '{}': feature_issue_types is empty, nothing to import",
            system.name
        );
    }
    let keys = system.project_keys.join(", ");
    let types = system
        .feature_issue_types
        .iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "project in ({keys}) AND issuetype in ({types}) ORDER BY key"
    ))
}

/// Read the feature issues through the client and map them, in key order.
pub fn fetch(
    client: &JiraClient,
    system: &SystemConfig,
    flags: LabelFlags,
) -> Result<Vec<RequirementRecord>> {
    let issues = client.search_issues(&jql(system)?, FIELDS)?;
    let mut records = issues
        .iter()
        .map(|i| record_from_issue(i, flags))
        .collect::<Result<Vec<_>>>()?;
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}

/// Map one Jira issue (REST v2 or v3 shape) to a requirement record.
pub fn record_from_issue(issue: &Value, flags: LabelFlags) -> Result<RequirementRecord> {
    let key = issue["key"]
        .as_str()
        .ok_or_else(|| anyhow!("Jira issue without a key"))?;
    let f = &issue["fields"];
    let issue_type = f["issuetype"]["name"].as_str().unwrap_or_default();
    let req_type = match issue_type {
        "Epic" => "system",
        "Story" | "Feature" => "HLR",
        other => bail!(
            "issue {key}: issue type '{other}' has no requirement type (Epic = system, Story/Feature = HLR)"
        ),
    };
    let updated = f["updated"]
        .as_str()
        .with_context(|| format!("issue {key}: no updated timestamp"))?;
    let status = f["status"]["name"].as_str().map(str::to_string);
    let labels: Vec<&str> = f["labels"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();

    let mut rec = RequirementRecord::new(key, "jira", req_type, updated, key);
    rec.title = f["summary"].as_str().map(str::to_string);
    rec.text = jira_client::adf_to_text(&f["description"]);
    rec.safety_related = flags.safety.map(|l| labels.contains(&l));
    rec.security_related = flags.security.map(|l| labels.contains(&l));
    if let Some(parent) = f["parent"]["key"].as_str() {
        rec.parent_ids.push(parent.to_string());
    }
    rec.external_refs.push(ExternalRef {
        system: "jira".into(),
        key: key.into(),
        status,
    });
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn issue(key: &str, itype: &str) -> Value {
        json!({
            "key": key,
            "fields": {
                "summary": "Pay by card",
                "description": {"type":"doc","version":1,"content":[
                    {"type":"paragraph","content":[{"type":"text","text":"Customer pays"}]}]},
                "issuetype": {"name": itype},
                "status": {"name": "In progress"},
                "updated": "2026-10-20T09:00:00.000+0000",
                "labels": ["safety", "other"],
                "parent": {"key": "PAY-1"}
            }
        })
    }

    #[test]
    fn epic_is_system_and_story_is_hlr() {
        let r = record_from_issue(&issue("PAY-2", "Epic"), LabelFlags::default()).unwrap();
        assert_eq!(r.req_type, "system");
        let s = record_from_issue(&issue("PAY-3", "Story"), LabelFlags::default()).unwrap();
        assert_eq!(s.req_type, "HLR");
        let f = record_from_issue(&issue("PAY-4", "Feature"), LabelFlags::default()).unwrap();
        assert_eq!(f.req_type, "HLR");
    }

    #[test]
    fn other_issue_types_are_refused() {
        let err = record_from_issue(&issue("PAY-5", "Bug"), LabelFlags::default()).unwrap_err();
        assert!(format!("{err}").contains("PAY-5"), "{err}");
    }

    #[test]
    fn fields_map_as_documented() {
        let r = record_from_issue(&issue("PAY-3", "Story"), LabelFlags::default()).unwrap();
        assert_eq!(r.id, "PAY-3");
        assert_eq!(r.source, "PAY-3");
        assert_eq!(r.source_system, "jira");
        assert_eq!(r.title.as_deref(), Some("Pay by card"));
        assert_eq!(r.text, "Customer pays");
        assert_eq!(r.version, "2026-10-20T09:00:00.000+0000");
        assert_eq!(r.parent_ids, vec!["PAY-1"]);
        assert_eq!(r.external_refs[0].status.as_deref(), Some("In progress"));
        assert_eq!(r.safety_related, None, "no label configured, no flag");
    }

    #[test]
    fn labels_set_flags_only_when_configured() {
        let flags = LabelFlags {
            safety: Some("safety"),
            security: Some("security"),
        };
        let r = record_from_issue(&issue("PAY-3", "Story"), flags).unwrap();
        assert_eq!(r.safety_related, Some(true));
        assert_eq!(r.security_related, Some(false));
    }

    #[test]
    fn jql_names_projects_and_types() {
        let sys = SystemConfig {
            name: "jira".into(),
            system_type: id_resolver::SystemType::JiraCloud,
            base_url: "https://x".into(),
            auth: None,
            id_pattern: "^[A-Z]+-[0-9]+$".into(),
            url_template: "{base_url}/browse/{id}".into(),
            url_pattern: None,
            document_map: Default::default(),
            project_keys: vec!["PAY".into(), "AUTH".into()],
            feature_issue_types: vec!["Epic".into(), "Story".into()],
            defect_issue_type: None,
            severity_to_priority: Default::default(),
            test_backend: None,
            tcms_issue_types: None,
            fields: Default::default(),
            status_values: Default::default(),
            milestone: None,
            write_policy: Default::default(),
            transitions: Default::default(),
        };
        assert_eq!(
            jql(&sys).unwrap(),
            "project in (PAY, AUTH) AND issuetype in (\"Epic\", \"Story\") ORDER BY key"
        );
    }
}
