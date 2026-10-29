//! Test run record (tcms-schemas `test-run`): one per execution, grouping the verdicts of the
//! test cases that ran together (TCMS-40.4).

use crate::verification::{BatchVerificationReport, RunInfo, TestCaseVerificationResult};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Outcome of one test case in a run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    NotExecuted,
    /// The execution aborted before the steps could run to a result (a failed hook)
    Error,
}

impl Verdict {
    /// Verdict of a verified test case. `errored` marks a case whose execution was aborted.
    pub fn of(result: &TestCaseVerificationResult, errored: bool) -> Self {
        if errored {
            Verdict::Error
        } else if result.overall_pass {
            Verdict::Pass
        } else if result.failed_steps > 0 {
            Verdict::Fail
        } else {
            Verdict::NotExecuted
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::NotExecuted => "not_executed",
            Verdict::Error => "error",
        }
    }
}

/// Outcome of one test case in a [`TestRunRecord`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunResult {
    pub test_case_id: String,

    pub verdict: Verdict,

    /// Problem report raised for a failed or errored case
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem_report_id: Option<String>,

    /// Where the detailed verification result is
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_result_ref: Option<String>,
}

/// A test run, in the form of the tcms-schemas `test-run` schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestRunRecord {
    /// Document type (envelope field)
    #[serde(rename = "type")]
    pub doc_type: String,

    /// Schema reference (envelope field)
    pub schema: String,

    pub run_id: String,
    pub build_id: String,
    pub environment: String,
    pub tester: String,

    /// RFC 3339
    pub started_at: String,

    /// RFC 3339
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,

    /// Who checked the results (must differ from the author when independence is required)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,

    pub results: Vec<TestRunResult>,
}

/// What [`TestRunRecord::build`] needs besides the verification report
#[derive(Debug, Clone, Default)]
pub struct TestRunInput<'a> {
    /// Run, build, environment and tester (all four are required by the schema)
    pub run: RunInfo,

    pub reviewer: Option<&'a str>,

    /// Earliest execution timestamp; the report's generation time when unknown
    pub started_at: Option<DateTime<Utc>>,

    /// Latest execution timestamp; the report's generation time when unknown
    pub finished_at: Option<DateTime<Utc>>,

    /// Problem report per test case ID; only for failed or errored cases
    pub problem_reports: BTreeMap<String, String>,

    /// Test cases whose execution was aborted (their verdict is `error`)
    pub errored: BTreeSet<String>,

    /// Reference to the verification results; each result is `<ref>#<test_case_id>`
    pub result_ref: Option<&'a str>,
}

impl TestRunRecord {
    /// Build the record of a run from its verification report.
    ///
    /// Fails with a message naming what is missing or wrong, never writing an invalid record.
    pub fn build(report: &BatchVerificationReport, input: &TestRunInput) -> Result<Self, String> {
        let run = input.run.clone().normalized();
        let missing: Vec<&str> = [
            ("run_id", &run.run_id),
            ("build_id", &run.build_id),
            ("environment", &run.environment),
            ("tester", &run.tester),
        ]
        .iter()
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();
        if !missing.is_empty() {
            return Err(format!(
                "a test run needs run_id, build_id, environment and tester; missing: {} \
                 (flags, TCMS_* environment variables or the config file)",
                missing.join(", ")
            ));
        }

        let mut results = Vec::new();
        for result in &report.test_cases {
            let verdict = Verdict::of(result, input.errored.contains(&result.test_case_id));
            results.push(TestRunResult {
                test_case_id: result.test_case_id.clone(),
                verdict,
                problem_report_id: input
                    .problem_reports
                    .get(&result.test_case_id)
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty()),
                verification_result_ref: input
                    .result_ref
                    .map(|reference| format!("{}#{}", reference, result.test_case_id)),
            });
        }

        for (test_case_id, problem_report) in &input.problem_reports {
            match results.iter().find(|r| &r.test_case_id == test_case_id) {
                None => {
                    return Err(format!(
                        "problem report {problem_report} names test case {test_case_id}, \
                         which is not in this run"
                    ))
                }
                Some(r) if !matches!(r.verdict, Verdict::Fail | Verdict::Error) => {
                    return Err(format!(
                        "problem report {problem_report} cannot be attached to test case \
                         {test_case_id}: its verdict is {}, only fail and error take one",
                        r.verdict.as_str()
                    ))
                }
                Some(_) => {}
            }
        }

