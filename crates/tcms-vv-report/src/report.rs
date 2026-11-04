//! The report as format-neutral blocks, in the fixed section order of TCMS-47.1. Every value is read
//! from the metrics document; this module adds no figure of its own.

use crate::input::{Metrics, Percent};

/// One block of the report. Cells are plain text; the renderer escapes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// The document title.
    Title(String),
    /// A section heading.
    Heading(String),
    /// A paragraph.
    Text(String),
    /// A table: one header row and zero or more rows of the same width.
    Table {
        header: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

pub fn blocks(m: &Metrics) -> Vec<Block> {
    let mut out = vec![Block::Title("V&V metrics report".into())];
    header(m, &mut out);
    gate(m, &mut out);
    vv_run(m, &mut out);
    requirements(m, &mut out);
    stability(m, &mut out);
    load_summary(m, &mut out);
    out
}

fn two_column(rows: Vec<(&str, String)>) -> Block {
    Block::Table {
        header: vec!["Measure".into(), "Value".into()],
        rows: rows
            .into_iter()
            .map(|(label, value)| vec![label.to_string(), value])
            .collect(),
    }
}

/// A pipe is escaped (`\|`) and a line break becomes a space, so a cell stays on one table row.
/// Shared by the Markdown and AsciiDoc table renderers.
pub(crate) fn escape_cell(cell: &str) -> String {
    cell.replace('|', "\\|")
        .replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
}

fn header(m: &Metrics, out: &mut Vec<Block>) {
    let threshold = &m.parameters.gate_threshold_percent;
    out.push(Block::Heading("Run".into()));
    out.push(Block::Table {
        header: vec!["Item".into(), "Value".into()],
        rows: vec![
            vec!["Run".into(), m.run.run_id.clone()],
            vec!["Started".into(), m.run.started_at.clone()],
            vec![
                "Finished".into(),
                m.run
                    .finished_at
                    .clone()
                    .unwrap_or_else(|| "in progress".into()),
            ],
            vec!["Provisional".into(), yes_no(m.run.provisional)],
            vec!["Generated".into(), m.generated_at.clone()],
            vec![
                "Gate threshold".into(),
                format!("{}% ({})", m.gate.gate_threshold_percent, threshold.source),
            ],
        ],
    });
}

fn gate(m: &Metrics, out: &mut Vec<Block>) {
    let g = &m.gate;
    out.push(Block::Heading("Gate verdict".into()));
    out.push(two_column(vec![
        ("Verdict", g.verdict.clone()),
        (
            "Gate pass rate",
            g.pass_rate
                .as_ref()
                .map_or_else(|| "NO_DATA".into(), |p| percent_text(Some(p))),
        ),
        (
            "Denominator (passed + failed + not executed)",
            g.denominator.to_string(),
        ),
        (
            "Counts: passed / failed / not executed / skipped",
            format!(
                "{} / {} / {} / {}",
                g.counts.passed, g.counts.failed, g.counts.not_executed, g.counts.skipped
            ),
        ),
        (
            "Delta from previous run",
            match g.delta_from_previous {
                None => "n/a".into(),
                Some(d) if d > 0 => format!("+{d}"),
                Some(d) => d.to_string(),
            },
        ),
        (
            "Previous run",
            g.previous_run_id.clone().unwrap_or_else(|| "none".into()),
        ),
    ]));
}

fn vv_run(m: &Metrics, out: &mut Vec<Block>) {
    let v = &m.vv_run;
    out.push(Block::Heading("V&V run figures".into()));
    out.push(two_column(vec![
        (
            "V&V test-case pass rate",
            percent_text(v.vv_pass_rate.as_ref()),
        ),
        (
            "Test cases passed / total",
            format!("{} / {}", v.passed_test_cases, v.total_test_cases),
        ),
        ("Step coverage", percent_text(v.step_coverage.as_ref())),
        (
            "Steps passed / total",
            format!("{} / {}", v.passed_steps, v.total_steps),
        ),
    ]));
}

fn requirements(m: &Metrics, out: &mut Vec<Block>) {
    let s = &m.requirements.summary;
    out.push(Block::Heading("Requirement coverage".into()));
    out.push(two_column(vec![
        ("Requirements", s.total.to_string()),
        ("Covered", s.covered.to_string()),
        ("Failing", s.failing.to_string()),
        ("Not run", s.not_run.to_string()),
        ("No cases", s.no_cases.to_string()),
        ("With problem report", s.with_defect.to_string()),
        ("Open problem reports", s.open_defects.to_string()),
    ]));
    out.push(Block::Table {
        header: vec![
            "Requirement".into(),
            "Coverage".into(),
            "Executed / total".into(),
            "Problem reports".into(),
            "Open problem reports".into(),
        ],
        rows: m
            .requirements
            .items
            .iter()
            .map(|r| {
                vec![
                    r.id.clone(),
                    r.coverage.clone(),
                    format!("{} / {}", r.executed, r.total),
                    list_text(&r.problem_reports),
                    list_text(&r.open_problem_reports),
                ]
            })
            .collect(),
    });
}

fn stability(m: &Metrics, out: &mut Vec<Block>) {
    let s = &m.stability;
    out.push(Block::Heading("Stability".into()));
    out.push(two_column(vec![
        (
            "Flaky threshold (flips)",
            format!(
                "{} ({})",
                s.flaky_threshold, m.parameters.flaky_threshold.source
            ),
        ),
        ("Flaky cases", s.flaky_count.to_string()),
    ]));
    let flagged: Vec<Vec<String>> = s
        .cases
        .iter()
        .filter(|c| c.stability == "FLAKY" || c.stability == "FAILING")
        .map(|c| vec![c.case_id.clone(), c.stability.clone(), c.flips.to_string()])
        .collect();
    if flagged.is_empty() {
        out.push(Block::Text(
            "No flaky or failing cases in the selected run.".into(),
        ));
    } else {
        out.push(Block::Text("Cases with stability FLAKY or FAILING:".into()));
        out.push(Block::Table {
            header: vec!["Case".into(), "Stability".into(), "Flips".into()],
            rows: flagged,
        });
    }
}

fn load_summary(m: &Metrics, out: &mut Vec<Block>) {
    let l = &m.load_summary;
    let first = l.schema_errors.first();
    out.push(Block::Heading("Load summary".into()));
    out.push(two_column(vec![
        ("Distinct cases", l.distinct_cases.to_string()),
        ("Runs recorded", l.runs_recorded.to_string()),
        ("Schema errors", l.schema_error_count.to_string()),
        (
            "First schema error file",
            first.map_or_else(|| "none".into(), |e| e.file.clone()),
        ),
        (
            "First schema error",
            first.map_or_else(|| "none".into(), |e| e.error.clone()),
        ),
    ]));
}

fn yes_no(flag: bool) -> String {
    if flag { "yes" } else { "no" }.into()
}

fn list_text(items: &[String]) -> String {
    if items.is_empty() {
        "-".into()
    } else {
        items.join(", ")
    }
}

/// The display figure and the exact figure of a percentage, or `n/a` when it is null.
fn percent_text(p: Option<&Percent>) -> String {
    match p {
        None => "n/a".into(),
        Some(p) => format!("{}% (exact {}%)", p.display, exact_text(p.exact)),
    }
}

/// The exact figure as the JSON document writes it (for example `100.0`, `83.33333333333333`).
fn exact_text(value: f64) -> String {
    serde_json::Number::from_f64(value).map_or_else(|| format!("{value:?}"), |n| n.to_string())
}
