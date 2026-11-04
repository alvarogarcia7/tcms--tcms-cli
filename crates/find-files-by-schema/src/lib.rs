//! Find the YAML and JSON files of a repository that declare a tcms-schemas type (TCMS-49).
//!
//! A file matches when its top-level `schema:` URI resolves to a vendored tcms-schemas file
//! under `<schemas-root>/tcms-schemas/tcms/`. The versioned aliases (for example
//! `tcms/test-case.schema.v1.json`) resolve through `testcase_common::envelope::vendored_alias`.
//! The declared `type:` value is reported alongside the match.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use testcase_common::envelope::{vendored_alias, VENDORED_SCHEMAS_DIR};

/// Suffix of a vendored schema file name, stripped to get the short type name.
const SCHEMA_SUFFIX: &str = ".schema.json";

/// Directory of the vendored schemas below the schemas root.
const TCMS_DIR: &str = "tcms";

/// File extensions that are scanned.
const SCANNED_EXTENSIONS: [&str; 3] = ["yaml", "yml", "json"];

/// One tcms-schemas type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaType {
    /// Short name, for example `test-run`. Used by `--only`.
    pub name: String,
    /// Schema file relative to the vendored directory, for example `tcms/test-run.schema.json`.
    pub file: String,
}

/// The tcms-schemas types found in a schemas root.
#[derive(Debug, Clone)]
pub struct Catalog {
    types: Vec<SchemaType>,
}

impl Catalog {
    /// Read the vendored types from `<schemas_root>/tcms-schemas/tcms/`.
    pub fn load(schemas_root: &Path) -> Result<Self> {
        let dir = schemas_root.join(VENDORED_SCHEMAS_DIR).join(TCMS_DIR);
        let entries = fs::read_dir(&dir).with_context(|| {
            format!(
                "cannot read tcms-schemas types from {} (use --schemas-root)",
                dir.display()
            )
        })?;
        let mut types = Vec::new();
        for entry in entries {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if let Some(name) = file_name.strip_suffix(SCHEMA_SUFFIX) {
                types.push(SchemaType {
                    name: name.to_string(),
                    file: format!("{TCMS_DIR}/{file_name}"),
                });
            }
        }
        types.sort_by(|a, b| a.name.cmp(&b.name));
        if types.is_empty() {
            bail!("no tcms-schemas types found in {}", dir.display());
        }
        Ok(Self { types })
    }

    /// All types, sorted by name.
    pub fn types(&self) -> &[SchemaType] {
        &self.types
    }

    /// The type a `schema:` URI declares, if it is a tcms-schemas type.
    pub fn resolve(&self, schema_uri: &str) -> Option<&SchemaType> {
        let file = vendored_alias(schema_uri).unwrap_or(schema_uri);
        self.types.iter().find(|t| t.file == file)
    }

    /// Check that every `--only` name is a known type; the error lists the valid names.
    pub fn validate_only(&self, only: &[String]) -> Result<()> {
        for name in only {
            if !self.types.iter().any(|t| &t.name == name) {
                let valid: Vec<&str> = self.types.iter().map(|t| t.name.as_str()).collect();
                bail!(
                    "unknown type '{name}' for --only; valid types: {}",
                    valid.join(", ")
                );
            }
        }
        Ok(())
    }
}

/// One file that declares a tcms-schemas type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Match {
    /// Path relative to the scan root, with `/` separators.
    pub path: String,
    /// Schema file relative to the vendored directory, for example `tcms/test-run.schema.json`.
    pub schema: String,
    /// Declared `type:` value, or `None` when absent.
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
}

/// The result of a scan: the matches (sorted by path) and warnings for skipped files.
#[derive(Debug, Default)]
pub struct Report {
    pub matches: Vec<Match>,
    pub warnings: Vec<String>,
}

/// Scan `root` for files whose `schema:` resolves to a type in `catalog`.
///
/// `only` restricts matches to the named types; an empty slice matches every type.
/// Files that are ignored by `.gitignore` are not scanned, even outside a git repository.
pub fn scan(root: &Path, catalog: &Catalog, only: &[String]) -> Result<Report> {
    if !root.is_dir() {
        bail!("scan root {} is not a directory", root.display());
    }
    let mut report = Report::default();
    let walker = ignore::WalkBuilder::new(root).require_git(false).build();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.warnings.push(format!("skipping entry: {err}"));
                continue;
            }
        };
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !SCANNED_EXTENSIONS.contains(&ext) {
            continue;
        }
        let document = match parse_document(path, ext) {
            Ok(document) => document,
            Err(err) => {
                report
                    .warnings
                    .push(format!("skipping {}: {err:#}", path.display()));
                continue;
            }
        };
        let Some(schema_uri) = document.get("schema").and_then(Value::as_str) else {
            continue;
        };
        let Some(schema_type) = catalog.resolve(schema_uri) else {
            continue;
        };
        if !only.is_empty() && !only.contains(&schema_type.name) {
            continue;
        }
        report.matches.push(Match {
            path: relative_display(root, path),
            schema: schema_type.file.clone(),
            doc_type: document
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    report.matches.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

fn parse_document(path: &Path, ext: &str) -> Result<Value> {
    let text = fs::read_to_string(path).context("cannot read file")?;
    if ext == "json" {
        serde_json::from_str(&text).context("invalid JSON")
    } else {
        serde_yaml::from_str(&text).context("invalid YAML")
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    let relative: PathBuf = path.strip_prefix(root).unwrap_or(path).to_path_buf();
    relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Human-readable output: one line per match.
pub fn render_plaintext(matches: &[Match]) -> String {
    let mut out = String::new();
    for m in matches {
        out.push_str(&format!(
            "{}  type: {}  schema: {}\n",
            m.path,
            m.doc_type.as_deref().unwrap_or("-"),
            m.schema
        ));
    }
    out
}

/// Machine-readable output: one JSON object per match, one per line.
pub fn render_jsonl(matches: &[Match]) -> Result<String> {
    let mut out = String::new();
    for m in matches {
        out.push_str(&serde_json::to_string(m)?);
        out.push('\n');
    }
    Ok(out)
}
