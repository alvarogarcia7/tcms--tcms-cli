//! `tcms vv metrics`: V&V metrics and the quality-gate verdict of a recorded V&V data set (TCMS-47).
//!
//! The data root holds `runs/<RUN_ID>/test-run.yaml` (and optionally `verification-results.yaml`),
//! the test case files (`type: test_case`, anywhere under the root except `runs/`) and an optional
//! `requirements.json`. The output is one JSON document; see `docs/VV_METRICS.md` for the fields.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context as _, Result};
use clap::Subcommand;
use serde::Deserialize;
use serde_json::{json, Value as Json};
use testcase_common::load_and_validate_yaml;

use crate::refcmd::Context;
use crate::vvcombine;
use crate::vvmetrics::{self, Coverage, Ratio, Stability, Status};

/// At most this many case files with schema errors are listed (as VvLoadSummarizer.MAX_ERRORS).
const MAX_SCHEMA_ERRORS: usize = 20;

#[derive(Subcommand)]
pub enum VvCommand {
    /// Compute the V&V metrics and gate verdict of the recorded runs under a data root, as JSON
    Metrics {
        /// Root of the recorded V&V data (runs/, test case files, requirements.json)
        #[arg(long)]
        root: PathBuf,
        /// Run to report (default: the last recorded run by start time)
        #[arg(long)]
        run: Option<String>,
        /// Plan to report on: only the runs whose `plan_id` is this plan. Error when no run has it.
        #[arg(long)]
        plan: Option<String>,
        /// Quality-gate threshold in percent, 50 to 100 (D-89)
        #[arg(long, default_value_t = 90)]
        gate: u64,
        /// Flips (changes between neighbouring kept results) at or above which a case is Flaky (D-97)
        #[arg(long, default_value_t = 4)]
        flaky_threshold: u64,
        /// Write the JSON report to this file (parent directories are created) instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
        /// Time to record as `generatedAt` (RFC 3339). Default: $TCMS_NOW, else the current time.
        /// Fixing it makes the output byte-for-byte reproducible.
        #[arg(long)]
        now: Option<String>,
    },
    /// Combine the per-case results of one run (runs/<RUN>/per-case, runs/<RUN>/execution_logs)
    /// into runs/<RUN>/test-run.yaml and runs/<RUN>/verification-results.yaml (TCMS-47.3)
    Combine {
        /// Root of the recorded V&V data
        #[arg(long)]
        root: PathBuf,
        /// Run folder name under runs/ to combine
        #[arg(long)]
        run: String,
        /// Plan the run executes (`plan_id`)
        #[arg(long)]
        plan: Option<String>,
        /// Build under test (`build_id`; required by the test-run schema)
        #[arg(long)]
        build_id: Option<String>,
        /// Environment description (required by the test-run schema)
        #[arg(long)]
        environment: Option<String>,
        /// Who executed the run (required by the test-run schema)
        #[arg(long)]
        tester: Option<String>,
    },
}

pub fn run(ctx: &Context, cmd: VvCommand) -> Result<()> {
    match cmd {
        VvCommand::Metrics {
            root,
            run,
            plan,
            gate,
            flaky_threshold,
            out,
            now,
        } => {
            if !(50..=100).contains(&gate) {
                bail!("--gate must be between 50 and 100, got {gate}");
            }
            if flaky_threshold == 0 {
                bail!("--flaky-threshold must be at least 1");
            }
            let generated_at =
                resolve_now(now.as_deref(), std::env::var("TCMS_NOW").ok().as_deref())?;
            let data = load(&root, ctx.schemas_root())?;
            let report = build_report(
                &data,
                run.as_deref(),
                plan.as_deref(),
                gate,
                flaky_threshold,
                &generated_at,
            )?;
            let text = format!("{}\n", serde_json::to_string_pretty(&report)?);
            match out {
                Some(path) => write_atomic(&path, text.as_bytes())?,
                None => print!("{text}"),
            }
            Ok(())
        }
        VvCommand::Combine {
            root,
            run,
            plan,
            build_id,
            environment,
            tester,
        } => {
            if !root.is_dir() {
                bail!("V&V data root {} is not a directory", root.display());
            }
            let meta = vvcombine::RunMetadata {
                run_id: &run,
                plan_id: plan.as_deref(),
                build_id: build_id.as_deref(),
                environment: environment.as_deref(),
                tester: tester.as_deref(),
            };
            // Both documents are validated inside combine(); nothing is written when that fails.
            let combined = vvcombine::combine(&root, &meta, ctx.schemas_root())?;
            let run_dir = root.join("runs").join(&run);
            write_atomic(&run_dir.join("test-run.yaml"), combined.test_run.as_bytes())?;
            write_atomic(
                &run_dir.join("verification-results.yaml"),
                combined.verification_results.as_bytes(),
            )?;
            println!(
                "combined run {run}: {} cases, {} pass, {} fail, {} not_executed",
                combined.cases, combined.pass, combined.fail, combined.not_executed
            );
            Ok(())
        }
    }
}

