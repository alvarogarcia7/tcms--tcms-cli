//! `tcms jira sync` (TCMS-42.9): one command for the write-backs of a product.
//!
//! Steps, in this order: `import` (requirements from the configured source), `tests` (test
//! status and run verdicts), `features` (per-feature roll-up), `milestones` (test plans as Fix
//! Versions), `defects` (problem reports as Jira defects, reconciliation). `--only` selects steps.
//!
//! Decisions (TCMS-42.9, worker C3):
//! * dry run is the default; `--apply` writes. A dry run sends no write and writes no file;
//!   reads (Jira searches, GETs) still run so the plan is real.
//! * State lives in `--state-dir` (default `.tcms/jira-sync`) as `state.yaml`: the entity key
//!   of each test case, the values TCMS last wrote (for hand-edit detection), the Fix Version id
//!   of each test plan. Problem-report keys live in the problem reports (see `tcms pr`).
//!   The state never holds a secret: the token is only read from the environment.
//! * Audit: one `audit-trail-entry` per record the run changed, in `<state-dir>/audit/<run-id>/`.
//!   Each names the acting user (`--as`, `$TCMS_USER`, `$USER`) and the fields written.
//! * Jira comments (when `write_policy.comments` is on): one summary comment per feature whose
//!   roll-up was written; its body names the acting user (jira-client adds it).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use clap::Args;
use id_resolver::{IntegrationConfig, SystemConfig};
use jira_client::{write_audit_entries, AuditTrailEntry, JiraClient, WriteTarget};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use test_backends::defects::{self, IssueState, ProblemReport, PushOutcome};
use test_backends::milestone::{MilestoneSync, PlanState};
use test_backends::rollup::{FeatureWriter, PlannedWrite, ReqState, RollupInput, ValidationState};
use test_backends::{
    build_backend, latest_verdict, status_fields, sync_run, CaseStatus, CaseSync, Ctx, EntityKey,
    JiraApi, LifecycleStatus, RunCase, RunSync, RunVerdict, StatusValue, TestBackend,
};
use testcase_models::{ExternalSystem, TestCase, TestCaseStatus};
use testcase_verification::{build_matrix, read_requirements, TestRunRecord, TraceMatrix, Verdict};

use crate::extrefs;
use crate::jiracmd::{acting_user, pick_jira_system};
use crate::prcmd;
use crate::refcmd::Context;

pub const STEPS: &[&str] = &["import", "tests", "features", "milestones", "defects"];

#[derive(Args, Clone, Debug)]
pub struct SyncArgs {
    /// Product id; must equal product_id of the integration config
    #[arg(long)]
    pub product: String,

    /// Write to Jira and to the sync state and audit files. Without it: a dry run (default)
    #[arg(long)]
    pub apply: bool,

    /// Plan only (the default; cannot be combined with --apply)
    #[arg(long, conflicts_with = "apply")]
    pub dry_run: bool,

    /// Run only these steps, comma separated: import, tests, features, milestones, defects
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Jira system name (needed when the config has several)
    #[arg(long)]
    pub system: Option<String>,

    /// Acting TCMS user (default: $TCMS_USER, then $USER)
    #[arg(long = "as")]
    pub actor: Option<String>,

    /// Test case YAML files (searched recursively)
    #[arg(long, default_value = "testcases")]
    pub tests_dir: PathBuf,

    /// Requirement records (written by the import step, read by the matrix)
    #[arg(long, default_value = "requirements")]
    pub requirements_dir: PathBuf,

    /// Test run records (`verifier --test-run` output)
    #[arg(long, default_value = "test-runs")]
    pub runs_dir: PathBuf,

    /// Problem report records
    #[arg(long, default_value = "problem-reports")]
    pub problem_reports: PathBuf,

    /// Test plan records (each becomes a Fix Version when milestone.target is fix_version)
    #[arg(long, default_value = "test-plans")]
    pub test_plans: PathBuf,

    /// Where state.yaml and audit/ live
    #[arg(long, default_value = ".tcms/jira-sync")]
    pub state_dir: PathBuf,

    /// Baseline id recorded on the trace matrix and used for plan progress
    #[arg(long)]
    pub baseline: Option<String>,

    /// Release date (YYYY-MM-DD) for a plan that completes now (default: today)
    #[arg(long)]
    pub release_date: Option<String>,

    /// StrictDoc source only: JSON export of the document set
    #[arg(long)]
    pub strictdoc_input: Option<PathBuf>,

    /// StrictDoc source only: commit the export was taken at
    #[arg(long)]
    pub commit: Option<String>,
}

