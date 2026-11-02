//! Machine-readable output of the traceability report: JSON and YAML (OUT-02, D-03, D-04).
//!
//! Each function builds the document with `build_document`, checks it with `validate_document`
//! against the schema it is given, and returns text only when that check passed. Pure: no file
//! reads, no clock, no environment. The CLI loads the schema from `schemas/` (plan 04-04).

use crate::model::ReportModel;
use crate::report_document::{build_document, validate_document};
use serde_json::Value;
use std::fmt;

/// Why no text was returned. A failed check never leaves partial output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputError {
    /// The document violated the schema, or the schema does not compile. One entry per violation.
    Schema(Vec<String>),
    /// The serialiser failed. Not reachable with the current document types.
    Serialize(String),
}

impl fmt::Display for OutputError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

impl std::error::Error for OutputError {}

/// The report as pretty JSON, one trailing newline. Keys follow the struct declaration order.
pub fn render_json(model: &ReportModel, schema: &Value) -> Result<String, OutputError> {
    let _ = (model, schema);
    Err(OutputError::Serialize(
        "render_json is not implemented yet".to_string(),
    ))
}

/// The report as YAML. The checked value is parsed back from the returned text.
pub fn render_yaml(model: &ReportModel, schema: &Value) -> Result<String, OutputError> {
    let _ = (model, schema);
    Err(OutputError::Serialize(
        "render_yaml is not implemented yet".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, RequirementDocuments};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::select_results;
    use serde_json::json;
    use std::collections::BTreeMap;
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    /// The report schema, embedded at compile time so this module has no file reads.
    fn schema() -> Value {
        serde_json::from_str(include_str!("../../../schemas/tcms/report.schema.v1.json"))
            .expect("report schema is JSON")
    }

    fn link(id: &str, level: RequirementLevel, cases: &[&str]) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: true,
            robustness_covered: false,
            run_ids: Vec::new(),
        }
    }

    /// Two requirements in two documents, one run with a pass and a fail. Built through the
    /// real pipeline: loaded runs -> selection -> model.
    fn small_model() -> ReportModel {
        let mut rec = record(
            "RUN-1",
            "BUILD-1",
            "2026-11-01T10:00:00Z",
            &[("TC-1", Verdict::Pass), ("TC-2", Verdict::Fail)],
        );
        rec.environment = "ENV-1".to_string();
        rec.tester = "TESTER-1".to_string();
        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-1".into(),
                baseline_id: None,
                links: vec![
                    link("REQ-1", RequirementLevel::Hlr, &["TC-1", "TC-2"]),
                    link("REQ-2", RequirementLevel::Llr, &["TC-2"]),
                ],
            },
            runs: vec![LoadedRun::new("RUN-1.yaml".into(), rec).expect("valid run")],
            reconciliation: Reconciliation::default(),
        };
        let selection = select_results(&inputs.runs);
        let documents = RequirementDocuments {
            documents: vec!["docs/a.sdoc".into(), "docs/b.sdoc".into()],
            requirement_document: BTreeMap::from([
                ("REQ-1".to_string(), "docs/a.sdoc".to_string()),
                ("REQ-2".to_string(), "docs/b.sdoc".to_string()),
            ]),
        };
        build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: None,
            documents: Some(&documents),
            generated_at: "2026-11-01T12:00:00Z",
        })
    }

    /// A schema that the report cannot satisfy: the top level requires a property it lacks.
    fn requires_missing_field() -> Value {
        json!({ "type": "object", "required": ["nonexistent_field"] })
    }

    fn assert_schema_error_without_text(result: Result<String, OutputError>) {
        match result {
            Ok(text) => panic!("expected no text, got: {text}"),
            Err(OutputError::Schema(messages)) => {
                assert!(!messages.is_empty(), "no violation messages");
                assert!(
                    messages.iter().any(|m| m.contains("nonexistent_field")),
                    "{messages:#?}"
                );
            }
            Err(other) => panic!("expected a schema error, got: {other:?}"),
        }
    }

    #[test]
    fn json_text_starts_with_brace_ends_with_one_newline_and_keeps_key_order() {
        let text = render_json(&small_model(), &schema()).expect("valid document renders");
        assert!(text.starts_with('{'), "{text}");
        assert!(text.ends_with("}\n"), "{text}");
        assert!(!text.ends_with("\n\n"), "{text}");
        let keys = ["\"type\"", "\"schema\"", "\"kind\"", "\"header\"", "\"content\""];
        let positions: Vec<usize> = keys
            .iter()
            .map(|k| text.find(k).unwrap_or_else(|| panic!("{k} missing")))
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "keys out of order: {positions:?}"
        );
    }

    #[test]
    fn json_text_parses_back_to_a_document_that_validates() {
        let text = render_json(&small_model(), &schema()).expect("valid document renders");
        let parsed: Value = serde_json::from_str(&text).expect("JSON text parses");
        assert_eq!(validate_document(&parsed, &schema()), Ok(()));
    }

    #[test]
    fn yaml_first_line_is_type_report_and_text_ends_with_one_newline() {
        let text = render_yaml(&small_model(), &schema()).expect("valid document renders");
        assert_eq!(text.lines().next(), Some("type: report"), "{text}");
        assert!(text.ends_with('\n') && !text.ends_with("\n\n"), "{text}");
    }

    #[test]
    fn yaml_and_json_of_one_model_parse_to_equal_values() {
        let model = small_model();
        let json_text = render_json(&model, &schema()).expect("JSON renders");
        let yaml_text = render_yaml(&model, &schema()).expect("YAML renders");
        let from_json: Value = serde_json::from_str(&json_text).expect("JSON parses");
        let from_yaml: Value = serde_yaml::from_str(&yaml_text).expect("YAML parses");
        assert_eq!(validate_document(&from_yaml, &schema()), Ok(()));
        assert_eq!(from_yaml, from_json);
    }

    #[test]
    fn json_schema_violation_gives_error_with_message_and_no_text() {
        assert_schema_error_without_text(render_json(&small_model(), &requires_missing_field()));
    }

    #[test]
    fn yaml_schema_violation_gives_error_with_message_and_no_text() {
        assert_schema_error_without_text(render_yaml(&small_model(), &requires_missing_field()));
    }

    #[test]
    fn schema_that_does_not_compile_gives_schema_error_starting_with_compile_message() {
        let bad = json!({ "type": "not-a-type" });
        for result in [
            render_json(&small_model(), &bad),
            render_yaml(&small_model(), &bad),
        ] {
            match result {
                Ok(text) => panic!("expected no text, got: {text}"),
                Err(OutputError::Schema(messages)) => assert!(
                    messages
                        .first()
                        .is_some_and(|m| m.starts_with("report schema does not compile")),
                    "{messages:#?}"
                ),
                Err(other) => panic!("expected a schema error, got: {other:?}"),
            }
        }
    }

    #[test]
    fn schema_error_display_has_header_line_then_one_bullet_per_violation() {
        let err = OutputError::Schema(vec!["/a: first".to_string(), "/b: second".to_string()]);
        assert_eq!(
            err.to_string(),
            "report failed validation against tcms/report.schema.v1.json\n  - /a: first\n  - /b: second"
        );
    }

    #[test]
    fn serialize_error_display_names_the_failure() {
        let err = OutputError::Serialize("boom".to_string());
        assert_eq!(err.to_string(), "report could not be serialised: boom");
    }

    #[test]
    fn output_error_implements_std_error() {
        fn assert_error<E: std::error::Error>(_: &E) {}
        assert_error(&OutputError::Serialize(String::new()));
    }
}