/// The `generatedAt` value: `--now`, else `$TCMS_NOW`, else the current time; always UTC, whole seconds.
fn resolve_now(arg: Option<&str>, env: Option<&str>) -> Result<String> {
    let raw = match (arg, env) {
        (Some(a), _) => Some(a),
        (None, Some(e)) if !e.is_empty() => Some(e),
        _ => None,
    };
    match raw {
        Some(text) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(text)
                .with_context(|| format!("--now / TCMS_NOW must be RFC 3339, got '{text}'"))?;
            Ok(parsed
                .with_timezone(&chrono::Utc)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string())
        }
        None => Ok(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    }
}

/// Write `bytes` to `path` through a temporary file in the same directory, so a reader never sees a
/// partial report. Parent directories are created.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(format!(".tmp-{}", std::process::id()));
    let tmp = path.with_file_name(tmp_name);
    fs::write(&tmp, bytes).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CaseResult {
    case_id: String,
    status: Status,
    problem_report: Option<String>,
}

#[derive(Debug, Clone)]
struct RecordedRun {
    run_id: String,
    /// The plan the run executes (`plan_id` in test-run.yaml), if recorded.
    plan_id: Option<String>,
    started_at: String,
    finished_at: Option<String>,
    results: Vec<CaseResult>,
    /// Case id -> (total steps, passed steps), from verification-results.yaml. Missing: (0, 0).
    steps: BTreeMap<String, (u64, u64)>,
    /// Case ids that appear in verification-results.yaml of this run.
    verified_case_ids: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct CaseFile {
    id: String,
    requirement: Option<String>,
}

#[derive(Debug, Clone)]
struct SchemaError {
    file: String,
    error: String,
}

#[derive(Debug, Deserialize)]
struct CaseHead {
    id: String,
    requirement: Option<String>,
}

#[derive(Debug)]
struct VvData {
    runs: Vec<RecordedRun>,
    cases: Vec<CaseFile>,
    schema_errors: Vec<SchemaError>,
    requirement_ids: Vec<String>,
}

fn load(root: &Path, schemas_root: &str) -> Result<VvData> {
    if !root.is_dir() {
        bail!("V&V data root {} is not a directory", root.display());
    }
    let runs = load_runs(root)?;
    let (cases, schema_errors) = load_cases(root, schemas_root)?;
    let requirement_ids = load_requirement_ids(root)?;
    Ok(VvData {
        runs,
        cases,
        schema_errors,
        requirement_ids,
    })
}

fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    Ok(paths)
}

fn load_yaml(path: &Path) -> Result<serde_yaml::Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn str_field(v: &serde_yaml::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(str::to_string)
}

fn u64_field(v: &serde_yaml::Value, key: &str) -> Option<u64> {
    v.get(key).and_then(|x| x.as_u64())
}

fn load_runs(root: &Path) -> Result<Vec<RecordedRun>> {
    let runs_dir = root.join("runs");
    if !runs_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    for dir in sorted_entries(&runs_dir)? {
        let record = dir.join("test-run.yaml");
        if !record.is_file() {
            continue;
        }
        let doc = load_yaml(&record)?;
        let dir_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let run_id = str_field(&doc, "run_id").unwrap_or(dir_name);
        let mut results = Vec::new();
        for item in doc
            .get("results")
            .and_then(|r| r.as_sequence())
            .into_iter()
            .flatten()
        {
            let case_id = str_field(item, "test_case_id")
                .ok_or_else(|| anyhow!("{}: result without test_case_id", record.display()))?;
            let verdict = str_field(item, "verdict")
                .ok_or_else(|| anyhow!("{}: result {case_id} without verdict", record.display()))?;
            let status = Status::from_verdict(&verdict).ok_or_else(|| {
                anyhow!(
                    "{}: result {case_id} has unknown verdict '{verdict}'",
                    record.display()
                )
            })?;
            results.push(CaseResult {
                case_id,
                status,
                problem_report: str_field(item, "problem_report_id"),
            });
        }
        let (steps, verified_case_ids) = load_steps(&dir.join("verification-results.yaml"))?;
        runs.push(RecordedRun {
            run_id,
            plan_id: str_field(&doc, "plan_id"),
            started_at: str_field(&doc, "started_at").unwrap_or_default(),
            finished_at: str_field(&doc, "finished_at"),
            results,
            steps,
            verified_case_ids,
        });
    }
    runs.sort_by(|a, b| {
        (a.started_at.as_str(), a.run_id.as_str()).cmp(&(b.started_at.as_str(), b.run_id.as_str()))
    });
    Ok(runs)
}

