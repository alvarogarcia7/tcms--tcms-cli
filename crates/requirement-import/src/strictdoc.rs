//! StrictDoc adapter: items of a StrictDoc export become requirement records.
//!
//! The layout read here is the JSON that `strictdoc export --formats json` writes, verified
//! against StrictDoc 0.20.0 and 0.30.4 (DECISIONS.md Q1):
//!
//! ```json
//! { "DOCUMENTS": [ { "TITLE": "Software Requirements", "_NODE_TYPE": "DOCUMENT",
//!     "NODES": [ { "_NODE_TYPE": "REQUIREMENT", "UID": "HLR-12", "LEVEL": "HLR",
//!                  "TITLE": "...", "STATEMENT": "...", "STATUS": "approved",
//!                  "RELATIONS": [ { "TYPE": "Parent", "VALUE": "SYS-4" } ] } ] } ] }
//! ```
//!
//! Only nodes with `_NODE_TYPE` `REQUIREMENT` are items. `SECTION` nodes are walked for their
//! `NODES` to any depth; `TEXT` and other nodes are ignored. The old lowercase layout
//! (`documents` / `path` / `requirements`) is not accepted and is refused with a message naming
//! `DOCUMENTS`. A StrictDoc export holds no file path, so an item's path is its document's `TITLE`.
//!
//! Mapping: `id` = UID; `type` = LEVEL; `text` = STATEMENT; `version` = the commit the export was
//! taken at (given by the caller, not read from the file); `source` = `<document TITLE>#<UID>`;
//! `parent_ids` = the VALUE of every RELATIONS entry whose TYPE is `Parent`; `external_refs` names
//! the UID in system `strictdoc`. A REQUIREMENT without a UID is skipped, since it cannot be
//! addressed by an ID. A REQUIREMENT without a LEVEL makes the whole export refused; no level is
//! derived from the UID prefix.
//!
//! `safety_related` and `security_related` are not read from StrictDoc exports; they are always
//! false until a real export carrying them is added to the fixtures (D-07-8).

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{ExternalRef, RequirementRecord, REQUIREMENT_STATUSES, REQUIREMENT_TYPES};

/// The real export, as StrictDoc writes it. Unknown keys (GRAMMAR, _TOC, COMMENT, ...) are ignored.
#[derive(Debug, Deserialize)]
struct RawExport {
    #[serde(rename = "DOCUMENTS")]
    documents: Vec<RawDocument>,
}

#[derive(Debug, Deserialize)]
struct RawDocument {
    #[serde(rename = "TITLE")]
    title: String,
    #[serde(rename = "NODES", default)]
    nodes: Vec<RawNode>,
}

#[derive(Debug, Deserialize)]
struct RawNode {
    #[serde(rename = "_NODE_TYPE", default)]
    node_type: String,
    #[serde(rename = "UID", default)]
    uid: Option<String>,
    #[serde(rename = "LEVEL", default)]
    level: Option<String>,
    #[serde(rename = "TITLE", default)]
    title: Option<String>,
    #[serde(rename = "STATEMENT", default)]
    statement: Option<String>,
    #[serde(rename = "STATUS", default)]
    status: Option<String>,
    #[serde(rename = "RELATIONS", default)]
    relations: Vec<RawRelation>,
    #[serde(rename = "NODES", default)]
    nodes: Vec<RawNode>,
}

#[derive(Debug, Deserialize)]
struct RawRelation {
    #[serde(rename = "TYPE")]
    rel_type: String,
    #[serde(rename = "VALUE", default)]
    value: Option<String>,
}

/// The internal model the readers and checks work on. `path` is the document TITLE.
#[derive(Debug)]
struct Export {
    documents: Vec<Document>,
}

#[derive(Debug)]
struct Document {
    path: String,
    requirements: Vec<Item>,
}

#[derive(Debug)]
struct Item {
    uid: String,
    level: String,
    title: Option<String>,
    statement: String,
    status: Option<String>,
    parent_uids: Vec<String>,
    safety_related: bool,
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

/// One item of an export, as `tcms requirements locate` shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportItem {
    pub uid: String,
    pub level: String,
    pub title: Option<String>,
    pub status: Option<String>,
    /// Title of the document the item is in.
    pub path: String,
}

/// Every item of an export, in document order, with the checks of [`parse_export`]. No filter on
/// level: every item counts as found, whatever its level (D-05). Duplicate UIDs are returned as
/// they are; the caller decides.
pub fn export_items(json: &str) -> Result<Vec<ExportItem>> {
    let export = read_export(json)?;
    Ok(export
        .documents
        .iter()
        .flat_map(|doc| {
            doc.requirements.iter().map(move |item| ExportItem {
                uid: item.uid.clone(),
                level: item.level.clone(),
                title: item.title.clone(),
                status: item.status.clone(),
                path: doc.path.clone(),
            })
        })
        .collect())
}

