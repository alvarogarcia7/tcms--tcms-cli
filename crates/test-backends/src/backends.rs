//! The four implementations of [`TestBackend`]. Each is a thin mapping onto its system's
//! endpoints; all writes go through [`Ctx::write`]. Endpoint shapes marked ASSUMPTION are
//! from public documentation as remembered and are not verified against the real products.

use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use id_resolver::SystemConfig;
use jira_client::WriteKind;
use serde_json::{json, Map, Value};
use testcase_models::ExternalSystem;

use crate::api::Api;
use crate::{
    detect_overwrites, latest_verdict, require_project_and_tcms_id, status_fields, CaseStatus,
    CaseSync, Ctx, EntityKey, Overwrite, RunSync, StatusValue, TestBackend, Unmapped,
};

/// Creating a test entity needs `write_policy.dedicated_issues`; without it the case is
/// unmapped (reported, no request sent).
fn creation_allowed(ctx: &Ctx) -> Result<()> {
    if ctx.system.write_policy.dedicated_issues {
        return Ok(());
    }
    Err(anyhow::Error::new(Unmapped(format!(
        "no entity in system '{}' and write_policy.dedicated_issues is off",
        ctx.system.name
    ))))
}

/// Which backend a Jira system uses (`test_backend` of the integration config).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// `none` or absent: TCMS-owned Test issues
    PlainJira,
    Xray,
    ZephyrScale,
    ZephyrSquad,
}

impl BackendKind {
    pub fn from_config(system: &SystemConfig) -> Result<Self> {
        if !system.system_type.is_jira() {
            bail!(
                "system '{}' is not a Jira system; test backends attach to Jira",
                system.name
            );
        }
        Ok(match system.test_backend.as_deref() {
            None | Some("none") => BackendKind::PlainJira,
            Some("xray") => BackendKind::Xray,
            Some("zephyr_scale") => BackendKind::ZephyrScale,
            Some("zephyr_squad") => BackendKind::ZephyrSquad,
            Some(other) => bail!(
                "system '{}': test_backend '{other}' is not one of none, xray, zephyr_scale, zephyr_squad",
                system.name
            ),
        })
    }

    /// The value of `test_backend` that selects this kind.
    pub fn config_name(self) -> &'static str {
        match self {
            BackendKind::PlainJira => "none",
            BackendKind::Xray => "xray",
            BackendKind::ZephyrScale => "zephyr_scale",
            BackendKind::ZephyrSquad => "zephyr_squad",
        }
    }

    /// The `external_refs[].system` value for entities of this backend.
    pub fn external_system(self) -> ExternalSystem {
        match self {
            BackendKind::PlainJira => ExternalSystem::Jira,
            BackendKind::Xray => ExternalSystem::Xray,
            BackendKind::ZephyrScale => ExternalSystem::ZephyrScale,
            BackendKind::ZephyrSquad => ExternalSystem::ZephyrSquad,
        }
    }
}

/// Build the backend a system configures. Refuses a configuration the backend cannot use.
pub fn build_backend(system: &SystemConfig, api: Box<dyn Api>) -> Result<Box<dyn TestBackend>> {
    let kind = BackendKind::from_config(system)?;
    require_project_and_tcms_id(system)?;
    let ctx = Ctx {
        api,
        system: system.clone(),
        overwrites: Vec::new(),
    };
    Ok(match kind {
        BackendKind::PlainJira => Box::new(PlainJira { ctx }),
        BackendKind::Xray => Box::new(Xray { ctx }),
        BackendKind::ZephyrScale => Box::new(ZephyrScale { ctx }),
        BackendKind::ZephyrSquad => Box::new(ZephyrSquad { ctx }),
    })
}

// ------------------------------------------------------------------ shared helpers

fn project(ctx: &Ctx) -> String {
    ctx.system.project_keys[0].clone()
}

fn tcms_field(ctx: &Ctx) -> String {
    ctx.system.fields.tcms_id.clone().unwrap_or_default()
}

