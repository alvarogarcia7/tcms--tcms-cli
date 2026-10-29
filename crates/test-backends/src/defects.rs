//! Defects (TCMS-42.8): a problem report becomes a Jira issue of `defect_issue_type`, or is
//! linked to an issue that exists. Creating and linking go through [`Ctx::write`] with
//! [`WriteKind::Defect`], so the write guard applies. The rules that decide blocking and
//! reconciliation are pure functions at the end of this module.
//!
//! Decisions (TCMS-42.8, worker C3):
//! * the Jira key of a problem report is kept in its own `external_refs` (the problem-report
//!   schema has the field); `tcms pr push` and `tcms pr link` write it there;
//! * a problem report that already names a Jira key is never pushed again;
//! * sync never changes a problem report's status and never a feature summary or description;
//!   a Jira status that disagrees with the report is a reported difference (AC #4).
//!
//! Idempotence without a local key (TCMS-42.9 defect, D-42.9a..c): every defect is created with
//! the label `tcms-pr-<problem report id>`. Before creating, `push` searches the first project
//! for that label (a read) and links to the issue it finds instead of creating a second one. A
//! fresh checkout, with no state and no key in the report, therefore creates nothing that exists.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Map, Value};

use crate::api::Ctx;
use jira_client::WriteKind;

/// Statuses after which a deficiency counts as resolved (problem-report `status`).
pub const RESOLVED_STATUSES: &[&str] = &["fixed", "verified", "closed", "rejected"];

/// The fields of a problem report that matter to Jira.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemReport {
    pub id: String,
    pub title: String,
    pub description: String,
    /// critical, major, minor or cosmetic
    pub severity: Option<String>,
    pub status: String,
    pub raised_by: String,
    /// Feature (requirement) keys named by `affected_items`
    pub feature_keys: Vec<String>,
    /// Jira key already recorded in `external_refs`
    pub external_key: Option<String>,
}

/// A status is open unless it is resolved. An unknown status counts as open (it blocks).
pub fn is_open(status: &str) -> bool {
    !RESOLVED_STATUSES.contains(&status)
}

/// A Jira issue as read back: its status name and whether its status category is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueState {
    pub key: String,
    pub status: String,
    pub done: bool,
}

/// The Jira label of the defect for a problem report (D-42.9a). Labels cannot hold spaces, so
/// only letters, digits, `-` and `_` are accepted in the report id; anything else is refused
/// before a request is built.
pub fn defect_label(pr_id: &str) -> Result<String> {
    let ok = !pr_id.is_empty()
        && pr_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        bail!(
            "problem report id '{pr_id}' cannot name a Jira label (letters, digits, '-' and '_' only)"
        );
    }
    Ok(format!("tcms-pr-{pr_id}"))
}

/// Issues of the first configured project that carry `label` (exact label match). A read: it is
/// not guarded and sends no write.
pub fn find_by_label(ctx: &mut Ctx, label: &str) -> Result<Vec<IssueState>> {
    let project = ctx
        .system
        .project_keys
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("system '{}': project_keys is empty", ctx.system.name))?;
    let jql = format!("project = \"{project}\" AND labels = \"{label}\"");
    let issues = ctx.api.search_jql(&jql, &["status"])?;
    issues
        .iter()
        .map(|issue| {
            let key = issue["key"]
                .as_str()
                .ok_or_else(|| anyhow!("search result without a key"))?;
            let status = issue["fields"]["status"]["name"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let done = issue["fields"]["status"]["statusCategory"]["key"].as_str() == Some("done");
            Ok(IssueState {
                key: key.to_string(),
                status,
                done,
            })
        })
        .collect()
}

/// Jira REST prefix of Cloud (v3) takes Atlassian Document Format for rich text.
fn rich_text(prefix: &str, text: &str) -> Value {
    if prefix == "/rest/api/3" {
        let paragraphs: Vec<Value> = text
            .lines()
            .map(|l| json!({"type": "paragraph", "content": [{"type": "text", "text": l}]}))
            .collect();
        json!({"type": "doc", "version": 1, "content": paragraphs})
    } else {
        Value::String(text.to_string())
    }
}

/// The key must belong to one of the configured projects (no request is sent otherwise).
fn in_scope(ctx: &Ctx, key: &str) -> Result<()> {
    let project = key.rsplit_once('-').map(|(p, _)| p).unwrap_or("");
    if ctx.system.project_keys.is_empty() || ctx.system.project_keys.iter().any(|k| k == project) {
        return Ok(());
    }
    bail!(
        "issue {key}: project '{project}' is not in project_keys {:?}",
        ctx.system.project_keys
    )
}

