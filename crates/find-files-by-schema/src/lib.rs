//! Find the YAML and JSON files of a repository that declare a tcms-schemas type (TCMS-49).
//!
//! A file matches when its top-level `schema:` URI resolves to a vendored tcms-schemas file
//! under `<schemas-root>/tcms-schemas/tcms/`. The versioned aliases (for example
//! `tcms/test-case.schema.v1.json`) resolve through `testcase_common::envelope::vendored_alias`.
//! The declared `type:` value is reported alongside the match.

use std::collections::HashMap;
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
    /// `<schemas-root>/tcms-schemas`; `SchemaType::file` is relative to it.
    vendored_root: PathBuf,
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
        Ok(Self {
            types,
            vendored_root: schemas_root.join(VENDORED_SCHEMAS_DIR),
        })
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
    /// Set only with `--verify`: whether the content validates against the declared schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    /// Set only with `--verify`: one `<pointer>: <message>` per violation (empty when valid).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
}

/// Options of a scan.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Restrict matches to these type names; empty matches every type.
    pub only: Vec<String>,
    /// Validate each matched document's content against its declared schema.
    pub verify: bool,
}

/// The result of a scan: the matches (sorted by path) and warnings for skipped files.
#[derive(Debug, Default)]
pub struct Report {
    pub matches: Vec<Match>,
    pub warnings: Vec<String>,
}

impl Report {
    /// Number of matches whose content failed verification (always 0 without `--verify`).
    pub fn verification_failures(&self) -> usize {
        self.matches
            .iter()
            .filter(|m| m.verified == Some(false))
            .count()
    }
}

/// Scan `root` for files whose `schema:` resolves to a type in `catalog`.
///
/// `options.only` restricts matches to the named types. With `options.verify`, the content of
/// each match is validated against its declared schema. Files that are ignored by `.gitignore`
/// are not scanned, even outside a git repository.
pub fn scan(root: &Path, catalog: &Catalog, options: &ScanOptions) -> Result<Report> {
    if !root.is_dir() {
        bail!("scan root {} is not a directory", root.display());
    }
    let mut report = Report::default();
    let mut compiled: HashMap<String, jsonschema::JSONSchema> = HashMap::new();
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
        if !options.only.is_empty() && !options.only.contains(&schema_type.name) {
            continue;
        }
        let (verified, errors) = if options.verify {
            if !compiled.contains_key(&schema_type.file) {
                let schema = compile_schema(&catalog.vendored_root, &schema_type.file)?;
                compiled.insert(schema_type.file.clone(), schema);
            }
            let errors = validation_errors(&compiled[&schema_type.file], &document);
            (Some(errors.is_empty()), Some(errors))
        } else {
            (None, None)
        };
        report.matches.push(Match {
            path: relative_display(root, path),
            schema: schema_type.file.clone(),
            doc_type: document
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string),
            verified,
            errors,
        });
    }
    report.matches.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

/// Compile the schema file `file` (relative to `vendored_root`).
fn compile_schema(vendored_root: &Path, file: &str) -> Result<jsonschema::JSONSchema> {
    let path = vendored_root.join(file);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("cannot read schema {}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .with_context(|| format!("schema {} is not JSON", path.display()))?;
    jsonschema::JSONSchema::compile(&value)
        .map_err(|err| anyhow::anyhow!("cannot compile schema {}: {err}", path.display()))
}

/// Violations of `document` against `schema`, one `<pointer>: <message>` each.
fn validation_errors(schema: &jsonschema::JSONSchema, document: &Value) -> Vec<String> {
    match schema.validate(document) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|err| {
                let pointer = err.instance_path.to_string();
                let pointer = if pointer.is_empty() {
                    "(root)".to_string()
                } else {
                    pointer
                };
                format!("{pointer}: {err}")
            })
            .collect(),
    }
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
            "{}  type: {}  schema: {}",
            m.path,
            m.doc_type.as_deref().unwrap_or("-"),
            m.schema
        ));
        match (m.verified, &m.errors) {
            (Some(true), _) => out.push_str("  verified: ok"),
            (Some(false), Some(errors)) => {
                out.push_str(&format!("  verified: FAILED ({} errors)", errors.len()));
                for error in errors {
                    out.push_str(&format!("\n    {error}"));
                }
            }
            _ => {}
        }
        out.push('\n');
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