/// Persisted between runs. Never contains a secret.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(rename = "type")]
    pub doc_type: String,
    pub schema: String,
    pub product_id: String,
    /// Test case id to its Jira entity and the values TCMS last wrote
    #[serde(default)]
    pub test_cases: BTreeMap<String, TestState>,
    /// Test plan id to its Fix Version
    #[serde(default)]
    pub plans: BTreeMap<String, PlanRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestState {
    pub system: String,
    pub key: String,
    #[serde(default)]
    pub last_written: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanRecord {
    pub version_id: String,
}

/// Plan of one sync run, printed as it is made; `audit` holds what a write changed.
struct Log {
    run_id: String,
    actor: String,
    apply: bool,
    audit: Vec<AuditTrailEntry>,
    planned: usize,
    differences: usize,
}

impl Log {
    fn line(&self, s: impl AsRef<str>) {
        println!("{}", s.as_ref());
    }

    /// A write that would happen (dry run) or did happen (apply).
    fn plan(&mut self, s: impl AsRef<str>) {
        self.planned += 1;
        let tag = if self.apply { "write" } else { "would write" };
        println!("  {tag}: {}", s.as_ref());
    }

    fn difference(&mut self, s: impl AsRef<str>) {
        self.differences += 1;
        println!("  difference (not changed): {}", s.as_ref());
    }

    /// Audit entry for a completed write (the changes of one record).
    fn audit(&mut self, target: &WriteTarget, written: &[String], refs: Vec<Value>, reason: &str) {
        let id = format!("{}-{:03}", self.run_id, self.audit.len() + 1);
        self.audit.push(AuditTrailEntry::sync(
            id,
            &self.actor,
            target,
            refs,
            written,
            reason,
        ));
    }
}

pub fn run(ctx: &Context, args: SyncArgs) -> Result<()> {
    let cfg = ctx.load_config()?;
    if args.product != cfg.product_id {
        bail!(
            "--product {} does not match product_id {} of the integration config",
            args.product,
            cfg.product_id
        );
    }
    let steps = selected_steps(&args.only)?;
    let actor = acting_user(args.actor.clone());
    let run_id = chrono::Utc::now().format("SYNC-%Y%m%dT%H%M%SZ").to_string();
    let mode = if args.apply { "apply" } else { "dry run" };
    let mut log = Log {
        run_id: run_id.clone(),
        actor: actor.clone(),
        apply: args.apply,
        audit: Vec::new(),
        planned: 0,
        differences: 0,
    };
    log.line(format!(
        "sync {run_id} for product {} as {actor}: {mode}; steps: {}",
        cfg.product_id,
        steps.join(", ")
    ));

    let state_path = args.state_dir.join("state.yaml");
    let mut state = load_state(&state_path, &cfg.product_id)?;
    let schemas = ctx.schemas_root().to_string();
    let jira = cfg.systems.iter().any(|s| s.system_type.is_jira());

    for step in &steps {
        match step.as_str() {
            "import" => import_step(&cfg, &args, &actor, &mut log, &schemas)?,
            _ if !jira => log.line(format!(
                "step {step}: skipped, the integration config has no Jira system"
            )),
            "tests" => tests_step(&cfg, &args, &actor, &mut log, &mut state, &schemas)?,
            "features" => features_step(&cfg, &args, &actor, &mut log, &schemas)?,
            "milestones" => milestones_step(&cfg, &args, &actor, &mut log, &mut state, &schemas)?,
            "defects" => defects_step(&cfg, &args, &actor, &mut log, &state, &schemas)?,
            other => bail!("unknown step '{other}'"),
        }
    }

    let mut audit_dir = None;
    if args.apply {
        std::fs::create_dir_all(&args.state_dir)
            .with_context(|| format!("create {}", args.state_dir.display()))?;
        std::fs::write(&state_path, testcase_common::to_yaml_string(&state)?)
            .with_context(|| format!("write {}", state_path.display()))?;
        if !log.audit.is_empty() {
            let dir = args.state_dir.join("audit").join(&run_id);
            write_audit_entries(&dir, &log.audit)?;
            audit_dir = Some(dir);
        }
    }

    log.line(format!(
        "summary: {} write(s) {}, {} planned, {} difference(s) reported",
        log.audit.len(),
        if args.apply { "applied" } else { "not applied" },
        log.planned,
        log.differences
    ));
    if let Some(d) = audit_dir {
        log.line(format!("audit: {}", d.display()));
    }
    if args.apply {
        log.line(format!("state: {}", state_path.display()));
    } else {
        log.line("dry run: nothing written; rerun with --apply");
    }
    Ok(())
}

fn selected_steps(only: &[String]) -> Result<Vec<String>> {
    if only.is_empty() {
        return Ok(STEPS.iter().map(|s| s.to_string()).collect());
    }
    for s in only {
        if !STEPS.contains(&s.as_str()) {
            bail!("--only: unknown step '{s}' (steps: {})", STEPS.join(", "));
        }
    }
    // keep the fixed order of the steps
    Ok(STEPS
        .iter()
        .filter(|s| only.iter().any(|o| o == *s))
        .map(|s| s.to_string())
        .collect())
}

fn load_state(path: &Path, product: &str) -> Result<State> {
    if !path.exists() {
        return Ok(State {
            doc_type: "jira_sync_state".into(),
            schema: "tcms/jira-sync-state.schema.json (local, not vendored)".into(),
            product_id: product.into(),
            ..Default::default()
        });
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let state: State = serde_yaml::from_str(&text)
        .with_context(|| format!("parse sync state {}", path.display()))?;
    if state.product_id != product {
        bail!(
            "sync state {} belongs to product {}, not {product}",
            path.display(),
            state.product_id
        );
    }
    Ok(state)
}

fn jira_system<'a>(cfg: &'a IntegrationConfig, args: &SyncArgs) -> Result<&'a SystemConfig> {
    pick_jira_system(&cfg.systems, args.system.as_deref())
}