/// Read one issue: status and status category. A read, never guarded.
pub fn observe(ctx: &mut Ctx, key: &str) -> Result<IssueState> {
    in_scope(ctx, key)?;
    let prefix = ctx.api.rest_prefix();
    let v = ctx.api.get(
        &format!("{prefix}/issue/{key}"),
        &[("fields", "status".into())],
    )?;
    let status = v["fields"]["status"]["name"]
        .as_str()
        .ok_or_else(|| anyhow!("issue {key}: no status in the response"))?
        .to_string();
    let done = v["fields"]["status"]["statusCategory"]["key"].as_str() == Some("done");
    Ok(IssueState {
        key: key.to_string(),
        status,
        done,
    })
}

/// What `push` did, or would do in a dry run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    /// The problem report already names a Jira issue; nothing was sent
    AlreadyLinked { key: String },
    /// An issue with the report's label already exists (created by an earlier run or by hand):
    /// nothing is created or linked; the caller records the key. Found by label, not by a key
    /// kept on this machine (D-42.9a).
    Found { key: String },
    /// Issue created; `links` names the targets linked, `notes` what was not done and why
    Created {
        key: String,
        links: Vec<String>,
        notes: Vec<String>,
    },
    /// Dry run: what would be created and linked
    Planned {
        summary: String,
        links: Vec<String>,
        notes: Vec<String>,
    },
}

/// Create the defect for a problem report, linked to its features and to `test_entity`.
///
/// `actor` is written into the issue description so the Jira side names the acting user.
pub fn push(
    ctx: &mut Ctx,
    pr: &ProblemReport,
    test_entity: Option<&str>,
    actor: &str,
    dry_run: bool,
) -> Result<PushOutcome> {
    if let Some(key) = &pr.external_key {
        return Ok(PushOutcome::AlreadyLinked { key: key.clone() });
    }
    let system = ctx.system.clone();
    let defect_type = system.defect_issue_type.clone().ok_or_else(|| {
        anyhow!(
            "system '{}': defect_issue_type is required to push problem reports",
            system.name
        )
    })?;
    let project = system
        .project_keys
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("system '{}': project_keys is empty", system.name))?;
    let prefix = ctx.api.rest_prefix();
    let mut notes = Vec::new();

    // The label is searched first, in dry runs too (a read): a second run finds the defect of the
    // first and creates nothing (D-42.9a). More than one match is refused, not guessed (D-42.9b).
    let label = defect_label(&pr.id)?;
    let found = find_by_label(ctx, &label)?;
    match found.as_slice() {
        [] => {}
        [one] => {
            return Ok(PushOutcome::Found {
                key: one.key.clone(),
            })
        }
        many => {
            let keys: Vec<&str> = many.iter().map(|i| i.key.as_str()).collect();
            bail!(
                "problem report {}: {} issues carry the label {label} in project {project} ({}); \
                 keep one, remove the others in Jira or record the key with tcms pr link",
                pr.id,
                many.len(),
                keys.join(", ")
            );
        }
    }

    let priority = match pr
        .severity
        .as_deref()
        .and_then(|s| system.severity_to_priority.get(s))
    {
        Some(p) => Some(p.clone()),
        None => {
            notes.push(format!(
                "severity {:?} has no severity_to_priority entry; priority left unset",
                pr.severity
            ));
            None
        }
    };

    let summary = format!("[{}] {}", pr.id, pr.title);
    let description = format!(
        "{}\n\nTCMS problem report {} raised by {}; defect pushed by {actor}.",
        pr.description.trim(),
        pr.id,
        pr.raised_by
    );
    let mut fields = Map::new();
    fields.insert("project".into(), json!({ "key": project }));
    fields.insert("issuetype".into(), json!({ "name": defect_type }));
    fields.insert("summary".into(), json!(summary));
    fields.insert("description".into(), rich_text(&prefix, &description));
    fields.insert("labels".into(), json!(["tcms", label]));
    if let Some(p) = &priority {
        fields.insert("priority".into(), json!({ "name": p }));
    }

    let link_type = system
        .tcms_issue_types
        .as_ref()
        .and_then(|t| t.link_type.clone());
    let mut targets: Vec<String> = pr.feature_keys.clone();
    targets.extend(test_entity.map(str::to_string));
    if !targets.is_empty() && link_type.is_none() {
        notes.push("tcms_issue_types.link_type is not set; the defect is not linked".into());
    }

    if dry_run {
        let links = if link_type.is_some() {
            targets.clone()
        } else {
            Vec::new()
        };
        return Ok(PushOutcome::Planned {
            summary,
            links,
            notes,
        });
    }

    let ids: Vec<&str> = fields.keys().map(String::as_str).collect();
    let created = ctx.write(
        WriteKind::Defect,
        "",
        &ids,
        "POST",
        &format!("{prefix}/issue"),
        &json!({ "fields": fields }),
    )?;
    let key = created["key"]
        .as_str()
        .ok_or_else(|| anyhow!("create defect: response has no key"))?
        .to_string();

    let mut links = Vec::new();
    if let Some(lt) = link_type {
        for target in &targets {
            link(ctx, &key, target, &lt)?;
            links.push(target.clone());
        }
    }
    Ok(PushOutcome::Created { key, links, notes })
}

