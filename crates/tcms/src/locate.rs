//! `tcms requirements locate`: check that requirement IDs exist in their source (Jira or a
//! StrictDoc export). Read-only.

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use clap::ValueEnum;
use id_resolver::{IntegrationConfig, Resolved, Resolver};
use jira_client::JiraClient;
use requirement_import::{
    jira,
    strictdoc::{self, ExportItem},
};
use serde::Serialize;

use crate::jiracmd::acting_user;
use crate::refcmd::Context;

/// Every ID was found.
pub const EXIT_FOUND: i32 = 0;
/// At least one ID was not found.
pub const EXIT_NOT_FOUND: i32 = 1;
/// An error: unresolvable ID, missing `--input`, bad config, credential, export or network.
pub const EXIT_ERROR: i32 = 2;

/// Output format of `tcms requirements locate` (D-07).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Marked lines and a summary line
    Text,
    /// One JSON array of result objects
    Json,
    /// Bare result lines, no marker, no colour, no summary
    Plaintext,
}

#[derive(clap::Args)]
pub struct LocateArgs {
    /// Requirement IDs, for example PAY-2231 HLR-12
    #[arg(required = true, num_args = 1..)]
    pub ids: Vec<String>,
    /// StrictDoc IDs: local JSON export of the document set (assumed layout, see docs/JIRA_INTEGRATION.md); no URL
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
    /// Acting TCMS user for the Jira read (default: $TCMS_USER, then $USER)
    #[arg(long = "as")]
    pub actor: Option<String>,
}

/// One checked ID, in command-line order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocateResult {
    pub id: String,
    pub found: bool,
    /// `jira` or `strictdoc`: the family of the system the ID resolved to
    pub source: &'static str,
    /// `systems[].name` the ID resolved to
    pub system: String,
    /// StrictDoc level (`HLR`, `LLR`, `system`, ...); always null for Jira
    pub level: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,
    pub url: String,
}

/// IDs with repeats removed, keeping the first occurrence (a repeated ID is reported once).
pub(crate) fn dedup_ids(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// Issues returned by one batch search that match one of `ids` by exact key (D-03). Any issue
/// type counts (D-04). Returns `key -> (summary, status name)`; either part is `None` when the
/// issue does not carry it.
fn jira_matches(
    ids: &[&str],
    issues: &[serde_json::Value],
) -> BTreeMap<String, (Option<String>, Option<String>)> {
    let mut found = BTreeMap::new();
    for issue in issues {
        let Some(key) = issue["key"].as_str() else {
            continue;
        };
        if !ids.contains(&key) {
            continue;
        }
        let title = issue["fields"]["summary"].as_str().map(str::to_string);
        let status = issue["fields"]["status"]["name"]
            .as_str()
            .map(str::to_string);
        found.entry(key.to_string()).or_insert((title, status));
    }
    found
}

/// StrictDoc export items indexed by UID. A UID that appears twice makes the export ambiguous,
/// which is an input error (exit 2).
fn strictdoc_index(items: Vec<ExportItem>) -> Result<BTreeMap<String, ExportItem>> {
    let mut index = BTreeMap::new();
    for item in items {
        if index.contains_key(&item.uid) {
            bail!("StrictDoc export: UID {} appears more than once", item.uid);
        }
        index.insert(item.uid.clone(), item);
    }
    Ok(index)
}

/// One result line, without marker or colour (the output contract of plan 06-02).
pub(crate) fn render_line(r: &LocateResult) -> String {
    if !r.found {
        return format!("{}: not found in {} ({})", r.id, r.source, r.system);
    }
    let level = r
        .level
        .as_deref()
        .map(|l| format!(" at level {l}"))
        .unwrap_or_default();
    let status = r
        .status
        .as_deref()
        .map(|s| format!(" [{s}]"))
        .unwrap_or_default();
    format!(
        "{}: found in {} ({}){} \"{}\"{} {}",
        r.id,
        r.source,
        r.system,
        level,
        r.title.as_deref().unwrap_or(""),
        status,
        r.url
    )
}

/// Render the whole result in one format. `colour` applies to the text format only.
fn render(results: &[LocateResult], format: Format, colour: bool) -> String {
    match format {
        Format::Json => {
            let mut s = serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string());
            s.push('\n');
            s
        }
        Format::Plaintext => results
            .iter()
            .map(|r| format!("{}\n", render_line(r)))
            .collect(),
        Format::Text => {
            let mut out = String::new();
            for r in results {
                let line = render_line(r);
                let (mark, code) = if r.found {
                    ("✓", "32")
                } else {
                    ("✗", "31")
                };
                if colour {
                    out.push_str(&format!("\x1b[{code}m{mark}\x1b[0m {line}\n"));
                } else {
                    out.push_str(&format!("{mark} {line}\n"));
                }
            }
            let found = results.iter().filter(|r| r.found).count();
            out.push_str(&format!(
                "{found} of {} requirement IDs found\n",
                results.len()
            ));
            out
        }
    }
}

