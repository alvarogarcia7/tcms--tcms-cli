//! `tcms ref resolve|open|fill`.

use std::path::PathBuf;

use anyhow::{bail, Context as _, Result};
use clap::Subcommand;
use id_resolver::{fill_yaml_value, IntegrationConfig, Resolver};

/// Config and schema locations shared by all subcommands.
pub struct Context {
    config: Option<PathBuf>,
    schemas_root: String,
}

impl Context {
    pub fn new(config: Option<PathBuf>, schemas_root: Option<String>) -> Self {
        Self {
            config,
            schemas_root: schemas_root
                .or_else(|| std::env::var("TCMS_SCHEMAS_ROOT").ok())
                .unwrap_or_else(|| "schemas/".to_string()),
        }
    }

    pub fn schemas_root(&self) -> &str {
        &self.schemas_root
    }

    pub fn config_path(&self) -> PathBuf {
        self.config
            .clone()
            .or_else(|| std::env::var_os("TCMS_INTEGRATION_CONFIG").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("integration-config.yaml"))
    }

    pub fn load_config(&self) -> Result<IntegrationConfig> {
        IntegrationConfig::load(self.config_path(), &self.schemas_root)
    }
}

#[derive(Subcommand)]
pub enum RefCommand {
    /// Print system, id and URL for an id or a URL
    Resolve {
        /// A Jira or StrictDoc id (PAY-2231, HLR-12) or a URL of a configured system
        id_or_url: String,
    },
    /// Open the page of an id in the browser (xdg-open / open)
    Open {
        id: String,
        /// Print the URL instead of opening a browser (also: TCMS_OPEN_PRINT=1)
        #[arg(long)]
        print_only: bool,
    },
    /// Fill empty external_refs[].url of a record from the config and print the result
    Fill {
        file: PathBuf,
        /// Write the result back into the file (URLs are derived, so by default they are not stored)
        #[arg(long)]
        write: bool,
    },
}

pub fn run(ctx: &Context, cmd: RefCommand) -> Result<()> {
    let resolver = Resolver::new(&ctx.load_config()?)?;
    match cmd {
        RefCommand::Resolve { id_or_url } => {
            let r = resolver.resolve(&id_or_url)?;
            println!("system: {}", r.system);
            println!("id:     {}", r.id);
            println!("url:    {}", r.url);
        }
        RefCommand::Open { id, print_only } => {
            let r = resolver.resolve(&id)?;
            let print = print_only
                || std::env::var("TCMS_OPEN_PRINT").is_ok_and(|v| !v.is_empty() && v != "0");
            if print {
                println!("{}", r.url);
            } else {
                open_in_browser(&r.url)?;
            }
        }
        RefCommand::Fill { file, write } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read {}", file.display()))?;
            let mut value: serde_yaml::Value = serde_yaml::from_str(&text)
                .with_context(|| format!("Failed to parse {}", file.display()))?;
            let n = fill_yaml_value(&resolver, &mut value)?;
            let out = testcase_common::to_yaml_string(&value)?;
            if write {
                std::fs::write(&file, &out)?;
                eprintln!("{n} url(s) filled in {}", file.display());
            } else {
                print!("{out}");
                eprintln!("{n} url(s) filled (file unchanged; use --write to store them)");
            }
        }
    }
    Ok(())
}

fn open_in_browser(url: &str) -> Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(opener)
        .arg(url)
        .status()
        .with_context(|| format!("Failed to start '{opener}'; use --print-only to get the URL"))?;
    if !status.success() {
        bail!("'{opener}' exited with {status}");
    }
    Ok(())
}
