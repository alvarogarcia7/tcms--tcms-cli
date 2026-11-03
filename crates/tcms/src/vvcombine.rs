//! `tcms vv combine` (TCMS-47.3): the per-case results of one run as the run record that
//! `tcms vv metrics` reads.
//!
//! Input, under `runs/<RUN_ID>/`:
//! - `per-case/<KEY>_result.yaml`: `type: test_result` as written by
//!   `scripts/convert_verification_to_result_yaml.py` (`test_case_id`, step counts, `overall_pass`).
//! - `execution_logs/<KEY>_execution_log.json`: the execution log of the case, with a `timestamp` per
//!   entry. `KEY` is the file name without `_result.yaml`; it equals the test case id for ordinary files.
//!
//! Output: `test-run.yaml` and `verification-results.yaml`, both validated before anything is written.
//! The rules are in `docs/RUN_RECORD_FORMAT.md` and `docs/VV_METRICS.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// Run metadata given on the command line. The per-case files do not carry it.
pub struct RunMetadata<'a> {
    pub run_id: &'a str,
    pub plan_id: Option<&'a str>,
    pub build_id: Option<&'a str>,
    pub environment: Option<&'a str>,
    pub tester: Option<&'a str>,
}

/// Verdict of one case in the run record (the vocabulary of the test-run schema, minus `error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    NotExecuted,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::NotExecuted => "not_executed",
        }
    }

    fn is_executed(self) -> bool {
        self != Verdict::NotExecuted
    }
}

/// Verdict of a case from its step counts. A case with steps and none executed is `not_executed`;
/// otherwise `overall_pass` decides between `pass` and `fail`.
pub fn verdict_of(
    total_steps: u64,
    passed_steps: u64,
    failed_steps: u64,
    overall_pass: bool,
) -> Verdict {
    if total_steps > 0 && passed_steps + failed_steps == 0 {
        Verdict::NotExecuted
    } else if overall_pass {
        Verdict::Pass
    } else {
        Verdict::Fail
    }
}

/// One per-case result file of the run, with the execution times of its log.
#[derive(Debug, Clone)]
pub struct CaseAttempt {
    pub test_case_id: String,
    /// File name without `_result.yaml`. Breaks ties between attempts of the same case.
    pub key: String,
    pub verdict: Verdict,
    pub total_steps: u64,
    pub passed_steps: u64,
    pub first: Option<DateTime<Utc>>,
    pub last: Option<DateTime<Utc>>,
}

/// The attempt that counts for a case. Among executed attempts the latest `started_at` wins. A
/// `not_executed` attempt counts only when no attempt was executed, so it never replaces an executed one.
pub fn winner(attempts: &[CaseAttempt]) -> Result<&CaseAttempt> {
    let executed: Vec<&CaseAttempt> = attempts
        .iter()
        .filter(|a| a.verdict.is_executed())
        .collect();
    let pool: Vec<&CaseAttempt> = if executed.is_empty() {
        attempts.iter().collect()
    } else {
        executed
    };
    pool.into_iter()
        .max_by(|a, b| (a.first, &a.key).cmp(&(b.first, &b.key)))
        .ok_or_else(|| anyhow!("no attempt to choose a winner from"))
}

/// The rendered documents and the verdict counts of one combined run.
#[derive(Debug)]
pub struct CombinedRun {
    pub test_run: String,
    pub verification_results: String,
    pub cases: usize,
    pub pass: usize,
    pub fail: usize,
    pub not_executed: usize,
}

#[derive(Deserialize)]
struct ResultFile {
    #[serde(rename = "type")]
    kind: Option<String>,
    test_case_id: String,
    total_steps: u64,
    passed_steps: u64,
    failed_steps: u64,
    overall_pass: bool,
}

#[derive(Serialize)]
struct TestRunDoc<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    schema: &'static str,
    run_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    build_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tester: Option<&'a str>,
    started_at: String,
    finished_at: String,
    results: Vec<ResultRow>,
}

#[derive(Serialize)]
struct ResultRow {
    test_case_id: String,
    verdict: &'static str,
}

#[derive(Serialize)]
struct VerificationResultsDoc<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    schema: &'static str,
    run_id: &'a str,
    test_results: Vec<StepRow>,
}

#[derive(Serialize)]
struct StepRow {
    test_case_id: String,
    total_steps: u64,
    passed_steps: u64,
}

