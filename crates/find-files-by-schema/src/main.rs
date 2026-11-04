//! Command line for `find-files-by-schema`: list the repo files that declare a tcms-schemas type.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

use find_files_by_schema::{render_jsonl, render_plaintext, scan, Catalog};

#[derive(Parser, Debug)]
#[command(
    name = "find-files-by-schema",
    version,
    about = "Find the YAML and JSON files in a repo that declare a tcms-schemas type"
)]
struct Cli {
    /// Directory to scan (recursively, honouring .gitignore)
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Schemas root that contains tcms-schemas/tcms/ (the vendored schemas)
    #[arg(long = "schemas-root", default_value = "schemas")]
    schemas_root: PathBuf,

    /// Only report this type, for example `test-run` (repeatable)
    #[arg(long = "only", value_name = "TYPE")]
    only: Vec<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = Output::Plaintext)]
    output: Output,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Output {
    /// One human-readable line per match (default)
    Plaintext,
    /// One JSON object per match, one per line
    Jsonl,
}

fn run(cli: &Cli) -> anyhow::Result<()> {
    let catalog = Catalog::load(&cli.schemas_root)?;
    catalog.validate_only(&cli.only)?;
    let report = scan(&cli.root, &catalog, &cli.only)?;
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    let stdout = match cli.output {
        Output::Plaintext => render_plaintext(&report.matches),
        Output::Jsonl => render_jsonl(&report.matches)?,
    };
    print!("{stdout}");
    Ok(())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}
