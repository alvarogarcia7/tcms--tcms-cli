//! Traceability matrix (tcms-schemas `trace-matrix`): requirement to test cases to runs, and
//! the glossary rule that every HLR and LLR needs a normal-range and a robustness test
//! (TCMS-40.5, TCMS-40.6).
//!
//! Requirement IDs and levels come from the requirements tool (StrictDoc or Jira, see
//! TCMS-42.3); this module only reads them. Traceability uses the test cases' `requirements`
//! list, never the legacy `requirement` string.

use crate::test_run::{TestRunRecord, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use testcase_models::{TestCase, TestCaseStatus, TestType};

/// Requirement level: requirements are high-level or low-level; "integration" is a test level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RequirementLevel {
    #[serde(rename = "HLR")]
    Hlr,
    #[serde(rename = "LLR")]
    Llr,
}

impl std::fmt::Display for RequirementLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Hlr => "HLR",
            Self::Llr => "LLR",
        })
    }
}

impl RequirementLevel {
    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_uppercase().as_str() {
            "HLR" => Some(Self::Hlr),
            "LLR" => Some(Self::Llr),
            _ => None,
        }
    }
}

/// A requirement the matrix covers
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub level: RequirementLevel,
}

/// Read requirements from an ID list (`.txt`, one `ID`, `ID:HLR` or `ID,LLR` per line, `#`
/// comments; a level is guessed from an `HLR-`/`LLR-` prefix), from a YAML file holding one
/// requirement record or a list of them, or from a directory of such files.
///
/// Only records of type HLR or LLR count: the matrix is about those levels.
pub fn read_requirements(path: &Path) -> anyhow::Result<Vec<Requirement>> {
    use anyhow::Context;
    let mut requirements = Vec::new();
    if path.is_dir() {
        let mut files: Vec<_> = std::fs::read_dir(path)
            .with_context(|| format!("Failed to read {}", path.display()))?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        for file in files {
            requirements.extend(read_requirements(&file)?);
        }
        return Ok(requirements);
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "yml" || extension == "yaml" {
        let value: serde_yaml::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        let records = match value {
            serde_yaml::Value::Sequence(records) => records,
            single => vec![single],
        };
        for record in records {
            let id = record.get("id").and_then(|v| v.as_str());
            let level = record
                .get("type")
                .and_then(|v| v.as_str())
                .and_then(RequirementLevel::parse);
            if let (Some(id), Some(level)) = (id, level) {
                requirements.push(Requirement {
                    id: id.to_string(),
                    level,
                });
            }
        }
    } else {
        for (number, line) in content.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (id, level) = match line.split_once([':', ',']) {
                Some((id, level)) => (id.trim(), RequirementLevel::parse(level)),
                None => {
                    let upper = line.to_ascii_uppercase();
                    let guessed = if upper.starts_with("HLR") {
                        Some(RequirementLevel::Hlr)
                    } else if upper.starts_with("LLR") {
                        Some(RequirementLevel::Llr)
                    } else {
                        None
                    };
                    (line, guessed)
                }
            };
            let level = level.with_context(|| {
                format!(
                    "{}:{}: cannot tell whether '{}' is an HLR or an LLR; write '{}:HLR' or '{}:LLR'",
                    path.display(),
                    number + 1,
                    id,
                    id,
                    id
                )
            })?;
            requirements.push(Requirement {
                id: id.to_string(),
                level,
            });
        }
    }
    Ok(requirements)
}

/// One requirement of the matrix
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceLink {
    pub requirement_id: String,
    pub level: RequirementLevel,
    pub test_case_ids: Vec<String>,
    pub normal_range_covered: bool,
    pub robustness_covered: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_ids: Vec<String>,
}

/// Traceability matrix, in the form of the tcms-schemas `trace-matrix` schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceMatrix {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub schema: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_id: Option<String>,
    pub links: Vec<TraceLink>,
}