/// Combine the per-case results of `meta.run_id` under `root`. Nothing is written: the caller writes
/// the two documents only when this returns `Ok`.
///
/// `schemas_root` is the `schemas/` directory of the repository; the test-run schema is looked up in
/// `tcms-schemas/tcms/` below it.
pub fn combine(root: &Path, meta: &RunMetadata, schemas_root: &str) -> Result<CombinedRun> {
    validate_run_id(meta.run_id)?;
    let run_dir = root.join("runs").join(meta.run_id);
    let per_case = run_dir.join("per-case");
    if !per_case.is_dir() {
        bail!(
            "run {}: no per-case results folder at {}",
            meta.run_id,
            per_case.display()
        );
    }

    let mut attempts = Vec::new();
    for file in yaml_files(&per_case)? {
        attempts.push(read_attempt(&run_dir, &file)?);
    }
    if attempts.is_empty() {
        bail!(
            "run {}: no per-case results in {}",
            meta.run_id,
            per_case.display()
        );
    }

    let mut by_case: BTreeMap<String, Vec<CaseAttempt>> = BTreeMap::new();
    for attempt in &attempts {
        by_case
            .entry(attempt.test_case_id.clone())
            .or_default()
            .push(attempt.clone());
    }
    let mut winners: Vec<&CaseAttempt> = Vec::with_capacity(by_case.len());
    for (case, group) in &by_case {
        winners.push(winner(group).with_context(|| format!("choosing the attempt for {case}"))?);
    }

    let started = attempts.iter().filter_map(|a| a.first).min();
    let finished = attempts.iter().filter_map(|a| a.last).max();
    let (Some(started), Some(finished)) = (started, finished) else {
        bail!(
            "run {}: no execution timestamps in the execution logs, so started_at cannot be set",
            meta.run_id
        );
    };

    let (mut pass, mut fail, mut not_executed) = (0, 0, 0);
    for w in &winners {
        match w.verdict {
            Verdict::Pass => pass += 1,
            Verdict::Fail => fail += 1,
            Verdict::NotExecuted => not_executed += 1,
        }
    }

    let test_run_doc = TestRunDoc {
        kind: "test_run",
        schema: "tcms/test-run.schema.json",
        run_id: meta.run_id,
        plan_id: meta.plan_id,
        build_id: meta.build_id,
        environment: meta.environment,
        tester: meta.tester,
        started_at: format_time(started),
        finished_at: format_time(finished),
        results: winners
            .iter()
            .map(|w| ResultRow {
                test_case_id: w.test_case_id.clone(),
                verdict: w.verdict.as_str(),
            })
            .collect(),
    };
    let steps_doc = VerificationResultsDoc {
        kind: "test_results_container",
        schema: "tcms/verification-results.schema.json",
        run_id: meta.run_id,
        test_results: winners
            .iter()
            .map(|w| StepRow {
                test_case_id: w.test_case_id.clone(),
                total_steps: w.total_steps,
                passed_steps: w.passed_steps,
            })
            .collect(),
    };

    let test_run = serde_yaml::to_string(&test_run_doc).context("rendering test-run.yaml")?;
    let verification_results =
        serde_yaml::to_string(&steps_doc).context("rendering verification-results.yaml")?;

    // The resolver maps `tcms/<name>` to `<dir>/tcms/<name>`. The test-run schema is vendored under
    // `tcms-schemas/`; the verification-results schema lives in `schemas/tcms/` (D-47-2).
    let test_run_dir = format!("{}/tcms-schemas/", schemas_root.trim_end_matches('/'));
    testcase_common::parse_and_validate_yaml_string::<serde_yaml::Value, _>(
        &test_run,
        &test_run_dir,
        "test-run.yaml",
    )
    .context("test-run.yaml fails schema validation; nothing was written")?;
    testcase_common::parse_and_validate_yaml_string::<serde_yaml::Value, _>(
        &verification_results,
        schemas_root,
        "verification-results.yaml",
    )
    .context("verification-results.yaml fails schema validation; nothing was written")?;

    Ok(CombinedRun {
        test_run,
        verification_results,
        cases: winners.len(),
        pass,
        fail,
        not_executed,
    })
}

