//! `tcms requirements export-ids`: read the product requirements file, check every ID with the
//! `locate` lookup, write the IDs the source finds. Writes only the `--output` file.

#![cfg_attr(not(test), allow(dead_code))]

use std::path::Path;

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::locate::{self, LocateResult};

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
        bail!("requirements file {}: 'requirements' is empty", path.display());
    }
    Ok(())
}

/// The IDs of the file in file order. Repeats are removed later by the lookup.
fn file_ids(file: &RequirementsFile) -> Vec<String> {
    file.requirements.iter().map(|r| r.id.clone()).collect()
}

/// The valid IDs in result order as the output JSON, with a trailing newline (D-13).
fn render_output(product_id: &str, results: &[LocateResult]) -> String {
    let file = RequirementsFile {
        product_id: product_id.to_string(),
        requirements: results
            .iter()
            .filter(|r| r.found)
            .map(|r| RequirementEntry { id: r.id.clone() })
            .collect(),
    };
    let mut s = serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".to_string());
    s.push('\n');
    s
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
    lines.push(format!("invalid: {missing} of {} requirement IDs", results.len()));
    lines
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
        assert!(format!("{err:#}").contains("'requirements' is empty"), "{err:#}");
    }

    #[test]
    fn matching_product_with_ids_is_accepted() {
        assert!(check_file(&file("PROD-001", &["PAY-1"]), "PROD-001", &p()).is_ok());
    }

    #[test]
    fn file_ids_keep_order_and_repeats() {
        assert_eq!(
            file_ids(&file("PROD-001", &["PAY-2", "PAY-99", "PAY-2"])),
            vec!["PAY-2".to_string(), "PAY-99".to_string(), "PAY-2".to_string()]
        );
    }

    #[test]
    fn output_keeps_only_found_ids_in_order() {
        let text = render_output(
            "PROD-001",
            &[found("PAY-2"), missing("PAY-99"), found("PAY-3")],
        );
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
        let text = render_output("PROD-001", &[]);
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