        let started_at = input.started_at.unwrap_or(report.generated_at);
        let finished_at = input
            .finished_at
            .unwrap_or(report.generated_at)
            .max(started_at);
        Ok(Self {
            doc_type: "test_run".to_string(),
            schema: "tcms/test-run.schema.json".to_string(),
            run_id: run.run_id.unwrap_or_default(),
            build_id: run.build_id.unwrap_or_default(),
            environment: run.environment.unwrap_or_default(),
            tester: run.tester.unwrap_or_default(),
            started_at: started_at.to_rfc3339_opts(SecondsFormat::Secs, true),
            finished_at: Some(finished_at.to_rfc3339_opts(SecondsFormat::Secs, true)),
            reviewer: input
                .reviewer
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty()),
            results,
        })
    }

    /// The record as YAML that tcms-schemas and PyYAML read back unchanged
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        testcase_common::to_yaml_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(
        id: &str,
        passed: usize,
        failed: usize,
        not_executed: usize,
    ) -> TestCaseVerificationResult {
        TestCaseVerificationResult {
            test_case_id: id.to_string(),
            description: "d".to_string(),
            sequences: vec![],
            total_steps: passed + failed + not_executed,
            passed_steps: passed,
            failed_steps: failed,
            not_executed_steps: not_executed,
            overall_pass: failed == 0 && not_executed == 0,
            requirement: None,
            item: None,
            tc: None,
            source_yaml_sha256: None,
            run: RunInfo::default(),
        }
    }

    fn report() -> BatchVerificationReport {
        let mut report = BatchVerificationReport::new();
        report.add_test_case_result(result("TC_PASS", 3, 0, 0));
        report.add_test_case_result(result("TC_FAIL", 1, 2, 0));
        report.add_test_case_result(result("TC_SKIPPED", 0, 0, 3));
        report.add_test_case_result(result("TC_HOOK", 0, 0, 3));
        report
    }

    fn full_run() -> RunInfo {
        RunInfo {
            run_id: Some("RUN-1".to_string()),
            build_id: Some("b-1".to_string()),
            environment: Some("Staging".to_string()),
            tester: Some("alice".to_string()),
            date: None,
        }
    }

    #[test]
    fn test_verdicts_pass_fail_not_executed_and_error() {
        let input = TestRunInput {
            run: full_run(),
            errored: BTreeSet::from(["TC_HOOK".to_string()]),
            ..Default::default()
        };
        let record = TestRunRecord::build(&report(), &input).unwrap();
        let verdicts: Vec<(&str, Verdict)> = record
            .results
            .iter()
            .map(|r| (r.test_case_id.as_str(), r.verdict))
            .collect();
        assert_eq!(
            verdicts,
            vec![
                ("TC_PASS", Verdict::Pass),
                ("TC_FAIL", Verdict::Fail),
                ("TC_SKIPPED", Verdict::NotExecuted),
                ("TC_HOOK", Verdict::Error),
            ]
        );
        assert_eq!(record.doc_type, "test_run");
        assert_eq!(record.schema, "tcms/test-run.schema.json");
    }

    #[test]
    fn test_reviewer_and_problem_reports_are_recorded() {
        let input = TestRunInput {
            run: full_run(),
            reviewer: Some(" bob "),
            problem_reports: BTreeMap::from([("TC_FAIL".to_string(), "PR-057".to_string())]),
            result_ref: Some("results.yaml"),
            ..Default::default()
        };
        let record = TestRunRecord::build(&report(), &input).unwrap();
        assert_eq!(record.reviewer.as_deref(), Some("bob"));
        let failed = &record.results[1];
        assert_eq!(failed.problem_report_id.as_deref(), Some("PR-057"));
        assert_eq!(
            failed.verification_result_ref.as_deref(),
            Some("results.yaml#TC_FAIL")
        );
        assert_eq!(record.results[0].problem_report_id, None);
    }

    #[test]
    fn test_problem_report_only_for_fail_or_error_of_a_case_in_the_run() {
        let attach = |case: &str| {
            let input = TestRunInput {
                run: full_run(),
                problem_reports: BTreeMap::from([(case.to_string(), "PR-1".to_string())]),
                ..Default::default()
            };
            TestRunRecord::build(&report(), &input)
        };
        assert!(attach("TC_FAIL").is_ok());
        assert!(attach("TC_PASS")
            .unwrap_err()
            .contains("only fail and error"));
        assert!(attach("TC_SKIPPED").unwrap_err().contains("not_executed"));
        assert!(attach("TC_NOPE").unwrap_err().contains("not in this run"));
    }

    #[test]
    fn test_missing_run_metadata_is_named() {
        let input = TestRunInput {
            run: RunInfo {
                run_id: Some("RUN-1".to_string()),
                tester: Some("  ".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let err = TestRunRecord::build(&report(), &input).unwrap_err();
        assert!(
            err.contains("missing: build_id, environment, tester"),
            "{err}"
        );
    }

    #[test]
    fn test_timestamps_are_rfc3339_and_ordered_and_yaml_has_no_tags() {
        let input = TestRunInput {
            run: full_run(),
            started_at: Some(Utc::now() - chrono::Duration::seconds(30)),
            finished_at: Some(Utc::now() - chrono::Duration::seconds(10)),
            ..Default::default()
        };
        let record = TestRunRecord::build(&report(), &input).unwrap();
        let started = DateTime::parse_from_rfc3339(&record.started_at).unwrap();
        let finished = DateTime::parse_from_rfc3339(record.finished_at.as_ref().unwrap()).unwrap();
        assert!(started <= finished);
        // the logs' last timestamp, not the time the report was made
        assert!(finished < Utc::now() - chrono::Duration::seconds(5));

        // an end before the start is never written
        let input = TestRunInput {
            run: full_run(),
            started_at: Some(Utc::now()),
            finished_at: Some(Utc::now() - chrono::Duration::seconds(60)),
            ..Default::default()
        };
        let record = TestRunRecord::build(&report(), &input).unwrap();
        assert!(record.started_at <= *record.finished_at.as_ref().unwrap());

        let yaml = record.to_yaml().unwrap();
        assert!(!yaml.contains('!'), "{yaml}");
        assert!(yaml.contains("verdict: not_executed"), "{yaml}");
        let back: TestRunRecord = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back, record);
    }
}
