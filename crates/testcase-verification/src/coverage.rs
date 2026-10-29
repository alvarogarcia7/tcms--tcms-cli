//! Import of coverage tool output into a tcms-schemas `coverage-report` (TCMS-40.10).
//!
//! Inputs (all optional, but at least one source is needed so the report has requirements-based
//! or structural figures):
//!
//! * LCOV tracefile (`SF`, `DA`, `BRDA`, `end_of_record`): statement coverage from `DA`
//!   records, decision coverage from `BRDA` records, per source file as a code unit.
//! * MC/DC side input (YAML): LCOV has no MC/DC, no standard format exists, so the figures
//!   and the uncovered conditions come from a small documented YAML file ([`McdcInput`]).
//! * A traceability matrix (see [`crate::TraceMatrix`]): requirements-based coverage.
//! * Gap annotations (YAML): the disposition of each gap, see [`GapAnnotation`].
//!
//! A gap is never patched away: every detected gap needs a classification and a rationale from
//! the annotations. Gaps without one are returned in [`CoverageFindings::unclassified`] and
//! are not part of the report, because the schema requires every gap to carry both.

use crate::trace_matrix::TraceMatrix;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

const CLASSIFICATIONS: [&str; 6] = [
    "missing_test",
    "missing_requirement",
    "dead_code",
    "deactivated_code",
    "justified",
    "tool_limitation",
];

/// Coverage kinds of the schema's `gaps[].coverage_type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageType {
    Requirements,
    Statement,
    Decision,
    Mcdc,
    DataCoupling,
    ControlCoupling,
}

impl std::fmt::Display for CoverageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Requirements => "requirements",
            Self::Statement => "statement",
            Self::Decision => "decision",
            Self::Mcdc => "mcdc",
            Self::DataCoupling => "data_coupling",
            Self::ControlCoupling => "control_coupling",
        })
    }
}

/// Percentages with two decimals
fn percent(covered: usize, total: usize) -> Option<f64> {
    (total > 0).then(|| ((covered as f64 / total as f64) * 10000.0).round() / 100.0)
}

/// One source file of an LCOV tracefile
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LcovFile {
    pub path: String,
    /// (line, hit count) in file order
    pub lines: Vec<(u32, u64)>,
    /// (line, block, branch, taken) in file order; not taken is 0
    pub branches: Vec<(u32, String, String, u64)>,
}

/// A parsed LCOV tracefile
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lcov {
    pub files: Vec<LcovFile>,
}

impl Lcov {
    /// Parse LCOV text. Unknown record types (`FN`, `FNDA`, `TN`, `LF`, ...) are ignored; the
    /// totals are computed from `DA` and `BRDA`, not taken from `LF`/`LH`/`BRF`/`BRH`.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let mut files: Vec<LcovFile> = Vec::new();
        let mut current: Option<LcovFile> = None;
        for (number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            let at = || format!("LCOV line {}: '{}'", number + 1, line);
            if let Some(path) = line.strip_prefix("SF:") {
                if current.is_some() {
                    bail!("{}: SF before end_of_record", at());
                }
                current = Some(LcovFile {
                    path: path.to_string(),
                    ..Default::default()
                });
            } else if line == "end_of_record" {
                let file = current
                    .take()
                    .with_context(|| format!("{}: end_of_record without SF", at()))?;
                match files.iter_mut().find(|f| f.path == file.path) {
                    // the same file from several test runs: keep both observations
                    Some(existing) => {
                        existing.lines.extend(file.lines);
                        existing.branches.extend(file.branches);
                    }
                    None => files.push(file),
                }
            } else if let Some(data) = line.strip_prefix("DA:") {
                let file = current
                    .as_mut()
                    .with_context(|| format!("{}: DA outside a record", at()))?;
                let mut parts = data.split(',');
                let (l, c) = (parts.next(), parts.next());
                let parsed = l
                    .and_then(|l| l.trim().parse::<u32>().ok())
                    .zip(c.and_then(|c| c.trim().parse::<u64>().ok()));
                match parsed {
                    Some(entry) => file.lines.push(entry),
                    None => bail!("{}: expected DA:<line>,<count>", at()),
                }
            } else if let Some(data) = line.strip_prefix("BRDA:") {
                let file = current
                    .as_mut()
                    .with_context(|| format!("{}: BRDA outside a record", at()))?;
                let parts: Vec<&str> = data.split(',').map(str::trim).collect();
                let entry = (parts.len() == 4)
                    .then(|| {
                        let taken = match parts[3] {
                            "-" => Some(0),
                            n => n.parse::<u64>().ok(),
                        };
                        parts[0]
                            .parse::<u32>()
                            .ok()
                            .zip(taken)
                            .map(|(l, t)| (l, parts[1].to_string(), parts[2].to_string(), t))
                    })
                    .flatten();
                match entry {
                    Some(entry) => file.branches.push(entry),
                    None => bail!("{}: expected BRDA:<line>,<block>,<branch>,<taken|->", at()),
                }
            }
        }
        if current.is_some() {
            bail!("LCOV input ends inside a record (missing end_of_record)");
        }
        if files.is_empty() {
            bail!("LCOV input contains no SF record");
        }
        Ok(Self { files })
    }

    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read LCOV file {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("Invalid LCOV file {}", path.display()))
    }
}

