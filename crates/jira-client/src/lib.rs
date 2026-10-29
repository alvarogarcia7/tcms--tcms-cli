//! Jira client (TCMS-42.2): Jira Cloud (REST v3) and Data Center (REST v2).
//!
//! * token auth; the token comes from the environment variable named by `auth.secret_ref`
//!   and is never written to a record, a log line or an error message;
//! * every write passes the write guard ([`guard::check_write`]) before a request is built;
//! * HTTP 429 is retried with back-off; any other failure is an error naming the issue key;
//! * every successful write yields an audit-trail entry (`action: sync`, acting user,
//!   external_refs written).
//!
//! Verified only against the mock in [`mock`] (feature `mock`), never against a real Jira.

pub mod audit;
pub mod guard;
#[cfg(feature = "mock")]
pub mod mock;

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use id_resolver::config::{SyncConfig, SystemConfig, SystemType};
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::{json, Map, Value};

pub use audit::{write_audit_entries, AuditTrailEntry, WriteTarget};
pub use guard::{check_write, fix_version_target, WriteKind, WriteRequest, FIX_VERSIONS_FIELD};

/// A secret that never prints.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: impl Into<String>) -> Self {
        Secret(s.into())
    }
    fn expose(&self) -> &str {
        &self.0
    }
    fn redact(&self, text: &str) -> String {
        if self.0.is_empty() {
            text.to_string()
        } else {
            text.replace(&self.0, "***")
        }
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Retries after the first attempt
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatus {
    pub key: String,
    pub name: Option<String>,
    /// False when the service account cannot see the project
    pub accessible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionReport {
    pub site: String,
    pub version: Option<String>,
    pub deployment: String,
    pub user: String,
    pub projects: Vec<ProjectStatus>,
}

impl ConnectionReport {
    pub fn all_projects_accessible(&self) -> bool {
        self.projects.iter().all(|p| p.accessible)
    }
}

pub struct JiraClient {
    http: Client,
    system: SystemConfig,
    api: String,
    cloud: bool,
    user: Option<String>,
    secret: Secret,
    retry: RetryPolicy,
    /// Acting TCMS user
    actor: String,
    /// Service account name from sync.identity
    identity: Option<String>,
    record_actor: bool,
    run_id: String,
    audit: Vec<AuditTrailEntry>,
}

impl std::fmt::Debug for JiraClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JiraClient")
            .field("system", &self.system.name)
            .field("api", &self.api)
            .field("actor", &self.actor)
            .field("secret", &self.secret)
            .finish()
    }
}

impl JiraClient {
    /// Build a client; the token is read from the environment variable `auth.secret_ref`.
    /// Cloud with `TCMS_JIRA_USER` set (service account email) uses Basic auth, otherwise Bearer;
    /// Data Center uses Bearer (personal access token) unless `TCMS_JIRA_USER` is set.
    pub fn from_config(system: &SystemConfig, sync: &SyncConfig, actor: &str) -> Result<Self> {
        let var = system
            .auth
            .as_ref()
            .and_then(|a| a.secret_ref.as_deref())
            .ok_or_else(|| {
                anyhow!(
                    "system '{}': auth.secret_ref is not configured",
                    system.name
                )
            })?;
        let token = std::env::var(var).map_err(|_| {
            anyhow!(
                "system '{}': environment variable {var} (auth.secret_ref) is not set",
                system.name
            )
        })?;
        if token.is_empty() {
            bail!(
                "system '{}': environment variable {var} is empty",
                system.name
            );
        }
        let user = std::env::var("TCMS_JIRA_USER")
            .ok()
            .filter(|u| !u.is_empty());
        Self::with_token(system, sync, actor, Secret::new(token), user)
    }

