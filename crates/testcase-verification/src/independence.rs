//! Independence between the author of a test case and whoever checks its results
//! (TCMS-40.8). JSON Schema cannot compare two fields, so the rules live here.
//!
//! - A test case with `independence_required` has an `author`. The tester may be the author,
//!   but the `reviewer` of a test run covering it may not be.
//! - An `independence-assignment` record whose `verifier_is_independent` is true must name a
//!   verifier other than the author.

use crate::test_run::TestRunRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn same_person(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// What the independence check needs to know about a test case
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaseIndependence {
    pub required: bool,
    pub author: Option<String>,
}

/// Refuse a test run whose reviewer is missing, or the author, for a test case that requires
/// independence. The error names every offending test case.
pub fn check_run_independence(
    run: &TestRunRecord,
    cases: &BTreeMap<String, CaseIndependence>,
) -> Result<(), String> {
    let reviewer = run.reviewer.as_deref().filter(|r| !r.trim().is_empty());
    let mut violations = Vec::new();
    for result in &run.results {
        let Some(case) = cases.get(&result.test_case_id).filter(|c| c.required) else {
            continue;
        };
        let author = case.author.as_deref().filter(|a| !a.trim().is_empty());
        match (reviewer, author) {
            (None, _) => violations.push(format!(
                "test case {} requires independent review but run {} has no reviewer",
                result.test_case_id, run.run_id
            )),
            (Some(_), None) => violations.push(format!(
                "test case {} requires independent review but records no author, so the \
                 reviewer cannot be checked against it",
                result.test_case_id
            )),
            (Some(reviewer), Some(author)) if same_person(reviewer, author) => {
                violations.push(format!(
                    "test case {} requires independent review but reviewer {} is its author",
                    result.test_case_id, reviewer
                ))
            }
            _ => {}
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations.join("\n"))
    }
}

/// Record that an artifact was checked by someone other than its author, in the form of the
/// tcms-schemas `independence-assignment` schema
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndependenceAssignment {
    /// Document type (envelope field)
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,

    /// Schema reference (envelope field)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    pub id: String,
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_id: Option<String>,
    pub artifact: String,
    pub author: String,
    pub verifier: String,
    pub verifier_is_independent: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

impl IndependenceAssignment {
    /// A new assignment; `verifier_is_independent` follows from author and verifier
    pub fn new(id: &str, objective: &str, artifact: &str, author: &str, verifier: &str) -> Self {
        Self {
            doc_type: Some("independence_assignment".to_string()),
            schema: Some("tcms/independence-assignment.schema.json".to_string()),
            id: id.trim().to_string(),
            objective: objective.trim().to_string(),
            objective_id: None,
            artifact: artifact.trim().to_string(),
            author: author.trim().to_string(),
            verifier: verifier.trim().to_string(),
            verifier_is_independent: !same_person(author, verifier),
            date: None,
        }
    }

    /// Reject a record that claims independence for a verifier who is the author
    pub fn validate(&self) -> Result<(), String> {
        if self.verifier_is_independent && same_person(&self.author, &self.verifier) {
            return Err(format!(
                "independence assignment {} says the verifier is independent, but the verifier \
                 {} is the author",
                self.id, self.verifier
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_run::{TestRunResult, Verdict};

    fn run(reviewer: Option<&str>, case_ids: &[&str]) -> TestRunRecord {
        TestRunRecord {
            doc_type: "test_run".into(),
            schema: "tcms/test-run.schema.json".into(),
            run_id: "RUN-1".into(),
            build_id: "b".into(),
            environment: "e".into(),
            tester: "alice".into(),
            started_at: "2026-10-28T10:00:00Z".into(),
            finished_at: None,
            reviewer: reviewer.map(str::to_string),
            results: case_ids
                .iter()
                .map(|id| TestRunResult {
                    test_case_id: id.to_string(),
                    verdict: Verdict::Pass,
                    problem_report_id: None,
                    verification_result_ref: None,
                })
                .collect(),
        }
    }

    fn cases() -> BTreeMap<String, CaseIndependence> {
        BTreeMap::from([
            (
                "STRICT".to_string(),
                CaseIndependence {
                    required: true,
                    author: Some("Alice".to_string()),
                },
            ),
            (
                "LAX".to_string(),
                CaseIndependence {
                    required: false,
                    author: Some("alice".to_string()),
                },
            ),
            (
                "NO_AUTHOR".to_string(),
                CaseIndependence {
                    required: true,
                    author: None,
                },
            ),
        ])
    }

    #[test]
    fn test_a_different_reviewer_is_accepted() {
        assert!(check_run_independence(&run(Some("bob"), &["STRICT", "LAX"]), &cases()).is_ok());
    }

    #[test]
    fn test_missing_reviewer_is_refused_naming_the_test_case() {
        for reviewer in [None, Some("  ")] {
            let err = check_run_independence(&run(reviewer, &["STRICT"]), &cases()).unwrap_err();
            assert!(
                err.contains("STRICT") && err.contains("no reviewer"),
                "{err}"
            );
        }
    }

    #[test]
    fn test_reviewer_equal_to_author_is_refused_ignoring_case_and_spaces() {
        let err = check_run_independence(&run(Some(" ALICE "), &["STRICT"]), &cases()).unwrap_err();
        assert!(
            err.contains("STRICT") && err.contains("is its author"),
            "{err}"
        );
    }

    #[test]
    fn test_cases_without_the_requirement_are_unaffected() {
        // the tester may be the author, and a case that does not require independence does
        // not care who reviews it, or whether anyone does
        assert!(check_run_independence(&run(None, &["LAX"]), &cases()).is_ok());
        assert!(check_run_independence(&run(Some("alice"), &["LAX"]), &cases()).is_ok());
        assert!(check_run_independence(&run(None, &["UNKNOWN"]), &cases()).is_ok());
    }

    #[test]
    fn test_required_independence_without_a_recorded_author_cannot_be_checked() {
        let err = check_run_independence(&run(Some("bob"), &["NO_AUTHOR"]), &cases()).unwrap_err();
        assert!(err.contains("records no author"), "{err}");
    }

    #[test]
    fn test_every_offending_case_is_named() {
        let err = check_run_independence(&run(Some("alice"), &["STRICT", "NO_AUTHOR"]), &cases())
            .unwrap_err();
        assert_eq!(err.lines().count(), 2, "{err}");
    }

    #[test]
    fn test_assignment_is_independent_only_for_a_different_verifier() {
        let ok = IndependenceAssignment::new("IA-1", "obj", "RUN-1", "alice", "bob");
        assert!(ok.verifier_is_independent && ok.validate().is_ok());

        let same = IndependenceAssignment::new("IA-2", "obj", "RUN-1", "alice", " Alice");
        assert!(!same.verifier_is_independent);
        assert!(
            same.validate().is_ok(),
            "an honest 'not independent' record is valid"
        );
    }

    #[test]
    fn test_assignment_claiming_independence_for_the_author_is_rejected() {
        let mut record = IndependenceAssignment::new("IA-3", "obj", "RUN-1", "alice", "bob");
        record.verifier = "Alice".to_string();
        let err = record.validate().unwrap_err();
        assert!(
            err.contains("IA-3") && err.contains("is the author"),
            "{err}"
        );
    }
}