/// `tcms_issue_types.test`, or the default issue type of the backend.
fn test_issue_type(ctx: &Ctx, default: Option<&str>) -> Result<String> {
    let configured = ctx
        .system
        .tcms_issue_types
        .as_ref()
        .and_then(|t| t.test.clone());
    match (configured, default) {
        (Some(t), _) => Ok(t),
        (None, Some(d)) => Ok(d.to_string()),
        (None, None) => bail!(
            "system '{}': tcms_issue_types.test is required for test_backend none",
            ctx.system.name
        ),
    }
}

pub(crate) fn id_text(v: &Value) -> Option<String> {
    v.as_str()
        .map(str::to_string)
        .or_else(|| v.as_i64().map(|n| n.to_string()))
        .or_else(|| v.as_u64().map(|n| n.to_string()))
}

/// The `values` of a Zephyr Scale listing (empty when the listing has none).
fn zephyr_values(v: &Value) -> Vec<Value> {
    v["values"].as_array().cloned().unwrap_or_default()
}

/// The cycles of a Zephyr Squad listing: an array, or an object whose object values are the
/// items (the form assumed for `GET /rest/zapi/latest/cycle`, keyed by cycle id).
fn squad_items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.clone(),
        Value::Object(m) => m.values().filter(|x| x.is_object()).cloned().collect(),
        _ => Vec::new(),
    }
}

fn entity_from(v: &Value) -> Result<EntityKey> {
    let key = v["key"]
        .as_str()
        .ok_or_else(|| anyhow!("response has no key: {v}"))?
        .to_string();
    Ok(EntityKey {
        key,
        id: id_text(&v["id"]),
    })
}

/// A Jira test issue of `issue_type`: the known one, else the one whose tcms_id matches, else a
/// new one linked to the case's features.
fn jira_ensure(ctx: &mut Ctx, issue_type: &str, case: &CaseSync) -> Result<EntityKey> {
    let prefix = ctx.api.rest_prefix();
    let tf = tcms_field(ctx);
    if let Some(known) = &case.known {
        if known.id.is_some() {
            return Ok(known.clone());
        }
        let v = ctx.api.get(
            &format!("{prefix}/issue/{}", known.key),
            &[("fields", "summary".into())],
        )?;
        return Ok(EntityKey {
            key: known.key.clone(),
            id: id_text(&v["id"]),
        });
    }
    let p = project(ctx);
    let jql = format!("project = \"{p}\" AND issuetype = \"{issue_type}\"");
    for issue in ctx.api.search_jql(&jql, &["summary", tf.as_str()])? {
        if issue["fields"][tf.as_str()].as_str() == Some(case.test_case_id.as_str()) {
            return entity_from(&issue);
        }
    }

    creation_allowed(ctx)?;
    let mut fields = Map::new();
    fields.insert("project".into(), json!({ "key": p }));
    fields.insert("issuetype".into(), json!({ "name": issue_type }));
    fields.insert("summary".into(), json!(case.title));
    fields.insert(tf.clone(), json!(case.test_case_id));
    let v = ctx.write(
        WriteKind::DedicatedIssue,
        "",
        &["project", "issuetype", "summary", tf.as_str()],
        "POST",
        &format!("{prefix}/issue"),
        &json!({ "fields": fields }),
    )?;
    let created = entity_from(&v)?;

    if !case.feature_keys.is_empty() {
        let link = ctx
            .system
            .tcms_issue_types
            .as_ref()
            .and_then(|t| t.link_type.clone())
            .ok_or_else(|| {
                anyhow!(
                    "system '{}': tcms_issue_types.link_type is required to link tests to features",
                    ctx.system.name
                )
            })?;
        for feature in &case.feature_keys {
            ctx.write(
                WriteKind::DedicatedIssue,
                &created.key,
                &[],
                "POST",
                &format!("{prefix}/issueLink"),
                &json!({
                    "type": { "name": link },
                    "outwardIssue": { "key": created.key },
                    "inwardIssue": { "key": feature }
                }),
            )?;
        }
    }
    Ok(created)
}

