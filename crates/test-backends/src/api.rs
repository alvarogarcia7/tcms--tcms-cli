//! Transport for the backends. `Api` is the only place requests leave the process; `Ctx::write`
//! is the only way a backend writes, and it runs the write guard of `jira-client` first.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use id_resolver::SystemConfig;
use jira_client::{check_write, JiraClient, WriteKind, WriteRequest};
use serde_json::Value;

/// Default Zephyr Scale API root (documented public cloud endpoint); override with
/// `TCMS_ZEPHYR_API_URL` (a non-secret setting, e.g. for a proxy).
pub const ZEPHYR_SCALE_DEFAULT_URL: &str = "https://api.zephyrscale.smartbear.com/v2";

/// Requests a backend sends. Paths are relative to the system's site root (Jira) or to the API
/// root (Zephyr Scale).
pub trait Api {
    /// Prefix of Jira's REST API for this deployment (`/rest/api/3` or `/rest/api/2`); empty
    /// for an API without one.
    fn rest_prefix(&self) -> String;

    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value>;

    /// Send a write (POST, PUT, DELETE). `issue_key` names the issue the write touches, when
    /// there is one; the guard has already passed.
    fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        issue_key: Option<&str>,
    ) -> Result<Value>;

    /// Issues matching a JQL query, all pages.
    fn search_jql(&mut self, jql: &str, fields: &[&str]) -> Result<Vec<Value>>;
}

/// The backend's view of its system: the transport plus the configuration the guard uses.
pub struct Ctx {
    pub api: Box<dyn Api>,
    pub system: SystemConfig,
    /// Hand edits overwritten by the writes so far
    pub overwrites: Vec<crate::Overwrite>,
}

impl Ctx {
    /// A write: checked against `write_policy`, the project scope and the configured field ids
    /// before a request is built. A refused write never reaches the transport.
    pub fn write(
        &mut self,
        kind: WriteKind,
        issue_key: &str,
        field_ids: &[&str],
        method: &str,
        path: &str,
        body: &Value,
    ) -> Result<Value> {
        check_write(
            &self.system,
            &WriteRequest {
                kind,
                issue_key: issue_key.to_string(),
                field_ids: field_ids.iter().map(|s| s.to_string()).collect(),
            },
        )?;
        let key = (!issue_key.is_empty()).then_some(issue_key);
        self.api.send(method, path, Some(body), key)
    }
}

/// Jira Cloud or Data Center, through the `jira-client` HTTP client (retries, auth, errors).
pub struct JiraApi {
    client: JiraClient,
    prefix: String,
}

impl JiraApi {
    pub fn new(client: JiraClient) -> Self {
        let prefix = if client.is_cloud() {
            "/rest/api/3"
        } else {
            "/rest/api/2"
        }
        .to_string();
        Self { client, prefix }
    }
}

impl Api for JiraApi {
    fn rest_prefix(&self) -> String {
        self.prefix.clone()
    }

    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.client.raw_json("GET", path, query, None, None)
    }

    fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        issue_key: Option<&str>,
    ) -> Result<Value> {
        self.client.raw_json(method, path, &[], body, issue_key)
    }

    fn search_jql(&mut self, jql: &str, fields: &[&str]) -> Result<Vec<Value>> {
        self.client.search_issues(jql, fields)
    }
}

/// Zephyr Scale's own REST API, authenticated with a bearer token read from the environment
/// variable named by `auth.secret_ref`. The token never appears in an error message.
pub struct ZephyrApi {
    http: reqwest::blocking::Client,
    base: String,
    token: String,
}

impl ZephyrApi {
    pub fn from_env(system: &SystemConfig) -> Result<Self> {
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
        let token = std::env::var(var)
            .ok()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                anyhow!(
                    "system '{}': environment variable {var} is not set",
                    system.name
                )
            })?;
        let base = std::env::var("TCMS_ZEPHYR_API_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| ZEPHYR_SCALE_DEFAULT_URL.to_string());
        Self::new(base, token)
    }

    pub fn new(base: impl Into<String>, token: impl Into<String>) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("build HTTP client")?;
        Ok(Self {
            http,
            base: base.into().trim_end_matches('/').to_string(),
            token: token.into(),
        })
    }

    fn redact(&self, text: &str) -> String {
        if self.token.is_empty() {
            text.to_string()
        } else {
            text.replace(&self.token, "***")
        }
    }

    fn exchange(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<&Value>,
    ) -> Result<Value> {
        let m = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| anyhow!("unsupported HTTP method '{method}'"))?;
        let url = format!("{}{path}", self.base);
        let mut rb = self
            .http
            .request(m, &url)
            .query(query)
            .bearer_auth(&self.token)
            .header("Accept", "application/json");
        if let Some(b) = body {
            rb = rb
                .header("Content-Type", "application/json")
                .body(b.to_string());
        }
        let resp = rb.send().map_err(|e| {
            anyhow!(
                "Zephyr Scale {method} {path} failed: {}",
                self.redact(&e.to_string())
            )
        })?;
        let status = resp.status().as_u16();
        let text = resp.text().unwrap_or_default();
        if !(200..300).contains(&status) {
            let snippet: String = self.redact(&text).chars().take(300).collect();
            bail!("Zephyr Scale {method} {path} failed with HTTP {status}: {snippet}");
        }
        if text.trim().is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_str(&text).with_context(|| format!("parse response of {method} {path}"))
    }
}

impl Api for ZephyrApi {
    fn rest_prefix(&self) -> String {
        String::new()
    }

    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.exchange("GET", path, query, None)
    }

    fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        _issue_key: Option<&str>,
    ) -> Result<Value> {
        self.exchange(method, path, &[], body)
    }

    fn search_jql(&mut self, _jql: &str, _fields: &[&str]) -> Result<Vec<Value>> {
        bail!("Zephyr Scale has no Jira search; the Zephyr Scale backend does not call it")
    }
}