type StepTable = (BTreeMap<String, (u64, u64)>, BTreeSet<String>);

fn load_steps(path: &Path) -> Result<StepTable> {
    let mut steps = BTreeMap::new();
    let mut ids = BTreeSet::new();
    if !path.is_file() {
        return Ok((steps, ids));
    }
    let doc = load_yaml(path)?;
    for item in doc
        .get("test_results")
        .and_then(|r| r.as_sequence())
        .into_iter()
        .flatten()
    {
        if let Some(id) = str_field(item, "test_case_id") {
            let total = u64_field(item, "total_steps").unwrap_or(0);
            let passed = u64_field(item, "passed_steps").unwrap_or(0);
            steps.insert(id.clone(), (total, passed));
            ids.insert(id);
        }
    }
    Ok((steps, ids))
}

/// Case files: every YAML file with `type: test_case` under the root, except those in `runs/`.
/// Each is validated against its schema; a file that fails is a schema error (its first error is kept).
fn load_cases(root: &Path, schemas_root: &str) -> Result<(Vec<CaseFile>, Vec<SchemaError>)> {
    let mut files = Vec::new();
    collect_yaml(root, root, &mut files)?;
    let mut cases = Vec::new();
    let mut errors = Vec::new();
    for path in files {
        let Ok(doc) = load_yaml(&path) else { continue };
        if str_field(&doc, "type").as_deref() != Some("test_case") {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        match load_and_validate_yaml::<CaseHead, _, _>(&path, schemas_root) {
            Ok(head) => cases.push(CaseFile {
                id: head.id,
                requirement: head.requirement,
            }),
            Err(e) => errors.push(SchemaError {
                file: rel,
                error: first_error_line(&format!("{e:#}")),
            }),
        }
    }
    Ok((cases, errors))
}

fn collect_yaml(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for path in sorted_entries(dir)? {
        if path.is_dir() {
            if path != root.join("runs") {
                collect_yaml(root, &path, out)?;
            }
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yml") | Some("yaml")
        ) {
            out.push(path);
        }
    }
    Ok(())
}

/// The first constraint violation of a validation error, or else the first line of the error.
fn first_error_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("Error #"))
        .or_else(|| message.lines().map(str::trim).find(|l| !l.is_empty()))
        .unwrap_or("")
        .to_string()
}