fn jira_api(cfg: &IntegrationConfig, sys: &SystemConfig, actor: &str) -> Result<JiraApi> {
    Ok(JiraApi::new(JiraClient::from_config(
        sys, &cfg.sync, actor,
    )?))
}

// ------------------------------------------------------------------ inputs

/// Every `*.yaml` / `*.yml` file under `dir`, sorted. A missing directory is empty.
fn yaml_files(dir: &Path) -> Result<Vec<PathBuf>> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
            let p = entry?.path();
            if p.is_dir() {
                walk(&p, out)?;
            } else if matches!(p.extension().and_then(|e| e.to_str()), Some("yaml" | "yml")) {
                out.push(p);
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if dir.is_dir() {
        walk(dir, &mut out)?;
    }
    out.sort();
    Ok(out)
}

/// Every test case with the file it was read from.
fn load_case_files(dir: &Path, schemas: &str) -> Result<Vec<(PathBuf, TestCase)>> {
    yaml_files(dir)?
        .into_iter()
        .map(|p| {
            let case = testcase_common::load_and_validate_yaml::<TestCase, _, _>(&p, schemas)
                .with_context(|| format!("test case {}", p.display()))?;
            Ok((p, case))
        })
        .collect()
}

fn load_cases(dir: &Path, schemas: &str) -> Result<Vec<TestCase>> {
    Ok(load_case_files(dir, schemas)?
        .into_iter()
        .map(|(_, c)| c)
        .collect())
}

/// The name of an external system as `external_refs[].system` writes it (`jira`, `xray`, ...).
fn system_name(s: ExternalSystem) -> String {
    serde_yaml::to_string(&s)
        .map(|t| t.trim().to_string())
        .unwrap_or_default()
}

/// Would `store_external_ref` change the record at `path`? (reads only)
fn ref_missing(path: &Path, system: &str, key: &str) -> Result<bool> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(extrefs::set_external_ref(&text, system, key, "")?.is_some())
}

/// Store `key` as the `system` entry of the record at `path`. Targeted edit (see `extrefs`);
/// the edited record must validate against its schema before it is written. True when the file
/// changed; a record that already holds the key is not touched.
fn store_external_ref(path: &Path, system: &str, key: &str, schemas: &str) -> Result<bool> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let stamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let Some(new) = extrefs::set_external_ref(&text, system, key, &stamp)
        .with_context(|| format!("{}: external_refs", path.display()))?
    else {
        return Ok(false);
    };
    testcase_common::parse_and_validate_yaml_string::<serde_yaml::Value, _>(
        &new,
        schemas,
        &path.display().to_string(),
    )
    .with_context(|| {
        format!(
            "{}: the record with the new external_refs does not validate; not written",
            path.display()
        )
    })?;
    std::fs::write(path, new).with_context(|| format!("write {}", path.display()))?;
    Ok(true)
}

fn load_runs(dir: &Path, schemas: &str) -> Result<Vec<TestRunRecord>> {
    yaml_files(dir)?
        .iter()
        .map(|p| {
            testcase_common::load_and_validate_yaml::<TestRunRecord, _, _>(p, schemas)
                .with_context(|| format!("test run {}", p.display()))
        })
        .collect()
}

/// The trace matrix of the product, built from the requirement records, the test cases and the
/// run records (`testcase_verification::build_matrix`).
fn matrix(args: &SyncArgs, cases: &[TestCase], runs: &[TestRunRecord]) -> Result<TraceMatrix> {
    let reqs = if args.requirements_dir.is_dir() {
        read_requirements(&args.requirements_dir)?
    } else {
        Vec::new()
    };
    let (m, _) = build_matrix("SYNC-matrix", args.baseline.as_deref(), &reqs, cases, runs);
    Ok(m)
}

/// The latest result of each test case over all run records (by `started_at`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Latest {
    run_id: String,
    verdict: Verdict,
    finished: bool,
}

fn latest_results(runs: &[TestRunRecord]) -> BTreeMap<String, Latest> {
    let mut sorted: Vec<&TestRunRecord> = runs.iter().collect();
    sorted.sort_by(|a, b| a.started_at.cmp(&b.started_at));
    let mut out = BTreeMap::new();
    for run in sorted {
        for r in &run.results {
            out.insert(
                r.test_case_id.clone(),
                Latest {
                    run_id: run.run_id.clone(),
                    verdict: r.verdict,
                    finished: run.finished_at.is_some(),
                },
            );
        }
    }
    out
}

fn run_verdict(v: Verdict) -> RunVerdict {
    match v {
        Verdict::Pass => RunVerdict::Pass,
        Verdict::Fail => RunVerdict::Fail,
        Verdict::NotExecuted => RunVerdict::NotExecuted,
        Verdict::Error => RunVerdict::Error,
    }
}

