use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use tcms_vv_report::Format;

const AFTER_HELP: &str = "\
Exit codes:
  0  Report written. Also for a FAILED verdict and for NO_DATA, unless --fail-on-gate is given.
  1  Error: unreadable file, invalid JSON, or a documented field missing (the message names it).
  2  --fail-on-gate and the verdict is FAILED. The report is written before the exit.

Null rendering:
  Gate pass rate: NO_DATA with no percentage when the run has no counted results.
  Delta from previous run: n/a when there is no previous run or either denominator is 0.
  V&V pass rate and step coverage: n/a when their total is 0.

Input: the JSON written by `tcms vv metrics --out`. The report computes no figure of its own.";

/// Render the V&V metrics of `tcms vv metrics` as a Markdown or AsciiDoc report (TCMS-47.1).
#[derive(Parser, Debug)]
#[command(name = "tcms-vv-report", version, about, after_help = AFTER_HELP)]
struct Cli {
    /// Metrics JSON written by `tcms vv metrics`.
    #[arg(long, value_name = "FILE")]
    metrics: PathBuf,

    /// Report format: markdown or asciidoc.
    #[arg(long, value_enum, default_value = "markdown")]
    format: Format,

    /// Write the report to FILE, creating parent directories. Default: stdout.
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,

    /// Exit with status 2 when the gate verdict is FAILED, after the report is written.
    #[arg(long)]
    fail_on_gate: bool,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("tcms-vv-report: error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let text = fs::read_to_string(&cli.metrics)
        .with_context(|| format!("reading {}", cli.metrics.display()))?;
    let metrics = tcms_vv_report::parse(&text)?;
    let report = tcms_vv_report::render_parsed(&metrics, cli.format);

    match &cli.out {
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            fs::write(path, &report).with_context(|| format!("writing {}", path.display()))?;
        }
        None => {
            io::stdout()
                .write_all(report.as_bytes())
                .context("writing report to stdout")?;
        }
    }

    if cli.fail_on_gate && metrics.gate.verdict == "FAILED" {
        return Ok(ExitCode::from(2));
    }
    Ok(ExitCode::SUCCESS)
}
