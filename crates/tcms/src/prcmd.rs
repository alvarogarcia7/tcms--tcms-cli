//! `tcms pr push` and `tcms pr link` (TCMS-42.8): a problem report becomes a Jira defect, or is
//! linked to an existing issue. The Jira key is written to the problem report's own
//! `external_refs`. Verified against the jira-client mock only.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use clap::Subcommand;
use jira_client::JiraClient;
use serde_yaml::Value;
use test_backends::defects::{self, ProblemReport, PushOutcome};
use test_backends::{Ctx, JiraApi};

use crate::jiracmd::{acting_user, pick_jira_system};
use crate::refcmd::Context;

#[derive(Subcommand)]
pub enum PrCommand {
    /// Create the Jira defect of a problem report (linked to its features and, with --test-entity,
    /// to a test entity). Writes the key to the problem report unless --dry-run.
    Push {
        /// Problem report id (file `<problem-reports>/<ID>.yaml`)
        id: String,
        /// Directory of problem report records
        #[arg(long, default_value = "problem-reports")]
        problem_reports: PathBuf,
        /// Jira key of the test entity the defect is linked to
        #[arg(long)]
        test_entity: Option<String>,
        /// Jira system name (needed when the config has several)
        #[arg(long)]
        system: Option<String>,
        /// Acting TCMS user (default: $TCMS_USER, then $USER)
        #[arg(long = "as")]
        actor: Option<String>,
        /// Print the plan; send nothing, write nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Link an existing Jira issue to a problem report and record its key
    Link {
        /// Problem report id
        id: String,
        /// Jira issue key, for example PAY-42
        key: String,
        #[arg(long, default_value = "problem-reports")]
        problem_reports: PathBuf,
        #[arg(long)]
        system: Option<String>,
        #[arg(long = "as")]
        actor: Option<String>,
    },
}

pub fn run(ctx: &Context, cmd: PrCommand) -> Result<()> {
    match cmd {
        PrCommand::Push {
            id,
            problem_reports,
            test_entity,
            system,
            actor,
            dry_run,
        } => {
            let cfg = ctx.load_config()?;
            let sys = pick_jira_system(&cfg.systems, system.as_deref())?;
            let actor = acting_user(actor);
            let path = record_path(&problem_reports, &id);
            let pr = load_problem_report(&path, ctx.schemas_root(), &id)?;
            let mut jira = jira_ctx(sys, &cfg.sync, &actor)?;
            let out = defects::push(&mut jira, &pr, test_entity.as_deref(), &actor, dry_run)?;
            match out {
                PushOutcome::AlreadyLinked { key } => {
                    println!("{id}: already linked to {key}; nothing sent");
                }
                PushOutcome::Found { key } => {
                    if dry_run {
                        println!(
                            "dry run: {key} already carries this report's label; nothing created"
                        );
                    } else {
                        record_jira_key(&path, &key, &sys.base_url)?;
                        println!("{id}: found {key} by label; nothing created");
                        println!("key written to {}", path.display());
                    }
                }
                PushOutcome::Planned {
                    summary,
                    links,
                    notes,
                } => {
                    println!(
                        "dry run: would create {} '{summary}'",
                        sys.defect_issue_type.as_deref().unwrap_or("?")
                    );
                    for l in &links {
                        println!("  would link {l}");
                    }
                    for n in &notes {
                        println!("  note: {n}");
                    }
                }
                PushOutcome::Created { key, links, notes } => {
                    record_jira_key(&path, &key, &sys.base_url)?;
                    println!("{id}: created {key}");
                    for l in &links {
                        println!("  linked {l}");
                    }
                    for n in &notes {
                        println!("  note: {n}");
                    }
                    println!("key written to {}", path.display());
                }
            }
            Ok(())
        }
        PrCommand::Link {
            id,
            key,
            problem_reports,
            system,
            actor,
        } => {
            let cfg = ctx.load_config()?;
            let sys = pick_jira_system(&cfg.systems, system.as_deref())?;
            let actor = acting_user(actor);
            let path = record_path(&problem_reports, &id);
            let pr = load_problem_report(&path, ctx.schemas_root(), &id)?;
            if let Some(existing) = &pr.external_key {
                if existing != &key {
                    bail!("{id} is already linked to {existing}; it is not re-linked to {key}");
                }
                println!("{id}: already linked to {key}");
                return Ok(());
            }
            let mut jira = jira_ctx(sys, &cfg.sync, &actor)?;
            let state = defects::link_existing(&mut jira, &key)?;
            record_jira_key(&path, &state.key, &sys.base_url)?;
            println!("{id}: linked {} (status '{}')", state.key, state.status);
            println!("key written to {}", path.display());
            Ok(())
        }
    }
}

fn record_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.yaml"))
}

/// The Jira context of a write: one client for the acting user, the guard from the config.
fn jira_ctx(
    sys: &id_resolver::SystemConfig,
    sync: &id_resolver::config::SyncConfig,
    actor: &str,
) -> Result<Ctx> {
    let client = JiraClient::from_config(sys, sync, actor)?;
    Ok(Ctx {
        api: Box::new(JiraApi::new(client)),
        system: sys.clone(),
        overwrites: Vec::new(),
    })
}

/// Read a problem report, validated against its schema, and the fields the defect needs.
pub(crate) fn load_problem_report(
    path: &Path,
    schemas_root: &str,
    id: &str,
) -> Result<ProblemReport> {
    let v: Value = testcase_common::load_and_validate_yaml(path, schemas_root)
        .with_context(|| format!("problem report {}", path.display()))?;
    let text = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let rec_id = text("id");
    if rec_id != id {
        bail!(
            "{}: record id '{rec_id}' does not match '{id}'",
            path.display()
        );
    }
    let feature_keys = v
        .get("affected_items")
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter(|i| i.get("entity_type").and_then(Value::as_str) == Some("requirement"))
                .filter_map(|i| {
                    i.get("entity_id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    let external_key = v
        .get("external_refs")
        .and_then(Value::as_sequence)
        .and_then(|refs| {
            refs.iter()
                .find(|r| r.get("system").and_then(Value::as_str) == Some("jira"))
        })
        .and_then(|r| r.get("key").and_then(Value::as_str).map(str::to_string));
    Ok(ProblemReport {
        id: rec_id,
        title: text("title"),
        description: text("description"),
        severity: v
            .get("severity")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: text("status"),
        raised_by: text("raised_by"),
        feature_keys,
        external_key,
    })
}

/// Write the Jira key into the record's `external_refs` (the Jira entry is replaced, others kept).
pub(crate) fn record_jira_key(path: &Path, key: &str, base_url: &str) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut v: Value = serde_yaml::from_str(&text).context("parse problem report")?;
    let url = format!("{}/browse/{key}", base_url.trim_end_matches('/'));
    let synced_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let entry = serde_yaml::to_value(serde_json::json!({
        "system": "jira", "key": key, "url": url, "synced_at": synced_at
    }))?;
    let map = v
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("{}: not a mapping", path.display()))?;
    let refs_key = Value::String("external_refs".into());
    let mut refs: Vec<Value> = map
        .get(&refs_key)
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();
    refs.retain(|r| r.get("system").and_then(Value::as_str) != Some("jira"));
    refs.push(entry);
    map.insert(refs_key, Value::Sequence(refs));
    let out = testcase_common::to_yaml_string(&v)?;
    std::fs::write(path, out).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
