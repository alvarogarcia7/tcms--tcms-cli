//! A run record together with its source file and parsed start instant.

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use testcase_verification::TestRunRecord;

/// A loaded run record. `started` is the parsed `started_at`; runs are ordered by instant,
/// never by the raw string (D-01).
#[derive(Debug, Clone)]
pub struct LoadedRun {
    pub path: PathBuf,
    pub record: TestRunRecord,
    pub started: DateTime<Utc>,
}

impl LoadedRun {
    pub fn new(path: PathBuf, record: TestRunRecord) -> Result<Self> {
        let started = DateTime::parse_from_rfc3339(&record.started_at)
            .map_err(|e| {
                anyhow::anyhow!(
                    "{}: started_at '{}' is not an RFC 3339 timestamp: {}",
                    path.display(),
                    record.started_at,
                    e
                )
            })?
            .with_timezone(&Utc);
        Ok(Self {
            path,
            record,
            started,
        })
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use testcase_verification::{TestRunRecord, TestRunResult, Verdict};

    pub(crate) fn record(
        run_id: &str,
        build_id: &str,
        started_at: &str,
        results: &[(&str, Verdict)],
    ) -> TestRunRecord {
        TestRunRecord {
            doc_type: "test_run".to_string(),
            schema: "tcms/test-run.schema.json".to_string(),
            run_id: run_id.to_string(),
            build_id: build_id.to_string(),
            environment: "e".to_string(),
            tester: "t".to_string(),
            started_at: started_at.to_string(),
            finished_at: None,
            reviewer: None,
            results: results
                .iter()
                .map(|(id, v)| TestRunResult {
                    test_case_id: id.to_string(),
                    verdict: *v,
                    problem_report_id: None,
                    verification_result_ref: None,
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::record;
    use super::*;

    #[test]
    fn utc_instant_is_parsed() {
        let r = LoadedRun::new(
            PathBuf::from("a.yaml"),
            record("R", "b", "2026-10-28T10:00:00Z", &[]),
        )
        .unwrap();
        assert_eq!(r.started.to_rfc3339(), "2026-10-28T10:00:00+00:00");
    }

    #[test]
    fn offsets_compare_as_instants() {
        let a = LoadedRun::new(
            PathBuf::from("a.yaml"),
            record("R", "b", "2026-10-28T12:00:00+02:00", &[]),
        )
        .unwrap();
        let b = LoadedRun::new(
            PathBuf::from("b.yaml"),
            record("R2", "b", "2026-10-28T10:00:00Z", &[]),
        )
        .unwrap();
        assert_eq!(a.started, b.started);
    }

    #[test]
    fn bad_started_at_names_file() {
        let e = LoadedRun::new(
            PathBuf::from("runs/x.yaml"),
            record("R", "b", "yesterday", &[]),
        )
        .unwrap_err()
        .to_string();
        assert!(e.contains("runs/x.yaml") && e.contains("yesterday") && e.contains("RFC 3339"));
    }
}
