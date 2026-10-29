//! Test backend adapters (TCMS-42.4): where a test case and a run live in Jira.
//!
//! * plain Jira (`test_backend: none`): one TCMS-owned Test issue per test case, found by the
//!   `tcms_id` field and linked to its features with `tcms_issue_types.link_type`;
//! * Xray (`xray`): Test issues, a Test Execution per run, per-test status through Xray's API;
//! * Zephyr Scale (`zephyr_scale`): test cases, a cycle and executions through Zephyr's own API;
//! * Zephyr Squad (`zephyr_squad`): Test issues, a cycle and executions through Squad's API.
//!
//! One trait, [`TestBackend`], four implementations. The sync functions in this module take
//! `&mut dyn TestBackend` and never look at the backend kind. Every write goes through
//! [`Ctx::write`], which runs the write guard of `jira-client` first.
//!
//! The endpoint shapes of Xray, Zephyr Scale and Zephyr Squad are ASSUMPTIONS taken from
//! their public documentation as remembered; they are listed in `docs/JIRA_INTEGRATION.md`.
//! Everything is verified against the in-process fake in [`fake`] (feature `mock`) and, for
//! plain Jira, against the jira-client HTTP mock. Nothing is verified against a real service.

pub mod api;
pub mod backends;
pub mod defects;
#[cfg(feature = "mock")]
pub mod fake;
pub mod milestone;
pub mod rollup;

use anyhow::{bail, Result};
use id_resolver::SystemConfig;
use serde_json::{json, Map, Value};
use testcase_models::{ExternalRef, ExternalSystem};

pub use api::{Api, Ctx, JiraApi, ZephyrApi};
pub use backends::{build_backend, BackendKind};

/// Lifecycle of a test case (`test-case-main.status`), written as is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleStatus {
    Draft,
    InReview,
    Approved,
    Obsolete,
}

impl LifecycleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LifecycleStatus::Draft => "draft",
            LifecycleStatus::InReview => "in_review",
            LifecycleStatus::Approved => "approved",
            LifecycleStatus::Obsolete => "obsolete",
        }
    }
}

/// The five TCMS status values written for a latest verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusValue {
    NotStarted,
    InProgress,
    Passed,
    Failed,
    Blocked,
}

impl StatusValue {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusValue::NotStarted => "not_started",
            StatusValue::InProgress => "in_progress",
            StatusValue::Passed => "passed",
            StatusValue::Failed => "failed",
            StatusValue::Blocked => "blocked",
        }
    }
}

/// Verdict of a test case in a run (`test-run.results[].verdict`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunVerdict {
    Pass,
    Fail,
    NotExecuted,
    Error,
}

/// Latest-verdict mapping of the mapping doc: pass -> passed; fail or error -> failed;
/// not_executed -> blocked when the run finished, else in_progress; no run -> not_started.
pub fn latest_verdict(verdict: Option<RunVerdict>, run_finished: bool) -> StatusValue {
    match verdict {
        None => StatusValue::NotStarted,
        Some(RunVerdict::Pass) => StatusValue::Passed,
        Some(RunVerdict::Fail | RunVerdict::Error) => StatusValue::Failed,
        Some(RunVerdict::NotExecuted) if run_finished => StatusValue::Blocked,
        Some(RunVerdict::NotExecuted) => StatusValue::InProgress,
    }
}

/// The entity of a test case in the backend's system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityKey {
    /// Issue key or Zephyr key
    pub key: String,
    /// Numeric id where the system has one (Jira issue id, Squad cycle id)
    pub id: Option<String>,
}

/// A test case as the sync sees it.
#[derive(Debug, Clone)]
pub struct CaseSync {
    pub test_case_id: String,
    pub title: String,
    pub lifecycle: LifecycleStatus,
    /// Feature issue keys the test case traces to (linked when the entity is created)
    pub feature_keys: Vec<String>,
    /// Entity already recorded in external_refs for this backend's system
    pub known: Option<EntityKey>,
}

/// A field that held a value other than the one TCMS last wrote: a hand edit, overwritten and
/// logged (TCMS-42.5 AC #3).
#[derive(Debug, Clone, PartialEq)]
pub struct Overwrite {
    pub entity: String,
    pub field: String,
    pub before: Value,
    pub after: Value,
}