/// 0 when every ID was found, 1 when any was not found.
fn exit_code(results: &[LocateResult]) -> i32 {
    if results.iter().all(|r| r.found) {
        EXIT_FOUND
    } else {
        EXIT_NOT_FOUND
    }
}

/// Batch lookup shared by `locate` and `export-ids` (D-12). Nothing is printed here. Order: resolve
/// every ID (D-02), refuse a StrictDoc ID without `--input`, read the export once, build every Jira
/// client, then one batch search per Jira system (D-03, D-08). A failure at any step returns before
/// a result exists.
pub(crate) fn lookup(
    cfg: &IntegrationConfig,
    ids: &[String],
    input: Option<&Path>,
    actor: Option<String>,
) -> Result<Vec<LocateResult>> {
    let resolver = Resolver::new(cfg)?;

    // D-02: every ID resolves before any lookup is sent
    let mut resolved: Vec<(String, Resolved)> = Vec::new();
    for id in dedup_ids(ids) {
        let r = resolver.resolve_id(&id)?;
        if !(r.system_type.is_jira() || r.system_type.is_strictdoc()) {
            bail!(
                "{id}: system '{}' is not a requirement source (only Jira and StrictDoc are)",
                r.system
            );
        }
        resolved.push((id, r));
    }

    // D-06: a StrictDoc ID needs the export
    if let Some((id, r)) = resolved.iter().find(|(_, r)| r.system_type.is_strictdoc()) {
        if input.is_none() {
            bail!(
                "{id}: belongs to StrictDoc system '{}'; pass --input <export.json>",
                r.system
            );
        }
    }

    // D-05: the export is read once per run
    let index = match (
        input,
        resolved.iter().any(|(_, r)| r.system_type.is_strictdoc()),
    ) {
        (Some(path), true) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("read {}", path.display()))?;
            strictdoc_index(strictdoc::export_items(&text)?)?
        }
        _ => BTreeMap::new(),
    };

    // Jira IDs grouped by system, input order kept inside each group
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, r) in &resolved {
        if r.system_type.is_jira() {
            groups.entry(r.system.clone()).or_default().push(id.clone());
        }
    }

    // D-08: every client is built (token read, nothing sent) before the first search
    let actor = acting_user(actor);
    let mut clients: BTreeMap<String, JiraClient> = BTreeMap::new();
    for name in groups.keys() {
        let system = cfg
            .systems
            .iter()
            .find(|s| &s.name == name)
            .with_context(|| format!("system '{name}' is not in the configuration"))?;
        clients.insert(
            name.clone(),
            JiraClient::from_config(system, &cfg.sync, &actor)?,
        );
    }

    // D-03: one search per Jira system, matched back by key
    let mut found_jira: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
    for (name, keys) in &groups {
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let jql = jira::jql_for_keys(&refs)?;
        let client = clients
            .get(name)
            .with_context(|| format!("no client was built for system '{name}'"))?;
        let issues = client.search_issues(&jql, jira::LOCATE_FIELDS)?;
        found_jira.extend(jira_matches(&refs, &issues));
    }

    let results = resolved
        .into_iter()
        .map(|(id, r)| {
            if r.system_type.is_jira() {
                let hit = found_jira.get(&id);
                LocateResult {
                    found: hit.is_some(),
                    source: "jira",
                    system: r.system,
                    level: None,
                    title: hit.and_then(|h| h.0.clone()),
                    status: hit.and_then(|h| h.1.clone()),
                    url: r.url,
                    id,
                }
            } else {
                let hit = index.get(&id);
                LocateResult {
                    found: hit.is_some(),
                    source: "strictdoc",
                    system: r.system,
                    level: hit.map(|i| i.level.clone()),
                    title: hit.and_then(|i| i.title.clone()),
                    status: hit.and_then(|i| i.status.clone()),
                    url: r.url,
                    id,
                }
            }
        })
        .collect();
    Ok(results)
}

