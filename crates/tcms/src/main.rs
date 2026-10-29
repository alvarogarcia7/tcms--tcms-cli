//! `tcms`: command line for external references and the Jira integration.

mod extrefs;
mod jiracmd;
mod prcmd;
mod refcmd;
mod reqcmd;
mod synccmd;

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
    /// Requirements: import from the configured source (Jira features or StrictDoc export)
    Requirements {
        #[command(subcommand)]
        command: reqcmd::RequirementsCommand,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let ctx = refcmd::Context::new(cli.config, cli.schemas_root);
    match cli.command {
        Command::Ref { command } => refcmd::run(&ctx, command),
        Command::Jira { command } => jiracmd::run(&ctx, command),
        Command::Requirements { command } => reqcmd::run(&ctx, command),
        Command::Pr { command } => prcmd::run(&ctx, command),
    }
}