/// Write the configured status fields of a Jira test issue (PUT on the issue).
fn jira_write_status(ctx: &mut Ctx, entity: &EntityKey, status: &CaseStatus) -> Result<()> {
    let fields = status_fields(&ctx.system, status);
    if fields.is_empty() {
        return Ok(());
    }
    let ids: Vec<&str> = fields.keys().map(String::as_str).collect();
    let prefix = ctx.api.rest_prefix();
    if status.last_written.is_some() {
        let current = ctx.api.get(
            &format!("{prefix}/issue/{}", entity.key),
            &[("fields", ids.join(","))],
        )?;
        detect_overwrites(
            &mut ctx.overwrites,
            &entity.key,
            &fields,
            status.last_written.as_ref(),
            &current["fields"],
        );
    }
    ctx.write(
        WriteKind::DedicatedIssue,
        &entity.key,
        &ids,
        "PUT",
        &format!("{prefix}/issue/{}", entity.key),
        &json!({ "fields": fields }),
    )?;
    Ok(())
}

/// Create a TCMS-owned Jira issue (a run holder) and return its key and id.
fn jira_create_run_issue(ctx: &mut Ctx, issue_type: &str, summary: &str) -> Result<EntityKey> {
    let prefix = ctx.api.rest_prefix();
    let p = project(ctx);
    let v = ctx.write(
        WriteKind::DedicatedIssue,
        "",
        &["project", "issuetype", "summary"],
        "POST",
        &format!("{prefix}/issue"),
        &json!({ "fields": {
            "project": { "key": p },
            "issuetype": { "name": issue_type },
            "summary": summary
        }}),
    )?;
    entity_from(&v)
}

/// Native status names of a backend for a latest verdict.
fn native(v: StatusValue, names: [&str; 5]) -> String {
    let i = match v {
        StatusValue::NotStarted => 0,
        StatusValue::InProgress => 1,
        StatusValue::Passed => 2,
        StatusValue::Failed => 3,
        StatusValue::Blocked => 4,
    };
    names[i].to_string()
}

fn verdict_of(run: &RunSync, c: &crate::RunCase) -> StatusValue {
    latest_verdict(Some(c.verdict), run.finished)
}

// ------------------------------------------------------------------ plain Jira

/// Plain Jira: one TCMS-owned Test issue per case; verdicts live on the Test issues.
pub struct PlainJira {
    ctx: Ctx,
}

impl TestBackend for PlainJira {
    fn kind(&self) -> BackendKind {
        BackendKind::PlainJira
    }
    fn take_overwrites(&mut self) -> Vec<Overwrite> {
        std::mem::take(&mut self.ctx.overwrites)
    }
    fn ensure_test(&mut self, case: &CaseSync) -> Result<EntityKey> {
        let t = test_issue_type(&self.ctx, None)?;
        jira_ensure(&mut self.ctx, &t, case)
    }
    fn write_case_status(&mut self, entity: &EntityKey, status: &CaseStatus) -> Result<()> {
        jira_write_status(&mut self.ctx, entity, status)
    }
    fn record_run(&mut self, _run: &RunSync) -> Result<Option<EntityKey>> {
        // No execution entity: sync_run writes each case's verdict on its Test issue.
        Ok(None)
    }
}

// ------------------------------------------------------------------ Xray

/// Xray on Jira: Test issues; a Test Execution per run, per-test status through Xray's API.
pub struct Xray {
    ctx: Ctx,
}

