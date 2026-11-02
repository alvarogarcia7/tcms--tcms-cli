//! Traceability report shapes A and B in AsciiDoc (OUT-01, D-02, D-05, D-06).
//!
//! Same frame, sections and columns as `render_shapes` (the Markdown shapes), written as AsciiDoc
//! tables. Reads the report model only: no clock, no environment lookups, no hash-ordered
//! iteration. Every model string passes through `escape_adoc`; the fixed words are literals.

use crate::model::{DocumentSection, ReportModel, RequirementRow};
use crate::render_asciidoc::{escape_adoc, header_block, inputs_block};

const NOT_RECORDED: &str = "not recorded";
const NO_CONTENT: &str = "No traceable content";
const EMPTY_CELL: &str = "-";
const SHAPE_A_HEADER: &str = "| Requirement | Test cases (ID, status) | Final verdict |";
const SHAPE_B_VERDICT_HEADER: &str = "| Requirement | Final verdict |";
const SHAPE_B_CASE_HEADER: &str = "| TESTCASE ID | Status |";

/// Shape A: one row per requirement with its test cases as `ID (outcome)` and the final verdict.
pub fn render_asciidoc_shape_a(model: &ReportModel) -> String {
    render(model, table_a)
}

/// Shape B: per requirement, a verdict table, then a case-status table when it has cases.
pub fn render_asciidoc_shape_b(model: &ReportModel) -> String {
    render(model, tables_b)
}