/// Link `defect` (outward) to `target` (inward) with the configured link type. Guarded as a
/// defect write: the defect's key is the issue that is checked.
fn link(ctx: &mut Ctx, defect: &str, target: &str, link_type: &str) -> Result<()> {
    in_scope(ctx, target)?;
    let prefix = ctx.api.rest_prefix();
    ctx.write(
        WriteKind::Defect,
        defect,
        &[],
        "POST",
        &format!("{prefix}/issueLink"),
        &json!({
            "type": { "name": link_type },
            "outwardIssue": { "key": defect },
            "inwardIssue": { "key": target }
        }),
    )?;
    Ok(())
}

/// `tcms pr link`: check that an existing issue is a Jira issue of a configured project and
/// return it. The caller records the key in the problem report. Reads only.
pub fn link_existing(ctx: &mut Ctx, key: &str) -> Result<IssueState> {
    observe(ctx, key)
}

/// A difference between a problem report and its Jira issue. Reported, never applied (AC #4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub problem_report: String,
    pub message: String,
}

/// Compare the report's status with the linked issue's status. Only disagreements are reported.
pub fn reconcile(pr_id: &str, pr_status: &str, issue: &IssueState) -> Option<Difference> {
    let resolved = !is_open(pr_status);
    let message = match (resolved, issue.done) {
        (false, true) => format!(
            "Jira issue {} is '{}' (done) but problem report {pr_id} is '{pr_status}'; not changed",
            issue.key, issue.status
        ),
        (true, false) => format!(
            "problem report {pr_id} is '{pr_status}' but Jira issue {} is '{}'; not changed",
            issue.key, issue.status
        ),
        _ => return None,
    };
    Some(Difference {
        problem_report: pr_id.to_string(),
        message,
    })
}

/// Blocking rule (TCMS-42.8 AC #3): a feature's verification cannot be `passed` while a
/// problem report on it is open, or while its linked Jira issue is not done.
pub fn blocks_passed(pr_status: &str, linked: Option<&IssueState>) -> bool {
    is_open(pr_status) || linked.is_some_and(|i| !i.done)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(done: bool) -> IssueState {
        IssueState {
            key: "PAY-9".into(),
            status: if done { "Done" } else { "To Do" }.into(),
            done,
        }
    }

    #[test]
    fn closed_in_jira_is_a_difference_not_a_status_change() {
        let d = reconcile("PR-1", "open", &issue(true)).expect("difference");
        assert!(d.message.contains("not changed"));
        assert!(reconcile("PR-1", "fixed", &issue(false)).is_some());
        assert!(reconcile("PR-1", "open", &issue(false)).is_none());
        assert!(reconcile("PR-1", "fixed", &issue(true)).is_none());
    }

    #[test]
    fn open_report_or_open_issue_blocks_passed() {
        assert!(blocks_passed("open", None));
        assert!(blocks_passed("deferred", Some(&issue(true))));
        assert!(blocks_passed("fixed", Some(&issue(false))));
        assert!(!blocks_passed("closed", Some(&issue(true))));
        assert!(!blocks_passed("verified", None));
    }

    #[test]
    fn rich_text_is_adf_on_cloud_only() {
        assert_eq!(rich_text("/rest/api/3", "a\nb")["type"], "doc");
        assert_eq!(rich_text("/rest/api/2", "a"), json!("a"));
    }
}