impl TestBackend for Xray {
    fn kind(&self) -> BackendKind {
        BackendKind::Xray
    }
    fn take_overwrites(&mut self) -> Vec<Overwrite> {
        std::mem::take(&mut self.ctx.overwrites)
    }
    fn ensure_test(&mut self, case: &CaseSync) -> Result<EntityKey> {
        let t = test_issue_type(&self.ctx, Some("Test"))?;
        jira_ensure(&mut self.ctx, &t, case)
    }
    fn write_case_status(&mut self, entity: &EntityKey, status: &CaseStatus) -> Result<()> {
        jira_write_status(&mut self.ctx, entity, status)
    }
    fn record_run(&mut self, run: &RunSync) -> Result<Option<EntityKey>> {
        // ASSUMPTION: Xray's raven REST API on the Jira site (testexec/{key}/test to add tests,
        // testrun/status to set a test's status in an execution).
        let exec_type = self
            .ctx
            .system
            .tcms_issue_types
            .as_ref()
            .and_then(|t| t.test_run.clone())
            .unwrap_or_else(|| "Test Execution".to_string());
        // one execution per run: a sync of the same run again reuses the Test Execution whose
        // summary is the run id (D-42.4m); without this every sync created a new one
        let p = project(&self.ctx);
        let jql = format!("project = \"{p}\" AND issuetype = \"{exec_type}\"");
        let same_run = self
            .ctx
            .api
            .search_jql(&jql, &["summary"])?
            .into_iter()
            .find(|i| i["fields"]["summary"].as_str() == Some(run.run_id.as_str()));
        let exec = match same_run {
            Some(issue) => entity_from(&issue)?,
            None => jira_create_run_issue(&mut self.ctx, &exec_type, &run.run_id)?,
        };
        let keys: Vec<String> = run.cases.iter().map(|c| c.entity.key.clone()).collect();
        self.ctx.write(
            WriteKind::DedicatedIssue,
            &exec.key,
            &[],
            "POST",
            &format!("/rest/raven/1.0/api/testexec/{}/test", exec.key),
            &json!({ "add": keys }),
        )?;
        for c in &run.cases {
            let status = native(
                verdict_of(run, c),
                ["TODO", "EXECUTING", "PASS", "FAIL", "BLOCKED"],
            );
            self.ctx.write(
                WriteKind::DedicatedIssue,
                &c.entity.key,
                &[],
                "PUT",
                "/rest/raven/1.0/api/testrun/status",
                &json!({ "testExecKey": exec.key, "testKey": c.entity.key, "status": status }),
            )?;
        }
        Ok(Some(exec))
    }
}

// ------------------------------------------------------------------ Zephyr Scale

/// Zephyr Scale: test cases, a cycle and executions through Zephyr's own REST API.
/// ASSUMPTION: the endpoints /testcases, /testcycles, /testexecutions and the `query` language
/// (`labels = "..."`) as in the public Zephyr Scale REST documentation.
pub struct ZephyrScale {
    ctx: Ctx,
}

fn zephyr_lifecycle_name(l: crate::LifecycleStatus) -> &'static str {
    use crate::LifecycleStatus::*;
    match l {
        Draft => "Draft",
        InReview => "In Review",
        Approved => "Approved",
        Obsolete => "Obsolete",
    }
}