/// A case that cannot be mapped to an entity because creating one is not allowed
/// (`write_policy.dedicated_issues` is off). It is reported, not failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmapped(pub String);

impl std::fmt::Display for Unmapped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unmapped: {}", self.0)
    }
}

impl std::error::Error for Unmapped {}

/// Result of syncing one case when unmapped cases are reported rather than failed.
#[derive(Debug, Clone, PartialEq)]
pub enum CaseOutcome {
    Mapped(EntityKey),
    Unmapped(String),
}

/// Values written on a test entity.
#[derive(Debug, Clone, PartialEq)]
pub struct CaseStatus {
    /// Lifecycle status; `None` leaves the lifecycle field alone
    pub lifecycle: Option<LifecycleStatus>,
    /// Latest verdict; `None` leaves the verdict field alone
    pub verdict: Option<StatusValue>,
    /// Baseline id the verdict refers to
    pub baseline: Option<String>,
    /// Field values TCMS last wrote to this entity (from the record). When given, a field whose
    /// current value differs is a hand edit: it is overwritten and an [`Overwrite`] is logged.
    pub last_written: Option<Map<String, Value>>,
}

/// One executed test case of a run.
#[derive(Debug, Clone)]
pub struct RunCase {
    pub test_case_id: String,
    pub entity: EntityKey,
    pub verdict: RunVerdict,
    /// Field values TCMS last wrote to this entity (see [`CaseStatus::last_written`])
    pub last_written: Option<Map<String, Value>>,
}

/// A run to record in the backend.
#[derive(Debug, Clone)]
pub struct RunSync {
    pub run_id: String,
    pub baseline_id: String,
    pub finished: bool,
    pub cases: Vec<RunCase>,
}

/// The one interface the sync code uses.
pub trait TestBackend {
    fn kind(&self) -> BackendKind;

    /// Find the entity of a test case, or create it (linked to its features). Returns its key.
    fn ensure_test(&mut self, case: &CaseSync) -> Result<EntityKey>;

    /// Write the lifecycle status and, when given, the latest verdict and baseline.
    fn write_case_status(&mut self, entity: &EntityKey, status: &CaseStatus) -> Result<()>;

    /// Record a run as the backend's execution entity. Returns that entity when the backend
    /// has one (plain Jira keeps the verdicts on the Test issues and returns `None`).
    fn record_run(&mut self, run: &RunSync) -> Result<Option<EntityKey>>;

    /// Hand edits overwritten since the last call (TCMS-42.5 AC #3).
    fn take_overwrites(&mut self) -> Vec<Overwrite>;

    /// A test case deleted in TCMS marks its entity obsolete; it is never deleted.
    fn mark_obsolete(&mut self, entity: &EntityKey) -> Result<()> {
        self.write_case_status(
            entity,
            &CaseStatus {
                lifecycle: Some(LifecycleStatus::Obsolete),
                verdict: None,
                baseline: None,
                last_written: None,
            },
        )
    }
}

/// Ensure the entity of a case and write its lifecycle status (no verdict yet).
pub fn sync_case(backend: &mut dyn TestBackend, case: &CaseSync) -> Result<EntityKey> {
    let entity = backend.ensure_test(case)?;
    backend.write_case_status(
        &entity,
        &CaseStatus {
            lifecycle: Some(case.lifecycle),
            verdict: None,
            baseline: None,
            last_written: None,
        },
    )?;
    Ok(entity)
}

/// Like [`sync_case`], but a case that cannot be mapped is reported as [`CaseOutcome::Unmapped`]
/// instead of failing the sync (TCMS-42.5 AC #4).
pub fn sync_case_or_unmapped(
    backend: &mut dyn TestBackend,
    case: &CaseSync,
) -> Result<CaseOutcome> {
    match sync_case(backend, case) {
        Ok(e) => Ok(CaseOutcome::Mapped(e)),
        Err(err) => match err.downcast_ref::<Unmapped>() {
            Some(u) => Ok(CaseOutcome::Unmapped(u.0.clone())),
            None => Err(err),
        },
    }
}

