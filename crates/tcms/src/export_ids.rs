//! `tcms requirements export-ids`: read the product requirements file, check every ID with the
//! `locate` lookup, write the IDs the source finds. Writes only the `--output` file.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::locate::{self, LocateResult};
use crate::refcmd::Context;

#[derive(clap::Args)]
pub struct ExportIdsArgs {
    /// Product requirements file: {"product_id": ..., "requirements": [{"id": ...}]} (default: ./requirements.json)
    #[arg(long, default_value = "requirements.json")]
    pub requirements: PathBuf,
    /// Write the valid IDs here instead of stdout
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// StrictDoc IDs: local JSON export of the document set (layout of `strictdoc export --formats json`, see docs/JIRA_INTEGRATION.md); no URL
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// Acting TCMS user for the Jira read (default: $TCMS_USER, then $USER)
    #[arg(long = "as")]
    pub actor: Option<String>,
}

/// One requirement of the product requirements file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementEntry {
    pub id: String,
}

/// The product requirements file (D-11) and the output shape of export-ids (D-13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementsFile {
    pub product_id: String,
    pub requirements: Vec<RequirementEntry>,
}

/// Parse the requirements file text. Unknown fields and a missing `requirements` array are errors.
fn parse_requirements(text: &str, path: &Path) -> Result<RequirementsFile> {
    serde_json::from_str(text)
        .with_context(|| format!("parse requirements file {}", path.display()))
}

/// The file must name the configured product and list at least one requirement (D-11).
fn check_file(file: &RequirementsFile, configured: &str, path: &Path) -> Result<()> {
    if file.product_id != configured {
        bail!(
            "requirements file {}: product_id '{}' does not match product_id '{}' of the integration config",
            path.display(),
            file.product_id,
            configured
        );
    }
    if file.requirements.is_empty() {
        bail!(
            "requirements file {}: 'requirements' is empty",
            path.display()
        );
    }
    Ok(())
}

/// The IDs of the file in file order. Repeats are removed later by the lookup.
fn file_ids(file: &RequirementsFile) -> Vec<String> {
    file.requirements.iter().map(|r| r.id.clone()).collect()
}

/// The valid IDs in result order as the output JSON, with a trailing newline (D-13).
fn render_output(product_id: &str, results: &[LocateResult]) -> Result<String> {
    let file = RequirementsFile {
        product_id: product_id.to_string(),
        requirements: results
            .iter()
            .filter(|r| r.found)
            .map(|r| RequirementEntry { id: r.id.clone() })
            .collect(),
    };
    let mut s = serde_json::to_string_pretty(&file).context("serialise export-ids output")?;
    s.push('\n');
    Ok(s)
}

/// One stderr line per invalid ID in result order, then the count line. Empty when all are valid.
fn invalid_lines(results: &[LocateResult]) -> Vec<String> {
    let missing = results.iter().filter(|r| !r.found).count();
    if missing == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = results
        .iter()
        .filter(|r| !r.found)
        .map(|r| format!("invalid: {}", locate::render_line(r)))
        .collect();
    lines.push(format!(
        "invalid: {missing} of {} requirement IDs",
        results.len()
    ));
    lines
}

/// Read, check and look up the requirements file. Nothing is printed or written here (D-11, D-12).
fn export_ids(ctx: &Context, args: &ExportIdsArgs) -> Result<(String, Vec<LocateResult>)> {
    let cfg = ctx.load_config()?;
    let text = std::fs::read_to_string(&args.requirements)
        .with_context(|| format!("read requirements file {}", args.requirements.display()))?;
    let file = parse_requirements(&text, &args.requirements)?;
    check_file(&file, &cfg.product_id, &args.requirements)?;
    let results = locate::lookup(
        &cfg,
        &file_ids(&file),
        args.input.as_deref(),
        args.actor.clone(),
    )?;
    let json = render_output(&cfg.product_id, &results)?;
    Ok((json, results))
}