/// What is missing for a requirement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    pub requirement_id: String,
    pub level: RequirementLevel,
    /// Test types without a test case: `normal_range`, `robustness`
    pub missing: Vec<&'static str>,
}

/// Findings besides the matrix itself
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatrixFindings {
    /// Requirements no test case traces to
    pub untraced: Vec<String>,
    /// Requirements lacking a normal-range or a robustness test case
    pub gaps: Vec<CoverageGap>,
    /// Test case to the requirement IDs it names that are not in the requirement set
    pub unknown_references: BTreeMap<String, Vec<String>>,
}

impl MatrixFindings {
    /// The glossary rule is met: no gaps (untraced requirements are gaps too)
    pub fn complete(&self) -> bool {
        self.gaps.is_empty()
    }

    /// One line per requirement with a gap, naming what is missing
    pub fn gap_report(&self) -> String {
        self.gaps
            .iter()
            .map(|gap| {
                format!(
                    "{} ({}): no {} test",
                    gap.requirement_id,
                    gap.level,
                    gap.missing.join(" and no ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Build the matrix of `requirements` from `test_cases` and the results of `runs`.
///
/// Obsolete test cases do not count. A run counts for a link when one of its test cases ran
/// (any verdict but `not_executed`).
pub fn build_matrix(
    id: &str,
    baseline_id: Option<&str>,
    requirements: &[Requirement],
    test_cases: &[TestCase],
    runs: &[TestRunRecord],
) -> (TraceMatrix, MatrixFindings) {
    let known: BTreeSet<&str> = requirements.iter().map(|r| r.id.as_str()).collect();
    let live: Vec<&TestCase> = test_cases
        .iter()
        .filter(|tc| tc.status != Some(TestCaseStatus::Obsolete))
        .collect();

    let mut findings = MatrixFindings::default();
    for tc in &live {
        let unknown: Vec<String> = tc
            .requirements
            .iter()
            .filter(|r| !known.contains(r.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            findings.unknown_references.insert(tc.id.clone(), unknown);
        }
    }

    let mut links = Vec::new();
    for requirement in requirements {
        let linked: Vec<&&TestCase> = live
            .iter()
            .filter(|tc| tc.requirements.iter().any(|r| r == &requirement.id))
            .collect();
        let covers = |test_type: TestType| linked.iter().any(|tc| tc.test_type == Some(test_type));
        let (normal_range, robustness) =
            (covers(TestType::NormalRange), covers(TestType::Robustness));

        let test_case_ids: Vec<String> = linked.iter().map(|tc| tc.id.clone()).collect();
        let run_ids: BTreeSet<String> = runs
            .iter()
            .filter(|run| {
                run.results.iter().any(|result| {
                    result.verdict != Verdict::NotExecuted
                        && test_case_ids.contains(&result.test_case_id)
                })
            })
            .map(|run| run.run_id.clone())
            .collect();

        let mut missing = Vec::new();
        if !normal_range {
            missing.push("normal_range");
        }
        if !robustness {
            missing.push("robustness");
        }
        if test_case_ids.is_empty() {
            findings.untraced.push(requirement.id.clone());
        }
        if !missing.is_empty() {
            findings.gaps.push(CoverageGap {
                requirement_id: requirement.id.clone(),
                level: requirement.level,
                missing,
            });
        }

        links.push(TraceLink {
            requirement_id: requirement.id.clone(),
            level: requirement.level,
            test_case_ids,
            normal_range_covered: normal_range,
            robustness_covered: robustness,
            run_ids: run_ids.into_iter().collect(),
        });
    }

    (
        TraceMatrix {
            doc_type: "trace_matrix".to_string(),
            schema: "tcms/trace-matrix.schema.json".to_string(),
            id: id.to_string(),
            baseline_id: baseline_id.map(str::to_string),
            links,
        },
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_run::{TestRunRecord, TestRunResult};

    fn test_case(
        id: &str,
        requirements: &[&str],
        test_type: Option<TestType>,
        legacy_requirement: &str,
    ) -> TestCase {
        let mut tc = TestCase::new(
            legacy_requirement.to_string(),
            1,
            1,
            id.to_string(),
            "d".into(),
        );
        tc.requirements = requirements.iter().map(|r| r.to_string()).collect();
        tc.test_type = test_type;
        tc
    }

    fn run(run_id: &str, results: &[(&str, Verdict)]) -> TestRunRecord {
        TestRunRecord {
            doc_type: "test_run".into(),
            schema: "tcms/test-run.schema.json".into(),
            run_id: run_id.into(),
            build_id: "b".into(),
            environment: "e".into(),
            tester: "t".into(),
            started_at: "2026-10-28T10:00:00Z".into(),
            finished_at: None,
            reviewer: None,
            results: results
                .iter()
                .map(|(id, verdict)| TestRunResult {
                    test_case_id: id.to_string(),
                    verdict: *verdict,
                    problem_report_id: None,
                    verification_result_ref: None,
                })
                .collect(),
        }
    }

    fn requirements() -> Vec<Requirement> {
        vec![
            Requirement {
                id: "HLR-1".into(),
                level: RequirementLevel::Hlr,
            },
            Requirement {
                id: "LLR-2".into(),
                level: RequirementLevel::Llr,
            },
            Requirement {
                id: "HLR-3".into(),
                level: RequirementLevel::Hlr,
            },
        ]
    }

    #[test]
    fn test_coverage_is_computed_from_test_type_and_the_legacy_string_is_ignored() {
        let cases = vec![
            test_case("N1", &["HLR-1"], Some(TestType::NormalRange), "LLR-2"),
            test_case(
                "R1",
                &["HLR-1", "LLR-2"],
                Some(TestType::Robustness),
                "HLR-3",
            ),
            test_case("S1", &["LLR-2"], Some(TestType::Security), "HLR-3"),
            test_case("U1", &["LLR-2"], None, "HLR-3"),
        ];
        let (matrix, findings) = build_matrix("TM-1", Some("BL-1"), &requirements(), &cases, &[]);

        let hlr1 = &matrix.links[0];
        assert_eq!(hlr1.test_case_ids, ["N1", "R1"]);
        assert!(hlr1.normal_range_covered && hlr1.robustness_covered);

        // LLR-2 has a robustness, a security and an untyped case, but no normal-range one
        let llr2 = &matrix.links[1];
        assert_eq!(llr2.test_case_ids, ["R1", "S1", "U1"]);
        assert!(!llr2.normal_range_covered && llr2.robustness_covered);

        // HLR-3 appears only as the legacy `requirement` string: untraced
        let hlr3 = &matrix.links[2];
        assert!(hlr3.test_case_ids.is_empty());
        assert_eq!(findings.untraced, ["HLR-3"]);
        assert_eq!(matrix.baseline_id.as_deref(), Some("BL-1"));
    }

    #[test]
    fn test_gaps_name_each_requirement_and_what_is_missing() {
        let cases = vec![test_case(
            "N1",
            &["HLR-1"],
            Some(TestType::NormalRange),
            "x",
        )];
        let (_, findings) = build_matrix("TM", None, &requirements(), &cases, &[]);
        assert!(!findings.complete());
        let report = findings.gap_report();
        assert!(
            report.contains("HLR-1 (HLR): no robustness test"),
            "{report}"
        );
        assert!(
            report.contains("LLR-2 (LLR): no normal_range and no robustness test"),
            "{report}"
        );
        assert!(report.contains("HLR-3"), "{report}");

        let both = vec![
            test_case(
                "N",
                &["HLR-1", "LLR-2", "HLR-3"],
                Some(TestType::NormalRange),
                "x",
            ),
            test_case(
                "R",
                &["HLR-1", "LLR-2", "HLR-3"],
                Some(TestType::Robustness),
                "x",
            ),
        ];
        let (_, findings) = build_matrix("TM", None, &requirements(), &both, &[]);
        assert!(findings.complete());
        assert_eq!(findings.gap_report(), "");
    }

    #[test]
    fn test_runs_link_through_run_ids_and_not_executed_does_not_count() {
        let cases = vec![
            test_case("N1", &["HLR-1"], Some(TestType::NormalRange), "x"),
            test_case("N2", &["LLR-2"], Some(TestType::NormalRange), "x"),
        ];
        let runs = vec![
            run("RUN-B", &[("N1", Verdict::Fail)]),
            run(
                "RUN-A",
                &[("N1", Verdict::Pass), ("N2", Verdict::NotExecuted)],
            ),
            run("RUN-C", &[("N2", Verdict::NotExecuted)]),
        ];
        let (matrix, _) = build_matrix("TM", None, &requirements(), &cases, &runs);
        assert_eq!(matrix.links[0].run_ids, ["RUN-A", "RUN-B"]);
        assert!(
            matrix.links[1].run_ids.is_empty(),
            "not_executed is not an exercise"
        );
    }

    #[test]
    fn test_obsolete_cases_are_ignored_and_unknown_references_reported() {
        let mut obsolete = test_case("OLD", &["HLR-1"], Some(TestType::NormalRange), "x");
        obsolete.status = Some(TestCaseStatus::Obsolete);
        let cases = vec![
            obsolete,
            test_case("N1", &["HLR-1", "HLR-99"], Some(TestType::NormalRange), "x"),
        ];
        let (matrix, findings) = build_matrix("TM", None, &requirements(), &cases, &[]);
        assert_eq!(matrix.links[0].test_case_ids, ["N1"]);
        assert_eq!(findings.unknown_references["N1"], ["HLR-99"]);
    }

    #[test]
    fn test_read_requirements_from_id_list_records_and_directories() {
        let dir = tempfile::tempdir().unwrap();
        let list = dir.path().join("ids.txt");
        std::fs::write(
            &list,
            "# the SRS\nHLR-1\nllr-2  # prefix is enough\nSR-7:HLR\nSR-8, LLR\n",
        )
        .unwrap();
        let ids = read_requirements(&list).unwrap();
        assert_eq!(
            ids.iter()
                .map(|r| (r.id.as_str(), r.level))
                .collect::<Vec<_>>(),
            [
                ("HLR-1", RequirementLevel::Hlr),
                ("llr-2", RequirementLevel::Llr),
                ("SR-7", RequirementLevel::Hlr),
                ("SR-8", RequirementLevel::Llr),
            ]
        );

        let unknown = dir.path().join("bad.txt");
        std::fs::write(&unknown, "SR-9\n").unwrap();
        let err = read_requirements(&unknown).unwrap_err().to_string();
        assert!(
            err.contains("bad.txt:1") && err.contains("SR-9:HLR"),
            "{err}"
        );

        let records = dir.path().join("reqs");
        std::fs::create_dir(&records).unwrap();
        std::fs::write(records.join("a.yml"), "id: HLR-12\ntype: HLR\nsource: x\n").unwrap();
        std::fs::write(
            records.join("b.yaml"),
            "- {id: LLR-5, type: LLR}\n- {id: SYS-1, type: system}\n",
        )
        .unwrap();
        let from_dir = read_requirements(&records).unwrap();
        assert_eq!(
            from_dir.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["HLR-12", "LLR-5"],
            "only HLR and LLR records count"
        );
    }

    #[test]
    fn test_matrix_yaml_is_plain_and_round_trips() {
        let cases = vec![test_case(
            "N1",
            &["HLR-1"],
            Some(TestType::NormalRange),
            "x",
        )];
        let (matrix, _) = build_matrix("TM-7", None, &requirements(), &cases, &[]);
        let yaml = testcase_common::to_yaml_string(&matrix).unwrap();
        assert!(
            yaml.contains("level: HLR") && yaml.contains("type: trace_matrix"),
            "{yaml}"
        );
        assert!(
            !yaml.contains("baseline_id") && !yaml.contains("run_ids"),
            "{yaml}"
        );
        let back: TraceMatrix = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, matrix);
    }
}