fn lifecycle_of(c: &TestCase) -> LifecycleStatus {
    match c.status {
        None | Some(TestCaseStatus::Draft) => LifecycleStatus::Draft,
        Some(TestCaseStatus::InReview) => LifecycleStatus::InReview,
        Some(TestCaseStatus::Approved) => LifecycleStatus::Approved,
        Some(TestCaseStatus::Obsolete) => LifecycleStatus::Obsolete,
    }
}

/// Features (requirement ids) a test case traces to, from the matrix.
fn features_of(m: &TraceMatrix, case_id: &str) -> Vec<String> {
    m.links
        .iter()
        .filter(|l| l.test_case_ids.iter().any(|t| t == case_id))
        .map(|l| l.requirement_id.clone())
        .collect()
}

fn feature_keys(m: &TraceMatrix) -> Vec<String> {
    let mut set = BTreeSet::new();
    for l in &m.links {
        set.insert(l.requirement_id.clone());
    }
    set.into_iter().collect()
}

/// A problem report of the product, with the test case it names (if any).
fn problem_reports(
    args: &SyncArgs,
    schemas: &str,
) -> Result<Vec<(PathBuf, ProblemReport, Option<String>)>> {
    let mut out = Vec::new();
    for path in yaml_files(&args.problem_reports)? {
        let pr = prcmd::load_problem_report(&path, schemas, &id_of(&path))?;
        let test_case = {
            let text = std::fs::read_to_string(&path)?;
            let v: serde_yaml::Value = serde_yaml::from_str(&text)?;
            v.get("test_case_id")
                .and_then(|t| t.as_str())
                .map(str::to_string)
        };
        out.push((path, pr, test_case));
    }
    Ok(out)
}

