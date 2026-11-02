//! Machine-readable output of the traceability report: JSON and YAML (OUT-02, D-03, D-04).
//!
//! Each function builds the document with `build_document`, checks it with `validate_document`
//! against the schema it is given, and returns text only when that check passed. Pure: no file
//! reads, no clock, no environment. The CLI loads the schema from `schemas/` (plan 04-04).

use crate::model::ReportModel;
use crate::report_document::{build_document, validate_document, ReportDocument};
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputError::Schema(messages) => {
                write!(
                    f,
                    "report failed validation against tcms/report.schema.v1.json"
                )?;
                for message in messages {
                    write!(f, "\n  - {message}")?;
                }
                Ok(())
            }
            OutputError::Serialize(message) => {
                write!(f, "report could not be serialised: {message}")
            }
        }
    }
}

impl std::error::Error for OutputError {}

/// The report as pretty JSON, one trailing newline. Keys follow the struct declaration order.
pub fn render_json(model: &ReportModel, schema: &Value) -> Result<String, OutputError> {
    let document = build_document(model);
    let value =
        serde_json::to_value(&document).map_err(|e| OutputError::Serialize(e.to_string()))?;
    check(&value, schema)?;
    let mut text = serde_json::to_string_pretty(&document)
        .map_err(|e| OutputError::Serialize(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// The report as YAML. The checked value is parsed back from the returned text.
pub fn render_yaml(model: &ReportModel, schema: &Value) -> Result<String, OutputError> {
    let document = build_document(model);
    let text =
        serde_yaml::to_string(&document).map_err(|e| OutputError::Serialize(e.to_string()))?;
    // Check what would be written: parse the returned text back and validate that value.
    let parsed: Value = serde_yaml::from_str(&text)
        .map_err(|e| OutputError::Serialize(format!("parse back: {e}")))?;
    check(&parsed, schema)?;
    Ok(text)
}

/// One or more report documents as JSON, in the given order. One document gives the text of
/// `render_json`; several give one pretty JSON array of the documents. Every document is checked
/// before any text is returned; each message starts with the kind of its document.
pub fn render_documents_json(
    documents: &[ReportDocument],
    schema: &Value,
) -> Result<String, OutputError> {
    let values = documents
        .iter()
        .map(|d| serde_json::to_value(d).map_err(|e| OutputError::Serialize(e.to_string())))
        .collect::<Result<Vec<Value>, OutputError>>()?;
    check_all(&values, schema)?;
    let mut text = if documents.len() == 1 {
        serde_json::to_string_pretty(&documents[0])
    } else {
        serde_json::to_string_pretty(documents)
    }
    .map_err(|e| OutputError::Serialize(e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// One or more report documents as YAML, in the given order. One document gives the text of
/// `render_yaml`; several give the texts joined by `---` lines. Each text is parsed back and the
/// parsed value is checked before any text is returned.
pub fn render_documents_yaml(
    documents: &[ReportDocument],
    schema: &Value,
) -> Result<String, OutputError> {
    let mut texts = Vec::with_capacity(documents.len());
    let mut values = Vec::with_capacity(documents.len());
    for document in documents {
        let text =
            serde_yaml::to_string(document).map_err(|e| OutputError::Serialize(e.to_string()))?;
        let parsed: Value = serde_yaml::from_str(&text)
            .map_err(|e| OutputError::Serialize(format!("parse back: {e}")))?;
        values.push(parsed);
        texts.push(text);
    }
    check_all(&values, schema)?;
    Ok(texts.join("---\n"))
}

/// Check one report document against the schema it is given (04-05). Returns no text: the CLI
/// calls this before any byte is written, for every format and every shape. A serialisation
/// failure is `OutputError::Serialize`; a violation is `OutputError::Schema` with every message.
pub fn check_document(document: &ReportDocument, schema: &Value) -> Result<(), OutputError> {
    let value =
        serde_json::to_value(document).map_err(|e| OutputError::Serialize(e.to_string()))?;
    check(&value, schema)
}

/// Validate one instance; a failed check becomes `OutputError::Schema` with every message.
fn check(instance: &Value, schema: &Value) -> Result<(), OutputError> {
    validate_document(instance, schema).map_err(OutputError::Schema)
}

/// Validate every document; the messages of each document start with its kind.
fn check_all(values: &[Value], schema: &Value) -> Result<(), OutputError> {
    let mut messages = Vec::new();
    for value in values {
        let kind = value["kind"].as_str().unwrap_or("unknown");
        if let Err(found) = validate_document(value, schema) {
            messages.extend(found.into_iter().map(|m| format!("{kind}: {m}")));
        }
    }
    if messages.is_empty() {
        Ok(())
    } else {
        Err(OutputError::Schema(messages))
    }
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
    pub(super) fn schema() -> Value {
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
    pub(super) fn small_model() -> ReportModel {
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
        let keys = [
            "\"type\"",
            "\"schema\"",
            "\"kind\"",
            "\"header\"",
            "\"content\"",
        ];
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

#[cfg(test)]
mod round_trip_tests {
    use super::tests::{schema, small_model};
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{build_model, ReportInputs, RequirementDocuments};
    use crate::report_document::build_document;
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::select_results;
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    const GENERATED: &str = "2026-11-01T12:00:00Z";

    /// Hostile strings, all distinct. Markup, YAML and JSON special values, line breaks, a bell
    /// character, a single space, non-ASCII text, a 2000 character string and the AsciiDoc
    /// strings of plan 04-01. Index 27 is one space: it is used only in fields that keep blank
    /// text (requirement and test case IDs), because the header drops blank run, build,
    /// environment and tester values by design.
    fn hostile_values() -> Vec<String> {
        let mut values: Vec<String> = [
            "A|B",
            "line one\nline two\rthree",
            "say \"hi\"",
            "back\\slash",
            "\\\"",
            ": colon first",
            "- dash first",
            "# not a comment",
            "{brace}",
            "[bracket]",
            "*star",
            "&anchor",
            "!tag",
            "> folded",
            "| literal",
            "%percent",
            "@at",
            "null",
            "true",
            "yes",
            "no",
            "~",
            "0123",
            "1e3",
            "0x1F",
            "2026-10-31",
            " leading and trailing ",
            " ",
            "Ünïcode 日本語 🙂",
            "bell\u{7}char",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        values.push("abcdefghij".repeat(200));
        for asciidoc in [
            "*bold* and _it_",
            "`code`",
            "[[anchor]]",
            "<<xref>>",
            "{attr}",
            "link:http://example.com[click]",
            "+++<b>raw</b>+++",
            "include::/etc/passwd[]",
            "image::x.png[]",
            "----",
            "|===",
            "== Fake heading",
            ".Title",
            "NOTE: x",
            "ifdef::x[]",
            ":attr: value",
            "a\\|b",
            "x\ny",
        ] {
            values.push(asciidoc.to_string());
        }
        values
    }

    fn link(
        id: &str,
        level: RequirementLevel,
        cases: Vec<String>,
        normal: bool,
        robust: bool,
    ) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level,
            test_case_ids: cases,
            normal_range_covered: normal,
            robustness_covered: robust,
            run_ids: Vec::new(),
        }
    }

    fn run(
        run_id: &str,
        build_id: &str,
        started: &str,
        env: &str,
        tester: &str,
        results: &[(String, Verdict)],
    ) -> LoadedRun {
        let refs: Vec<(&str, Verdict)> = results.iter().map(|(id, v)| (id.as_str(), *v)).collect();
        let mut rec = record(run_id, build_id, started, &refs);
        rec.environment = env.to_string();
        rec.tester = tester.to_string();
        LoadedRun::new(format!("{run_id}.yaml").into(), rec).expect("valid run")
    }

    /// Every hostile string in every field a document can carry, built through the real pipeline
    /// (loaded runs -> selection -> model). With `reverse`, links, their test case IDs, runs and
    /// document names are given in reverse order; the output must not change.
    fn hostile_model(reverse: bool, generated_at: &str) -> ReportModel {
        let hv = hostile_values();
        let n = hv.len();
        let mut links: Vec<TraceLink> = (0..n)
            .map(|i| {
                link(
                    &hv[i],
                    if i % 2 == 0 {
                        RequirementLevel::Hlr
                    } else {
                        RequirementLevel::Llr
                    },
                    vec![hv[(i + 1) % n].clone(), hv[(i + 2) % n].clone()],
                    i % 2 == 0,
                    i % 3 == 0,
                )
            })
            .collect();

        let run_ids = [hv[12].clone(), hv[13].clone(), hv[20].clone()];
        let build_ids = [hv[14].clone(), hv[15].clone(), hv[16].clone()];
        let envs = [hv[17].clone(), hv[18].clone(), hv[19].clone()];
        let testers = [hv[3].clone(), hv[4].clone(), hv[5].clone()];
        let starts = [
            "2026-10-28T10:00:00Z",
            "2026-10-28T11:00:00Z",
            "2026-10-28T12:00:00Z",
        ];
        // Test case k is decided by run k % 3, with a verdict by k % 4. Test case 20 has no
        // result, so every case that is 20 shows NOT RUN.
        let mut runs: Vec<LoadedRun> = (0..3)
            .map(|r| {
                let results: Vec<(String, Verdict)> = (0..n)
                    .filter(|&k| k % 3 == r && k != 20)
                    .map(|k| {
                        let verdict = match k % 4 {
                            1 => Verdict::Fail,
                            2 => Verdict::Error,
                            _ => Verdict::Pass,
                        };
                        (hv[k].clone(), verdict)
                    })
                    .collect();
                run(
                    &run_ids[r],
                    &build_ids[r],
                    starts[r],
                    &envs[r],
                    &testers[r],
                    &results,
                )
            })
            .collect();

        let docs3 = [hv[14].clone(), hv[17].clone(), hv[0].clone()];
        let mut document_names = vec![
            hv[14].clone(),
            hv[17].clone(),
            hv[0].clone(),
            hv[19].clone(),
        ];
        let mut requirement_document: BTreeMap<String, String> = (0..n)
            .filter(|i| i % 4 != 3)
            .map(|i| (hv[i].clone(), docs3[i % 3].clone()))
            .collect();

        if reverse {
            links.reverse();
            for l in links.iter_mut() {
                l.test_case_ids.reverse();
            }
            runs.reverse();
            document_names.reverse();
            requirement_document = requirement_document.into_iter().rev().collect();
        }

        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: hv[1].clone(),
                baseline_id: Some(hv[2].clone()),
                links,
            },
            runs,
            reconciliation: Reconciliation::default(),
        };
        let selection = select_results(&inputs.runs);
        let documents = RequirementDocuments {
            documents: document_names,
            requirement_document,
        };
        build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: None,
            documents: Some(&documents),
            generated_at,
        })
    }

    fn collect_strings(value: &Value, out: &mut BTreeSet<String>) {
        match value {
            Value::String(s) => {
                out.insert(s.clone());
            }
            Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
            Value::Object(map) => map.values().for_each(|v| collect_strings(v, out)),
            _ => {}
        }
    }

    /// (documents, requirements, cases) of a parsed document.
    fn parsed_counts(value: &Value) -> (usize, usize, usize) {
        let documents = value["content"]["documents"]
            .as_array()
            .expect("documents array");
        let mut requirements = 0;
        let mut cases = 0;
        for document in documents {
            for requirement in document["requirements"].as_array().expect("requirements") {
                requirements += 1;
                cases += requirement["cases"].as_array().expect("cases").len();
            }
        }
        (documents.len(), requirements, cases)
    }

    /// (documents, requirements, cases) of the model, counted from the model itself.
    fn model_counts(model: &ReportModel) -> (usize, usize, usize) {
        let requirements: usize = model.documents.iter().map(|d| d.rows.len()).sum();
        let cases: usize = model
            .documents
            .iter()
            .flat_map(|d| &d.rows)
            .map(|r| r.cases.len())
            .sum();
        (model.documents.len(), requirements, cases)
    }

    /// Checks shared by both formats: the parsed document validates, equals the built document,
    /// has the model's structure, and holds every hostile string at its place.
    fn assert_hostile_round_trip(text: &str, parsed: &Value, model: &ReportModel) {
        assert_eq!(validate_document(parsed, &schema()), Ok(()), "{text}");
        let built = serde_json::to_value(build_document(model)).expect("document serialises");
        assert_eq!(parsed, &built);
        assert_eq!(parsed_counts(parsed), model_counts(model));

        let hv = hostile_values();
        let mut strings = BTreeSet::new();
        collect_strings(parsed, &mut strings);
        for value in &hv {
            assert!(
                strings.contains(value),
                "{value:?} did not come back as a string"
            );
        }
        assert_eq!(parsed["header"]["matrix_id"], json!(hv[1]));
        assert_eq!(parsed["header"]["baseline_id"], json!(hv[2]));
        let environments = parsed["header"]["environments"]
            .as_array()
            .expect("environments");
        for env in [&hv[17], &hv[18], &hv[19]] {
            assert!(
                environments.contains(&json!(env)),
                "{env:?} missing from {environments:?}"
            );
        }
    }

    // ------------------------------------------------------------------ hostile round trip

    #[test]
    fn hostile_json_round_trip_keeps_every_string_and_structure() {
        let model = hostile_model(false, GENERATED);
        let text = render_json(&model, &schema()).expect("hostile document renders");
        let parsed: Value = serde_json::from_str(&text).expect("JSON text parses");
        assert_hostile_round_trip(&text, &parsed, &model);
    }

    #[test]
    fn hostile_yaml_round_trip_keeps_every_string_and_structure() {
        let model = hostile_model(false, GENERATED);
        let text = render_yaml(&model, &schema()).expect("hostile document renders");
        let parsed: Value = serde_yaml::from_str(&text).expect("YAML text parses");
        assert_hostile_round_trip(&text, &parsed, &model);
    }

    #[test]
    fn yaml_scalars_that_look_like_other_types_come_back_as_strings() {
        let model = hostile_model(false, GENERATED);
        let text = render_yaml(&model, &schema()).expect("hostile document renders");
        let parsed: Value = serde_yaml::from_str(&text).expect("YAML text parses");
        let mut requirement_ids: Vec<&Value> = Vec::new();
        for document in parsed["content"]["documents"]
            .as_array()
            .expect("documents")
        {
            for requirement in document["requirements"].as_array().expect("requirements") {
                requirement_ids.push(&requirement["requirement_id"]);
            }
        }
        for look_alike in [
            "0123",
            "null",
            "yes",
            "no",
            "~",
            "true",
            "1e3",
            "0x1F",
            "2026-10-31",
        ] {
            let expected = Value::String(look_alike.to_string());
            assert!(
                requirement_ids.contains(&&expected),
                "{look_alike:?} is not a string in the YAML text:\n{text}"
            );
        }
    }

    // ------------------------------------------------------------------ determinism (D-05)

    #[test]
    fn byte_identical_json_for_separately_built_identical_inputs() {
        let first = render_json(&hostile_model(false, GENERATED), &schema()).expect("renders");
        let second = render_json(&hostile_model(false, GENERATED), &schema()).expect("renders");
        assert_eq!(first, second);
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn byte_identical_yaml_for_separately_built_identical_inputs() {
        let first = render_yaml(&hostile_model(false, GENERATED), &schema()).expect("renders");
        let second = render_yaml(&hostile_model(false, GENERATED), &schema()).expect("renders");
        assert_eq!(first, second);
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[test]
    fn shuffled_json_gives_the_same_bytes_as_sorted_input() {
        let sorted = render_json(&hostile_model(false, GENERATED), &schema()).expect("renders");
        let shuffled = render_json(&hostile_model(true, GENERATED), &schema()).expect("renders");
        assert_eq!(sorted.as_bytes(), shuffled.as_bytes());
    }

    #[test]
    fn shuffled_yaml_gives_the_same_bytes_as_sorted_input() {
        let sorted = render_yaml(&hostile_model(false, GENERATED), &schema()).expect("renders");
        let shuffled = render_yaml(&hostile_model(true, GENERATED), &schema()).expect("renders");
        assert_eq!(sorted.as_bytes(), shuffled.as_bytes());
    }

    /// The lines that differ between two texts, as (index, left, right).
    fn differing_lines<'a>(a: &'a str, b: &'a str) -> Vec<(usize, &'a str, &'a str)> {
        let left: Vec<&str> = a.lines().collect();
        let right: Vec<&str> = b.lines().collect();
        assert_eq!(left.len(), right.len(), "line counts differ");
        left.iter()
            .zip(right.iter())
            .enumerate()
            .filter(|(_, (l, r))| l != r)
            .map(|(i, (l, r))| (i, *l, *r))
            .collect()
    }

    #[test]
    fn a_different_generated_at_changes_exactly_one_json_line() {
        let a =
            render_json(&hostile_model(false, "2026-11-01T12:00:00Z"), &schema()).expect("renders");
        let b =
            render_json(&hostile_model(false, "2026-11-01T13:00:00Z"), &schema()).expect("renders");
        let diff = differing_lines(&a, &b);
        assert_eq!(diff.len(), 1, "{diff:?}");
        assert!(diff[0].1.contains("\"generated_at\""), "{diff:?}");
        assert!(diff[0].2.contains("2026-11-01T13:00:00Z"), "{diff:?}");
    }

    #[test]
    fn a_different_generated_at_changes_exactly_one_yaml_line() {
        let a =
            render_yaml(&hostile_model(false, "2026-11-01T12:00:00Z"), &schema()).expect("renders");
        let b =
            render_yaml(&hostile_model(false, "2026-11-01T13:00:00Z"), &schema()).expect("renders");
        let diff = differing_lines(&a, &b);
        assert_eq!(diff.len(), 1, "{diff:?}");
        assert!(
            diff[0].1.trim_start().starts_with("generated_at:"),
            "{diff:?}"
        );
        assert!(diff[0].2.contains("2026-11-01T13:00:00Z"), "{diff:?}");
    }

    // ------------------------------------------------------------------ golden outputs

    #[test]
    fn golden_json_of_the_small_fixture() {
        let text = render_json(&small_model(), &schema()).expect("renders");
        assert_eq!(text, GOLDEN_JSON);
    }

    #[test]
    fn golden_yaml_of_the_small_fixture() {
        let text = render_yaml(&small_model(), &schema()).expect("renders");
        let head: Vec<&str> = text.lines().take(12).collect();
        assert_eq!(head, GOLDEN_YAML_HEAD);
        assert_eq!(text.lines().count(), GOLDEN_YAML_LINES);
    }

    const GOLDEN_JSON: &str = r#"{
  "type": "report",
  "schema": "tcms/report.schema.v1.json",
  "kind": "traceability",
  "header": {
    "matrix_id": "TM-1",
    "baseline_id": null,
    "run_ids": [
      "RUN-1"
    ],
    "build_ids": [
      "BUILD-1"
    ],
    "environments": [
      "ENV-1"
    ],
    "testers": [
      "TESTER-1"
    ],
    "generated_at": "2026-11-01T12:00:00Z",
    "obsolete_excluded": null
  },
  "content": {
    "documents": [
      {
        "name": "docs/a.sdoc",
        "requirements": [
          {
            "requirement_id": "REQ-1",
            "level": "HLR",
            "status": "FAILED",
            "normal_range_covered": true,
            "robustness_covered": false,
            "cases": [
              {
                "test_case_id": "TC-1",
                "column": "other",
                "outcome": "pass",
                "run_id": "RUN-1"
              },
              {
                "test_case_id": "TC-2",
                "column": "other",
                "outcome": "fail",
                "run_id": "RUN-1"
              }
            ]
          }
        ]
      },
      {
        "name": "docs/b.sdoc",
        "requirements": [
          {
            "requirement_id": "REQ-2",
            "level": "LLR",
            "status": "FAILED",
            "normal_range_covered": true,
            "robustness_covered": false,
            "cases": [
              {
                "test_case_id": "TC-2",
                "column": "other",
                "outcome": "fail",
                "run_id": "RUN-1"
              }
            ]
          }
        ]
      }
    ]
  }
}
"#;
    const GOLDEN_YAML_HEAD: [&str; 12] = [
        "type: report",
        "schema: tcms/report.schema.v1.json",
        "kind: traceability",
        "header:",
        "  matrix_id: TM-1",
        "  baseline_id: null",
        "  run_ids:",
        "  - RUN-1",
        "  build_ids:",
        "  - BUILD-1",
        "  environments:",
        "  - ENV-1",
    ];
    const GOLDEN_YAML_LINES: usize = 46;
}

#[cfg(test)]
mod documents_tests {
    use super::*;
    use crate::report_document::{
        build_document, build_gaps_document, build_problems_document, build_summary_document,
        ReportDocument,
    };
    use crate::view::{test_support::fixture, ViewInput};

    fn schema() -> Value {
        serde_json::from_str(include_str!("../../../schemas/tcms/report.schema.v1.json"))
            .expect("report schema is JSON")
    }

    /// The schema with one more required top-level property, so every document violates it.
    fn schema_requiring_missing_field() -> Value {
        let mut schema = schema();
        schema["required"]
            .as_array_mut()
            .expect("top-level required list")
            .push(Value::from("nonexistent_field"));
        schema
    }

    /// The four documents of one model, in the fixed order of the CLI.
    fn four_documents() -> Vec<ReportDocument> {
        let (model, selection) = fixture();
        let input = ViewInput {
            model: &model,
            selection: &selection,
        };
        vec![
            build_document(&model),
            build_summary_document(input),
            build_gaps_document(input),
            build_problems_document(input),
        ]
    }

    #[test]
    fn one_document_json_equals_render_json() {
        let (model, _) = fixture();
        let text = render_documents_json(&[build_document(&model)], &schema())
            .expect("one document renders");
        assert_eq!(text, render_json(&model, &schema()).expect("renders"));
    }

    #[test]
    fn four_documents_json_is_an_array_in_the_given_order() {
        let documents = four_documents();
        let text = render_documents_json(&documents, &schema()).expect("four documents render");
        assert!(text.starts_with("[\n"), "{text}");
        assert!(text.ends_with("]\n"), "{text}");
        let parsed: Value = serde_json::from_str(&text).expect("JSON parses");
        let expected: Vec<Value> = documents
            .iter()
            .map(|d| serde_json::to_value(d).expect("serialises"))
            .collect();
        assert_eq!(parsed, Value::Array(expected));
        let kinds: Vec<&str> = parsed
            .as_array()
            .expect("array")
            .iter()
            .map(|d| d["kind"].as_str().expect("kind"))
            .collect();
        assert_eq!(kinds, ["traceability", "summary", "gaps", "problems"]);
    }

    #[test]
    fn json_violation_names_the_kind_and_gives_no_text() {
        let result = render_documents_json(&four_documents(), &schema_requiring_missing_field());
        match result {
            Err(OutputError::Schema(messages)) => {
                assert!(messages[0].starts_with("traceability: "), "{messages:?}");
                assert!(
                    messages.iter().any(|m| m.starts_with("problems: ")),
                    "{messages:?}"
                );
                assert!(messages.iter().any(|m| m.contains("nonexistent_field")));
            }
            other => panic!("expected a schema error, got {other:?}"),
        }
    }

    #[test]
    fn one_document_yaml_equals_render_yaml() {
        let (model, _) = fixture();
        let text = render_documents_yaml(&[build_document(&model)], &schema())
            .expect("one document renders");
        assert_eq!(text, render_yaml(&model, &schema()).expect("renders"));
    }

    #[test]
    fn four_documents_yaml_parse_to_the_same_values_as_json() {
        let documents = four_documents();
        let yaml = render_documents_yaml(&documents, &schema()).expect("four documents render");
        assert!(yaml.lines().any(|l| l == "---"), "{yaml}");
        let from_yaml: Vec<Value> = serde_yaml::Deserializer::from_str(&yaml)
            .map(|doc| {
                let value: serde_yaml::Value =
                    serde::Deserialize::deserialize(doc).expect("document parses");
                serde_json::to_value(value).expect("converts")
            })
            .collect();
        let json = render_documents_json(&documents, &schema()).expect("renders");
        let from_json: Vec<Value> = serde_json::from_str::<Value>(&json)
            .expect("JSON parses")
            .as_array()
            .expect("array")
            .clone();
        assert_eq!(from_yaml, from_json);
    }

    #[test]
    fn yaml_violation_names_the_kind_and_gives_no_text() {
        let result = render_documents_yaml(&four_documents(), &schema_requiring_missing_field());
        match result {
            Err(OutputError::Schema(messages)) => {
                assert!(messages[0].starts_with("traceability: "), "{messages:?}");
            }
            other => panic!("expected a schema error, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------ shape documents (red, 04-05)

    fn shape_schema_requiring_missing_field() -> Value {
        let mut schema = schema();
        schema["required"]
            .as_array_mut()
            .expect("top-level required list")
            .push(Value::from("nonexistent_field"));
        schema
    }

    #[test]
    fn check_document_accepts_a_valid_shape_a_document() {
        let model = super::tests::small_model();
        let doc = crate::report_document::build_shape_document(
            &model,
            crate::report_document::TraceShape::A,
        );
        assert!(check_document(&doc, &schema()).is_ok());
    }

    #[test]
    fn check_document_accepts_a_valid_shape_b_document() {
        let model = super::tests::small_model();
        let doc = crate::report_document::build_shape_document(
            &model,
            crate::report_document::TraceShape::B,
        );
        assert!(check_document(&doc, &schema()).is_ok());
    }

    #[test]
    fn check_document_rejects_a_schema_mismatch_with_messages() {
        let model = super::tests::small_model();
        let doc = crate::report_document::build_shape_document(
            &model,
            crate::report_document::TraceShape::A,
        );
        match check_document(&doc, &shape_schema_requiring_missing_field()) {
            Err(OutputError::Schema(messages)) => {
                assert!(
                    messages.iter().any(|m| m.contains("nonexistent_field")),
                    "{messages:?}"
                );
            }
            other => panic!("expected a schema error, got {other:?}"),
        }
    }

    #[test]
    fn check_document_mismatch_returns_schema_error_variant() {
        let model = super::tests::small_model();
        let doc = crate::report_document::build_shape_document(
            &model,
            crate::report_document::TraceShape::B,
        );
        let result = check_document(&doc, &shape_schema_requiring_missing_field());
        assert!(
            matches!(result, Err(OutputError::Schema(_))),
            "expected exactly the Schema variant, got {result:?}"
        );
    }
}
