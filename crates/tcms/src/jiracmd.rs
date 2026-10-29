//! `tcms jira test-connection`: authenticate against a configured Jira system and report it.

use anyhow::{bail, Result};
use clap::Subcommand;
use id_resolver::SystemConfig;
use jira_client::JiraClient;

use crate::refcmd::Context;

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum JiraCommand {
    /// Authenticate with the service account and report site, user and the projects in project_keys.
    /// Exits non-zero when a project of project_keys is not accessible.
    TestConnection {
        /// Jira system name in the integration config (needed when the config has several)
        #[arg(long)]
        system: Option<String>,
        /// Acting TCMS user recorded with writes (default: $TCMS_USER, then $USER)
        #[arg(long = "as")]
        actor: Option<String>,
    },
    /// Sync the product's write-backs: requirements, test status, roll-up, milestones, defects
    /// (TCMS-42.9). Dry run unless --apply.
    Sync(crate::synccmd::SyncArgs),
}

pub fn run(ctx: &Context, cmd: JiraCommand) -> Result<()> {
    match cmd {
        JiraCommand::Sync(args) => crate::synccmd::run(ctx, args),
        JiraCommand::TestConnection { system, actor } => {
            let cfg = ctx.load_config()?;
            let sys = pick_jira_system(&cfg.systems, system.as_deref())?;
            let actor = acting_user(actor);
            let client = JiraClient::from_config(sys, &cfg.sync, &actor)?;
            let report = client.test_connection()?;

            println!("system:     {}", sys.name);
            println!("site:       {}", report.site);
            println!("deployment: {}", report.deployment);
            if let Some(v) = &report.version {
                println!("version:    {v}");
            }
            println!("user:       {}", report.user);
            println!("projects:");
            for p in &report.projects {
                println!(
                    "  {:<10} {:<24} {}",
                    p.key,
                    p.name.as_deref().unwrap_or("-"),
                    if p.accessible {
                        "accessible"
                    } else {
                        "NOT ACCESSIBLE"
                    }
                );
            }
            let missing: Vec<&str> = report
                .projects
                .iter()
                .filter(|p| !p.accessible)
                .map(|p| p.key.as_str())
                .collect();
            if !missing.is_empty() {
                bail!(
                    "projects not accessible to the service account: {}",
                    missing.join(", ")
                );
            }
            Ok(())
        }
    }
}

/// The acting TCMS user: the `--as` value, else `$TCMS_USER`, else `$USER`, else `unknown`.
pub fn acting_user(arg: Option<String>) -> String {
    arg.or_else(|| std::env::var("TCMS_USER").ok().filter(|v| !v.is_empty()))
        .or_else(|| std::env::var("USER").ok().filter(|v| !v.is_empty()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// The Jira system to use: the named one, or the only Jira system of the config.
pub fn pick_jira_system<'a>(
    systems: &'a [SystemConfig],
    name: Option<&str>,
) -> Result<&'a SystemConfig> {
    let jira: Vec<&SystemConfig> = systems.iter().filter(|s| s.system_type.is_jira()).collect();
    match (name, jira.as_slice()) {
        (Some(n), _) => {
            jira.iter().copied().find(|s| s.name == n).ok_or_else(|| {
                anyhow::anyhow!("no Jira system named '{n}' in the integration config")
            })
        }
        (None, [one]) => Ok(one),
        (None, []) => {
            bail!("the integration config has no Jira system (jira_cloud or jira_datacenter)")
        }
        (None, many) => bail!(
            "the integration config has several Jira systems ({}); choose one with --system",
            many.iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
