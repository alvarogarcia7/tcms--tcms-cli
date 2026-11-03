//! Renders the V&V metrics document written by `tcms vv metrics` as a Markdown or AsciiDoc report
//! (TCMS-47.1). The crate computes no figure of its own and does not depend on the tcms crate.

use std::fmt;

/// Output format of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// GitHub-flavoured Markdown tables.
    #[value(name = "markdown")]
    Markdown,
    /// AsciiDoc tables.
    #[value(name = "asciidoc")]
    AsciiDoc,
}

/// The metrics document is not valid or lacks a documented field. The message names the field.
#[derive(Debug)]
pub struct InputError(String);

impl InputError {
    pub(crate) fn new(message: String) -> Self {
        InputError(message)
    }
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InputError {}

mod asciidoc;
pub mod input;
mod markdown;
mod report;

pub use input::{parse, Metrics};

/// Renders the metrics JSON text as a report in the given format.
pub fn render(metrics: &str, format: Format) -> Result<String, InputError> {
    let parsed = parse(metrics)?;
    Ok(render_parsed(&parsed, format))
}

/// Renders an already parsed document.
pub fn render_parsed(metrics: &Metrics, format: Format) -> String {
    let blocks = report::blocks(metrics);
    match format {
        Format::Markdown => markdown::render(&blocks),
        Format::AsciiDoc => asciidoc::render(&blocks),
    }
}