impl TestBackend for ZephyrScale {
    fn kind(&self) -> BackendKind {
        BackendKind::ZephyrScale
    }
    fn take_overwrites(&mut self) -> Vec<Overwrite> {
        std::mem::take(&mut self.ctx.overwrites)
    }
    fn ensure_test(&mut self, case: &CaseSync) -> Result<EntityKey> {
        if let Some(known) = &case.known {
            return Ok(known.clone());
        }
        let p = project(&self.ctx);
        let label = format!("tcms:{}", case.test_case_id);
        let found = self.ctx.api.get(
            "/testcases",
            &[
                ("projectKey", p.clone()),
                ("query", format!("labels = \"{label}\"")),
            ],
        )?;
        if let Some(first) = found["values"].as_array().and_then(|a| a.first()) {
            return entity_from(first);
        }
        creation_allowed(&self.ctx)?;
        let v = self.ctx.write(
            WriteKind::DedicatedIssue,
            "",
            &[],
            "POST",
            "/testcases",
            &json!({
                "projectKey": p,
                "name": case.title,
                "labels": [label],
                "statusName": zephyr_lifecycle_name(case.lifecycle)
            }),
        )?;
        entity_from(&v)
    }
    fn write_case_status(&mut self, entity: &EntityKey, status: &CaseStatus) -> Result<()> {
        let fields = status_fields(&self.ctx.system, status);
        let mut body = Map::new();
        if let Some(l) = status.lifecycle {
            body.insert("statusName".into(), json!(zephyr_lifecycle_name(l)));
        }
        if !fields.is_empty() {
            body.insert("customFields".into(), Value::Object(fields.clone()));
        }
        if body.is_empty() {
            return Ok(());
        }
        let ids: Vec<&str> = fields.keys().map(String::as_str).collect();
        if status.last_written.is_some() {
            let current = self
                .ctx
                .api
                .get(&format!("/testcases/{}", entity.key), &[])?;
            detect_overwrites(
                &mut self.ctx.overwrites,
                &entity.key,
                &fields,
                status.last_written.as_ref(),
                &current["customFields"],
            );
        }
        self.ctx.write(
            WriteKind::DedicatedIssue,
            &entity.key,
            &ids,
            "PUT",
            &format!("/testcases/{}", entity.key),
            &Value::Object(body),
        )?;
        Ok(())
    }
    fn record_run(&mut self, run: &RunSync) -> Result<Option<EntityKey>> {
        let p = project(&self.ctx);
        // one cycle per run: a sync of the same run again reuses the cycle named by the run id
        // (D-42.4o, as D-42.4m for Xray); the name filter is sent and the match is checked here
        let listed = self.ctx.api.get(
            "/testcycles",
            &[
                ("projectKey", p.clone()),
                ("query", format!("name = \"{}\"", run.run_id)),
            ],
        )?;
        let same_run = zephyr_values(&listed)
            .into_iter()
            .find(|c| c["name"].as_str() == Some(run.run_id.as_str()));
        let cycle = match same_run {
            Some(c) => entity_from(&c)?,
            None => entity_from(&self.ctx.write(
                WriteKind::DedicatedIssue,
                "",
                &[],
                "POST",
                "/testcycles",
                &json!({ "projectKey": p, "name": run.run_id }),
            )?)?,
        };
        // the executions already in the cycle: an existing one is updated, not duplicated
        let existing = self.ctx.api.get(
            "/testexecutions",
            &[
                ("projectKey", p.clone()),
                ("testCycleKey", cycle.key.clone()),
            ],
        )?;
        let mut by_case: BTreeMap<String, String> = BTreeMap::new();
        for e in zephyr_values(&existing) {
            if let (Some(case), Some(id)) = (e["testCaseKey"].as_str(), id_text(&e["id"])) {
                by_case.insert(case.to_string(), id);
            }
        }
        for c in &run.cases {
            let status = native(
                verdict_of(run, c),
                ["Not Executed", "In Progress", "Pass", "Fail", "Blocked"],
            );
            match by_case.get(&c.entity.key) {
                Some(id) => {
                    self.ctx.write(
                        WriteKind::DedicatedIssue,
                        "",
                        &[],
                        "PUT",
                        &format!("/testexecutions/{id}"),
                        &json!({ "statusName": status }),
                    )?;
                }
                None => {
                    self.ctx.write(
                        WriteKind::DedicatedIssue,
                        &c.entity.key,
                        &[],
                        "POST",
                        "/testexecutions",
                        &json!({
                            "projectKey": p,
                            "testCaseKey": c.entity.key,
                            "testCycleKey": cycle.key,
                            "statusName": status
                        }),
                    )?;
                }
            }
        }
        Ok(Some(cycle))
    }
}

// ------------------------------------------------------------------ Zephyr Squad

/// Zephyr Squad on Jira: Test issues; a cycle and executions through Squad's `zapi` API.
/// ASSUMPTION: POST /rest/zapi/latest/cycle returns the cycle id; POST .../execution takes
/// issueId, cycleId and a numeric status (1 pass, 2 fail, 3 work in progress, 4 blocked,
/// -1 unexecuted).
pub struct ZephyrSquad {
    ctx: Ctx,
}

fn squad_status(v: StatusValue) -> &'static str {
    match v {
        StatusValue::Passed => "1",
        StatusValue::Failed => "2",
        StatusValue::InProgress => "3",
        StatusValue::Blocked => "4",
        StatusValue::NotStarted => "-1",
    }
}

