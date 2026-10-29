//! StrictDoc adapter: items of a StrictDoc export become requirement records.
//!
//! ASSUMPTION (TCMS-42.3, not verified): StrictDoc is not installed in this environment and no
//! export sample is available, so the JSON layout read here is an assumed shape, not StrictDoc's
//! documented output. It is isolated in this module, and the fixture
//! `tests/fixtures/strictdoc_export.json` shows it:
//!
//! ```json
//! { "documents": [ { "path": "docs/srs.sdoc",
//!     "requirements": [ { "uid": "HLR-12", "level": "HLR", "title": "...", "statement": "...",
//!                         "status": "approved", "parent_uids": ["SYS-4"],
//!                         "safety_related": false, "security_related": true } ] } ] }
//! ```
//!
//! Mapping: `id` = UID; `type` = level; `text` = statement; `version` = the commit the export was
//! taken at (given by the caller, not read from the file); `source` = `<document path>#<UID>`;
//! `external_refs` names the UID in system `strictdoc`.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{ExternalRef, RequirementRecord, REQUIREMENT_STATUSES, REQUIREMENT_TYPES};

#[derive(Debug, Deserialize)]
struct Export {
    documents: Vec<Document>,
}

#[derive(Debug, Deserialize)]
struct Document {
    path: String,
    #[serde(default)]
    requirements: Vec<Item>,
}

#[derive(Debug, Deserialize)]
struct Item {
    uid: String,
    level: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    statement: String,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    parent_uids: Vec<String>,
    #[serde(default)]
    safety_related: bool,
    #[serde(default)]
    security_related: bool,
}

/// Parse an export taken at `commit` into requirement records, in UID order.
pub fn parse_export(json: &str, commit: &str) -> Result<Vec<RequirementRecord>> {
    if commit.trim().is_empty() || commit.chars().any(char::is_whitespace) {
        bail!("StrictDoc import needs the commit of the export (--commit), got '{commit}'");
    }
    let export = read_export(json)?;

    let mut records = Vec::new();
    for doc in &export.documents {
        for item in &doc.requirements {
            let source = format!("{}#{}", doc.path, item.uid);
            let mut rec =
                RequirementRecord::new(&item.uid, "strictdoc", &item.level, commit, &source);
            rec.title = item.title.clone();
            rec.text = item.statement.clone();
            rec.status = item.status.clone();
            rec.safety_related = Some(item.safety_related);
            rec.security_related = Some(item.security_related);
            rec.parent_ids = item.parent_uids.clone();
            rec.external_refs.push(ExternalRef {
                system: "strictdoc".into(),
                key: item.uid.clone(),
                status: None,
            });
            records.push(rec);
        }
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(records)
}

/// The `(UID, level)` of every item of an export, in document order, with the same checks as
/// [`parse_export`] but without a commit. For consumers that need only the levels, such as the
/// trace matrix. Items of other levels (`system`, `SFR`, ...) are returned too: the caller decides
/// which levels count.
pub fn export_levels(json: &str) -> Result<Vec<(String, String)>> {
    let export = read_export(json)?;
    Ok(export
        .documents
        .iter()
        .flat_map(|doc| doc.requirements.iter())
        .map(|item| (item.uid.clone(), item.level.clone()))
        .collect())
}

/// Parse the assumed layout and check every item (level and status), so both entry points refuse
/// the same exports.
fn read_export(json: &str) -> Result<Export> {
    let export: Export = serde_json::from_str(json)
        .context("StrictDoc export: expected a JSON object with a 'documents' array (assumed layout, see module docs)")?;
    for doc in &export.documents {
        for item in &doc.requirements {
            check_item(&doc.path, item)?;
        }
    }
    Ok(export)
}

fn check_item(path: &str, item: &Item) -> Result<()> {
    if !REQUIREMENT_TYPES.contains(&item.level.as_str()) {
        bail!(
            "StrictDoc item {} in {}: level '{}' is not a requirement type ({})",
            item.uid,
            path,
            item.level,
            REQUIREMENT_TYPES.join(", ")
        );
    }
    if let Some(s) = &item.status {
        if !REQUIREMENT_STATUSES.contains(&s.as_str()) {
            bail!(
                "StrictDoc item {}: status '{s}' is not a requirement status",
                item.uid
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = r#"{"documents":[{"path":"docs/srs.sdoc","requirements":[
        {"uid":"HLR-12","level":"HLR","title":"SMS update","statement":"Apply within 5 s.",
         "status":"approved","parent_uids":["SYS-4"],"security_related":true}]}]}"#;

    #[test]
    fn items_map_to_requirement_records() {
        let recs = parse_export(EXPORT, "9f1c2ab").unwrap();
        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.id, "HLR-12");
        assert_eq!(r.source_system, "strictdoc");
        assert_eq!(r.req_type, "HLR");
        assert_eq!(r.version, "9f1c2ab", "version is the commit");
        assert_eq!(r.source, "docs/srs.sdoc#HLR-12", "document path and UID");
        assert_eq!(r.status.as_deref(), Some("approved"));
        assert_eq!(r.security_related, Some(true));
        assert_eq!(r.safety_related, Some(false));
        assert_eq!(r.parent_ids, vec!["SYS-4"]);
    }

    #[test]
    fn commit_is_required() {
        assert!(parse_export(EXPORT, "").is_err());
        assert!(parse_export(EXPORT, "a b").is_err());
    }

    #[test]
    fn unknown_level_is_refused() {
        let bad = EXPORT.replace("\"HLR\"", "\"Epic\"");
        let err = parse_export(&bad, "abc").unwrap_err();
        assert!(format!("{err:#}").contains("HLR-12"), "{err:#}");
    }

    #[test]
    fn wrong_layout_is_refused_with_the_assumption_named() {
        let err = parse_export("[1,2]", "abc").unwrap_err();
        assert!(format!("{err:#}").contains("assumed layout"), "{err:#}");
    }

    #[test]
    fn export_levels_lists_every_item_with_the_same_checks() {
        let levels = export_levels(EXPORT).unwrap();
        assert_eq!(levels, vec![("HLR-12".to_string(), "HLR".to_string())]);
        let bad = EXPORT.replace("\"HLR\"", "\"Epic\"");
        assert!(export_levels(&bad).is_err());
        assert!(export_levels("[1,2]").is_err());
    }
}