/// Run `tcms requirements export-ids` and return the process exit code (0, 1 or 2). Valid IDs are
/// written also when some ID is invalid (exit 1).
pub fn run(ctx: &Context, args: ExportIdsArgs) -> i32 {
    let (json, results) = match export_ids(ctx, &args) {
        Ok(out) => out,
        Err(e) => {
            eprintln!("error: {e:#}");
            return locate::EXIT_ERROR;
        }
    };
    match &args.output {
        Some(path) => {
            if let Err(e) =
                std::fs::write(path, &json).with_context(|| format!("write {}", path.display()))
            {
                eprintln!("error: {e:#}");
                return locate::EXIT_ERROR;
            }
        }
        None => {
            use std::io::Write as _;
            let mut out = std::io::stdout().lock();
            // `json` already ends in one newline; writeln adds it back after trimming, so the bytes match.
            let written =
                writeln!(out, "{}", json.trim_end_matches('\n')).and_then(|()| out.flush());
            if let Err(e) = written {
                eprintln!("error: write stdout: {e}");
                return locate::EXIT_ERROR;
            }
        }
    }
    for line in invalid_lines(&results) {
        eprintln!("{line}");
    }
    if results.iter().any(|r| !r.found) {
        locate::EXIT_NOT_FOUND
    } else {
        locate::EXIT_FOUND
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p() -> PathBuf {
        PathBuf::from("req.json")
    }

    fn file(product: &str, ids: &[&str]) -> RequirementsFile {
        RequirementsFile {
            product_id: product.into(),
            requirements: ids
                .iter()
                .map(|id| RequirementEntry { id: (*id).into() })
                .collect(),
        }
    }

    fn found(id: &str) -> LocateResult {
        LocateResult {
            id: id.into(),
            found: true,
            source: "jira",
            system: "jira".into(),
            level: None,
            title: Some("t".into()),
            status: None,
            url: format!("https://x/browse/{id}"),
        }
    }

    fn missing(id: &str) -> LocateResult {
        LocateResult {
            id: id.into(),
            found: false,
            source: "jira",
            system: "jira".into(),
            level: None,
            title: None,
            status: None,
            url: format!("https://x/browse/{id}"),
        }
    }

    #[test]
    fn parses_the_file_shape() {
        let text = r#"{"product_id":"PROD-001","requirements":[{"id":"PAY-1"},{"id":"PAY-2"}]}"#;
        assert_eq!(
            parse_requirements(text, &p()).unwrap(),
            file("PROD-001", &["PAY-1", "PAY-2"])
        );
    }

    #[test]
    fn unparseable_text_names_the_path() {
        let err = parse_requirements("{not json", &p()).unwrap_err();
        let text = format!("{err:#}");
        assert!(text.contains("parse requirements file"), "{text}");
        assert!(text.contains("req.json"), "{text}");
    }

    #[test]
    fn missing_requirements_array_is_a_parse_error() {
        let err = parse_requirements(r#"{"product_id":"PROD-001"}"#, &p()).unwrap_err();
        assert!(format!("{err:#}").contains("parse requirements file"));
    }

    #[test]
    fn unknown_field_inside_a_requirement_is_refused() {
        let text = r#"{"product_id":"PROD-001","requirements":[{"id":"PAY-1","title":"x"}]}"#;
        let err = parse_requirements(text, &p()).unwrap_err();
        assert!(format!("{err:#}").contains("title"), "{err:#}");
    }

    #[test]
    fn unknown_top_level_field_is_refused() {
        let text = r#"{"product_id":"PROD-001","requirements":[],"extra":1}"#;
        assert!(parse_requirements(text, &p()).is_err());
    }

    #[test]
    fn product_mismatch_is_refused() {
        let err = check_file(&file("PROD-999", &["PAY-1"]), "PROD-001", &p()).unwrap_err();
        assert!(
            format!("{err:#}").contains(
                "product_id 'PROD-999' does not match product_id 'PROD-001' of the integration config"
            ),
            "{err:#}"
        );
    }

    #[test]
    fn empty_requirements_is_refused() {
        let err = check_file(&file("PROD-001", &[]), "PROD-001", &p()).unwrap_err();
        assert!(
            format!("{err:#}").contains("'requirements' is empty"),
            "{err:#}"
        );
    }

    #[test]
    fn matching_product_with_ids_is_accepted() {
        assert!(check_file(&file("PROD-001", &["PAY-1"]), "PROD-001", &p()).is_ok());
    }

    #[test]
    fn file_ids_keep_order_and_repeats() {
        assert_eq!(
            file_ids(&file("PROD-001", &["PAY-2", "PAY-99", "PAY-2"])),
            vec![
                "PAY-2".to_string(),
                "PAY-99".to_string(),
                "PAY-2".to_string()
            ]
        );
    }

    #[test]
    fn output_keeps_only_found_ids_in_order() {
        let text = render_output(
            "PROD-001",
            &[found("PAY-2"), missing("PAY-99"), found("PAY-3")],
        )
        .unwrap();
        assert!(text.ends_with('\n'));
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"product_id":"PROD-001","requirements":[{"id":"PAY-2"},{"id":"PAY-3"}]})
        );
        assert!(text.find("product_id").unwrap() < text.find("requirements").unwrap());
    }

    #[test]
    fn output_with_no_valid_ids_is_an_empty_list() {
        let text = render_output("PROD-001", &[]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value,
            serde_json::json!({"product_id":"PROD-001","requirements":[]})
        );
    }

    #[test]
    fn invalid_lines_list_each_missing_id_and_the_count() {
        assert_eq!(
            invalid_lines(&[found("PAY-2"), missing("PAY-99")]),
            vec![
                "invalid: PAY-99: not found in jira (jira)".to_string(),
                "invalid: 1 of 2 requirement IDs".to_string(),
            ]
        );
    }

    #[test]
    fn invalid_lines_are_empty_when_all_are_valid() {
        assert!(invalid_lines(&[found("PAY-2")]).is_empty());
    }
}