fn id_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// `validation_status` of each requirement record that has one (requirement.schema). A record
/// without the field, or with another value, counts as not set.
fn requirement_validations(dir: &Path) -> Result<BTreeMap<String, ValidationState>> {
    let mut out = BTreeMap::new();
    for path in yaml_files(dir)? {
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let v: serde_yaml::Value =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let state = match v.get("validation_status").and_then(|x| x.as_str()) {
            Some("validated") => ValidationState::Validated,
            Some("not_validated") => ValidationState::NotValidated,
            Some("rejected") => ValidationState::Rejected,
            _ => continue,
        };
        if let Some(id) = v.get("id").and_then(|x| x.as_str()) {
            out.insert(id.to_string(), state);
        }
    }
    Ok(out)
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

// ------------------------------------------------------------------ steps

fn import_step(
    cfg: &IntegrationConfig,
    args: &SyncArgs,
    actor: &str,
    log: &mut Log,
    schemas: &str,
) -> Result<()> {
    use id_resolver::RequirementSource;
    let records = match cfg.requirement_source {
        RequirementSource::Jira => {
            let sys = jira_system(cfg, args)?;
            let client = JiraClient::from_config(sys, &cfg.sync, actor)?;
            requirement_import::jira::fetch(
                &client,
                sys,
                requirement_import::jira::LabelFlags {
                    safety: None,
                    security: None,
                },
            )?
        }
        RequirementSource::Strictdoc => {
            let input = args
                .strictdoc_input
                .as_ref()
                .context("import: the StrictDoc source needs --strictdoc-input")?;
            let commit = args
                .commit
                .as_deref()
                .context("import: the StrictDoc source needs --commit")?;
            let text = std::fs::read_to_string(input)
                .with_context(|| format!("read {}", input.display()))?;
            requirement_import::strictdoc::parse_export(&text, commit)?
        }
    };
    if log.apply {
        let report =
            requirement_import::write_requirements(&args.requirements_dir, &records, schemas)?;
        log.line(format!(
            "step import: {} requirement(s): {} created, {} updated, {} unchanged",
            records.len(),
            report.created.len(),
            report.updated.len(),
            report.unchanged.len()
        ));
    } else {
        log.plan(format!(
            "{} requirement record(s) into {}",
            records.len(),
            args.requirements_dir.display()
        ));
        log.line("step import: planned (reads from the source, writes no record)");
    }
    Ok(())
}

fn tests_step(
    cfg: &IntegrationConfig,
    args: &SyncArgs,
    actor: &str,
    log: &mut Log,
    state: &mut State,
    schemas: &str,
) -> Result<()> {
    let sys = jira_system(cfg, args)?.clone();
    let files = load_case_files(&args.tests_dir, schemas)?;
    let cases: Vec<TestCase> = files.iter().map(|(_, c)| c.clone()).collect();
    let mut paths: BTreeMap<String, PathBuf> = BTreeMap::new();
    for (p, c) in &files {
        if paths.insert(c.id.clone(), p.clone()).is_some() {
            bail!("test case id {} is in two files", c.id);
        }
    }
    let runs = load_runs(&args.runs_dir, schemas)?;
    let m = matrix(args, &cases, &runs)?;
    let latest = latest_results(&runs);
    let kind = test_backends::BackendKind::from_config(&sys)?;
    let ext = kind.external_system();
    let sys_name = system_name(ext);
    log.line(format!(
        "step tests: {} test case(s), {} run record(s) on {}",
        cases.len(),
        runs.len(),
        sys.name
    ));

    let mut backend: Option<Box<dyn TestBackend>> = if log.apply {
        // Zephyr Scale has its own API (TCMS_ZEPHYR_API_URL); the other backends use Jira's
        let api: Box<dyn test_backends::api::Api> = match kind {
            test_backends::BackendKind::ZephyrScale => {
                Box::new(test_backends::api::ZephyrApi::from_env(&sys)?)
            }
            _ => Box::new(jira_api(cfg, &sys, actor)?),
        };
        Some(build_backend(&sys, api)?)
    } else {
        // a dry run still checks the configuration without sending anything
        test_backends::BackendKind::from_config(&sys)?;
        None
    };

    let mut sorted: Vec<&TestCase> = cases.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    for case in sorted {
        let lifecycle = lifecycle_of(case);
        let path = paths
            .get(&case.id)
            .with_context(|| format!("test case {}: no file", case.id))?;
        // the entity known from the sync state, else the one the record already holds
        let from_record = case
            .external_refs
            .iter()
            .find(|r| r.system == ext)
            .map(|r| EntityKey {
                key: r.key.clone(),
                id: None,
            });
        let known = state
            .test_cases
            .get(&case.id)
            .map(|t| EntityKey {
                key: t.key.clone(),
                id: None,
            })
            .or(from_record);
        let target = WriteTarget::new("test-case", &case.id);
        let Some(be) = backend.as_mut() else {
            let found = known
                .as_ref()
                .map(|k| format!("entity {}", k.key))
                .unwrap_or_else(|| "a new entity".into());
            log.plan(format!(
                "test {}: find or create {found}, lifecycle {}",
                case.id,
                lifecycle.as_str()
            ));
            if let Some(k) = &known {
                if ref_missing(path, &sys_name, &k.key)? {
                    log.plan(format!(
                        "test {}: external_refs {sys_name} {} stored in {}",
                        case.id,
                        k.key,
                        path.display()
                    ));
                }
            }
            continue;
        };
        let entity = be.ensure_test(&CaseSync {
            test_case_id: case.id.clone(),
            title: case.description.clone(),
            lifecycle,
            feature_keys: features_of(&m, &case.id),
            known,
        })?;
        let last = state
            .test_cases
            .get(&case.id)
            .map(|t| map_of(&t.last_written));
        let st = CaseStatus {
            lifecycle: Some(lifecycle),
            verdict: None,
            baseline: None,
            last_written: last,
        };
        let fields = status_fields(&sys, &st);
        be.write_case_status(&entity, &st)?;
        for o in be.take_overwrites() {
            log.audit(
                &target,
                std::slice::from_ref(&o.field),
                vec![],
                "hand edit in the system overwritten by the TCMS sync",
            );
            log.difference(format!(
                "test {}: {} hand-edited, overwritten",
                case.id, o.field
            ));
        }
        record_test_state(state, &sys, &case.id, &entity, &fields);
        log.audit(
            &target,
            &fields.keys().cloned().collect::<Vec<_>>(),
            vec![serde_json::json!({"system": sys.name, "key": entity.key})],
            "test status",
        );
        if store_external_ref(path, &sys_name, &entity.key, schemas)? {
            log.audit(
                &target,
                &["external_refs".to_string()],
                vec![serde_json::json!({"system": sys_name, "key": entity.key})],
                "external ref stored in the test case",
            );
        }
        log.plan(format!(
            "test {} -> {} lifecycle {}",
            case.id,
            entity.key,
            lifecycle.as_str()
        ));
    }

    // TCMS-42.4 AC 5: a test case removed from the product marks its entity obsolete; the entity is
    // never deleted, and an entity already recorded as obsolete is not written again
    let live: BTreeSet<&str> = cases.iter().map(|c| c.id.as_str()).collect();
    let removed: Vec<(String, TestState)> = state
        .test_cases
        .iter()
        .filter(|(id, t)| t.system == sys.name && !live.contains(id.as_str()))
        .map(|(id, t)| (id.clone(), t.clone()))
        .collect();
    for (case_id, t) in removed {
        let obsolete = CaseStatus {
            lifecycle: Some(LifecycleStatus::Obsolete),
            verdict: None,
            baseline: None,
            last_written: Some(map_of(&t.last_written)),
        };
        let fields = status_fields(&sys, &obsolete);
        if fields.iter().all(|(k, v)| t.last_written.get(k) == Some(v)) {
            continue;
        }
        let Some(be) = backend.as_mut() else {
            log.plan(format!(
                "test {case_id} removed from the product: entity {} marked obsolete (never deleted)",
                t.key
            ));
            continue;
        };
        let entity = EntityKey {
            key: t.key.clone(),
            id: None,
        };
        be.write_case_status(&entity, &obsolete)?;
        let target = WriteTarget::new("test-case", &case_id);
        for o in be.take_overwrites() {
            log.audit(
                &target,
                std::slice::from_ref(&o.field),
                vec![],
                "hand edit in the system overwritten by the TCMS sync",
            );
            log.difference(format!(
                "test {case_id}: {} hand-edited, overwritten",
                o.field
            ));
        }
        record_test_state(state, &sys, &case_id, &entity, &fields);
        log.audit(
            &target,
            &fields.keys().cloned().collect::<Vec<_>>(),
            vec![serde_json::json!({"system": sys.name, "key": entity.key})],
            "test removed from the product: marked obsolete",
        );
        log.plan(format!(
            "test {case_id} removed from the product -> {} marked obsolete",
            entity.key
        ));
    }

    // latest verdicts, one run record at a time
    let mut by_run: BTreeMap<String, Vec<(String, Verdict, bool)>> = BTreeMap::new();
    for (case_id, l) in &latest {
        by_run
            .entry(l.run_id.clone())
            .or_default()
            .push((case_id.clone(), l.verdict, l.finished));
    }
    for run in runs.iter() {
        let Some(cases_here) = by_run.get(&run.run_id) else {
            continue;
        };
        let mut run_cases = Vec::new();
        for (case_id, verdict, _) in cases_here {
            match state.test_cases.get(case_id) {
                Some(t) => run_cases.push((case_id.clone(), t.clone(), *verdict)),
                None => log.difference(format!(
                    "run {}: test {case_id} has no Jira entity yet; verdict not written",
                    run.run_id
                )),
            }
        }
        if run_cases.is_empty() {
            continue;
        }
        let finished = run.finished_at.is_some();
        let baseline = args.baseline.clone().unwrap_or_else(|| "none".to_string());
        match backend.as_mut() {
            None => {
                log.plan(format!(
                    "run {}: verdicts of {} test case(s) on baseline {baseline}",
                    run.run_id,
                    run_cases.len()
                ));
            }
            Some(be) => {
                let rs = RunSync {
                    run_id: run.run_id.clone(),
                    baseline_id: baseline.clone(),
                    finished,
                    cases: run_cases
                        .iter()
                        .map(|(case_id, t, verdict)| RunCase {
                            test_case_id: case_id.clone(),
                            entity: EntityKey {
                                key: t.key.clone(),
                                id: None,
                            },
                            verdict: run_verdict(*verdict),
                            last_written: Some(map_of(&t.last_written)),
                        })
                        .collect(),
                };
                sync_run(be.as_mut(), &rs)?;
                for (case_id, t, verdict) in &run_cases {
                    let st = CaseStatus {
                        lifecycle: None,
                        verdict: Some(latest_verdict(Some(run_verdict(*verdict)), finished)),
                        baseline: Some(baseline.clone()),
                        last_written: None,
                    };
                    let fields = status_fields(&sys, &st);
                    let entity = EntityKey {
                        key: t.key.clone(),
                        id: None,
                    };
                    record_test_state(state, &sys, case_id, &entity, &fields);
                    log.audit(
                        &WriteTarget::new("test-case", case_id),
                        &fields.keys().cloned().collect::<Vec<_>>(),
                        vec![serde_json::json!({"system": sys.name, "key": t.key})],
                        &format!("verdict of run {}", run.run_id),
                    );
                    log.plan(format!(
                        "test {case_id} verdict {:?} in run {}",
                        verdict, run.run_id
                    ));
                }
                for o in be.take_overwrites() {
                    log.difference(format!(
                        "{}: {} hand-edited, overwritten",
                        o.entity, o.field
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Record the values TCMS wrote to a test entity (the hand-edit check of the next run).
fn record_test_state(
    state: &mut State,
    sys: &SystemConfig,
    case_id: &str,
    entity: &EntityKey,
    fields: &Map<String, Value>,
) {
    let t = state
        .test_cases
        .entry(case_id.to_string())
        .or_insert_with(|| TestState {
            system: sys.name.clone(),
            key: entity.key.clone(),
            last_written: BTreeMap::new(),
        });
    t.key = entity.key.clone();
    for (k, v) in fields {
        t.last_written.insert(k.clone(), v.clone());
    }
}

fn map_of(m: &BTreeMap<String, Value>) -> Map<String, Value> {
    m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn features_step(
    cfg: &IntegrationConfig,
    args: &SyncArgs,
    actor: &str,
    log: &mut Log,
    schemas: &str,
) -> Result<()> {
    let sys = jira_system(cfg, args)?.clone();
    let cases = load_cases(&args.tests_dir, schemas)?;
    let runs = load_runs(&args.runs_dir, schemas)?;
    let m = matrix(args, &cases, &runs)?;
    let latest = latest_results(&runs);
    let prs = problem_reports(args, schemas)?;

    // reads of the open problem reports (linked issue state) run in both modes
    let mut read_ctx = Ctx {
        api: Box::new(jira_api(cfg, &sys, actor)?),
        system: sys.clone(),
        overwrites: Vec::new(),
    };
    let mut open_for: BTreeMap<String, bool> = BTreeMap::new();
    for (_, pr, _) in &prs {
        let linked: Option<IssueState> = match &pr.external_key {
            Some(k) => Some(defects::observe(&mut read_ctx, k)?),
            None => None,
        };
        let blocks = defects::blocks_passed(&pr.status, linked.as_ref());
        for f in &pr.feature_keys {
            *open_for.entry(f.clone()).or_insert(false) |= blocks;
        }
    }

    let validations = requirement_validations(&args.requirements_dir)?;
    let mut writer = FeatureWriter::new(&sys, Box::new(jira_api(cfg, &sys, actor)?));
    let features = feature_keys(&m);
    log.line(format!(
        "step features: {} feature(s) on {}",
        features.len(),
        sys.name
    ));
    for feature in &features {
        let mut input = RollupInput {
            open_problem_reports: open_for.get(feature).copied().unwrap_or(false),
            ..Default::default()
        };
        for link in m.links.iter().filter(|l| &l.requirement_id == feature) {
            input.requirements.push(ReqState {
                covered: link.normal_range_covered && link.robustness_covered,
                validation: validations
                    .get(feature)
                    .copied()
                    .unwrap_or(ValidationState::NotSet),
            });
            for tc in &link.test_case_ids {
                input.test_statuses.push(match latest.get(tc) {
                    None => StatusValue::NotStarted,
                    Some(l) => latest_verdict(Some(run_verdict(l.verdict)), l.finished),
                });
            }
        }
        let planned = writer.write(feature, &input, !log.apply)?;
        let mut field_lines = Vec::new();
        for p in &planned {
            match p {
                PlannedWrite::Field { field, value, .. } => {
                    field_lines.push(format!("{field}={value}"));
                }
                PlannedWrite::Transition { name, .. } => {
                    field_lines.push(format!("transition '{name}'"))
                }
                PlannedWrite::Skipped { reason, .. } => {
                    log.difference(format!("feature {feature}: {reason}"));
                }
                PlannedWrite::Unreported { reason, .. } => {
                    log.line(format!("  feature {feature}: not written: {reason}"));
                }
            }
        }
        if field_lines.is_empty() {
            continue;
        }
        if log.apply {
            log.audit(
                &WriteTarget::new("requirement", feature),
                &field_lines,
                vec![serde_json::json!({"system": sys.name, "key": feature})],
                "feature roll-up",
            );
        }
        log.plan(format!("feature {feature}: {}", field_lines.join(", ")));
        if log.apply && sys.write_policy.comments {
            comment_feature(cfg, &sys, actor, feature, &field_lines, log)?;
        }
    }
    Ok(())
}

/// One summary comment on a feature whose roll-up changed. The jira-client adds the acting user
/// to the body and records the audit entry.
fn comment_feature(
    cfg: &IntegrationConfig,
    sys: &SystemConfig,
    actor: &str,
    feature: &str,
    lines: &[String],
    log: &mut Log,
) -> Result<()> {
    let mut client = JiraClient::from_config(sys, &cfg.sync, actor)?;
    let target = WriteTarget::new("requirement", feature);
    let text = format!("TCMS roll-up updated: {}", lines.join(", "));
    client.add_comment(feature, &text, &target, "feature roll-up comment")?;
    for e in client.audit_entries() {
        log.audit.push(e.clone());
    }
    Ok(())
}

fn milestones_step(
    cfg: &IntegrationConfig,
    args: &SyncArgs,
    actor: &str,
    log: &mut Log,
    state: &mut State,
    schemas: &str,
) -> Result<()> {
    let sys = jira_system(cfg, args)?.clone();
    if sys.milestone.is_none() {
        log.line("step milestones: skipped, milestone is not configured");
        return Ok(());
    }
    let cases = load_cases(&args.tests_dir, schemas)?;
    let runs = load_runs(&args.runs_dir, schemas)?;
    let m = matrix(args, &cases, &runs)?;
    let latest = latest_results(&runs);
    let features = feature_keys(&m);
    let release_date = args.release_date.clone().unwrap_or_else(today);
    let active: Vec<&TestCase> = cases
        .iter()
        .filter(|c| lifecycle_of(c) != LifecycleStatus::Obsolete)
        .collect();
    let total = active.len() as u32;
    let executed = active.iter().filter(|c| latest.contains_key(&c.id)).count() as u32;
    let passed = active
        .iter()
        .filter(|c| {
            latest
                .get(&c.id)
                .is_some_and(|l| l.verdict == Verdict::Pass)
        })
        .count() as u32;

    let mut sync = MilestoneSync::new(&sys, Box::new(jira_api(cfg, &sys, actor)?))?;
    log.line(format!(
        "step milestones: {} plan record(s)",
        yaml_files(&args.test_plans)?.len()
    ));
    for path in yaml_files(&args.test_plans)? {
        let v: serde_yaml::Value = testcase_common::load_and_validate_yaml(&path, schemas)
            .with_context(|| format!("test plan {}", path.display()))?;
        let id = v
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let name = v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or(&id)
            .to_string();
        let baseline_id = v
            .get("baseline_id")
            .and_then(|x| x.as_str())
            .unwrap_or("none")
            .to_string();
        let plan = PlanState {
            name,
            baseline_id,
            executed,
            total,
            passed,
            complete: total > 0 && executed == total,
            tcms_link: format!("tcms://test-plan/{id}"),
        };
        // the Fix Version the plan record holds, else the one in the sync state
        let from_record = v
            .get("external_refs")
            .and_then(|r| r.as_sequence())
            .and_then(|refs| {
                refs.iter()
                    .find(|r| r.get("system").and_then(|s| s.as_str()) == Some("jira"))
            })
            .and_then(|r| r.get("key"))
            .and_then(|k| k.as_str())
            .map(str::to_string);
        let known = state
            .plans
            .get(&id)
            .map(|p| p.version_id.clone())
            .or(from_record);
        let report = sync.sync(
            &plan,
            known.as_deref(),
            &features,
            &release_date,
            !log.apply,
        )?;
        for note in &report.notes {
            log.line(format!("  plan {id}: {note}"));
        }
        if log.apply {
            state.plans.insert(
                id.clone(),
                PlanRecord {
                    version_id: report.version_id.clone(),
                },
            );
            log.audit(
                &WriteTarget::new("test-plan", &id),
                &["fixVersions".to_string()],
                vec![serde_json::json!({"system": sys.name, "key": report.version_id, "synced_at": today()})],
                "milestone",
            );
            if store_external_ref(
                &path,
                &system_name(ExternalSystem::Jira),
                &report.version_id,
                schemas,
            )? {
                log.audit(
                    &WriteTarget::new("test-plan", &id),
                    &["external_refs".to_string()],
                    vec![serde_json::json!({"system": sys.name, "key": report.version_id})],
                    "Fix Version id stored in the test plan",
                );
            }
        }
        log.plan(format!(
            "plan {id} as Fix Version {} ({} feature(s), {}/{} executed)",
            report.version_id,
            report.features_assigned.len(),
            executed,
            total
        ));
    }
    Ok(())
}

fn defects_step(
    cfg: &IntegrationConfig,
    args: &SyncArgs,
    actor: &str,
    log: &mut Log,
    state: &State,
    schemas: &str,
) -> Result<()> {
    let sys = jira_system(cfg, args)?.clone();
    let prs = problem_reports(args, schemas)?;
    log.line(format!("step defects: {} problem report(s)", prs.len()));
    let mut ctx = Ctx {
        api: Box::new(jira_api(cfg, &sys, actor)?),
        system: sys.clone(),
        overwrites: Vec::new(),
    };
    for (path, pr, test_case) in prs {
        if let Some(key) = &pr.external_key {
            let st = defects::observe(&mut ctx, key)?;
            if let Some(d) = defects::reconcile(&pr.id, &pr.status, &st) {
                log.difference(d.message);
            }
            continue;
        }
        if !defects::is_open(&pr.status) {
            continue;
        }
        let test_entity = test_case
            .as_deref()
            .and_then(|t| state.test_cases.get(t))
            .map(|t| t.key.clone());
        let out = defects::push(&mut ctx, &pr, test_entity.as_deref(), actor, !log.apply)?;
        match out {
            PushOutcome::Created { key, links, notes } => {
                prcmd::record_jira_key(&path, &key, &sys.base_url)?;
                log.audit(
                    &WriteTarget::new("problem-report", &pr.id),
                    &["summary", "description", "priority", "labels"].map(str::to_string),
                    vec![serde_json::json!({"system": "jira", "key": key})],
                    "defect pushed",
                );
                log.plan(format!(
                    "problem report {} -> {key} (links: {})",
                    pr.id,
                    links.join(", ")
                ));
                for n in notes {
                    log.line(format!("  note: {n}"));
                }
            }
            PushOutcome::Planned {
                summary,
                links,
                notes,
            } => {
                log.plan(format!(
                    "problem report {} -> defect '{summary}' (links: {})",
                    pr.id,
                    links.join(", ")
                ));
                for n in notes {
                    log.line(format!("  note: {n}"));
                }
            }
            PushOutcome::Found { key } => {
                // an issue with the report's label exists: its key is recorded, nothing is created
                if log.apply {
                    prcmd::record_jira_key(&path, &key, &sys.base_url)?;
                    log.audit(
                        &WriteTarget::new("problem-report", &pr.id),
                        &["external_refs"].map(str::to_string),
                        vec![serde_json::json!({"system": "jira", "key": key})],
                        "defect found by label",
                    );
                }
                log.plan(format!(
                    "problem report {} -> existing {key} (found by label; nothing created)",
                    pr.id
                ));
            }
            PushOutcome::AlreadyLinked { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_status_is_read_from_records_that_have_it() {
        let dir = tempfile::tempdir().unwrap();
        let rec = |id: &str, v: Option<&str>| {
            let line = v
                .map(|v| format!("validation_status: {v}\n"))
                .unwrap_or_default();
            std::fs::write(
                dir.path().join(format!("{id}.yaml")),
                format!("id: {id}\n{line}"),
            )
            .unwrap();
        };
        rec("PAY-1", Some("validated"));
        rec("PAY-2", Some("rejected"));
        rec("PAY-3", None);
        rec("PAY-4", Some("not_validated"));
        let v = requirement_validations(dir.path()).unwrap();
        assert_eq!(v.get("PAY-1"), Some(&ValidationState::Validated));
        assert_eq!(v.get("PAY-2"), Some(&ValidationState::Rejected));
        assert_eq!(v.get("PAY-4"), Some(&ValidationState::NotValidated));
        assert!(!v.contains_key("PAY-3"), "no field, no entry: not set");
    }
}