/// Record a run, then write the latest verdict and baseline of every case in it.
pub fn sync_run(backend: &mut dyn TestBackend, run: &RunSync) -> Result<Option<EntityKey>> {
    let execution = backend.record_run(run)?;
    for c in &run.cases {
        backend.write_case_status(
            &c.entity,
            &CaseStatus {
                lifecycle: None,
                verdict: Some(latest_verdict(Some(c.verdict), run.finished)),
                baseline: Some(run.baseline_id.clone()),
                last_written: c.last_written.clone(),
            },
        )?;
    }
    Ok(execution)
}

/// The external_refs entry for an entity of a backend.
pub fn external_ref(kind: BackendKind, entity: &EntityKey, synced_at: &str) -> ExternalRef {
    ExternalRef {
        system: kind.external_system(),
        key: entity.key.clone(),
        url: None,
        synced_at: Some(synced_at.to_string()),
    }
}

/// The entity a record already holds for this backend's system, if any.
pub fn known_entity(refs: &[ExternalRef], kind: BackendKind) -> Option<EntityKey> {
    let want: ExternalSystem = kind.external_system();
    refs.iter().find(|r| r.system == want).map(|r| EntityKey {
        key: r.key.clone(),
        id: None,
    })
}

/// Field values for the configured status fields of a test entity. Unset fields are omitted,
/// so nothing outside `fields` is ever written.
pub fn status_fields(system: &SystemConfig, status: &CaseStatus) -> Map<String, Value> {
    let mut out = Map::new();
    let f = &system.fields;
    if let (Some(id), Some(l)) = (&f.test_lifecycle_status, status.lifecycle) {
        out.insert(id.clone(), json!(l.as_str()));
    }
    if let (Some(id), Some(v)) = (&f.latest_verdict, status.verdict) {
        out.insert(id.clone(), json!(status_text(system, v)));
    }
    if let (Some(id), Some(b)) = (&f.baseline, &status.baseline) {
        out.insert(id.clone(), json!(b));
    }
    out
}

/// The Jira option name for a status value (`status_values`), or the TCMS value itself.
pub(crate) fn status_text(system: &SystemConfig, v: StatusValue) -> String {
    system
        .status_values
        .get(v.as_str())
        .cloned()
        .unwrap_or_else(|| v.as_str().to_string())
}

/// Record each configured field whose current value differs from the value TCMS last wrote.
/// Without a last-written value nothing is known to be a hand edit, so nothing is logged.
pub(crate) fn detect_overwrites(
    out: &mut Vec<Overwrite>,
    entity: &str,
    wanted: &Map<String, Value>,
    last: Option<&Map<String, Value>>,
    current: &Value,
) {
    let Some(last) = last else { return };
    for (field, after) in wanted {
        let Some(prev) = last.get(field) else {
            continue;
        };
        let before = current.get(field).cloned().unwrap_or(Value::Null);
        if &before != prev {
            out.push(Overwrite {
                entity: entity.to_string(),
                field: field.clone(),
                before,
                after: after.clone(),
            });
        }
    }
}

/// Refuse a configuration the backends cannot work with, before any request is sent.
pub(crate) fn require_project_and_tcms_id(system: &SystemConfig) -> Result<()> {
    if system.project_keys.is_empty() {
        bail!(
            "system '{}': project_keys must name the project of the test entities",
            system.name
        );
    }
    if system.fields.tcms_id.is_none() {
        bail!(
            "system '{}': fields.tcms_id is required (it finds a test entity again)",
            system.name
        );
    }
    Ok(())
}

/// The audit-trail entry (`action: sync`) that records an overwritten hand edit: before is the
/// value found in the system, after is the value TCMS wrote (TCMS-42.5 AC #3).
pub fn overwrite_audit_entry(
    o: &Overwrite,
    id: String,
    who: &str,
    test_case_id: &str,
    when: &str,
) -> jira_client::AuditTrailEntry {
    jira_client::AuditTrailEntry {
        doc_type: "audit_trail_entry".into(),
        schema: jira_client::audit::AUDIT_SCHEMA_URI.into(),
        id,
        who: who.into(),
        when: when.into(),
        action: "sync".into(),
        entity: jira_client::audit::AuditEntity {
            entity_type: "test-case".into(),
            entity_id: test_case_id.into(),
            version: None,
        },
        before: Some(json!({ "field": o.field, "value": o.before, "entity": o.entity })),
        after: Some(json!({ "field": o.field, "value": o.after, "entity": o.entity })),
        reason: Some("hand edit in the system overwritten by the TCMS sync".into()),
    }
}
