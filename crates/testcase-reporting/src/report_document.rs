//! Machine-readable report documents for all four kinds (OUT-02, D-03, D-04).
//!
//! `build_document` maps the Phase 2 `ReportModel` (traceability) and the three Phase 3 view
//! builders (summary, gaps, problems) field by field into serialisable structs. The output is
//! checked by `validate_document` against `schemas/tcms/report.schema.v1.json`.
//!
//! Pure: no clock, no file reads, no hash-ordered maps. A missing value is `null` or an empty
//! array, never a placeholder text. Nothing is skipped when it is empty (the schema requires
//! every key). The views are read as they are; they are not changed here.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{
        build_model, CaseEntry, DocumentSection, Header, InputsSection, ReportInputs, ReportModel,
        RequirementDocuments, RequirementRow, TestColumn,
    };
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection};
    use crate::view::test_support::fixture;
    use crate::view::{ResultCounts, ViewInput};
    use serde_json::{json, Value};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    fn schema() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("schemas/tcms/report.schema.v1.json");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        serde_json::from_str(&text).expect("report schema is JSON")
    }

    fn value(doc: &ReportDocument) -> Value {
        serde_json::to_value(doc).expect("report document serialises")
    }

    fn text(doc: &ReportDocument) -> String {
        serde_json::to_string(doc).expect("report document serialises")
    }

    fn assert_valid(doc: &ReportDocument) {
        if let Err(messages) = validate_document(&value(doc), &schema()) {
            panic!("expected a valid {:?} document, got: {messages:#?}", doc.kind);
        }
    }

    /// Hand-built Phase 2 golden model: one populated document, one empty document, every header
    /// field set except the baseline, and one obsolete count.
    fn golden_model() -> ReportModel {
        ReportModel {
            inputs: InputsSection {
                details: "Trace matrix: TM-1 (baseline: not recorded)\nRuns selected: RUN-1, RUN-2"
                    .to_string(),
            },
            header: Header {
                matrix_id: "TM-1".to_string(),
                baseline_id: None,
                run_ids: vec!["RUN-1".to_string(), "RUN-2".to_string()],
                build_ids: vec!["b1".to_string(), "b2".to_string()],
                environments: vec!["lab-1".to_string()],
                testers: vec!["alice".to_string()],
                generated_at: "2026-10-31T09:00:00Z".to_string(),
                obsolete_excluded: Some(2),
            },
            documents: vec![
                DocumentSection {
                    name: Some("docs/srs.sdoc".to_string()),
                    rows: vec![
                        RequirementRow {
                            requirement_id: "HLR-1".to_string(),
                            level: RequirementLevel::Hlr,
                            status: VerificationStatus::Verified,
                            normal_range_covered: true,
                            robustness_covered: true,
                            cases: vec![
                                CaseEntry {
                                    test_case_id: "T1".to_string(),
                                    column: TestColumn::NormalRange,
                                    outcome: CaseOutcome::Pass,
                                    run_id: Some("RUN-1".to_string()),
                                },
                                CaseEntry {
                                    test_case_id: "T2".to_string(),
                                    column: TestColumn::Robustness,
                                    outcome: CaseOutcome::Pass,
                                    run_id: Some("RUN-2".to_string()),
                                },
                            ],
                        },
                        RequirementRow {
                            requirement_id: "LLR-2".to_string(),
                            level: RequirementLevel::Llr,
                            status: VerificationStatus::Failed,
                            normal_range_covered: true,
                            robustness_covered: false,
                            cases: vec![
                                CaseEntry {
                                    test_case_id: "T3".to_string(),
                                    column: TestColumn::Other,
                                    outcome: CaseOutcome::Fail,
                                    run_id: Some("RUN-2".to_string()),
                                },
                                CaseEntry {
                                    test_case_id: "T4".to_string(),
                                    column: TestColumn::Other,
                                    outcome: CaseOutcome::NotRun,
                                    run_id: None,
                                },
                            ],
                        },
                    ],
                },
                DocumentSection {
                    name: Some("docs/empty.sdoc".to_string()),
                    rows: Vec::new(),
                },
            ],
        }
    }

    fn link(
        id: &str,
        level: RequirementLevel,
        cases: &[&str],
        normal: bool,
        robustness: bool,
    ) -> TraceLink {
        TraceLink {
            requirement_id: id.to_string(),
            level,
            test_case_ids: cases.iter().map(|c| c.to_string()).collect(),
            normal_range_covered: normal,
            robustness_covered: robustness,
            run_ids: Vec::new(),
        }
    }

    /// Distinct strings in every field a document can carry, built through the real pipeline:
    /// loaded runs -> selection -> model with documents. No value is repeated or a substring of
    /// another, except as noted in the test that uses it.
    fn distinct_fixture() -> (ReportModel, RunSelection) {
        let mut rec1 = record(
            "RUN-DIST-1",
            "BUILD-DIST-1",
            "2026-11-01T10:00:00Z",
            &[("TC-A1", Verdict::Pass), ("TC-B1", Verdict::Fail)],
        );
        rec1.environment = "ENV-DIST".to_string();
        rec1.tester = "TESTER-ONE".to_string();
        for result in rec1.results.iter_mut() {
            if result.test_case_id == "TC-B1" {
                result.problem_report_id = Some("PR-DIST-9".to_string());
            }
        }
        let mut rec2 = record(
            "RUN-DIST-2",
            "BUILD-DIST-2",
            "2026-11-01T11:00:00Z",
            &[("TC-A2", Verdict::Error)],
        );
        rec2.environment = "ENV-DIST".to_string();
        rec2.tester = "TESTER-TWO".to_string();
        let runs = vec![
            LoadedRun::new("RUN-DIST-1.yaml".into(), rec1).expect("valid run"),
            LoadedRun::new("RUN-DIST-2.yaml".into(), rec2).expect("valid run"),
        ];
        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: "TM-DIST".into(),
                baseline_id: Some("BL-DIST".into()),
                links: vec![
                    link("REQ-DIST-A", RequirementLevel::Hlr, &["TC-A1", "TC-A2"], true, false),
                    link("REQ-DIST-B", RequirementLevel::Llr, &["TC-B1"], false, false),
                ],
            },
            runs,
            reconciliation: Reconciliation::default(),
        };
        let selection = select_results(&inputs.runs);
        let documents = RequirementDocuments {
            documents: vec!["docs/dist-a.sdoc".into(), "docs/dist-b.sdoc".into()],
            requirement_document: BTreeMap::from([
                ("REQ-DIST-A".to_string(), "docs/dist-a.sdoc".to_string()),
                ("REQ-DIST-B".to_string(), "docs/dist-b.sdoc".to_string()),
            ]),
        };
        let model = build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: None,
            documents: Some(&documents),
            generated_at: "2026-11-01T12:00:00Z",
        });
        (model, selection)
    }

    /// Header values of the distinct fixture: every one must reach every kind's text.
    const DISTINCT_HEADER: [&str; 10] = [
        "TM-DIST",
        "BL-DIST",
        "RUN-DIST-1",
        "RUN-DIST-2",
        "BUILD-DIST-1",
        "BUILD-DIST-2",
        "ENV-DIST",
        "TESTER-ONE",
        "TESTER-TWO",
        "2026-11-01T12:00:00Z",
    ];

    fn assert_contains_all(text: &str, expected: &[&str]) {
        for needle in expected {
            assert!(text.contains(needle), "{needle} missing from {text}");
        }
    }

    // ------------------------------------------------------------------ constants

    #[test]
    fn constants_match_the_schema_file() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["schema"]["const"].as_str(),
            Some(REPORT_SCHEMA_URI)
        );
        assert_eq!(schema["properties"]["type"]["const"].as_str(), Some(REPORT_TYPE));
        assert_eq!(REPORT_TYPE, "report");
        assert_eq!(REPORT_SCHEMA_URI, "tcms/report.schema.v1.json");
    }

    // ------------------------------------------------------------------ acceptance against the schema

    #[test]
    fn golden_traceability_document_validates() {
        let doc = build_document(&golden_model());
        assert_eq!(doc.kind, ReportKind::Traceability);
        assert_valid(&doc);
    }

    #[test]
    fn view_documents_of_the_shared_fixture_validate() {
        let (model, selection) = fixture();
        let input = || ViewInput {
            model: &model,
            selection: &selection,
        };
        let summary = build_summary_document(input());
        let gaps = build_gaps_document(input());
        let problems = build_problems_document(input());
        assert_eq!(summary.kind, ReportKind::Summary);
        assert_eq!(gaps.kind, ReportKind::Gaps);
        assert_eq!(problems.kind, ReportKind::Problems);
        assert_valid(&summary);
        assert_valid(&gaps);
        assert_valid(&problems);
    }

    #[test]
    fn distinct_fixture_validates_for_every_kind() {
        let (model, selection) = distinct_fixture();
        let input = || ViewInput {
            model: &model,
            selection: &selection,
        };
        assert_valid(&build_document(&model));
        assert_valid(&build_summary_document(input()));
        assert_valid(&build_gaps_document(input()));
        assert_valid(&build_problems_document(input()));
    }

    #[test]
    fn two_simultaneous_violations_give_two_messages_with_paths() {
        let mut instance = value(&build_document(&golden_model()));
        instance["content"]["documents"][0]["requirements"][0]["status"] = json!("DONE");
        instance["header"]
            .as_object_mut()
            .expect("header object")
            .remove("matrix_id");
        let messages = validate_document(&instance, &schema()).expect_err("two violations");
        assert_eq!(messages.len(), 2, "{messages:#?}");
        assert!(messages.iter().all(|m| m.starts_with('/')), "{messages:#?}");
        assert!(
            messages.iter().any(|m| m.starts_with("/content/documents/0/requirements/0/status")
                && m.contains("DONE")),
            "{messages:#?}"
        );
        assert!(
            messages.iter().any(|m| m.starts_with("/header") && m.contains("matrix_id")),
            "{messages:#?}"
        );
    }

    #[test]
    fn valid_instance_gives_ok_from_validate_document() {
        let instance = value(&build_document(&golden_model()));
        assert_eq!(validate_document(&instance, &schema()), Ok(()));
    }

    #[test]
    fn problems_kind_with_traceability_content_is_rejected_by_the_schema() {
        let mut instance = value(&build_document(&golden_model()));
        instance["kind"] = json!("problems");
        let messages = validate_document(&instance, &schema()).expect_err("kind mismatch");
        assert!(!messages.is_empty(), "{messages:#?}");
        assert!(messages.iter().all(|m| m.starts_with('/')), "{messages:#?}");
    }

    #[test]
    fn schema_that_does_not_compile_gives_one_message() {
        let messages = validate_document(&json!({}), &json!({ "type": 7 }))
            .expect_err("schema must not compile");
        assert_eq!(messages.len(), 1, "{messages:#?}");
        assert!(
            messages[0].starts_with("report schema does not compile"),
            "{messages:#?}"
        );
    }

    // ------------------------------------------------------------------ field mapping

    #[test]
    fn envelope_fields_and_kind_strings() {
        let doc = value(&build_document(&golden_model()));
        assert_eq!(doc["type"], "report");
        assert_eq!(doc["schema"], REPORT_SCHEMA_URI);
        assert_eq!(doc["kind"], "traceability");
        let summary = value(&build_summary_document(ViewInput {
            model: &fixture().0,
            selection: &fixture().1,
        }));
        assert_eq!(summary["kind"], "summary");
    }

    #[test]
    fn absent_values_are_null_or_empty_never_placeholder_text() {
        let doc = value(&build_document(&golden_model()));
        assert_eq!(doc["header"]["baseline_id"], Value::Null);
        assert_eq!(doc["header"]["obsolete_excluded"], 2);
        assert_eq!(doc["content"]["documents"][1]["name"], "docs/empty.sdoc");
        assert_eq!(doc["content"]["documents"][1]["requirements"], json!([]));
        assert_eq!(doc["content"]["documents"][0]["requirements"][1]["cases"][1]["run_id"], Value::Null);

        let mut model = golden_model();
        model.header.obsolete_excluded = None;
        model.documents[0].name = None;
        let doc = value(&build_document(&model));
        assert_eq!(doc["header"]["obsolete_excluded"], Value::Null);
        assert_eq!(doc["content"]["documents"][0]["name"], Value::Null);

        let text = text(&build_document(&golden_model()));
        assert!(!text.contains("not recorded"), "{text}");
    }

    #[test]
    fn document_name_none_is_null_in_the_document() {
        let (model, selection) = fixture();
        let doc = value(&build_gaps_document(ViewInput {
            model: &model,
            selection: &selection,
        }));
        // The shared fixture has no source documents: every gap requirement's document is null.
        assert_eq!(doc["content"]["no_tests"][0]["document"], Value::Null);
        assert_eq!(doc["content"]["missing_normal_range"][0]["document"], Value::Null);
    }

    #[test]
    fn problem_report_id_none_is_null_and_present_id_is_kept() {
        let (model, selection) = fixture();
        let doc = value(&build_problems_document(ViewInput {
            model: &model,
            selection: &selection,
        }));
        let rows = doc["content"]["rows"].as_array().expect("rows");
        let t2 = rows.iter().find(|r| r["test_case_id"] == "T2").expect("T2 row");
        let t3 = rows.iter().find(|r| r["test_case_id"] == "T3").expect("T3 row");
        assert_eq!(t2["problem_report_id"], "PR-7");
        assert_eq!(t3["problem_report_id"], Value::Null);
        assert_eq!(doc["content"]["missing_count"], 1);
    }

    #[test]
    fn status_and_outcome_strings_are_the_label_text() {
        let doc = value(&build_document(&golden_model()));
        let requirement = &doc["content"]["documents"][0]["requirements"][1];
        assert_eq!(requirement["status"], "FAILED");
        assert_eq!(requirement["cases"][1]["outcome"], "NOT RUN");
        assert_eq!(requirement["cases"][0]["outcome"], "fail");

        let (model, selection) = fixture();
        let summary = value(&build_summary_document(ViewInput {
            model: &model,
            selection: &selection,
        }));
        let statuses: Vec<&str> = summary["content"]["requirements"]
            .as_array()
            .expect("requirements")
            .iter()
            .map(|r| r["status"].as_str().expect("status"))
            .collect();
        assert_eq!(statuses, ["FAILED", "NO TESTS", "ERRORED", "NOT RUN"]);
    }

    #[test]
    fn level_column_and_verdict_use_explicit_words() {
        let doc = value(&build_document(&golden_model()));
        let requirements = &doc["content"]["documents"][0]["requirements"];
        assert_eq!(requirements[0]["level"], "HLR");
        assert_eq!(requirements[1]["level"], "LLR");
        assert_eq!(requirements[0]["cases"][0]["column"], "normal_range");
        assert_eq!(requirements[0]["cases"][1]["column"], "robustness");
        assert_eq!(requirements[1]["cases"][0]["column"], "other");

        let (model, selection) = fixture();
        let problems = value(&build_problems_document(ViewInput {
            model: &model,
            selection: &selection,
        }));
        let verdicts: Vec<&str> = problems["content"]["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r["verdict"].as_str().expect("verdict"))
            .collect();
        assert_eq!(verdicts, ["fail", "error"]);
    }

    #[test]
    fn result_counts_map_to_named_keys() {
        let counts = ResultCounts {
            pass: 1,
            fail: 2,
            not_executed: 3,
            error: 4,
            total: 10,
        };
        assert_eq!(
            serde_json::to_value(CountsOut::from(&counts)).expect("counts"),
            json!({ "pass": 1, "fail": 2, "not_executed": 3, "error": 4, "total": 10 })
        );
    }

    // ------------------------------------------------------------------ order and key order

    #[test]
    fn view_order_is_kept_for_documents_requirements_and_cases() {
        let doc = value(&build_document(&golden_model()));
        assert_eq!(doc["content"]["documents"][0]["name"], "docs/srs.sdoc");
        assert_eq!(doc["content"]["documents"][1]["name"], "docs/empty.sdoc");
        assert_eq!(doc["content"]["documents"][0]["requirements"][0]["requirement_id"], "HLR-1");
        assert_eq!(doc["content"]["documents"][0]["requirements"][1]["requirement_id"], "LLR-2");
        assert_eq!(
            doc["content"]["documents"][0]["requirements"][0]["cases"][0]["test_case_id"],
            "T1"
        );
    }

    fn assert_in_order(text: &str, needles: &[&str]) {
        let positions: Vec<usize> = needles
            .iter()
            .map(|n| text.find(n).unwrap_or_else(|| panic!("{n} missing from {text}")))
            .collect();
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "key order differs from the struct order: {needles:?} at {positions:?}"
        );
    }

    #[test]
    fn traceability_keys_follow_the_struct_declaration_order() {
        let t = text(&build_document(&golden_model()));
        assert_in_order(
            &t,
            &[
                "\"type\":",
                "\"schema\":",
                "\"kind\":",
                "\"header\":",
                "\"content\":",
            ],
        );
        assert_in_order(
            &t,
            &["\"matrix_id\":", "\"generated_at\":", "\"obsolete_excluded\":"],
        );
    }

    #[test]
    fn summary_keys_follow_the_struct_declaration_order() {
        let (model, selection) = fixture();
        let t = text(&build_summary_document(ViewInput {
            model: &model,
            selection: &selection,
        }));
        assert_in_order(
            &t,
            &[
                "\"type\":",
                "\"schema\":",
                "\"kind\":",
                "\"header\":",
                "\"content\":",
            ],
        );
        assert_in_order(&t, &["\"matrix_id\":", "\"generated_at\":", "\"obsolete_excluded\":"]);
    }

    // ------------------------------------------------------------------ nothing dropped, nothing invented

    #[test]
    fn distinct_values_of_each_kind_reach_its_text() {
        let (model, selection) = distinct_fixture();
        let input = || ViewInput {
            model: &model,
            selection: &selection,
        };

        let traceability = text(&build_document(&model));
        assert_contains_all(&traceability, &DISTINCT_HEADER);
        assert_contains_all(
            &traceability,
            &[
                "docs/dist-a.sdoc",
                "docs/dist-b.sdoc",
                "REQ-DIST-A",
                "REQ-DIST-B",
                "TC-A1",
                "TC-A2",
                "TC-B1",
            ],
        );

        let summary = text(&build_summary_document(input()));
        assert_contains_all(&summary, &DISTINCT_HEADER);
        assert_contains_all(&summary, &["REQ-DIST-A", "REQ-DIST-B"]);

        let gaps = text(&build_gaps_document(input()));
        assert_contains_all(&gaps, &DISTINCT_HEADER);
        assert_contains_all(&gaps, &["REQ-DIST-A", "REQ-DIST-B", "docs/dist-a.sdoc"]);

        let problems = text(&build_problems_document(input()));
        assert_contains_all(&problems, &DISTINCT_HEADER);
        assert_contains_all(
            &problems,
            &["TC-A2", "TC-B1", "PR-DIST-9", "REQ-DIST-A", "REQ-DIST-B"],
        );
    }

    #[test]
    fn distinct_fixture_text_has_no_placeholder_and_no_invented_values() {
        let (model, selection) = distinct_fixture();
        let input = || ViewInput {
            model: &model,
            selection: &selection,
        };
        for t in [
            text(&build_document(&model)),
            text(&build_summary_document(input())),
            text(&build_gaps_document(input())),
            text(&build_problems_document(input())),
        ] {
            assert!(!t.contains("not recorded"), "{t}");
            assert!(!t.contains("PR-7"), "a value from another fixture leaked: {t}");
        }
    }

    #[test]
    fn document_serialisation_is_deterministic() {
        let (model, selection) = distinct_fixture();
        let input = || ViewInput {
            model: &model,
            selection: &selection,
        };
        assert_eq!(text(&build_document(&model)), text(&build_document(&model)));
        assert_eq!(
            text(&build_summary_document(input())),
            text(&build_summary_document(input()))
        );
    }
}
