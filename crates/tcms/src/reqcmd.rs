//! `tcms requirements import`: write requirement YAML from the configured requirement source.

use std::path::PathBuf;

use anyhow::{bail, Context as _, Result};
use clap::Subcommand;
use jira_client::JiraClient;
use requirement_import::{jira, strictdoc, write_requirements, RequirementRecord};

use crate::jiracmd::acting_user;
use crate::refcmd::Context;

#[derive(Subcommand)]
pub enum RequirementsCommand {
    /// Import requirements from the source named by integration-config.requirement_source
    Import {
        /// Directory for the requirement YAML files (one `<id>.yaml` each)
        #[arg(long, default_value = "requirements")]
        out: PathBuf,
        /// StrictDoc only: JSON export of the document set (assumed layout, see docs/JIRA_INTEGRATION.md)
        #[arg(long)]
        input: Option<PathBuf>,
        /// StrictDoc only: commit the export was taken at; recorded as each requirement's version
        #[arg(long)]
        commit: Option<String>,
        /// Jira only: label that marks a safety feature (sets safety_related)
        #[arg(long)]
        safety_label: Option<String>,
        /// Jira only: label that marks a security feature (sets security_related)
        #[arg(long)]
        security_label: Option<String>,
        /// Acting TCMS user for the Jira read (default: $TCMS_USER, then $USER)
        #[arg(long = "as")]
        actor: Option<String>,
    },
}

pub fn run(ctx: &Context, cmd: RequirementsCommand) -> Result<()> {
    match cmd {
        RequirementsCommand::Import {
            out,
            input,
            commit,
            safety_label,
            security_label,
            actor,
        } => {
            // A config naming both sources is refused here, at load time
            let cfg = ctx.load_config()?;
            let system = cfg.source_system()?;
            let records: Vec<RequirementRecord> = match cfg.requirement_source {
                id_resolver::RequirementSource::Jira => {
                    if input.is_some() || commit.is_some() {
                        bail!("--input and --commit apply to a StrictDoc source; this product reads Jira");
                    }
                    let flags = jira::LabelFlags {
                        safety: safety_label.as_deref(),
                        security: security_label.as_deref(),
                    };
                    let client = JiraClient::from_config(system, &cfg.sync, &acting_user(actor))?;
                    jira::fetch(&client, system, flags)?
                }
                id_resolver::RequirementSource::Strictdoc => {
                    if safety_label.is_some() || security_label.is_some() {
                        bail!("--safety-label and --security-label apply to a Jira source; this product reads StrictDoc");
                    }
                    let input =
                        input.context("the StrictDoc source needs --input <export.json>")?;
                    let commit = commit.context(
                        "the StrictDoc source needs --commit <sha> (the commit of the export)",
                    )?;
                    let text = std::fs::read_to_string(&input)
                        .with_context(|| format!("read {}", input.display()))?;
                    strictdoc::parse_export(&text, &commit)?
                }
            };

            let report = write_requirements(&out, &records, ctx.schemas_root())?;
            println!(
                "requirements from {} ({}): {} created, {} updated, {} unchanged, {} not in source",
                cfg.requirement_source.as_str(),
                system.name,
                report.created.len(),
                report.updated.len(),
                report.unchanged.len(),
                report.stale.len()
            );
            for id in &report.stale {
                println!("  not in source, kept: {id}");
            }
            println!("written to {}", out.display());
            Ok(())
        }
    }
}
