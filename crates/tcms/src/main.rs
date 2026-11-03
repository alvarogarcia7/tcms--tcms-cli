//! `tcms`: command line for external references and the Jira integration.

mod export_ids;
mod extrefs;
mod jiracmd;
mod locate;
mod prcmd;
mod refcmd;
mod reqcmd;
mod synccmd;
mod vvcmd;
mod vvcombine;
mod vvconfig;
mod vvmetrics;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tcms", version, about = "TCMS command line")]
struct Cli {
    /// Integration config (default: $TCMS_INTEGRATION_CONFIG or ./integration-config.yaml)
    #[arg(long, global = true)]
    config: Option<std::path::PathBuf>,

    /// Directory holding tcms-schemas/ (default: $TCMS_SCHEMAS_ROOT or ./schemas/)
    #[arg(long, global = true)]
    schemas_root: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Resolve, open and fill external references (Jira and StrictDoc ids and URLs)
    Ref {
        #[command(subcommand)]
        command: refcmd::RefCommand,
    },
    /// Jira integration: connection test (TCMS-42.2)
    Jira {
        #[command(subcommand)]
        command: jiracmd::JiraCommand,
    },
    /// Problem reports: create the Jira defect or link an existing issue (TCMS-42.8)
    Pr {
        #[command(subcommand)]
        command: prcmd::PrCommand,
    },
    /// Requirements: import from the configured source, or locate IDs in Jira or a StrictDoc export
    Requirements {
        #[command(subcommand)]
        command: reqcmd::RequirementsCommand,
    },
    /// V&V metrics: pass rate, quality-gate verdict, coverage and stability of recorded runs (TCMS-47)
    Vv {
        #[command(subcommand)]
        command: vvcmd::VvCommand,
    },
}

fn main() {
    let cli = Cli::parse();
    // Every error exits 1 except the codes a command reports itself (`requirements locate`: 0, 1, 2)
    match run(cli) {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
    }
}

fn run(cli: Cli) -> Result<i32> {
    let ctx = refcmd::Context::new(cli.config, cli.schemas_root);
    match cli.command {
        Command::Ref { command } => refcmd::run(&ctx, command).map(|()| 0),
        Command::Jira { command } => jiracmd::run(&ctx, command).map(|()| 0),
        Command::Requirements { command } => reqcmd::run(&ctx, command),
        Command::Pr { command } => prcmd::run(&ctx, command).map(|()| 0),
        Command::Vv { command } => vvcmd::run(&ctx, command).map(|()| 0),
    }
}
