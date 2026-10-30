//! Markdown renderer for the traceability report (D-07..D-14).
//!
//! Reads `ReportModel` only. No clock, no environment lookups, no hash-ordered iteration, so the
//! same model always gives the same bytes (D-12). Markdown is the only format in Phase 2 (D-13).

use crate::model::{DocumentSection, Header, ReportModel, RequirementRow, TestColumn};

const NOT_RECORDED: &str = "not recorded";
const NO_CONTENT: &str = "No traceable content";
const EMPTY_CELL: &str = "-";
const TABLE_HEADER: &str = "| Requirement | Level | Status | Normal-range test | Robustness test | normal_range | robustness | other |";
const TABLE_RULE: &str = "| --- | --- | --- | --- | --- | --- | --- | --- |";

/// Render the whole report as one Markdown document ending with exactly one newline.
pub fn render_markdown(model: &ReportModel) -> String {
    let mut blocks: Vec<String> = vec![
        "# Traceability Report".to_string(),
        inputs_block(&model.inputs.details),
        header_block(&model.header),
    ];
    blocks.extend(model.documents.iter().map(document_block));

    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

/// The Phase 1 input text, verbatim, inside a fence longer than any backtick run in it (D-14).
fn inputs_block(details: &str) -> String {
    let fence = "`".repeat(fence_len(details));
    // One trailing newline is structural (the line break before the fence), not content.
    let body = details.strip_suffix('\n').unwrap_or(details);
    format!("## Inputs\n\n{fence}\n{body}\n{fence}")
}

/// Fence length: at least three, and one more than the longest run of backticks in `text`.
fn fence_len(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    (longest + 1).max(3)
}

fn header_block(header: &Header) -> String {
    let rows = [
        ("Matrix", cell(&header.matrix_id)),
        (
            "Baseline",
            header
                .baseline_id
                .as_deref()
                .map_or_else(|| NOT_RECORDED.to_string(), cell),
        ),
        ("Runs", list(&header.run_ids)),
        ("Builds", list(&header.build_ids)),
        ("Environments", list(&header.environments)),
        ("Testers", list(&header.testers)),
        (
            "Obsolete test cases excluded",
            header
                .obsolete_excluded
                .map_or_else(|| NOT_RECORDED.to_string(), |n| n.to_string()),
        ),
        ("Generated", cell(&header.generated_at)),
    ];

    let mut lines = vec![
        "## Report header".to_string(),
        String::new(),
        "| Field | Value |".to_string(),
        "| --- | --- |".to_string(),
    ];
    lines.extend(
        rows.iter()
            .map(|(field, value)| format!("| {field} | {value} |")),
    );
    lines.join("\n")
}

fn document_block(section: &DocumentSection) -> String {
    let name = section
        .name
        .as_deref()
        .map_or_else(|| NOT_RECORDED.to_string(), cell);
    let body = if section.rows.is_empty() {
        NO_CONTENT.to_string()
    } else {
        let mut lines = vec![TABLE_HEADER.to_string(), TABLE_RULE.to_string()];
        lines.extend(section.rows.iter().map(row_line));
        lines.join("\n")
    };
    format!("## Document: {name}\n\n{body}")
}

fn row_line(row: &RequirementRow) -> String {
    let mut cells = vec![
        cell(&row.requirement_id),
        row.level.to_string(),
        row.status.label().to_string(),
        yes_no(row.normal_range_covered).to_string(),
        yes_no(row.robustness_covered).to_string(),
    ];
    cells.extend(
        TestColumn::ALL
            .iter()
            .map(|&column| column_cell(row, column)),
    );
    format!("| {} |", cells.join(" | "))
}

/// The cases of one test-type column; `-` when the column has none (TRC-04).
fn column_cell(row: &RequirementRow, column: TestColumn) -> String {
    let entries: Vec<String> = row
        .cases
        .iter()
        .filter(|case| case.column == column)
        .map(|case| {
            let run = case
                .run_id
                .as_deref()
                .map_or_else(|| "no result".to_string(), cell);
            format!(
                "{} ({}, {run})",
                cell(&case.test_case_id),
                case.outcome.label()
            )
        })
        .collect();
    if entries.is_empty() {
        EMPTY_CELL.to_string()
    } else {
        entries.join("<br>")
    }
}

fn yes_no(covered: bool) -> &'static str {
    if covered {
        "yes"
    } else {
        "no"
    }
}

/// Values joined with `, `; an empty list is "not recorded" (D-11).
fn list(values: &[String]) -> String {
    if values.is_empty() {
        NOT_RECORDED.to_string()
    } else {
        values
            .iter()
            .map(|v| cell(v))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Make one model-derived value safe inside a table cell: `|` is escaped, and line breaks become
/// a space so a value cannot forge extra columns or rows (T-02-06). This is table safety only.
fn cell(text: &str) -> String {
    text.replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
        .replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CaseEntry, DocumentSection, Header, ReportModel, RequirementRow};
    use crate::rollup::{CaseOutcome, VerificationStatus};
    use testcase_verification::RequirementLevel;

    /// Hand-built model: one populated document, one empty document, every header field set
    /// except the baseline, and one obsolete count.
    fn golden_model() -> ReportModel {
        ReportModel {
            inputs: crate::model::InputsSection {
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

    #[test]
    fn golden_layout_matches_exact_text() {
        let expected = r#"# Traceability Report

## Inputs

```
Trace matrix: TM-1 (baseline: not recorded)
Runs selected: RUN-1, RUN-2
```

## Report header

| Field | Value |
| --- | --- |
| Matrix | TM-1 |
| Baseline | not recorded |
| Runs | RUN-1, RUN-2 |
| Builds | b1, b2 |
| Environments | lab-1 |
| Testers | alice |
| Obsolete test cases excluded | 2 |
| Generated | 2026-10-31T09:00:00Z |

## Document: docs/srs.sdoc

| Requirement | Level | Status | Normal-range test | Robustness test | normal_range | robustness | other |
| --- | --- | --- | --- | --- | --- | --- | --- |
| HLR-1 | HLR | VERIFIED | yes | yes | T1 (pass, RUN-1) | T2 (pass, RUN-2) | - |
| LLR-2 | LLR | FAILED | yes | no | - | - | T3 (fail, RUN-2)<br>T4 (NOT RUN, no result) |

## Document: docs/empty.sdoc

No traceable content
"#;
        assert_eq!(render_markdown(&golden_model()), expected);
    }
}