/// Parse the real StrictDoc layout into the internal model and check every item (level, status),
/// so all entry points refuse the same exports.
fn read_export(json: &str) -> Result<Export> {
    let raw: RawExport = serde_json::from_str(json).context(
        "StrictDoc export: expected the JSON written by 'strictdoc export --formats json' \
         (an object with a 'DOCUMENTS' array)",
    )?;
    let export = to_export(raw)?;
    for doc in &export.documents {
        for item in &doc.requirements {
            check_item(&doc.path, item)?;
        }
    }
    Ok(export)
}

fn to_export(raw: RawExport) -> Result<Export> {
    let mut documents = Vec::with_capacity(raw.documents.len());
    for doc in raw.documents {
        let mut requirements = Vec::new();
        collect_items(&doc.title, &doc.nodes, &mut requirements)?;
        documents.push(Document {
            path: doc.title,
            requirements,
        });
    }
    Ok(Export { documents })
}

/// Depth-first walk in document order. A REQUIREMENT node is an item; every node's own `NODES`
/// are walked whatever its type, so sections (and any children of a requirement) are reached.
fn collect_items(title: &str, nodes: &[RawNode], out: &mut Vec<Item>) -> Result<()> {
    for node in nodes {
        if node.node_type == "REQUIREMENT" {
            if let Some(uid) = &node.uid {
                let Some(level) = &node.level else {
                    bail!(
                        "StrictDoc item {uid} in {title}: has no LEVEL; set LEVEL to one of the \
                         requirement types ({})",
                        REQUIREMENT_TYPES.join(", ")
                    );
                };
                let parent_uids = node
                    .relations
                    .iter()
                    .filter(|r| r.rel_type == "Parent")
                    .filter_map(|r| r.value.clone())
                    .collect();
                out.push(Item {
                    uid: uid.clone(),
                    level: level.clone(),
                    title: node.title.clone(),
                    statement: node.statement.clone().unwrap_or_default(),
                    status: node.status.clone(),
                    parent_uids,
                    // Not read from the export: always false until a real export carries them (D-07-8).
                    safety_related: false,
                    // Not read from the export: always false until a real export carries them (D-07-8).
                    security_related: false,
                });
            }
        }
        collect_items(title, &node.nodes, out)?;
    }
    Ok(())
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

    const EXPORT: &str = r#"{"DOCUMENTS":[{"TITLE":"Software Requirements","_NODE_TYPE":"DOCUMENT","NODES":[
        {"_NODE_TYPE":"REQUIREMENT","UID":"HLR-12","LEVEL":"HLR","TITLE":"SMS update",
         "STATEMENT":"Apply within 5 s.","STATUS":"approved",
         "RELATIONS":[{"TYPE":"Parent","VALUE":"SYS-4"}]}]}]}"#;

    #[test]
    fn items_map_to_requirement_records() {
        let recs = parse_export(EXPORT, "9f1c2ab").unwrap();
        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.id, "HLR-12");
        assert_eq!(r.source_system, "strictdoc");
        assert_eq!(r.req_type, "HLR");
        assert_eq!(r.version, "9f1c2ab", "version is the commit");
        assert_eq!(
            r.source, "Software Requirements#HLR-12",
            "document TITLE and UID"
        );
        assert_eq!(r.status.as_deref(), Some("approved"));
        assert_eq!(
            r.safety_related,
            Some(false),
            "the export carries no such flag"
        );
        assert_eq!(
            r.security_related,
            Some(false),
            "the export carries no such flag"
        );
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
    fn wrong_layout_is_refused_naming_documents() {
        let err = parse_export("[1,2]", "abc").unwrap_err();
        assert!(format!("{err:#}").contains("DOCUMENTS"), "{err:#}");
    }

    #[test]
    fn old_lowercase_layout_is_refused() {
        let old =
            r#"{"documents":[{"path":"d.sdoc","requirements":[{"uid":"HLR-1","level":"HLR"}]}]}"#;
        let err = parse_export(old, "abc").unwrap_err();
        assert!(format!("{err:#}").contains("DOCUMENTS"), "{err:#}");
        let err = export_levels(old).unwrap_err();
        assert!(format!("{err:#}").contains("DOCUMENTS"), "{err:#}");
        let err = export_items(old).unwrap_err();
        assert!(format!("{err:#}").contains("DOCUMENTS"), "{err:#}");
    }

    const FIXTURE: &str = include_str!("../tests/fixtures/strictdoc_export.json");

    #[test]
    fn nested_sections_are_walked_and_other_nodes_ignored() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[
            {"_NODE_TYPE":"TEXT","UID":"TXT-1","STATEMENT":"Intro"},
            {"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"HLR"},
            {"_NODE_TYPE":"SECTION","TITLE":"outer","NODES":[
                {"_NODE_TYPE":"SECTION","TITLE":"inner","NODES":[
                    {"_NODE_TYPE":"REQUIREMENT","UID":"LLR-1","LEVEL":"LLR"}]}]}]}]}"#;
        let items = export_items(json).unwrap();
        let uids: Vec<&str> = items.iter().map(|i| i.uid.as_str()).collect();
        assert_eq!(uids, vec!["HLR-1", "LLR-1"]);
    }

    #[test]
    fn only_parent_relations_become_parent_ids() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1",
            "LEVEL":"HLR","RELATIONS":[{"TYPE":"Parent","VALUE":"SYS-1"},
            {"TYPE":"File","VALUE":"a.c"},{"TYPE":"Child","VALUE":"LLR-9"}]}]}]}"#;
        let recs = parse_export(json, "abc").unwrap();
        assert_eq!(recs[0].parent_ids, vec!["SYS-1"]);
    }

    #[test]
    fn path_is_the_document_title() {
        let json = r#"{"DOCUMENTS":[
            {"TITLE":"System Requirements","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"SYS-1","LEVEL":"system"}]},
            {"TITLE":"Software Requirements","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"HLR"}]}]}"#;
        let items = export_items(json).unwrap();
        let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        assert_eq!(paths, vec!["System Requirements", "Software Requirements"]);
    }

    #[test]
    fn item_without_level_is_refused() {
        let json =
            r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-5"}]}]}"#;
        for err in [
            parse_export(json, "abc").unwrap_err(),
            export_levels(json).unwrap_err(),
            export_items(json).unwrap_err(),
        ] {
            let text = format!("{err:#}");
            assert!(
                text.contains("HLR-5") && text.contains("has no LEVEL"),
                "{text}"
            );
        }
    }

    #[test]
    fn item_without_uid_is_skipped() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[
            {"_NODE_TYPE":"REQUIREMENT","LEVEL":"HLR","STATEMENT":"no id"},
            {"_NODE_TYPE":"REQUIREMENT","UID":"HLR-6","LEVEL":"HLR"}]}]}"#;
        let items = export_items(json).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].uid, "HLR-6");
    }

    #[test]
    fn export_items_lists_every_level_in_document_order() {
        let items = export_items(FIXTURE).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].uid, "SYS-4");
        assert_eq!(items[0].level, "system");
        assert_eq!(items[0].path, "System Requirements");
        assert_eq!(items[0].title.as_deref(), Some("Remote profile management"));
        assert_eq!(items[0].status.as_deref(), Some("approved"));
        assert_eq!(items[1].uid, "HLR-12");
        assert_eq!(items[1].level, "HLR");
        assert_eq!(items[1].path, "Software Requirements");
        assert_eq!(items[2].uid, "HLR-3");
        assert_eq!(items[2].path, "Software Requirements");
        assert_eq!(items[2].status.as_deref(), Some("in_review"));
    }

    #[test]
    fn export_items_keeps_missing_title_and_status_as_none() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"LLR-1","LEVEL":"LLR"}]}]}"#;
        let items = export_items(json).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, None);
        assert_eq!(items[0].status, None);
    }

    #[test]
    fn export_items_refuses_a_level_the_importer_refuses() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"CH-1","LEVEL":"chapter"}]}]}"#;
        let err = export_items(json).unwrap_err();
        assert!(
            format!("{err:#}").contains("is not a requirement type"),
            "{err:#}"
        );
    }

    #[test]
    fn export_items_refuses_a_bad_status() {
        let json = r#"{"DOCUMENTS":[{"TITLE":"d","NODES":[{"_NODE_TYPE":"REQUIREMENT","UID":"HLR-1","LEVEL":"HLR","STATUS":"draft-ish"}]}]}"#;
        let err = export_items(json).unwrap_err();
        assert!(
            format!("{err:#}").contains("is not a requirement status"),
            "{err:#}"
        );
    }

    #[test]
    fn export_items_refuses_the_wrong_layout() {
        let err = export_items("[1,2]").unwrap_err();
        assert!(format!("{err:#}").contains("DOCUMENTS"), "{err:#}");
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