fn load_requirement_ids(root: &Path) -> Result<Vec<String>> {
    let path = root.join("requirements.json");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let doc: Json =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(doc
        .get("requirements")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(|r| r.get("id").and_then(Json::as_str).map(str::to_string))
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------------------------

/// `{display, exact}` for a ratio, or `null` when it has no denominator.
fn percent_json(ratio: Option<Ratio>, display: fn(&Ratio) -> u64) -> Json {
    match ratio {
        None => Json::Null,
        Some(r) => json!({ "display": display(&r), "exact": r.exact() }),
    }
}

fn build_report(
    all: &VvData,
    run_id: Option<&str>,
    plan: Option<&str>,
    gate: u64,
    flaky: u64,
    generated_at: &str,
) -> Result<Json> {
    if all.runs.is_empty() {
        bail!("no recorded V&V runs (runs/*/test-run.yaml) under the data root");
    }
    // With --plan, only the runs of that plan take part: selection, previous run and stability.
    let scoped;
    let data = match plan {
        Some(p) => {
            scoped = VvData {
                runs: all
                    .runs
                    .iter()
                    .filter(|r| r.plan_id.as_deref() == Some(p))
                    .cloned()
                    .collect(),
                cases: all.cases.clone(),
                schema_errors: all.schema_errors.clone(),
                requirement_ids: all.requirement_ids.clone(),
            };
            if scoped.runs.is_empty() {
                bail!("no recorded run for plan {p} (no run has plan_id {p})");
            }
            &scoped
        }
        None => all,
    };
    let sel = match run_id {
        Some(id) => data
            .runs
            .iter()
            .position(|r| r.run_id == id)
            .ok_or_else(|| match plan {
                Some(p) => anyhow!("run {id} is not a run of plan {p}"),
                None => anyhow!("run {id} is not among the recorded runs"),
            })?,
        None => data.runs.len() - 1,
    };
    let run = &data.runs[sel];
    if run.results.is_empty() {
        bail!(
            "run {} has no test cases; a run with zero test cases is an error",
            run.run_id
        );
    }
    let previous = sel.checked_sub(1).map(|i| &data.runs[i]);

    // Gate (D-95) and delta (D-96).
    let mut counts = vvmetrics::Counts::default();
    for r in &run.results {
        counts.add(r.status);
    }
    let gate_result = vvmetrics::gate(counts, gate);
    let previous_rate = previous.and_then(|p| {
        let mut c = vvmetrics::Counts::default();
        for r in &p.results {
            c.add(r.status);
        }
        vvmetrics::gate(c, gate).pass_rate
    });
    let delta = vvmetrics::delta(gate_result.pass_rate, previous_rate);

    // V&V run figures (TestRunDataService): test cases and steps.
    let total_cases = run.results.len() as u64;
    let passed_cases = run
        .results
        .iter()
        .filter(|r| r.status == Status::Passed)
        .count() as u64;
    let (mut total_steps, mut passed_steps) = (0u64, 0u64);
    for r in &run.results {
        if let Some((t, p)) = run.steps.get(&r.case_id) {
            total_steps += t;
            passed_steps += p;
        }
    }
    let vv_pass_rate = Ratio::new(passed_cases, total_cases);
    let step_coverage = Ratio::new(passed_steps, total_steps);

    let requirements = requirement_coverage(data, run);
    let stability = stability_section(data, sel, flaky);

    let provisional = run.finished_at.is_none();
    let schema_errors: Vec<Json> = data
        .schema_errors
        .iter()
        .take(MAX_SCHEMA_ERRORS)
        .map(|e| json!({ "file": e.file, "error": e.error }))
        .collect();
    let mut distinct: BTreeSet<&str> = data.cases.iter().map(|c| c.id.as_str()).collect();
    for r in &data.runs {
        distinct.extend(r.verified_case_ids.iter().map(String::as_str));
    }

    Ok(json!({
        "generatedAt": generated_at,
        "run": {
            "runId": run.run_id,
            "startedAt": run.started_at,
            "finishedAt": run.finished_at,
            "provisional": provisional,
        },
        "gate": {
            "gateThresholdPercent": gate,
            "denominator": gate_result.counts.denominator(),
            "counts": {
                "passed": counts.passed,
                "failed": counts.failed,
                "blocked": counts.blocked,
                "skipped": counts.skipped,
                "untested": counts.untested,
            },
            "passRate": percent_json(gate_result.pass_rate, Ratio::floor_percent),
            "verdict": gate_result.verdict.as_str(),
            "provisional": provisional,
            "deltaFromPrevious": delta,
            "previousRunId": previous.map(|p| p.run_id.clone()),
        },
        "vvRun": {
            "totalTestCases": total_cases,
            "passedTestCases": passed_cases,
            "vvPassRate": percent_json(vv_pass_rate, Ratio::round_percent),
            "totalSteps": total_steps,
            "passedSteps": passed_steps,
            "stepCoverage": percent_json(step_coverage, Ratio::round_percent),
        },
        "requirements": requirements,
        "stability": stability,
        "loadSummary": {
            "distinctCases": distinct.len(),
            "schemaErrorCount": data.schema_errors.len(),
            "schemaErrors": schema_errors,
            "runsRecorded": data.runs.len(),
        },
    }))
}

/// Requirement coverage in the selected run (D-116.3). Requirements are those listed in
/// requirements.json plus those cited by a case file.
fn requirement_coverage(data: &VvData, run: &RecordedRun) -> Json {
    let mut ids: BTreeSet<&str> = data.requirement_ids.iter().map(String::as_str).collect();
    for c in &data.cases {
        if let Some(r) = &c.requirement {
            ids.insert(r.as_str());
        }
    }

    let mut total_cov = [0u64; 4]; // covered, failing, not run, no cases
    let mut with_defect = 0u64;
    let mut open_defects: BTreeSet<String> = BTreeSet::new();
    let mut items = Vec::new();
    for id in ids {
        let linked: Vec<&CaseFile> = data
            .cases
            .iter()
            .filter(|c| c.requirement.as_deref() == Some(id))
            .collect();
        let mut statuses = Vec::new();
        let mut case_rows = Vec::new();
        let mut problem_reports: BTreeSet<String> = BTreeSet::new();
        let mut open_here: BTreeSet<String> = BTreeSet::new();
        for case in &linked {
            let results: Vec<&CaseResult> = run
                .results
                .iter()
                .filter(|r| r.case_id == case.id)
                .collect();
            // The last result of the case in the run is the one that counts.
            let status = results.last().map(|r| r.status);
            statuses.push(status);
            case_rows.push(json!({
                "caseId": case.id,
                "status": status.map(Status::as_str),
            }));
            for r in &results {
                if let Some(pr) = &r.problem_report {
                    problem_reports.insert(pr.clone());
                    if r.status != Status::Passed {
                        open_here.insert(pr.clone());
                    }
                }
            }
        }
        let cov = vvmetrics::coverage(&statuses);
        match cov.coverage {
            Coverage::Covered => total_cov[0] += 1,
            Coverage::Failing => total_cov[1] += 1,
            Coverage::NotRun => total_cov[2] += 1,
            Coverage::NoCases => total_cov[3] += 1,
        }
        if !problem_reports.is_empty() {
            with_defect += 1;
        }
        open_defects.extend(open_here.iter().cloned());
        items.push(json!({
            "id": id,
            "coverage": cov.coverage.as_str(),
            "executed": cov.executed,
            "total": cov.total,
            "cases": case_rows,
            "problemReports": problem_reports.into_iter().collect::<Vec<_>>(),
            "openProblemReports": open_here.into_iter().collect::<Vec<_>>(),
        }));
    }
    json!({
        "summary": {
            "total": items.len(),
            "covered": total_cov[0],
            "failing": total_cov[1],
            "notRun": total_cov[2],
            "noCases": total_cov[3],
            "withDefect": with_defect,
            "openDefects": open_defects.len(),
        },
        "items": items,
    })
}

/// Stability (D-97) of every case in the selected run, over the runs up to and including it.
fn stability_section(data: &VvData, sel: usize, flaky: u64) -> Json {
    let run = &data.runs[sel];
    let case_ids: BTreeSet<&str> = run.results.iter().map(|r| r.case_id.as_str()).collect();
    let mut flaky_count = 0u64;
    let mut cases = Vec::new();
    for id in case_ids {
        let history: Vec<Status> = data.runs[..=sel]
            .iter()
            .flat_map(|r| r.results.iter())
            .filter(|r| r.case_id == id)
            .map(|r| r.status)
            .collect();
        let s = vvmetrics::stability(&history, flaky);
        if s.stability == Stability::Flaky {
            flaky_count += 1;
        }
        cases.push(json!({
            "caseId": id,
            "stability": s.stability.as_str(),
            "flips": s.flips,
        }));
    }
    json!({
        "flakyThreshold": flaky,
        "flakyCount": flaky_count,
        "cases": cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_error_line_prefers_constraint_violation() {
        let msg = "Failed to validate\n\nSchema constraint violations:\n  Error #1: Path '/id'\n  Error #2: x";
        assert_eq!(first_error_line(msg), "Error #1: Path '/id'");
        assert_eq!(first_error_line("plain failure\nmore"), "plain failure");
        assert_eq!(first_error_line(""), "");
    }

    #[test]
    fn requirement_ids_are_read_from_requirements_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("requirements.json"),
            r#"{"requirements":[{"id":"A-1"},{"id":"A-2"}]}"#,
        )
        .unwrap();
        assert_eq!(
            load_requirement_ids(dir.path()).unwrap(),
            vec!["A-1", "A-2"]
        );
        let empty = tempfile::tempdir().unwrap();
        assert!(load_requirement_ids(empty.path()).unwrap().is_empty());
    }

    #[test]
    fn run_with_zero_test_cases_is_an_error() {
        let data = VvData {
            runs: vec![RecordedRun {
                run_id: "R-EMPTY".to_string(),
                plan_id: None,
                started_at: "2026-01-01T00:00:00Z".to_string(),
                finished_at: Some("2026-01-01T01:00:00Z".to_string()),
                results: Vec::new(),
                steps: BTreeMap::new(),
                verified_case_ids: BTreeSet::new(),
            }],
            cases: Vec::new(),
            schema_errors: Vec::new(),
            requirement_ids: Vec::new(),
        };
        let err = build_report(&data, None, None, 90, 4, "2026-01-02T00:00:00Z")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("R-EMPTY") && err.contains("zero test cases"),
            "{err}"
        );
    }

    #[test]
    fn unknown_verdict_is_an_error_naming_the_case() {
        let dir = tempfile::tempdir().unwrap();
        let run = dir.path().join("runs/R1");
        fs::create_dir_all(&run).unwrap();
        fs::write(
            run.join("test-run.yaml"),
            "run_id: R1\nresults:\n- test_case_id: T1\n  verdict: maybe\n",
        )
        .unwrap();
        let err = load_runs(dir.path()).unwrap_err().to_string();
        assert!(err.contains("T1") && err.contains("maybe"), "{err}");
    }
}