    pub fn with_token(
        system: &SystemConfig,
        sync: &SyncConfig,
        actor: &str,
        secret: Secret,
        user: Option<String>,
    ) -> Result<Self> {
        let (cloud, version) = match system.system_type {
            SystemType::JiraCloud => (true, 3),
            SystemType::JiraDatacenter => (false, 2),
            other => bail!(
                "system '{}' has type {other:?}, not a Jira system",
                system.name
            ),
        };
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("build HTTP client")?;
        let run_id = chrono::Utc::now().format("SYNC-%Y%m%dT%H%M%SZ").to_string();
        Ok(Self {
            http,
            api: format!(
                "{}/rest/api/{version}",
                system.base_url.trim_end_matches('/')
            ),
            system: system.clone(),
            cloud,
            user,
            secret,
            retry: RetryPolicy::default(),
            actor: actor.to_string(),
            identity: sync.identity.clone(),
            record_actor: sync.record_actor.unwrap_or(true),
            run_id,
            audit: Vec::new(),
        })
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn is_cloud(&self) -> bool {
        self.cloud
    }

    /// Audit entries produced by this client so far.
    pub fn audit_entries(&self) -> &[AuditTrailEntry] {
        &self.audit
    }

    // ---------------------------------------------------------------- reads

    /// Site, user and the projects of `project_keys`.
    pub fn test_connection(&self) -> Result<ConnectionReport> {
        let myself = self
            .get_json("/myself", &[], None)
            .context("authenticate (GET myself)")?;
        let user = ["displayName", "name", "emailAddress"]
            .iter()
            .find_map(|k| myself.get(*k).and_then(Value::as_str))
            .unwrap_or("unknown")
            .to_string();
        let info = self
            .get_json("/serverInfo", &[], None)
            .unwrap_or(Value::Null);
        let mut projects = Vec::new();
        for key in &self.system.project_keys {
            match self.request(Method::GET, &format!("/project/{key}"), &[], None, None) {
                Ok((_, body)) => {
                    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                    projects.push(ProjectStatus {
                        key: key.clone(),
                        name: v.get("name").and_then(Value::as_str).map(str::to_string),
                        accessible: true,
                    });
                }
                Err(e) => {
                    log::warn!("project {key} not accessible: {e:#}");
                    projects.push(ProjectStatus {
                        key: key.clone(),
                        name: None,
                        accessible: false,
                    });
                }
            }
        }
        Ok(ConnectionReport {
            site: self.system.base_url.clone(),
            version: info
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
            deployment: if self.cloud {
                "Cloud (REST v3)"
            } else {
                "Data Center (REST v2)"
            }
            .into(),
            user,
            projects,
        })
    }

    /// All issues matching `jql`, following the pagination of the deployment type.
    pub fn search_issues(&self, jql: &str, fields: &[&str]) -> Result<Vec<Value>> {
        let page = 50;
        let mut issues = Vec::new();
        if self.cloud {
            // Cloud: token-paginated /search/jql
            let mut token: Option<String> = None;
            loop {
                let mut q = vec![
                    ("jql", jql.to_string()),
                    ("fields", fields.join(",")),
                    ("maxResults", page.to_string()),
                ];
                if let Some(t) = &token {
                    q.push(("nextPageToken", t.clone()));
                }
                let v = self
                    .get_json("/search/jql", &q, None)
                    .context("search issues")?;
                issues.extend(v["issues"].as_array().cloned().unwrap_or_default());
                token = v
                    .get("nextPageToken")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if token.is_none() || v["isLast"].as_bool() == Some(true) {
                    break;
                }
            }
        } else {
            // Data Center: startAt/total
            let mut start = 0usize;
            loop {
                let q = vec![
                    ("jql", jql.to_string()),
                    ("fields", fields.join(",")),
                    ("maxResults", page.to_string()),
                    ("startAt", start.to_string()),
                ];
                let v = self
                    .get_json("/search", &q, None)
                    .context("search issues")?;
                let got = v["issues"].as_array().cloned().unwrap_or_default();
                let n = got.len();
                issues.extend(got);
                start += n;
                let total = v["total"].as_u64().unwrap_or(0) as usize;
                if n == 0 || start >= total {
                    break;
                }
            }
        }
        Ok(issues)
    }

    // --------------------------------------------------------------- writes

    /// Set fields on an issue. `role` selects the write policy that applies.
    pub fn update_fields(
        &mut self,
        issue_key: &str,
        kind: WriteKind,
        fields: &Map<String, Value>,
        target: &WriteTarget,
        reason: &str,
    ) -> Result<AuditTrailEntry> {
        let ids: Vec<String> = fields.keys().cloned().collect();
        self.guard(WriteRequest {
            kind,
            issue_key: issue_key.into(),
            field_ids: ids.clone(),
        })?;
        let body = json!({ "fields": fields }).to_string();
        self.request(
            Method::PUT,
            &format!("/issue/{issue_key}"),
            &[],
            Some(body),
            Some(issue_key),
        )?;
        Ok(self.record(issue_key, target, &ids, reason))
    }

    /// Comment on an issue. The body always ends with the acting user when `record_actor` is on.
    pub fn add_comment(
        &mut self,
        issue_key: &str,
        text: &str,
        target: &WriteTarget,
        reason: &str,
    ) -> Result<AuditTrailEntry> {
        self.guard(WriteRequest {
            kind: WriteKind::Comment,
            issue_key: issue_key.into(),
            field_ids: vec![],
        })?;
        let full = self.comment_text(text);
        let body = if self.cloud {
            json!({"body": {"type":"doc","version":1,"content":[
                {"type":"paragraph","content":[{"type":"text","text": full}]}]}})
        } else {
            json!({ "body": full })
        };
        self.request(
            Method::POST,
            &format!("/issue/{issue_key}/comment"),
            &[],
            Some(body.to_string()),
            Some(issue_key),
        )?;
        Ok(self.record(issue_key, target, &["comment".to_string()], reason))
    }

    /// Apply a workflow transition by name. A name the workflow does not offer is an error.
    pub fn transition(
        &mut self,
        issue_key: &str,
        transition_name: &str,
        target: &WriteTarget,
        reason: &str,
    ) -> Result<AuditTrailEntry> {
        self.guard(WriteRequest {
            kind: WriteKind::FeatureTransition {
                name: transition_name.into(),
            },
            issue_key: issue_key.into(),
            field_ids: vec![],
        })?;
        let avail = self.get_json(
            &format!("/issue/{issue_key}/transitions"),
            &[],
            Some(issue_key),
        )?;
        let id = avail["transitions"]
            .as_array()
            .and_then(|a| {
                a.iter().find(|t| t["name"].as_str() == Some(transition_name))
            })
            .and_then(|t| t["id"].as_str().map(str::to_string))
            .ok_or_else(|| {
                anyhow!("issue {issue_key}: transition '{transition_name}' does not exist in its workflow")
            })?;
        self.request(
            Method::POST,
            &format!("/issue/{issue_key}/transitions"),
            &[],
            Some(json!({"transition": {"id": id}}).to_string()),
            Some(issue_key),
        )?;
        Ok(self.record(
            issue_key,
            target,
            &[format!("transition:{transition_name}")],
            reason,
        ))
    }

    /// Create a TCMS-owned issue or a defect; returns the new key and the audit entry.
    pub fn create_issue(
        &mut self,
        kind: WriteKind,
        fields: &Map<String, Value>,
        target: &WriteTarget,
        reason: &str,
    ) -> Result<(String, AuditTrailEntry)> {
        if !matches!(kind, WriteKind::DedicatedIssue | WriteKind::Defect) {
            bail!("create_issue only creates dedicated issues and defects");
        }
        // A create has no issue key yet: the project it lands in must be one of project_keys.
        let project = fields
            .get("project")
            .and_then(|p| p.get("key"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if !self.system.project_keys.is_empty()
            && !self.system.project_keys.iter().any(|k| k == project)
        {
            bail!(
                "create refused: project '{project}' is not in project_keys {:?}",
                self.system.project_keys
            );
        }
        let ids: Vec<String> = fields.keys().cloned().collect();
        self.guard(WriteRequest {
            kind,
            issue_key: String::new(),
            field_ids: ids.clone(),
        })?;
        let (_, text) = self.request(
            Method::POST,
            "/issue",
            &[],
            Some(json!({ "fields": fields }).to_string()),
            None,
        )?;
        let v: Value = serde_json::from_str(&text).context("parse create-issue response")?;
        let key = v["key"]
            .as_str()
            .ok_or_else(|| anyhow!("create issue: response has no key"))?
            .to_string();
        let entry = self.record(&key, target, &ids, reason);
        Ok((key, entry))
    }

    // ------------------------------------------------------------- internals

    fn guard(&self, req: WriteRequest) -> Result<()> {
        check_write(&self.system, &req)
    }

    fn comment_text(&self, text: &str) -> String {
        if self.record_actor {
            let svc = self
                .identity
                .as_deref()
                .map(|i| format!(" (service account {i})"))
                .unwrap_or_default();
            format!(
                "{text}\n\nTCMS sync run {} by {}{svc}",
                self.run_id, self.actor
            )
        } else {
            text.to_string()
        }
    }

    /// Record the audit entry of a completed write.
    fn record(
        &mut self,
        issue_key: &str,
        target: &WriteTarget,
        written: &[String],
        reason: &str,
    ) -> AuditTrailEntry {
        let url = format!(
            "{}/browse/{issue_key}",
            self.system.base_url.trim_end_matches('/')
        );
        let synced_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let refs =
            vec![json!({"system": "jira", "key": issue_key, "url": url, "synced_at": synced_at})];
        let id = format!("{}-{:03}", self.run_id, self.audit.len() + 1);
        let entry = AuditTrailEntry::sync(id, &self.actor, target, refs, written, reason);
        self.audit.push(entry.clone());
        entry
    }

    fn get_json(&self, path: &str, query: &[(&str, String)], issue: Option<&str>) -> Result<Value> {
        let (_, body) = self.request(Method::GET, path, query, None, issue)?;
        serde_json::from_str(&body).with_context(|| format!("parse response of GET {path}"))
    }

    /// One HTTP exchange under the REST API prefix (`/rest/api/{v}`).
    fn request(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<String>,
        issue: Option<&str>,
    ) -> Result<(u16, String)> {
        self.request_url(&self.api, method, path, query, body, issue)
    }

    /// A request outside the REST API prefix, relative to the site root, for other plugins'
    /// endpoints (Xray `/rest/raven/1.0`, Zephyr Squad `/rest/zapi/latest`).
    ///
    /// No write guard runs here: the caller must check the write first (see `test-backends`).
    /// The response body is parsed as JSON; an empty body is `Value::Null`.
    pub fn raw_json(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
        issue: Option<&str>,
    ) -> Result<Value> {
        let m = Method::from_bytes(method.as_bytes())
            .map_err(|_| anyhow!("unsupported HTTP method '{method}'"))?;
        let root = self.system.base_url.trim_end_matches('/');
        let (_, text) =
            self.request_url(root, m, path, query, body.map(|b| b.to_string()), issue)?;
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).with_context(|| format!("parse response of {method} {path}"))
    }

    /// One HTTP exchange with 429 back-off. Errors name the issue key when there is one.
    fn request_url(
        &self,
        base: &str,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<String>,
        issue: Option<&str>,
    ) -> Result<(u16, String)> {
        let url = format!("{base}{path}");
        let who = issue.map(|k| format!("issue {k}: ")).unwrap_or_default();
        let mut attempt = 0u32;
        loop {
            let mut rb = self.http.request(method.clone(), &url).query(query);
            rb = match (&self.user, self.cloud) {
                (Some(u), _) => rb.basic_auth(u, Some(self.secret.expose())),
                _ => rb.bearer_auth(self.secret.expose()),
            };
            rb = rb.header("Accept", "application/json");
            if let Some(b) = &body {
                rb = rb
                    .header("Content-Type", "application/json")
                    .body(b.clone());
            }
            log::debug!("jira {method} {path} (attempt {})", attempt + 1);
            let resp = rb.send().map_err(|e| {
                anyhow!(
                    "{who}Jira {method} {path} failed: {}",
                    self.secret.redact(&e.to_string())
                )
            })?;
            let status = resp.status().as_u16();
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok());
            let text = resp.text().unwrap_or_default();
            if status == 429 {
                if attempt >= self.retry.max_retries {
                    bail!(
                        "{who}Jira {method} {path} rate limited (HTTP 429) after {} retries",
                        attempt
                    );
                }
                let backoff = self
                    .retry
                    .base_delay
                    .saturating_mul(1u32 << attempt.min(16));
                let delay = retry_after
                    .map(Duration::from_secs)
                    .unwrap_or(backoff)
                    .min(self.retry.max_delay);
                log::warn!(
                    "jira {method} {path}: HTTP 429, retry {} in {:?}",
                    attempt + 1,
                    delay
                );
                std::thread::sleep(delay);
                attempt += 1;
                continue;
            }
            if (200..300).contains(&status) {
                return Ok((status, text));
            }
            let snippet: String = self.secret.redact(&text).chars().take(300).collect();
            bail!("{who}Jira {method} {path} failed with HTTP {status}: {snippet}");
        }
    }
}

/// Plain text of an Atlassian Document Format value (Cloud v3 descriptions and comments);
/// a plain string (Data Center v2) is returned as is.
pub fn adf_to_text(v: &Value) -> String {
    fn walk(v: &Value, out: &mut String) {
        match v {
            Value::String(s) => out.push_str(s),
            Value::Object(o) => {
                if let Some(t) = o.get("text").and_then(Value::as_str) {
                    out.push_str(t);
                }
                if o.get("type").and_then(Value::as_str) == Some("hardBreak") {
                    out.push('\n');
                }
                if let Some(c) = o.get("content") {
                    walk(c, out);
                }
                if matches!(
                    o.get("type").and_then(Value::as_str),
                    Some("paragraph" | "heading" | "listItem")
                ) {
                    out.push('\n');
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut s = String::new();
    walk(v, &mut s);
    s.trim_end().to_string()
}