/// MC/DC side input. LCOV cannot express MC/DC and there is no standard tool format, so the
/// MC/DC figures and the uncovered conditions are handed over in this small YAML file:
///
/// ```yaml
/// mcdc_percent: 96.2            # overall, optional
/// units:                        # optional, per code unit
///   - code_unit_id: src/es6.c
///     mcdc_percent: 95.0
/// gaps:                         # optional, conditions without independence pair
///   - location: src/es6.c:88
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McdcInput {
    #[serde(default)]
    pub mcdc_percent: Option<f64>,
    #[serde(default)]
    pub units: Vec<McdcUnit>,
    #[serde(default)]
    pub gaps: Vec<McdcGap>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McdcUnit {
    pub code_unit_id: String,
    pub mcdc_percent: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McdcGap {
    pub location: String,
    #[serde(default)]
    pub code_unit_id: Option<String>,
}

impl McdcInput {
    pub fn read(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read MC/DC file {}", path.display()))?;
        let input: Self = serde_yaml::from_str(&text)
            .with_context(|| format!("Invalid MC/DC file {}", path.display()))?;
        let bad = input
            .mcdc_percent
            .iter()
            .copied()
            .chain(input.units.iter().map(|u| u.mcdc_percent))
            .find(|p| !(0.0..=100.0).contains(p));
        if let Some(p) = bad {
            bail!(
                "Invalid MC/DC file {}: {} is not a percentage (0 to 100)",
                path.display(),
                p
            );
        }
        Ok(input)
    }
}

/// The disposition of a gap, from the annotation file.
///
/// A gap is matched by `location` (and `coverage_type` when given): the exact location of the
/// gap, a source file path (all gaps of that file), or `file:LINE` inside a gap's line range.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GapAnnotation {
    pub location: String,
    #[serde(default)]
    pub coverage_type: Option<CoverageType>,
    pub classification: String,
    pub rationale: String,
    #[serde(default)]
    pub approver: Option<String>,
    #[serde(default)]
    pub code_unit_id: Option<String>,
    #[serde(default)]
    pub requirement_id: Option<String>,
    #[serde(default)]
    pub test_case_id: Option<String>,
    #[serde(default)]
    pub problem_report_id: Option<String>,
    #[serde(default)]
    pub action_item_id: Option<String>,
}

/// Read a gap annotation file (a YAML list, or a mapping with an `annotations` list) and check
/// each entry: known classification, non-empty rationale, an approver for `justified`.
pub fn read_annotations(path: &Path) -> anyhow::Result<Vec<GapAnnotation>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read annotation file {}", path.display()))?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .with_context(|| format!("Invalid annotation file {}", path.display()))?;
    let entries = match value {
        serde_yaml::Value::Sequence(entries) => entries,
        serde_yaml::Value::Mapping(map) => match map.get("annotations") {
            Some(serde_yaml::Value::Sequence(entries)) => entries.clone(),
            _ => bail!(
                "Invalid annotation file {}: expected a list or a mapping with an 'annotations' list",
                path.display()
            ),
        },
        _ => bail!(
            "Invalid annotation file {}: expected a list of annotations",
            path.display()
        ),
    };
    let mut annotations = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        annotations.push(
            serde_yaml::from_value::<GapAnnotation>(entry).with_context(|| {
                format!(
                "Invalid annotation {} in {} (location, classification and rationale are required)",
                index + 1,
                path.display()
            )
            })?,
        );
    }
    for a in &annotations {
        let origin = format!("annotation for '{}' in {}", a.location, path.display());
        if !CLASSIFICATIONS.contains(&a.classification.as_str()) {
            bail!(
                "{origin}: unknown classification '{}' (use {})",
                a.classification,
                CLASSIFICATIONS.join(", ")
            );
        }
        if a.rationale.trim().is_empty() {
            bail!("{origin}: a rationale is required");
        }
        if a.classification == "justified"
            && a.approver.as_deref().is_none_or(|s| s.trim().is_empty())
        {
            bail!("{origin}: a justified gap needs an approver");
        }
    }
    Ok(annotations)
}