/// Perform the `locate` lookup for the parsed arguments (see [`lookup`]).
fn locate(ctx: &Context, args: &LocateArgs) -> Result<Vec<LocateResult>> {
    let cfg = ctx.load_config()?;
    lookup(&cfg, &args.ids, args.input.as_deref(), args.actor.clone())
}

/// Run `tcms requirements locate` and return the process exit code (0, 1 or 2).
pub fn run(ctx: &Context, args: LocateArgs) -> i32 {
    let results = match locate(ctx, &args) {
        Ok(results) => results,
        Err(e) => {
            eprintln!("error: {e:#}");
            return EXIT_ERROR;
        }
    };
    let colour = args.format == Format::Text
        && std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none();
    print!("{}", render(&results, args.format, colour));
    let code = exit_code(&results);
    if code == EXIT_NOT_FOUND {
        let missing = results.iter().filter(|r| !r.found).count();
        eprintln!("not found: {missing} of {} requirement IDs", results.len());
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn jira_hit(id: &str, title: Option<&str>, status: Option<&str>) -> LocateResult {
        LocateResult {
            id: id.into(),
            found: true,
            source: "jira",
            system: "jira".into(),
            level: None,
            title: title.map(Into::into),
            status: status.map(Into::into),
            url: format!("https://x/browse/{id}"),
        }
    }

    fn jira_miss(id: &str) -> LocateResult {
        LocateResult {
            id: id.into(),
            found: false,
            source: "jira",
            system: "jira".into(),
            level: None,
            title: None,
            status: None,
            url: format!("https://x/browse/{id}"),
        }
    }

    fn issue(
        key: &str,
        summary: Option<&str>,
        status: Option<&str>,
        kind: &str,
    ) -> serde_json::Value {
        let mut fields = json!({ "issuetype": { "name": kind } });
        if let Some(s) = summary {
            fields["summary"] = json!(s);
        }
        if let Some(s) = status {
            fields["status"] = json!({ "name": s });
        }
        json!({ "key": key, "fields": fields })
    }

    #[test]
    fn jira_matches_only_the_issue_whose_key_is_an_id() {
        let issues = vec![
            issue("PAY-2", Some("Pay by card"), Some("In progress"), "Feature"),
            issue("PAY-4", Some("Other"), Some("Done"), "Feature"),
        ];
        let found = jira_matches(&["PAY-2"], &issues);
        assert_eq!(found.len(), 1);
        assert_eq!(
            found.get("PAY-2"),
            Some(&(Some("Pay by card".into()), Some("In progress".into())))
        );
        assert!(!found.contains_key("PAY-4"));
    }

    #[test]
    fn jira_matches_is_empty_when_no_key_matches() {
        let issues = vec![issue("PAY-2", Some("Pay"), None, "Feature")];
        assert!(jira_matches(&["PAY-9"], &issues).is_empty());
    }

    #[test]
    fn jira_matches_keeps_any_issue_type() {
        let issues = vec![issue("PAY-7", Some("Crash"), Some("Open"), "Bug")];
        assert!(jira_matches(&["PAY-7"], &issues).contains_key("PAY-7"));
    }

    #[test]
    fn jira_match_without_status_or_summary_has_none() {
        let issues = vec![issue("PAY-3", None, None, "Story")];
        assert_eq!(
            jira_matches(&["PAY-3"], &issues).get("PAY-3"),
            Some(&(None, None))
        );
    }

    #[test]
    fn strictdoc_index_refuses_a_repeated_uid() {
        let item = |uid: &str| ExportItem {
            uid: uid.into(),
            level: "HLR".into(),
            title: None,
            status: None,
            path: "reqs.sdoc".into(),
        };
        let err = strictdoc_index(vec![item("HLR-1"), item("HLR-1")]).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("HLR-1"), "{text}");
        assert!(text.contains("more than once"), "{text}");
    }

    #[test]
    fn strictdoc_index_keeps_each_uid() {
        let item = |uid: &str| ExportItem {
            uid: uid.into(),
            level: "LLR".into(),
            title: Some("t".into()),
            status: None,
            path: "p".into(),
        };
        let index = strictdoc_index(vec![item("LLR-1"), item("LLR-2")]).unwrap();
        assert_eq!(index.len(), 2);
        assert_eq!(index["LLR-2"].level, "LLR");
    }

    #[test]
    fn render_line_for_a_found_jira_issue() {
        let r = jira_hit("PAY-2", Some("Pay by card"), Some("In progress"));
        assert_eq!(
            render_line(&r),
            "PAY-2: found in jira (jira) \"Pay by card\" [In progress] https://x/browse/PAY-2"
        );
    }

    #[test]
    fn render_line_for_a_found_strictdoc_item_without_status() {
        let r = LocateResult {
            id: "SYS-4".into(),
            found: true,
            source: "strictdoc",
            system: "strictdoc".into(),
            level: Some("system".into()),
            title: Some("Remote profile management".into()),
            status: None,
            url: "https://p/system-requirements.html#SYS-4".into(),
        };
        assert_eq!(
            render_line(&r),
            "SYS-4: found in strictdoc (strictdoc) at level system \"Remote profile management\" https://p/system-requirements.html#SYS-4"
        );
    }

    #[test]
    fn render_line_for_a_missing_id() {
        assert_eq!(
            render_line(&jira_miss("PAY-99")),
            "PAY-99: not found in jira (jira)"
        );
    }

    #[test]
    fn plaintext_is_bare_lines_only() {
        let results = vec![jira_hit("PAY-2", Some("P"), None), jira_miss("PAY-9")];
        let text = render(&results, Format::Plaintext, false);
        assert_eq!(
            text,
            "PAY-2: found in jira (jira) \"P\" https://x/browse/PAY-2\nPAY-9: not found in jira (jira)\n"
        );
        assert!(!text.contains('✓') && !text.contains('✗') && !text.contains("\x1b["));
    }

    #[test]
    fn text_has_markers_and_summary() {
        let results = vec![jira_hit("PAY-2", Some("P"), None), jira_miss("PAY-9")];
        let text = render(&results, Format::Text, false);
        assert_eq!(
            text,
            "✓ PAY-2: found in jira (jira) \"P\" https://x/browse/PAY-2\n\
             ✗ PAY-9: not found in jira (jira)\n\
             1 of 2 requirement IDs found\n"
        );
    }

    #[test]
    fn text_colours_found_green_and_missing_red_when_asked() {
        let results = vec![jira_hit("PAY-2", Some("P"), None), jira_miss("PAY-9")];
        let text = render(&results, Format::Text, true);
        assert!(text.contains("\x1b[32m"));
        assert!(text.contains("\x1b[31m"));
    }

    #[test]
    fn json_is_one_array_of_objects() {
        let results = vec![jira_hit("PAY-2", Some("P"), None), jira_miss("PAY-9")];
        let text = render(&results, Format::Json, false);
        assert!(text.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let array = value.as_array().unwrap();
        assert_eq!(array.len(), 2);
        assert_eq!(array[0]["id"], "PAY-2");
        assert_eq!(array[0]["found"], true);
        assert_eq!(array[0]["source"], "jira");
        assert_eq!(array[1]["found"], false);
        assert_eq!(array[1]["title"], serde_json::Value::Null);
        assert_eq!(array[1]["level"], serde_json::Value::Null);
    }

    #[test]
    fn exit_codes_follow_found_state() {
        assert_eq!(exit_code(&[jira_hit("PAY-2", None, None)]), EXIT_FOUND);
        assert_eq!(
            exit_code(&[jira_hit("PAY-2", None, None), jira_miss("PAY-9")]),
            EXIT_NOT_FOUND
        );
        assert_eq!(EXIT_ERROR, 2);
    }

    #[test]
    fn dedup_keeps_the_first_occurrence() {
        let ids: Vec<String> = ["A", "B", "A"].iter().map(|s| s.to_string()).collect();
        assert_eq!(dedup_ids(&ids), vec!["A".to_string(), "B".to_string()]);
    }
}