fn validate_run_id(run_id: &str) -> Result<()> {
    let unsafe_id = run_id.is_empty()
        || run_id == "."
        || run_id == ".."
        || run_id.contains('/')
        || run_id.contains('\\');
    if unsafe_id {
        bail!("--run must be a single folder name, got '{run_id}'");
    }
    Ok(())
}

fn yaml_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let is_yaml = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml" | "yml")
        );
        if path.is_file() && is_yaml {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_attempt(run_dir: &Path, file: &Path) -> Result<CaseAttempt> {
    let text = fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let doc: ResultFile =
        serde_yaml::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
    if doc.kind.as_deref() != Some("test_result") {
        bail!("{}: type must be test_result", file.display());
    }
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let key = stem.strip_suffix("_result").unwrap_or(&stem).to_string();
    let log = run_dir
        .join("execution_logs")
        .join(format!("{key}_execution_log.json"));
    let Span { first, last } = log_times(&log)?;
    Ok(CaseAttempt {
        verdict: verdict_of(
            doc.total_steps,
            doc.passed_steps,
            doc.failed_steps,
            doc.overall_pass,
        ),
        test_case_id: doc.test_case_id,
        key,
        total_steps: doc.total_steps,
        passed_steps: doc.passed_steps,
        first,
        last,
    })
}

/// Earliest and latest `timestamp` of an execution log. A missing log gives no times.
fn log_times(log: &Path) -> Result<Span> {
    if !log.is_file() {
        return Ok(Span {
            first: None,
            last: None,
        });
    }
    let text = fs::read_to_string(log).with_context(|| format!("reading {}", log.display()))?;
    let doc: Json =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", log.display()))?;
    let entries = doc
        .as_array()
        .ok_or_else(|| anyhow!("{}: execution log must be a JSON array", log.display()))?;
    let mut times = Vec::new();
    for entry in entries {
        if let Some(raw) = entry.get("timestamp").and_then(Json::as_str) {
            let parsed = DateTime::parse_from_rfc3339(raw)
                .with_context(|| format!("{}: timestamp '{raw}' is not RFC 3339", log.display()))?;
            times.push(parsed.with_timezone(&Utc));
        }
    }
    Ok(Span {
        first: times.iter().min().copied(),
        last: times.iter().max().copied(),
    })
}

/// First and last execution time of one case.
struct Span {
    first: Option<DateTime<Utc>>,
    last: Option<DateTime<Utc>>,
}

fn format_time(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> Option<DateTime<Utc>> {
        Some(DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc))
    }

    fn attempt(key: &str, verdict: Verdict, first: Option<&str>) -> CaseAttempt {
        CaseAttempt {
            test_case_id: "CASE".into(),
            key: key.into(),
            verdict,
            total_steps: 1,
            passed_steps: u64::from(verdict == Verdict::Pass),
            first: first.and_then(at),
            last: first.and_then(at),
        }
    }

    #[test]
    fn verdict_mapping() {
        assert_eq!(verdict_of(2, 2, 0, true), Verdict::Pass);
        assert_eq!(verdict_of(2, 1, 1, false), Verdict::Fail);
        assert_eq!(verdict_of(2, 0, 0, false), Verdict::NotExecuted);
        assert_eq!(verdict_of(0, 0, 0, true), Verdict::Pass);
        assert_eq!(verdict_of(0, 0, 0, false), Verdict::Fail);
    }

    #[test]
    fn latest_started_executed_attempt_wins() {
        let a = [
            attempt("CASE__a1", Verdict::Pass, Some("2026-11-02T06:00:00Z")),
            attempt("CASE__a2", Verdict::Fail, Some("2026-11-02T09:00:00Z")),
        ];
        assert_eq!(winner(&a).unwrap().key, "CASE__a2");
        let reversed = [a[1].clone(), a[0].clone()];
        assert_eq!(winner(&reversed).unwrap().key, "CASE__a2");
    }

    #[test]
    fn not_executed_never_overrides_executed() {
        let a = [
            attempt("CASE", Verdict::Pass, Some("2026-11-02T06:00:00Z")),
            attempt(
                "CASE__late",
                Verdict::NotExecuted,
                Some("2026-11-02T10:00:00Z"),
            ),
        ];
        assert_eq!(winner(&a).unwrap().verdict, Verdict::Pass);
    }

    #[test]
    fn not_executed_wins_only_when_nothing_was_executed() {
        let a = [
            attempt(
                "CASE__a1",
                Verdict::NotExecuted,
                Some("2026-11-02T06:00:00Z"),
            ),
            attempt(
                "CASE__a2",
                Verdict::NotExecuted,
                Some("2026-11-02T09:00:00Z"),
            ),
        ];
        assert_eq!(winner(&a).unwrap().key, "CASE__a2");
    }

    #[test]
    fn winner_of_no_attempts_is_an_error() {
        assert!(winner(&[]).is_err());
    }

    #[test]
    fn rejects_run_ids_that_are_not_folder_names() {
        assert!(validate_run_id("../x").is_err());
        assert!(validate_run_id("a/b").is_err());
        assert!(validate_run_id("").is_err());
        assert!(validate_run_id("RUN-1").is_ok());
    }

    #[test]
    fn empty_per_case_folder_is_refused() {
        let tmp = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("runs/RUN-E/per-case")).unwrap();
        let meta = RunMetadata {
            run_id: "RUN-E",
            plan_id: None,
            build_id: Some("b"),
            environment: Some("e"),
            tester: Some("t"),
        };
        let err = combine(tmp.path(), &meta, "unused").unwrap_err();
        assert!(format!("{err:#}").contains("no per-case results"));
    }

    fn steps_doc(entry: &str) -> String {
        format!(
            "type: test_results_container\nschema: tcms/verification-results.schema.json\n\
             run_id: RUN-1\ntest_results:\n{entry}"
        )
    }

    fn validate_steps(text: &str) -> Result<serde_yaml::Value> {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas");
        testcase_common::parse_and_validate_yaml_string(text, root, "verification-results.yaml")
    }

    #[test]
    fn steps_schema_accepts_well_formed_counts() {
        let ok = steps_doc("- test_case_id: A\n  total_steps: 2\n  passed_steps: 1\n");
        assert!(validate_steps(&ok).is_ok());
    }

    #[test]
    fn steps_schema_rejects_missing_passed_steps() {
        let bad = steps_doc("- test_case_id: A\n  total_steps: 2\n");
        assert!(validate_steps(&bad).is_err());
    }

    #[test]
    fn steps_schema_rejects_non_integer_counts() {
        let bad = steps_doc("- test_case_id: A\n  total_steps: '2'\n  passed_steps: 1\n");
        assert!(validate_steps(&bad).is_err());
    }

    /// AC 4 (D-47-3): a case with no per-case file gets no entry, even when its execution log exists.
    #[test]
    fn case_without_a_per_case_file_is_absent_from_the_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("runs").join("RUN-ABS");
        fs::create_dir_all(run_dir.join("per-case")).unwrap();
        fs::create_dir_all(run_dir.join("execution_logs")).unwrap();
        let result = "type: test_result\nschema: tcms/test-result.schema.v1.json\n\
                      test_case_id: CASE_A\ndescription: A\nsequences: []\ntotal_steps: 2\n\
                      passed_steps: 2\nfailed_steps: 0\nnot_executed_steps: 0\noverall_pass: true\n";
        fs::write(run_dir.join("per-case").join("CASE_A_result.yaml"), result).unwrap();
        let log = "[{\"test_sequence\": 1, \"step\": 1, \"command\": \"true\", \"exit_code\": 0, \
                   \"output\": \"\", \"result_verification_pass\": true, \
                   \"output_verification_pass\": true, \"timestamp\": \"2026-11-02T06:00:00Z\"}]\n";
        fs::write(
            run_dir
                .join("execution_logs")
                .join("CASE_A_execution_log.json"),
            log,
        )
        .unwrap();
        // CASE_B has an execution log but no per-case file: it must not appear in either document.
        fs::write(
            run_dir
                .join("execution_logs")
                .join("CASE_B_execution_log.json"),
            log,
        )
        .unwrap();

        let meta = RunMetadata {
            run_id: "RUN-ABS",
            plan_id: None,
            build_id: Some("build-abs"),
            environment: Some("bench"),
            tester: Some("tester"),
        };
        let schemas = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schemas");
        let combined = combine(tmp.path(), &meta, schemas).unwrap();
        assert_eq!(combined.cases, 1);
        assert!(combined.test_run.contains("CASE_A"));
        assert!(!combined.test_run.contains("CASE_B"));
        assert!(!combined.verification_results.contains("CASE_B"));
    }
}