impl TestBackend for ZephyrSquad {
    fn kind(&self) -> BackendKind {
        BackendKind::ZephyrSquad
    }
    fn take_overwrites(&mut self) -> Vec<Overwrite> {
        std::mem::take(&mut self.ctx.overwrites)
    }
    fn ensure_test(&mut self, case: &CaseSync) -> Result<EntityKey> {
        let t = test_issue_type(&self.ctx, Some("Test"))?;
        jira_ensure(&mut self.ctx, &t, case)
    }
    fn write_case_status(&mut self, entity: &EntityKey, status: &CaseStatus) -> Result<()> {
        jira_write_status(&mut self.ctx, entity, status)
    }
    fn record_run(&mut self, run: &RunSync) -> Result<Option<EntityKey>> {
        let prefix = self.ctx.api.rest_prefix();
        let p = project(&self.ctx);
        let project_info = self.ctx.api.get(&format!("{prefix}/project/{p}"), &[])?;
        let project_id = id_text(&project_info["id"])
            .ok_or_else(|| anyhow!("project {p} has no id in the Jira response"))?;
        // one cycle per run (D-42.4o): the cycle named by the run id is reused
        let listed = self.ctx.api.get(
            "/rest/zapi/latest/cycle",
            &[("projectId", project_id.clone())],
        )?;
        let same_run = squad_items(&listed)
            .into_iter()
            .find(|c| c["name"].as_str() == Some(run.run_id.as_str()));
        let cycle_id = match same_run {
            Some(c) => {
                id_text(&c["id"]).ok_or_else(|| anyhow!("Squad cycle {} has no id", run.run_id))?
            }
            None => {
                let cycle_resp = self.ctx.write(
                    WriteKind::DedicatedIssue,
                    "",
                    &[],
                    "POST",
                    "/rest/zapi/latest/cycle",
                    &json!({ "name": run.run_id, "projectId": project_id }),
                )?;
                id_text(&cycle_resp["id"])
                    .ok_or_else(|| anyhow!("Squad cycle response has no id"))?
            }
        };
        // the executions already in the cycle, by issue id: an existing one is updated (D-42.4o)
        let existing = self.ctx.api.get(
            "/rest/zapi/latest/execution",
            &[("cycleId", cycle_id.clone())],
        )?;
        let mut by_issue: BTreeMap<String, String> = BTreeMap::new();
        for e in existing["executions"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            if let (Some(issue), Some(id)) = (id_text(&e["issueId"]), id_text(&e["id"])) {
                by_issue.insert(issue, id);
            }
        }
        for c in &run.cases {
            // the sync state keeps only the key of a case: the issue id comes from Jira
            let issue_id = match &c.entity.id {
                Some(id) => id.clone(),
                None => {
                    let v = self
                        .ctx
                        .api
                        .get(&format!("{prefix}/issue/{}", c.entity.key), &[])?;
                    id_text(&v["id"]).ok_or_else(|| {
                        anyhow!("{}: no Jira issue id for the execution", c.entity.key)
                    })?
                }
            };
            let status = squad_status(verdict_of(run, c));
            match by_issue.get(&issue_id) {
                Some(exec) => {
                    self.ctx.write(
                        WriteKind::DedicatedIssue,
                        "",
                        &[],
                        "PUT",
                        &format!("/rest/zapi/latest/execution/{exec}/execute"),
                        &json!({ "status": status }),
                    )?;
                }
                None => {
                    self.ctx.write(
                        WriteKind::DedicatedIssue,
                        &c.entity.key,
                        &[],
                        "POST",
                        "/rest/zapi/latest/execution",
                        &json!({
                            "issueId": issue_id,
                            "cycleId": cycle_id,
                            "status": status
                        }),
                    )?;
                }
            }
        }
        Ok(Some(EntityKey {
            key: run.run_id.clone(),
            id: Some(cycle_id),
        }))
    }
}