/// A gap found in the coverage data, before any disposition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedGap {
    pub location: String,
    pub coverage_type: CoverageType,
    pub code_unit_id: Option<String>,
    pub requirement_id: Option<String>,
}

/// A gap with its disposition, as in the schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageGapRecord {
    pub location: String,
    pub coverage_type: CoverageType,
    pub classification: String,
    pub rationale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_unit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_case_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem_report_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_item_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct RequirementsBased {
    pub total_requirements: usize,
    pub covered_normal_range: usize,
    pub covered_robustness: usize,
    pub uncovered: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Structural {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcdc_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerCodeUnit {
    pub code_unit_id: String,
    #[serde(flatten)]
    pub figures: Structural,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerRequirement {
    pub requirement_id: String,
    pub normal_range_covered: bool,
    pub robustness_covered: bool,
    pub test_case_ids: Vec<String>,
}

/// The coverage report, in the form of the tcms-schemas `coverage-report` schema
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoverageReport {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub schema: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub baseline_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirements_based: Option<RequirementsBased>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural: Option<Structural>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub per_code_unit: Vec<PerCodeUnit>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub per_requirement: Vec<PerRequirement>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<CoverageGapRecord>,
}

/// What is wrong besides the report
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageFindings {
    /// Gaps no annotation dispositions; they are not in the report
    pub unclassified: Vec<DetectedGap>,
    /// Annotations that matched no gap (stale or mistyped)
    pub unused_annotations: Vec<String>,
}

/// Everything the report is built from
#[derive(Debug, Default)]
pub struct CoverageInput {
    pub id: String,
    pub baseline_id: String,
    pub run_id: Option<String>,
    pub tool: Option<String>,
    pub tool_version: Option<String>,
    pub lcov: Option<Lcov>,
    pub mcdc: Option<McdcInput>,
    pub matrix: Option<TraceMatrix>,
    pub annotations: Vec<GapAnnotation>,
}

/// `file:LINE`, `file:A-B` or a bare name: (file, first line, last line)
fn split_location(location: &str) -> (&str, Option<(u32, u32)>) {
    if let Some((file, tail)) = location.rsplit_once(':') {
        let range = match tail.split_once('-') {
            Some((a, b)) => a.parse().ok().zip(b.parse().ok()),
            None => tail.parse().ok().map(|n| (n, n)),
        };
        if let Some(range) = range {
            return (file, Some(range));
        }
    }
    (location, None)
}

fn annotation_matches(annotation: &GapAnnotation, gap: &DetectedGap) -> bool {
    if let Some(kind) = annotation.coverage_type {
        if kind != gap.coverage_type {
            return false;
        }
    }
    if annotation.location == gap.location {
        return true;
    }
    let (a_file, a_range) = split_location(&annotation.location);
    let (g_file, g_range) = split_location(&gap.location);
    if a_file != g_file {
        return false;
    }
    match (a_range, g_range) {
        (None, _) => true,
        (Some((a1, a2)), Some((g1, g2))) => a1 >= g1 && a2 <= g2,
        (Some(_), None) => false,
    }
}

fn lcov_figures(lcov: &Lcov) -> (Structural, Vec<PerCodeUnit>, Vec<DetectedGap>) {
    let mut gaps = Vec::new();
    let mut units = Vec::new();
    let (mut lines_total, mut lines_hit, mut br_total, mut br_hit) = (0, 0, 0, 0);
    for file in &lcov.files {
        let covered_lines = file.lines.iter().filter(|(_, c)| *c > 0).count();
        let covered_branches = file.branches.iter().filter(|b| b.3 > 0).count();
        lines_total += file.lines.len();
        lines_hit += covered_lines;
        br_total += file.branches.len();
        br_hit += covered_branches;
        units.push(PerCodeUnit {
            code_unit_id: file.path.clone(),
            figures: Structural {
                statement_percent: percent(covered_lines, file.lines.len()),
                decision_percent: percent(covered_branches, file.branches.len()),
                mcdc_percent: None,
            },
        });

        // runs of uncovered statements without a covered one between them
        let mut run: Option<(u32, u32)> = None;
        let flush = |run: &mut Option<(u32, u32)>, gaps: &mut Vec<DetectedGap>| {
            if let Some((a, b)) = run.take() {
                let location = if a == b {
                    format!("{}:{}", file.path, a)
                } else {
                    format!("{}:{}-{}", file.path, a, b)
                };
                gaps.push(DetectedGap {
                    location,
                    coverage_type: CoverageType::Statement,
                    code_unit_id: Some(file.path.clone()),
                    requirement_id: None,
                });
            }
        };
        for (line, count) in &file.lines {
            if *count == 0 {
                run = Some(match run {
                    Some((a, _)) => (a, *line),
                    None => (*line, *line),
                });
            } else {
                flush(&mut run, &mut gaps);
            }
        }
        flush(&mut run, &mut gaps);

        let mut missed: Vec<u32> = file
            .branches
            .iter()
            .filter(|b| b.3 == 0)
            .map(|b| b.0)
            .collect();
        missed.sort_unstable();
        missed.dedup();
        for line in missed {
            gaps.push(DetectedGap {
                location: format!("{}:{}", file.path, line),
                coverage_type: CoverageType::Decision,
                code_unit_id: Some(file.path.clone()),
                requirement_id: None,
            });
        }
    }
    (
        Structural {
            statement_percent: percent(lines_hit, lines_total),
            decision_percent: percent(br_hit, br_total),
            mcdc_percent: None,
        },
        units,
        gaps,
    )
}

/// Build the report. Errors when there is no source of figures, or when the baseline ID is empty.
pub fn build_coverage_report(
    input: CoverageInput,
) -> anyhow::Result<(CoverageReport, CoverageFindings)> {
    if input.baseline_id.trim().is_empty() {
        bail!("A baseline ID is required (the schema requires baseline_id)");
    }
    if input.lcov.is_none() && input.mcdc.is_none() && input.matrix.is_none() {
        bail!(
            "Nothing to import: give an LCOV file, an MC/DC file or a traceability matrix so \
             the report has structural or requirements-based figures"
        );
    }

    let mut detected: Vec<DetectedGap> = Vec::new();
    let mut structural: Option<Structural> = None;
    let mut per_code_unit: Vec<PerCodeUnit> = Vec::new();

    if let Some(lcov) = &input.lcov {
        let (figures, units, gaps) = lcov_figures(lcov);
        structural = Some(figures);
        per_code_unit = units;
        detected.extend(gaps);
    }
    if let Some(mcdc) = &input.mcdc {
        let figures = structural.get_or_insert_with(Structural::default);
        figures.mcdc_percent = mcdc.mcdc_percent;
        for unit in &mcdc.units {
            match per_code_unit
                .iter_mut()
                .find(|u| u.code_unit_id == unit.code_unit_id)
            {
                Some(existing) => existing.figures.mcdc_percent = Some(unit.mcdc_percent),
                None => per_code_unit.push(PerCodeUnit {
                    code_unit_id: unit.code_unit_id.clone(),
                    figures: Structural {
                        mcdc_percent: Some(unit.mcdc_percent),
                        ..Default::default()
                    },
                }),
            }
        }
        for gap in &mcdc.gaps {
            detected.push(DetectedGap {
                location: gap.location.clone(),
                coverage_type: CoverageType::Mcdc,
                code_unit_id: gap
                    .code_unit_id
                    .clone()
                    .or_else(|| Some(split_location(&gap.location).0.to_string())),
                requirement_id: None,
            });
        }
    }
    if structural == Some(Structural::default()) {
        // a side input without any figure and an LCOV without statements or branches
        structural = None;
    }

    let mut requirements_based = None;
    let mut per_requirement = Vec::new();
    if let Some(matrix) = &input.matrix {
        let mut uncovered = Vec::new();
        for link in &matrix.links {
            if !(link.normal_range_covered && link.robustness_covered) {
                uncovered.push(link.requirement_id.clone());
                detected.push(DetectedGap {
                    location: link.requirement_id.clone(),
                    coverage_type: CoverageType::Requirements,
                    code_unit_id: None,
                    requirement_id: Some(link.requirement_id.clone()),
                });
            }
            per_requirement.push(PerRequirement {
                requirement_id: link.requirement_id.clone(),
                normal_range_covered: link.normal_range_covered,
                robustness_covered: link.robustness_covered,
                test_case_ids: link.test_case_ids.clone(),
            });
        }
        requirements_based = Some(RequirementsBased {
            total_requirements: matrix.links.len(),
            covered_normal_range: matrix
                .links
                .iter()
                .filter(|l| l.normal_range_covered)
                .count(),
            covered_robustness: matrix.links.iter().filter(|l| l.robustness_covered).count(),
            uncovered,
        });
    }
    if structural.is_none() && requirements_based.is_none() {
        bail!(
            "The inputs hold no coverage figures (LCOV without DA/BRDA records and no MC/DC \
             percentage or traceability matrix)"
        );
    }

    let mut findings = CoverageFindings::default();
    let mut used = vec![false; input.annotations.len()];
    let mut gaps = Vec::new();
    for gap in detected {
        match input
            .annotations
            .iter()
            .enumerate()
            .find(|(_, a)| annotation_matches(a, &gap))
        {
            Some((index, a)) => {
                used[index] = true;
                gaps.push(CoverageGapRecord {
                    location: gap.location,
                    coverage_type: gap.coverage_type,
                    classification: a.classification.clone(),
                    rationale: a.rationale.clone(),
                    approver: a.approver.clone(),
                    code_unit_id: a.code_unit_id.clone().or(gap.code_unit_id),
                    requirement_id: a.requirement_id.clone().or(gap.requirement_id),
                    test_case_id: a.test_case_id.clone(),
                    problem_report_id: a.problem_report_id.clone(),
                    action_item_id: a.action_item_id.clone(),
                });
            }
            None => findings.unclassified.push(gap),
        }
    }
    findings.unused_annotations = input
        .annotations
        .iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|(a, _)| a.location.clone())
        .collect();

    let report = CoverageReport {
        doc_type: "coverage_report".to_string(),
        schema: "tcms/coverage-report.schema.json".to_string(),
        id: input.id,
        run_id: input.run_id,
        baseline_id: input.baseline_id,
        tool: input.tool,
        tool_version: input.tool_version,
        requirements_based,
        structural,
        per_code_unit,
        per_requirement,
        gaps,
    };
    Ok((report, findings))
}

/// Sorted summary of gaps by classification, for messages
pub fn classification_counts(report: &CoverageReport) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for gap in &report.gaps {
        *counts.entry(gap.classification.as_str()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    const LCOV: &str = "TN:\nSF:src/a.c\nFN:1,f\nDA:1,3\nDA:2,0\nDA:3,0\nDA:4,5\nDA:5,0\n\
        BRDA:2,0,0,1\nBRDA:2,0,1,-\nBRDA:4,0,0,2\nBRDA:4,0,1,2\nLF:5\nLH:2\nend_of_record\n\
        SF:src/b.c\nDA:1,1\nDA:2,1\nend_of_record\n";

    fn annotation(location: &str, classification: &str) -> GapAnnotation {
        GapAnnotation {
            location: location.into(),
            coverage_type: None,
            classification: classification.into(),
            rationale: "because".into(),
            approver: None,
            code_unit_id: None,
            requirement_id: None,
            test_case_id: None,
            problem_report_id: None,
            action_item_id: None,
        }
    }

    fn input(annotations: Vec<GapAnnotation>) -> CoverageInput {
        CoverageInput {
            id: "COV-1".into(),
            baseline_id: "BL-1".into(),
            lcov: Some(Lcov::parse(LCOV).unwrap()),
            annotations,
            ..Default::default()
        }
    }

    #[test]
    fn lcov_gives_statement_decision_and_per_unit_figures_and_merged_line_gaps() {
        let (report, findings) = build_coverage_report(input(vec![])).unwrap();
        let s = report.structural.unwrap();
        // 4 of 7 statements, 3 of 4 branches
        assert_eq!(s.statement_percent, Some(57.14));
        assert_eq!(s.decision_percent, Some(75.0));
        assert_eq!(
            report.per_code_unit[1].figures.statement_percent,
            Some(100.0)
        );
        assert_eq!(report.per_code_unit[1].figures.decision_percent, None);
        let locations: Vec<_> = findings
            .unclassified
            .iter()
            .map(|g| (g.location.as_str(), g.coverage_type))
            .collect();
        assert_eq!(
            locations,
            [
                ("src/a.c:2-3", CoverageType::Statement),
                ("src/a.c:5", CoverageType::Statement),
                ("src/a.c:2", CoverageType::Decision),
            ]
        );
        assert!(report.gaps.is_empty());
    }

    #[test]
    fn annotations_match_by_exact_location_file_or_line_inside_a_range() {
        let mut dead = annotation("src/a.c:3", "dead_code");
        dead.coverage_type = Some(CoverageType::Statement);
        let mut justified = annotation("src/a.c:5", "justified");
        justified.approver = Some("qa".into());
        let file_wide = annotation("src/a.c", "tool_limitation");
        let stale = annotation("src/zzz.c:1", "dead_code");
        let (report, findings) =
            build_coverage_report(input(vec![dead, justified, file_wide, stale])).unwrap();
        let got: Vec<_> = report
            .gaps
            .iter()
            .map(|g| (g.location.as_str(), g.classification.as_str()))
            .collect();
        // src/a.c:3 lies inside the range 2-3; the file-wide annotation takes the rest
        assert_eq!(
            got,
            [
                ("src/a.c:2-3", "dead_code"),
                ("src/a.c:5", "justified"),
                ("src/a.c:2", "tool_limitation"),
            ]
        );
        assert_eq!(report.gaps[1].approver.as_deref(), Some("qa"));
        assert!(findings.unclassified.is_empty());
        assert_eq!(findings.unused_annotations, ["src/zzz.c:1"]);
    }

    #[test]
    fn partly_annotated_gaps_leave_the_rest_unclassified() {
        let (report, findings) =
            build_coverage_report(input(vec![annotation("src/a.c:5", "dead_code")])).unwrap();
        assert_eq!(report.gaps.len(), 1);
        assert_eq!(findings.unclassified.len(), 2);
    }

    #[test]
    fn mcdc_side_input_adds_figures_units_and_gaps() {
        let mcdc: McdcInput = serde_yaml::from_str(
            "mcdc_percent: 90.5\nunits:\n  - {code_unit_id: src/a.c, mcdc_percent: 80}\n  - {code_unit_id: src/c.c, mcdc_percent: 100}\ngaps:\n  - {location: 'src/a.c:9'}\n",
        )
        .unwrap();
        let mut i = input(vec![]);
        i.mcdc = Some(mcdc);
        let (report, findings) = build_coverage_report(i).unwrap();
        assert_eq!(report.structural.unwrap().mcdc_percent, Some(90.5));
        assert_eq!(report.per_code_unit[0].figures.mcdc_percent, Some(80.0));
        assert_eq!(report.per_code_unit[2].code_unit_id, "src/c.c");
        let g = findings
            .unclassified
            .iter()
            .find(|g| g.coverage_type == CoverageType::Mcdc)
            .unwrap();
        assert_eq!(g.code_unit_id.as_deref(), Some("src/a.c"));
    }

    #[test]
    fn mcdc_alone_is_a_valid_source_and_the_matrix_gives_requirements_based_figures() {
        use crate::trace_matrix::{RequirementLevel, TraceLink};
        let matrix = TraceMatrix {
            doc_type: "trace_matrix".into(),
            schema: "tcms/trace-matrix.schema.json".into(),
            id: "TM".into(),
            baseline_id: None,
            links: vec![
                TraceLink {
                    requirement_id: "HLR-1".into(),
                    level: RequirementLevel::Hlr,
                    test_case_ids: vec!["T1".into()],
                    normal_range_covered: true,
                    robustness_covered: true,
                    run_ids: vec![],
                },
                TraceLink {
                    requirement_id: "HLR-2".into(),
                    level: RequirementLevel::Hlr,
                    test_case_ids: vec![],
                    normal_range_covered: false,
                    robustness_covered: false,
                    run_ids: vec![],
                },
            ],
        };
        let (report, findings) = build_coverage_report(CoverageInput {
            id: "C".into(),
            baseline_id: "B".into(),
            matrix: Some(matrix),
            annotations: vec![annotation("HLR-2", "missing_test")],
            ..Default::default()
        })
        .unwrap();
        let rb = report.requirements_based.unwrap();
        assert_eq!(
            (
                rb.total_requirements,
                rb.covered_normal_range,
                rb.uncovered.clone()
            ),
            (2, 1, vec!["HLR-2".to_string()])
        );
        assert_eq!(report.gaps[0].coverage_type, CoverageType::Requirements);
        assert_eq!(report.gaps[0].requirement_id.as_deref(), Some("HLR-2"));
        assert!(findings.unclassified.is_empty());
    }

    #[test]
    fn invalid_inputs_are_rejected_with_clear_errors() {
        let err = build_coverage_report(CoverageInput {
            id: "C".into(),
            baseline_id: "B".into(),
            ..Default::default()
        })
        .unwrap_err();
        assert!(err.to_string().contains("Nothing to import"));

        assert!(Lcov::parse("DA:1,1\n")
            .unwrap_err()
            .to_string()
            .contains("outside a record"));
        assert!(Lcov::parse("SF:a\nDA:x,1\nend_of_record\n")
            .unwrap_err()
            .to_string()
            .contains("expected DA"));
        assert!(Lcov::parse("SF:a\nDA:1,1\n")
            .unwrap_err()
            .to_string()
            .contains("missing end_of_record"));
        assert!(Lcov::parse("")
            .unwrap_err()
            .to_string()
            .contains("no SF record"));
    }

    #[test]
    fn annotation_files_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.yaml");
        let check = |text: &str| {
            std::fs::write(&path, text).unwrap();
            read_annotations(&path).map_err(|e| format!("{e:#}"))
        };
        assert!(check("- {location: a, classification: dead_code, rationale: r}").is_ok());
        assert!(
            check("annotations:\n- {location: a, classification: dead_code, rationale: r}").is_ok()
        );
        assert!(
            check("- {location: a, classification: nonsense, rationale: r}")
                .unwrap_err()
                .contains("unknown classification 'nonsense'")
        );
        assert!(
            check("- {location: a, classification: dead_code, rationale: ' '}")
                .unwrap_err()
                .contains("rationale is required")
        );
        assert!(
            check("- {location: a, classification: justified, rationale: r}")
                .unwrap_err()
                .contains("needs an approver")
        );
        assert!(check("- {location: a, classification: dead_code}")
            .unwrap_err()
            .contains("rationale"));
    }
}