/// The shared frame: title, Inputs, header, then one block per document, one trailing newline.
fn render(model: &ReportModel, body: fn(&[RequirementRow]) -> String) -> String {
    let mut blocks: Vec<String> = vec![
        "= Traceability Report".to_string(),
        inputs_block(&model.inputs.details),
        header_block(&model.header),
    ];
    blocks.extend(
        model
            .documents
            .iter()
            .map(|section| document_block(section, body)),
    );

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// Same heading and empty-document rule as the Markdown shapes.
fn document_block(section: &DocumentSection, body: fn(&[RequirementRow]) -> String) -> String {
    let name = section
        .name
        .as_deref()
        .map_or_else(|| NOT_RECORDED.to_string(), escape_adoc);
    let text = if section.rows.is_empty() {
        NO_CONTENT.to_string()
    } else {
        body(&section.rows)
    };
    format!("== Document: {name}\n\n{text}")
}

/// One AsciiDoc table: the column attribute, a fence, a single header row, the body cells (one
/// per `| ` line) and the closing fence.
fn shape_table(columns: usize, header_row: &str, cell_lines: &[String]) -> String {
    let mut lines = vec![
        format!("[cols=\"{columns}\",options=\"header\"]"),
        "|===".to_string(),
        header_row.to_string(),
    ];
    lines.extend(cell_lines.iter().cloned());
    lines.push("|===".to_string());
    lines.join("\n")
}

/// Shape A body: one table per document, one row per requirement.
fn table_a(rows: &[RequirementRow]) -> String {
    let cell_lines: Vec<String> = rows
        .iter()
        .flat_map(|row| {
            [
                format!("| {}", escape_adoc(&row.requirement_id)),
                format!("| {}", case_list(row)),
                format!("| {}", row.status.label()),
            ]
        })
        .collect();
    shape_table(3, SHAPE_A_HEADER, &cell_lines)
}

/// `ID (outcome)` entries in model order, joined with hard line breaks; `-` when there are none.
fn case_list(row: &RequirementRow) -> String {
    if row.cases.is_empty() {
        return EMPTY_CELL.to_string();
    }
    row.cases
        .iter()
        .map(|case| {
            format!(
                "{} ({})",
                escape_adoc(&case.test_case_id),
                case.outcome.label()
            )
        })
        .collect::<Vec<_>>()
        .join(" +\n")
}

/// Shape B body: per requirement, a verdict table, then a case-status table when it has cases.
/// Tables are separated by a blank line; a requirement without cases gets only the verdict table.
fn tables_b(rows: &[RequirementRow]) -> String {
    rows.iter()
        .map(|row| {
            let verdict_lines = [
                format!("| {}", escape_adoc(&row.requirement_id)),
                format!("| {}", row.status.label()),
            ];
            let mut tables = vec![shape_table(2, SHAPE_B_VERDICT_HEADER, &verdict_lines)];
            if !row.cases.is_empty() {
                let case_lines: Vec<String> = row
                    .cases
                    .iter()
                    .flat_map(|case| {
                        [
                            format!("| {}", escape_adoc(&case.test_case_id)),
                            format!("| {}", case.outcome.label()),
                        ]
                    })
                    .collect();
                tables.push(shape_table(2, SHAPE_B_CASE_HEADER, &case_lines));
            }
            tables.join("\n\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gaps::build_gaps;
    use crate::load::{LoadedInputs, Reconciliation};
    use crate::model::{
        build_model, CaseEntry, DocumentSection, Header, InputsSection, ReportInputs,
        RequirementDocuments, RequirementRow, TestColumn,
    };
    use crate::problems::build_problems;
    use crate::render_asciidoc::{
        escape_adoc, header_block, render_asciidoc, render_asciidoc_gaps, render_asciidoc_problems,
        render_asciidoc_summary, LINE_START_CHARS,
    };
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use crate::run::{test_support::record, LoadedRun};
    use crate::select::{select_results, RunSelection};
    use crate::summary::build_summary;
    use crate::view::ViewInput;
    use testcase_models::{TestCase, TestType};
    use testcase_verification::{RequirementLevel, TraceLink, TraceMatrix, Verdict};

    const GENERATED: &str = "2026-10-31T09:00:00Z";
    const DETAILS: &str = "Trace matrix: TM-1 (baseline: not recorded)\nRuns selected: RUN-1";

    // ---- Golden fixture: hand-built, as in the Phase 2 Markdown shape tests ----

    fn header() -> Header {
        Header {
            matrix_id: "TM-1".to_string(),
            baseline_id: Some("BL-1".to_string()),
            run_ids: vec!["RUN-1".to_string()],
            build_ids: vec!["b1".to_string()],
            environments: vec!["lab-1".to_string()],
            testers: vec!["alice".to_string()],
            generated_at: GENERATED.to_string(),
            obsolete_excluded: Some(0),
        }
    }

    fn model(documents: Vec<DocumentSection>) -> ReportModel {
        ReportModel {
            inputs: InputsSection {
                details: DETAILS.to_string(),
            },
            header: header(),
            documents,
        }
    }

    fn doc(name: Option<&str>, rows: Vec<RequirementRow>) -> DocumentSection {
        DocumentSection {
            name: name.map(str::to_string),
            rows,
        }
    }

    fn row(id: &str, status: VerificationStatus, cases: Vec<CaseEntry>) -> RequirementRow {
        RequirementRow {
            requirement_id: id.to_string(),
            level: RequirementLevel::Hlr,
            status,
            normal_range_covered: true,
            robustness_covered: false,
            cases,
        }
    }

    fn case(id: &str, outcome: CaseOutcome) -> CaseEntry {
        CaseEntry {
            test_case_id: id.to_string(),
            column: TestColumn::NormalRange,
            outcome,
            run_id: Some("RUN-1".to_string()),
        }
    }

    /// R-1 verified by two passing cases, R-2 with no tests, R-3 partial with one NOT RUN case.
    fn golden_model() -> ReportModel {
        model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![
                row(
                    "R-1",
                    VerificationStatus::Verified,
                    vec![
                        case("T-1", CaseOutcome::Pass),
                        case("T-2", CaseOutcome::Pass),
                    ],
                ),
                row("R-2", VerificationStatus::NoTests, Vec::new()),
                row(
                    "R-3",
                    VerificationStatus::Partial,
                    vec![
                        case("T-3", CaseOutcome::Pass),
                        case("T-4", CaseOutcome::NotRun),
                    ],
                ),
            ],
        )])
    }

    /// The top of both golden documents: title, Inputs paragraph, report header, first document.
    fn golden_top() -> Vec<&'static str> {
        vec![
            "= Traceability Report",
            "",
            "== Inputs",
            "",
            "Trace matrix: TM-1 (baseline: not recorded) +",
            "Runs selected: RUN-1",
            "",
            "== Report header",
            "",
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| Field",
            "| Value",
            "| Matrix",
            "| TM-1",
            "| Baseline",
            "| BL-1",
            "| Runs",
            "| RUN-1",
            "| Builds",
            "| b1",
            "| Environments",
            "| lab-1",
            "| Testers",
            "| alice",
            "| Obsolete test cases excluded",
            "| 0",
            "| Generated",
            "| 2026-10-31T09:00:00Z",
            "|===",
            "",
            "== Document: docs/srs.sdoc",
            "",
        ]
    }

    fn lines(parts: Vec<&str>) -> String {
        let mut text = parts.join("\n");
        text.push('\n');
        text
    }

    // ---- Golden tests ----

    #[test]
    fn a_golden_matches_exact_text() {
        let mut expected = golden_top();
        expected.extend([
            "[cols=\"3\",options=\"header\"]",
            "|===",
            "| Requirement | Test cases (ID, status) | Final verdict |",
            "| R-1",
            "| T-1 (pass) +",
            "T-2 (pass)",
            "| VERIFIED",
            "| R-2",
            "| -",
            "| NO TESTS",
            "| R-3",
            "| T-3 (pass) +",
            "T-4 (NOT RUN)",
            "| PARTIAL",
            "|===",
        ]);
        assert_eq!(render_asciidoc_shape_a(&golden_model()), lines(expected));
    }

    #[test]
    fn b_golden_matches_exact_text() {
        let mut expected = golden_top();
        expected.extend([
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| Requirement | Final verdict |",
            "| R-1",
            "| VERIFIED",
            "|===",
            "",
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| TESTCASE ID | Status |",
            "| T-1",
            "| pass",
            "| T-2",
            "| pass",
            "|===",
            "",
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| Requirement | Final verdict |",
            "| R-2",
            "| NO TESTS",
            "|===",
            "",
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| Requirement | Final verdict |",
            "| R-3",
            "| PARTIAL",
            "|===",
            "",
            "[cols=\"2\",options=\"header\"]",
            "|===",
            "| TESTCASE ID | Status |",
            "| T-3",
            "| pass",
            "| T-4",
            "| NOT RUN",
            "|===",
        ]);
        assert_eq!(render_asciidoc_shape_b(&golden_model()), lines(expected));
    }

    #[test]
    fn a_case_id_with_pipe_and_newline_stays_on_one_cell_line() {
        let m = model(vec![doc(
            Some("docs/srs.sdoc"),
            vec![row(
                "R-4",
                VerificationStatus::Verified,
                vec![case("T|1\nx", CaseOutcome::Pass)],
            )],
        )]);
        let out = render_asciidoc_shape_a(&m);
        assert!(
            out.contains("\n| R-4\n| T&#124;1 x (pass)\n| VERIFIED\n"),
            "{out}"
        );
    }

    // ---- Empty documents, frame and newline ----

    fn assert_empty_document_marked(out: &str) {
        assert!(
            out.contains("== Document: not recorded\n\nNo traceable content"),
            "{out}"
        );
    }

    #[test]
    fn c1_document_without_requirements_says_no_traceable_content() {
        let m = model(vec![doc(None, Vec::new())]);
        assert_empty_document_marked(&render_asciidoc_shape_a(&m));
        assert_empty_document_marked(&render_asciidoc_shape_b(&m));
    }

    #[test]
    fn c2_top_of_document_matches_the_grid() {
        let m = golden_model();
        for out in [render_asciidoc_shape_a(&m), render_asciidoc_shape_b(&m)] {
            assert!(
                out.starts_with("= Traceability Report\n\n== Inputs\n\n"),
                "{out}"
            );
            assert!(out.contains(&header_block(&m.header)), "{out}");
        }
    }

    #[test]
    fn d2_document_ends_with_exactly_one_newline() {
        let m = golden_model();
        for out in [render_asciidoc_shape_a(&m), render_asciidoc_shape_b(&m)] {
            assert!(out.ends_with('\n') && !out.ends_with("\n\n"), "{out:?}");
        }
    }

    // ---- Pipeline fixtures (public API: select_results, build_model, the six renderers) ----

    /// The 21 hostile strings of the plan: markup, macros, line starts, a pipe, a backslash
    /// before a pipe, a 2000 character string and non-ASCII text.
    fn hostile_values() -> Vec<String> {
        let mut values: Vec<String> = [
            "A|B",
            "x\ny",
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
            "Ünïcode 日本語 🙂",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        values.push("abcdefghij".repeat(200));
        values
    }

    /// Characters that the escape must change when they occur in a value.
    const SPECIAL: &str = "&<>*_`#+^~[]{}\\|@:=.-'\n\r";

    fn case_def(id: &str, test_type: Option<TestType>) -> TestCase {
        let mut tc = TestCase::new("LEGACY".into(), 1, 1, id.into(), "d".into());
        tc.test_type = test_type;
        tc
    }

    /// A loaded run with the tester and environment given, and per-result problem report IDs.
    fn loaded(
        run_id: &str,
        build_id: &str,
        started: &str,
        env: &str,
        tester: &str,
        results: &[(&str, Verdict, Option<&str>)],
    ) -> LoadedRun {
        let base: Vec<(&str, Verdict)> = results
            .iter()
            .map(|(id, verdict, _)| (*id, *verdict))
            .collect();
        let mut rec = record(run_id, build_id, started, &base);
        rec.environment = env.to_string();
        rec.tester = tester.to_string();
        for (result, entry) in rec.results.iter_mut().zip(results) {
            result.problem_report_id = entry.2.map(str::to_string);
        }
        LoadedRun::new(format!("{run_id}.yaml").into(), rec).expect("valid run")
    }

    /// Every field carries hostile text. Requirement `i` links two cases, the first decided by
    /// run `i % 3`; requirement 20's first case has no result, so it is NOT RUN.
    /// With `reverse`, links, their test case IDs, runs and test case definitions are reversed.
    fn hostile_pipeline(reverse: bool, generated_at: &str) -> (ReportModel, RunSelection) {
        let hv = hostile_values();
        let n = hv.len();
        let mut links: Vec<TraceLink> = (0..n)
            .map(|i| TraceLink {
                requirement_id: hv[i].clone(),
                level: RequirementLevel::Hlr,
                test_case_ids: vec![hv[(i + 1) % n].clone(), hv[(i + 2) % n].clone()],
                normal_range_covered: i % 2 == 0,
                robustness_covered: i % 3 == 0,
                run_ids: Vec::new(),
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
        let mut runs: Vec<LoadedRun> = (0..3)
            .map(|r| {
                let owned: Vec<(String, Verdict, Option<String>)> = (0..n)
                    .filter(|&i| i % 3 == r && i != 20)
                    .map(|i| {
                        let (verdict, problem) = match i % 4 {
                            1 => (Verdict::Fail, Some(hv[(i + 7) % n].clone())),
                            2 => (Verdict::Error, None),
                            _ => (Verdict::Pass, None),
                        };
                        (hv[(i + 1) % n].clone(), verdict, problem)
                    })
                    .collect();
                let refs: Vec<(&str, Verdict, Option<&str>)> = owned
                    .iter()
                    .map(|(id, verdict, problem)| (id.as_str(), *verdict, problem.as_deref()))
                    .collect();
                loaded(
                    &run_ids[r],
                    &build_ids[r],
                    starts[r],
                    &envs[r],
                    &testers[r],
                    &refs,
                )
            })
            .collect();

        let mut test_cases: Vec<TestCase> = (0..n)
            .map(|k| {
                let test_type = match k % 3 {
                    1 => Some(TestType::Robustness),
                    _ if k % 5 == 0 => None,
                    _ => Some(TestType::NormalRange),
                };
                case_def(&hv[k], test_type)
            })
            .collect();

        let docs3 = [hv[14].clone(), hv[17].clone(), hv[0].clone()];
        let documents = RequirementDocuments {
            documents: vec![
                hv[14].clone(),
                hv[17].clone(),
                hv[0].clone(),
                hv[19].clone(),
            ],
            requirement_document: (0..n)
                .filter(|i| i % 4 != 3)
                .map(|i| (hv[i].clone(), docs3[i % 3].clone()))
                .collect(),
        };

        if reverse {
            links.reverse();
            for link in &mut links {
                link.test_case_ids.reverse();
            }
            runs.reverse();
            test_cases.reverse();
        }

        let inputs = LoadedInputs {
            matrix_path: "matrix.yaml".into(),
            matrix: TraceMatrix {
                doc_type: "trace_matrix".into(),
                schema: "tcms/trace-matrix.schema.json".into(),
                id: hv[11].clone(),
                baseline_id: Some("BL-1".into()),
                links,
            },
            runs,
            reconciliation: Reconciliation::default(),
        };
        let selection = select_results(&inputs.runs);
        let model = build_model(&ReportInputs {
            inputs: &inputs,
            selection: &selection,
            test_cases: Some(test_cases.as_slice()),
            documents: Some(&documents),
            generated_at,
        });
        (model, selection)
    }

    /// The six AsciiDoc renderers over one model, in a fixed order.
    fn six_outputs(model: &ReportModel, selection: &RunSelection) -> Vec<(&'static str, String)> {
        let view = ViewInput { model, selection };
        vec![
            ("grid", render_asciidoc(model)),
            ("shape_a", render_asciidoc_shape_a(model)),
            ("shape_b", render_asciidoc_shape_b(model)),
            ("summary", render_asciidoc_summary(&build_summary(view))),
            ("gaps", render_asciidoc_gaps(&build_gaps(view))),
            ("problems", render_asciidoc_problems(&build_problems(view))),
        ]
    }

    // ---- Structure classifier ----

    /// What one rendered document must contain, computed from the model.
    struct Expect {
        title: &'static str,
        /// Every `==` heading line, in order.
        headings: Vec<String>,
        /// The column count of each table, in order.
        columns: Vec<usize>,
    }

    fn document_headings(model: &ReportModel) -> Vec<String> {
        model
            .documents
            .iter()
            .map(|section| {
                let name = section
                    .name
                    .as_deref()
                    .map_or_else(|| "not recorded".to_string(), escape_adoc);
                format!("== Document: {name}")
            })
            .collect()
    }

    fn expect_for(name: &str, model: &ReportModel, selection: &RunSelection) -> Expect {
        let view = ViewInput { model, selection };
        match name {
            "grid" | "shape_a" | "shape_b" => {
                let mut headings = vec!["== Inputs".to_string(), "== Report header".to_string()];
                headings.extend(document_headings(model));
                let mut columns = vec![2];
                for section in model.documents.iter().filter(|d| !d.rows.is_empty()) {
                    match name {
                        "grid" => columns.push(8),
                        "shape_a" => columns.push(3),
                        _ => {
                            for row in &section.rows {
                                columns.push(2);
                                if !row.cases.is_empty() {
                                    columns.push(2);
                                }
                            }
                        }
                    }
                }
                Expect {
                    title: "= Traceability Report",
                    headings,
                    columns,
                }
            }
            "summary" => {
                let report = build_summary(view);
                let mut columns = vec![2];
                if !report.runs.is_empty() {
                    columns.push(8);
                }
                if !report.requirements.is_empty() {
                    columns.push(8);
                }
                Expect {
                    title: "= Run Summary",
                    headings: vec![
                        "== Report header".to_string(),
                        "== Results per run".to_string(),
                        "== Results per requirement".to_string(),
                        "== Linked test cases".to_string(),
                    ],
                    columns,
                }
            }
            "gaps" => {
                let report = build_gaps(view);
                let mut columns = vec![2];
                for list in [
                    &report.no_tests,
                    &report.missing_normal_range,
                    &report.missing_robustness,
                ] {
                    if !list.is_empty() {
                        columns.push(3);
                    }
                }
                if !report.no_result.is_empty() {
                    columns.push(2);
                }
                Expect {
                    title: "= Coverage Gaps",
                    headings: vec![
                        "== Report header".to_string(),
                        format!(
                            "== Requirements with no test cases ({})",
                            report.no_tests.len()
                        ),
                        format!(
                            "== Requirements missing a normal-range test ({})",
                            report.missing_normal_range.len()
                        ),
                        format!(
                            "== Requirements missing a robustness test ({})",
                            report.missing_robustness.len()
                        ),
                        format!(
                            "== Test cases with no recorded result ({})",
                            report.no_result.len()
                        ),
                    ],
                    columns,
                }
            }
            "problems" => {
                let report = build_problems(view);
                let mut columns = vec![2];
                if !report.rows.is_empty() {
                    columns.push(6);
                }
                Expect {
                    title: "= Problem Reports",
                    headings: vec!["== Report header".to_string()],
                    columns,
                }
            }
            other => panic!("unknown renderer {other}"),
        }
    }

    const ADMONITION_LABELS: [&str; 5] = ["NOTE:", "TIP:", "WARNING:", "CAUTION:", "IMPORTANT:"];
    /// The only cell texts exempt from the start-of-text rule, by name: the empty-cell dash and
    /// the visible missing-problem-report flag.
    const EXEMPT_TEXTS: [&str; 2] = ["-", "**MISSING problem_report_id**"];

    /// A cell or paragraph text (after its `| ` marker) must not start with a block, heading,
    /// list, attribute or macro character, nor with an admonition label.
    fn assert_text_ok(text: &str, at: &str) {
        if EXEMPT_TEXTS.contains(&text) {
            return;
        }
        if let Some(first) = text.chars().next() {
            assert!(
                !LINE_START_CHARS.contains(&first),
                "{at}: text starts with a block character: {text:?}"
            );
        }
        assert!(
            !ADMONITION_LABELS
                .iter()
                .any(|label| text.starts_with(label)),
            "{at}: admonition label: {text:?}"
        );
        assert!(
            !["include::", "image::", "ifdef::"]
                .iter()
                .any(|mac| text.starts_with(mac)),
            "{at}: macro: {text:?}"
        );
    }

    /// Number of cells a header line holds: `| A | B | C |` holds three.
    fn header_cell_count(text: &str) -> usize {
        text.trim_end_matches(" |").split(" | ").count()
    }

    fn is_count_line(line: &str) -> bool {
        let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        if let Some(n) = line.strip_prefix("Failed or errored results: ") {
            return digits(n);
        }
        if let Some(n) = line.strip_prefix("Without problem report: ") {
            return digits(n);
        }
        let parts: Vec<&str> = line.split(", ").collect();
        let labels = ["pass ", "fail ", "not_executed ", "error ", "total "];
        parts.len() == labels.len()
            && parts
                .iter()
                .zip(labels)
                .all(|(part, label)| part.strip_prefix(label).is_some_and(digits))
    }

    fn is_fixed_paragraph(line: &str) -> bool {
        [
            "No traceable content",
            "No runs loaded",
            "No requirements",
            "None",
            "No failed or errored results",
        ]
        .contains(&line)
            || is_count_line(line)
    }

    /// Classify every line of one rendered document. Each line must be a title, a heading from
    /// the model, an attribute line, a fence, a cell line, a hard-break continuation inside a
    /// table, or a paragraph line of the Inputs block or a fixed word or count line.
    fn check_structure(text: &str, expect: &Expect) {
        assert!(
            text.ends_with('\n') && !text.ends_with("\n\n"),
            "one trailing newline"
        );
        let mut headings: Vec<String> = Vec::new();
        let mut tables: Vec<usize> = Vec::new();
        let mut in_inputs = false;
        let mut pending_columns: Option<usize> = None;
        let mut in_table = false;
        let mut table_columns = 0usize;
        let mut cells = 0usize;
        let mut header_next = false;

        for (idx, line) in text.lines().enumerate() {
            let at = format!("line {} {line:?}", idx + 1);
            if in_table {
                if line == "|===" {
                    assert!(
                        table_columns > 0 && cells.is_multiple_of(table_columns),
                        "{at}: {cells} cells for {table_columns} columns"
                    );
                    tables.push(table_columns);
                    in_table = false;
                } else if let Some(cell) = line.strip_prefix("| ") {
                    if header_next {
                        // The header row carries one pipe per column; it is asserted by literal.
                        header_next = false;
                        cells += header_cell_count(cell);
                    } else {
                        assert!(
                            line.matches('|').count() == 1,
                            "{at}: body cell line must hold exactly one pipe"
                        );
                        cells += 1;
                    }
                    assert_text_ok(cell, &at);
                } else {
                    assert!(
                        !line.is_empty() && !line.contains('|'),
                        "{at}: not a continuation line"
                    );
                    assert_text_ok(line, &at);
                }
                continue;
            }
            if line.is_empty() {
                continue;
            }
            if idx == 0 {
                assert_eq!(line, expect.title, "{at}: title");
                continue;
            }
            if line.starts_with("== ") {
                headings.push(line.to_string());
                in_inputs = line == "== Inputs";
                continue;
            }
            assert!(
                !line.starts_with('='),
                "{at}: heading or title that is not expected"
            );
            if let Some(rest) = line.strip_prefix("[cols=\"") {
                let columns: usize = rest
                    .split('"')
                    .next()
                    .and_then(|c| c.parse().ok())
                    .unwrap_or_else(|| panic!("{at}: bad attribute line"));
                assert_eq!(
                    line,
                    format!("[cols=\"{columns}\",options=\"header\"]"),
                    "{at}"
                );
                pending_columns = Some(columns);
                in_inputs = false;
                continue;
            }
            if line == "|===" {
                let columns = pending_columns
                    .take()
                    .unwrap_or_else(|| panic!("{at}: table fence without attribute line"));
                in_table = true;
                table_columns = columns;
                cells = 0;
                header_next = true;
                continue;
            }
            if in_inputs {
                assert_text_ok(line, &at);
                continue;
            }
            assert!(is_fixed_paragraph(line), "{at}: unexpected line");
            assert_text_ok(line, &at);
        }
        assert!(!in_table, "unclosed table");
        assert!(pending_columns.is_none(), "attribute line without table");
        assert_eq!(headings, expect.headings, "section headings");
        assert_eq!(tables, expect.columns, "tables and their column counts");
    }

    /// Output-wide rules: no live markup, macro or admonition, and every `&` starts a numeric
    /// reference.
    fn check_safety(text: &str) {
        for banned in [
            "<<",
            "[[",
            "{",
            "}",
            "<",
            ">",
            "+++",
            "include::",
            "image::",
            "ifdef::",
            "NOTE:",
            "TIP:",
            "WARNING:",
            "CAUTION:",
            "IMPORTANT:",
        ] {
            assert!(!text.contains(banned), "banned {banned:?} in output");
        }
        for line in text.lines() {
            if line.starts_with("[cols=") || line == "|===" {
                continue;
            }
            let body = line.strip_prefix("| ").unwrap_or(line);
            let body = body
                .replace("**MISSING problem_report_id**", "")
                .replace(" | ", " ");
            let body = body.trim_end_matches(" |");
            let plain = strip_references(body);
            assert!(
                !plain.chars().any(|c| "&<>*`#^~[]{}\\|@".contains(c)),
                "markup character left unescaped in {line:?}"
            );
        }
    }

    /// Remove every `&#NN;` reference from a line.
    fn strip_references(text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find("&#") {
            out.push_str(&rest[..start]);
            match rest[start..].find(';') {
                Some(end) => rest = &rest[start + end + 1..],
                None => {
                    rest = &rest[start..];
                    break;
                }
            }
        }
        out.push_str(rest);
        out
    }

    /// Every value a renderer shows must appear in its escaped form.
    fn check_values(name: &str, text: &str, model: &ReportModel, selection: &RunSelection) {
        let view = ViewInput { model, selection };
        let mut shown: Vec<String> = vec![model.header.matrix_id.clone()];
        shown.extend(model.header.run_ids.iter().cloned());
        shown.extend(model.header.build_ids.iter().cloned());
        shown.extend(model.header.environments.iter().cloned());
        shown.extend(model.header.testers.iter().cloned());
        match name {
            "grid" | "shape_a" | "shape_b" => {
                for section in &model.documents {
                    shown.extend(section.name.iter().cloned());
                    for row in &section.rows {
                        shown.push(row.requirement_id.clone());
                        shown.extend(row.cases.iter().map(|c| c.test_case_id.clone()));
                    }
                }
            }
            "summary" => {
                shown.extend(
                    build_summary(view)
                        .requirements
                        .into_iter()
                        .map(|r| r.requirement_id),
                );
            }
            "gaps" => {
                let report = build_gaps(view);
                for item in report
                    .no_tests
                    .iter()
                    .chain(&report.missing_normal_range)
                    .chain(&report.missing_robustness)
                {
                    shown.push(item.requirement_id.clone());
                    shown.extend(item.document.iter().cloned());
                }
                shown.extend(report.no_result.iter().map(|c| c.test_case_id.clone()));
            }
            "problems" => {
                for row in build_problems(view).rows {
                    shown.push(row.test_case_id);
                    shown.extend(row.problem_report_id);
                    shown.extend(row.requirement_ids);
                }
            }
            other => panic!("unknown renderer {other}"),
        }
        for value in shown {
            let escaped = escape_adoc(&value);
            assert!(text.contains(&escaped), "{name}: {escaped:?} missing");
        }
    }

    // ---- Hostile strings and structure, all six renderers ----

    #[test]
    fn hostile_strings_keep_structure_in_all_six_renderers() {
        let (model, selection) = hostile_pipeline(false, GENERATED);
        for (name, text) in six_outputs(&model, &selection) {
            let expect = expect_for(name, &model, &selection);
            check_structure(&text, &expect);
            check_safety(&text);
            check_values(name, &text, &model, &selection);
        }
        for value in hostile_values() {
            if value.chars().any(|c| SPECIAL.contains(c)) {
                assert_ne!(escape_adoc(&value), value, "{value:?} is not escaped");
            }
        }
    }

    // ---- Honesty: NOT RUN is never pass ----

    #[test]
    fn not_run_is_never_pass_and_verified_rows_list_only_pass() {
        let m = golden_model();
        let grid = render_asciidoc(&m);
        let shape_a = render_asciidoc_shape_a(&m);
        let shape_b = render_asciidoc_shape_b(&m);
        for text in [&grid, &shape_a] {
            assert!(text.contains("T-4 (NOT RUN"), "{text}");
            assert!(!text.contains("T-4 (pass"), "{text}");
            assert!(
                text.contains("T-1 (pass") && text.contains("T-2 (pass"),
                "{text}"
            );
        }
        assert!(shape_b.contains("| T-4\n| NOT RUN\n"), "{shape_b}");
        assert!(!shape_b.contains("| T-4\n| pass\n"), "{shape_b}");
        assert!(shape_b.contains("| T-1\n| pass\n"), "{shape_b}");

        let (hostile, selection) = hostile_pipeline(false, GENERATED);
        let not_run = escape_adoc(&hostile_values()[0]);
        let hostile_a = render_asciidoc_shape_a(&hostile);
        assert!(
            hostile_a.contains(&format!("{not_run} (NOT RUN)")),
            "{hostile_a}"
        );
        assert!(
            !hostile_a.contains(&format!("{not_run} (pass)")),
            "{hostile_a}"
        );
        for row in hostile.documents.iter().flat_map(|d| &d.rows) {
            if row.status == VerificationStatus::Verified {
                assert!(row.cases.iter().all(|c| c.outcome == CaseOutcome::Pass));
            }
        }
        assert!(!selection.results.is_empty());
    }

    // ---- Determinism (D-05) ----

    #[test]
    fn byte_identical_for_separately_built_inputs_all_six() {
        let (first_model, first_selection) = hostile_pipeline(false, GENERATED);
        let (second_model, second_selection) = hostile_pipeline(false, GENERATED);
        let first = six_outputs(&first_model, &first_selection);
        let second = six_outputs(&second_model, &second_selection);
        for ((name, a), (_, b)) in first.iter().zip(&second) {
            assert_eq!(a.as_bytes(), b.as_bytes(), "{name}");
        }
    }

    #[test]
    fn shuffled_links_cases_and_runs_give_same_bytes_all_six() {
        let (sorted_model, sorted_selection) = hostile_pipeline(false, GENERATED);
        let (shuffled_model, shuffled_selection) = hostile_pipeline(true, GENERATED);
        let sorted = six_outputs(&sorted_model, &sorted_selection);
        let shuffled = six_outputs(&shuffled_model, &shuffled_selection);
        for ((name, a), (_, b)) in sorted.iter().zip(&shuffled) {
            assert_eq!(a.as_bytes(), b.as_bytes(), "{name}");
        }
    }

    #[test]
    fn different_generated_at_changes_only_the_generated_row() {
        let (first_model, first_selection) = hostile_pipeline(false, GENERATED);
        let (second_model, second_selection) = hostile_pipeline(false, "2026-11-01T08:30:00Z");
        let first = six_outputs(&first_model, &first_selection);
        let second = six_outputs(&second_model, &second_selection);
        for ((name, a), (_, b)) in first.iter().zip(&second) {
            assert_ne!(a, b, "{name}: generated_at must appear");
            let la: Vec<&str> = a.lines().collect();
            let lb: Vec<&str> = b.lines().collect();
            assert_eq!(la.len(), lb.len(), "{name}");
            for (i, (x, y)) in la.iter().zip(&lb).enumerate() {
                if x != y {
                    assert!(
                        i > 0 && la[i - 1] == "| Generated",
                        "{name}: line {} differs outside the Generated row: {x:?} / {y:?}",
                        i + 1
                    );
                }
            }
        }
    }
}
