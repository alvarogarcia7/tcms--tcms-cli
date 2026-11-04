//! Command line for `find-files-by-schema`: list the repo files that declare a tcms-schemas type.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};

use find_files_by_schema::{render_jsonl, render_plaintext, scan, Catalog, ScanOptions};

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

    /// Validate each matched document against the schema it declares; exit 3 if any fails
    #[arg(long)]
    verify: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Output {
    /// One human-readable line per match (default)
    Plaintext,
    /// One JSON object per match, one per line
    Jsonl,
}

/// Exit code when `--verify` finds at least one document that fails its schema.
const EXIT_VERIFICATION_FAILED: u8 = 3;

fn run(cli: &Cli) -> anyhow::Result<ExitCode> {
    let catalog = Catalog::load(&cli.schemas_root)?;
    catalog.validate_only(&cli.only)?;
    let options = ScanOptions {
        only: cli.only.clone(),
        verify: cli.verify,
    };
    let report = scan(&cli.root, &catalog, &options)?;
    for warning in &report.warnings {
        eprintln!("warning: {warning}");
    }
    let stdout = match cli.output {
        Output::Plaintext => render_plaintext(&report.matches),
        Output::Jsonl => render_jsonl(&report.matches)?,
    };
    print!("{stdout}");
    let failures = report.verification_failures();
    if failures > 0 {
        eprintln!("error: {failures} document(s) failed verification");
        return Ok(ExitCode::from(EXIT_VERIFICATION_FAILED));
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}
