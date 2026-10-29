//! In-process fake of the four backends' endpoints (feature `mock`). It answers the requests the
//! backends send with the shapes this crate assumes, keeps every write, and never deletes.
//! It is a model built from this crate's assumptions, not a recording of a real system.

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use serde_json::{json, Value};

use crate::api::Api;

/// A recorded request.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub method: String,
    pub path: String,
    pub body: Value,
    pub issue_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct FakeApi {
    pub calls: Vec<Call>,
    /// Jira issues by key: `{key, id, fields}`
    pub issues: BTreeMap<String, Value>,
    /// Zephyr Scale and Squad objects by key or id: `{kind, key, body}`
    pub objects: BTreeMap<String, Value>,
    /// Jira links created: `{type, outward, inward}`
    pub links: Vec<Value>,
    /// Workflow transitions every issue offers: (id, name)
    pub transitions: Vec<(String, String)>,
    next: u32,
}

impl FakeApi {
    pub fn new() -> Self {
        Self {
            next: 100,
            transitions: vec![
                ("21".into(), "In Progress".into()),
                ("31".into(), "Done".into()),
            ],
            ..Self::default()
        }
    }

    /// Writes (everything except GET); the fake never sees a DELETE.
    pub fn writes(&self) -> Vec<&Call> {
        self.calls.iter().filter(|c| c.method != "GET").collect()
    }

    pub fn issue_field(&self, key: &str, field: &str) -> Option<Value> {
        self.issues
            .get(key)
            .and_then(|i| i["fields"].get(field).cloned())
    }

    /// Zephyr Scale or Squad object of kind `kind` (`case`, `cycle`, `execution`, ...) with key
    /// or id `key`.
    pub fn object(&self, key: &str) -> Option<&Value> {
        self.objects.get(key)
    }

    fn fresh(&mut self) -> u32 {
        self.next += 1;
        self.next
    }

    /// Merge `body` into the object of `kind` whose id is `id`; a missing one is an error.
    fn update_by_id(&mut self, kind: &str, id: &str, body: &Value) -> Result<Value> {
        let Some(obj) = self.objects.values_mut().find(|o| {
            o["kind"] == kind && o["id"].as_u64().map(|n| n.to_string()).as_deref() == Some(id)
        }) else {
            bail!("fake: {kind} {id} does not exist");
        };
        Self::merge(&mut obj["body"], body);
        Ok(Value::Null)
    }

    fn merge(target: &mut Value, body: &Value) {
        if let (Some(t), Some(b)) = (target.as_object_mut(), body.as_object()) {
            for (k, v) in b {
                match (t.get_mut(k), v) {
                    (Some(Value::Object(tm)), Value::Object(bm)) => {
                        for (ik, iv) in bm {
                            tm.insert(ik.clone(), iv.clone());
                        }
                    }
                    _ => {
                        t.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
}

impl Api for FakeApi {
    fn rest_prefix(&self) -> String {
        "/rest/api/2".into()
    }

    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.calls.push(Call {
            method: "GET".into(),
            path: path.into(),
            body: Value::Null,
            issue_key: None,
        });
        if path.contains("/project/") {
            let key = path.rsplit('/').next().unwrap_or("");
            return Ok(json!({ "id": "10000", "key": key }));
        }
        if path == "/testcases" {
            let q = query
                .iter()
                .find(|(k, _)| *k == "query")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            let values: Vec<Value> = self
                .objects
                .values()
                .filter(|o| o["kind"] == "case")
                .filter(|o| {
                    o["body"]["labels"].as_array().is_some_and(|ls| {
                        ls.iter().any(|l| q.contains(l.as_str().unwrap_or("\u{0}")))
                    })
                })
                .map(|o| json!({ "key": o["key"] }))
                .collect();
            return Ok(json!({ "values": values }));
        }
        if path == "/testcycles" {
            let name = query_value(query, "query");
            let values: Vec<Value> = self
                .objects
                .values()
                .filter(|o| o["kind"] == "cycle")
                .filter(|o| name.contains(o["body"]["name"].as_str().unwrap_or("\u{0}")))
                .map(|o| json!({ "id": o["id"], "key": o["key"], "name": o["body"]["name"] }))
                .collect();
            return Ok(json!({ "values": values }));
        }
        if path == "/testexecutions" {
            let cycle = query_value(query, "testCycleKey");
            let values: Vec<Value> = self
                .objects
                .values()
                .filter(|o| o["kind"] == "execution" && o["body"]["testCycleKey"] == cycle.as_str())
                .map(|o| {
                    json!({ "id": o["id"], "testCaseKey": o["body"]["testCaseKey"], "statusName": o["body"]["statusName"] })
                })
                .collect();
            return Ok(json!({ "values": values }));
        }
        if path == "/rest/zapi/latest/cycle" {
            let project = query_value(query, "projectId");
            let mut cycles = serde_json::Map::new();
            for o in self.objects.values().filter(|o| o["kind"] == "squad_cycle") {
                if o["body"]["projectId"].as_str() == Some(project.as_str()) {
                    cycles.insert(
                        o["key"].as_str().unwrap_or("").to_string(),
                        json!({ "id": o["id"], "name": o["body"]["name"] }),
                    );
                }
            }
            return Ok(Value::Object(cycles));
        }
        if path == "/rest/zapi/latest/execution" {
            let cycle = query_value(query, "cycleId");
            let executions: Vec<Value> = self
                .objects
                .values()
                .filter(|o| {
                    o["kind"] == "squad_execution"
                        && o["body"]["cycleId"].to_string().trim_matches('"') == cycle
                })
                .map(|o| json!({ "id": o["id"], "issueId": o["body"]["issueId"] }))
                .collect();
            return Ok(json!({ "executions": executions }));
        }
        if path.contains("/version/") {
            let key = format!("version-{}", path.rsplit('/').next().unwrap_or(""));
            return Ok(self
                .objects
                .get(&key)
                .map(|o| o["body"].clone())
                .unwrap_or(Value::Null));
        }
        if path.ends_with("/transitions") {
            let t: Vec<Value> = self
                .transitions
                .iter()
                .map(|(id, n)| json!({ "id": id, "name": n }))
                .collect();
            return Ok(json!({ "transitions": t }));
        }
        if let Some(key) = path.strip_prefix("/rest/api/2/issue/") {
            return Ok(self
                .issues
                .get(key)
                .map(|i| json!({ "id": i["id"], "key": i["key"], "fields": i["fields"] }))
                .unwrap_or(Value::Null));
        }
        if let Some(key) = path.strip_prefix("/testcases/") {
            return Ok(self
                .objects
                .get(key)
                .map(|o| o["body"].clone())
                .unwrap_or(Value::Null));
        }
        Ok(Value::Null)
    }

    fn send(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        issue_key: Option<&str>,
    ) -> Result<Value> {
        let body = body.cloned().unwrap_or(Value::Null);
        self.calls.push(Call {
            method: method.into(),
            path: path.into(),
            body: body.clone(),
            issue_key: issue_key.map(str::to_string),
        });
        if method == "DELETE" {
            return Ok(Value::Null);
        }
        // Jira issues
        if method == "POST" && path.ends_with("/issue") {
            let n = self.fresh();
            let p = body["fields"]["project"]["key"]
                .as_str()
                .unwrap_or("PAY")
                .to_string();
            let key = format!("{p}-{n}");
            self.issues.insert(
                key.clone(),
                json!({ "id": n.to_string(), "key": key, "fields": body["fields"] }),
            );
            return Ok(json!({ "id": n.to_string(), "key": key }));
        }
        if method == "PUT" && path.contains("/rest/api/2/issue/") {
            let key = path.rsplit('/').next().unwrap_or("").to_string();
            let Some(issue) = self.issues.get_mut(&key) else {
                bail!("fake: issue {key} does not exist");
            };
            if let Some(f) = body["fields"].as_object() {
                for (k, v) in f {
                    issue["fields"][k] = v.clone();
                }
            }
            return Ok(Value::Null);
        }
        // Fix Versions
        if method == "POST" && path.ends_with("/version") {
            let n = self.fresh();
            let key = format!("version-{n}");
            self.objects.insert(
                key.clone(),
                json!({ "kind": "version", "key": key, "body": body }),
            );
            return Ok(json!({ "id": n.to_string() }));
        }
        if method == "PUT" && path.contains("/version/") {
            let key = format!("version-{}", path.rsplit('/').next().unwrap_or(""));
            let Some(obj) = self.objects.get_mut(&key) else {
                bail!("fake: version {key} does not exist");
            };
            Self::merge(&mut obj["body"], &body);
            return Ok(Value::Null);
        }
        if method == "POST" && path.ends_with("/transitions") {
            return Ok(Value::Null);
        }
        if method == "POST" && path.ends_with("/issueLink") {
            self.links.push(body);
            return Ok(Value::Null);
        }
        // Zephyr Scale
        if method == "POST" && path == "/testcases" {
            let n = self.fresh();
            let key = format!("{}-T{n}", body["projectKey"].as_str().unwrap_or("PAY"));
            self.objects.insert(
                key.clone(),
                json!({ "kind": "case", "key": key, "body": body }),
            );
            return Ok(json!({ "key": key, "id": n }));
        }
        if method == "PUT" && path.starts_with("/testcases/") {
            let key = path.trim_start_matches("/testcases/").to_string();
            let Some(obj) = self.objects.get_mut(&key) else {
                bail!("fake: test case {key} does not exist");
            };
            Self::merge(&mut obj["body"], &body);
            return Ok(Value::Null);
        }
        if method == "POST" && (path == "/testcycles" || path == "/testexecutions") {
            let n = self.fresh();
            let (kind, letter) = if path == "/testcycles" {
                ("cycle", "C")
            } else {
                ("execution", "E")
            };
            let key = format!(
                "{}-{letter}{n}",
                body["projectKey"].as_str().unwrap_or("PAY")
            );
            self.objects.insert(
                key.clone(),
                json!({ "kind": kind, "key": key, "id": n, "body": body }),
            );
            return Ok(json!({ "key": key, "id": n }));
        }
        // Zephyr Squad (zapi)
        if method == "POST" && path == "/rest/zapi/latest/cycle" {
            let n = self.fresh();
            let key = n.to_string();
            self.objects.insert(
                key.clone(),
                json!({ "kind": "squad_cycle", "key": key, "id": n, "body": body }),
            );
            return Ok(json!({ "id": n }));
        }
        if method == "POST" && path == "/rest/zapi/latest/execution" {
            let n = self.fresh();
            self.objects.insert(
                format!("exec-{n}"),
                json!({ "kind": "squad_execution", "key": format!("exec-{n}"), "id": n, "body": body }),
            );
            return Ok(json!({ "id": n }));
        }
        // updates of an existing execution (Zephyr Scale, Squad)
        if method == "PUT" && path.starts_with("/testexecutions/") {
            let id = path.trim_start_matches("/testexecutions/");
            return self.update_by_id("execution", id, &body);
        }
        if method == "PUT"
            && path.starts_with("/rest/zapi/latest/execution/")
            && path.ends_with("/execute")
        {
            let id = path
                .trim_start_matches("/rest/zapi/latest/execution/")
                .trim_end_matches("/execute");
            return self.update_by_id("squad_execution", id, &body);
        }
        // Xray (raven)
        if method == "POST" && path.starts_with("/rest/raven/1.0/api/testexec/") {
            return Ok(Value::Null);
        }
        if method == "PUT" && path == "/rest/raven/1.0/api/testrun/status" {
            let key = format!("xray-status-{}", body["testKey"].as_str().unwrap_or(""));
            self.objects.insert(
                key.clone(),
                json!({ "kind": "xray_status", "key": key, "body": body }),
            );
            return Ok(Value::Null);
        }
        bail!("fake: no route for {method} {path}")
    }

    fn search_jql(&mut self, jql: &str, _fields: &[&str]) -> Result<Vec<Value>> {
        self.calls.push(Call {
            method: "GET".into(),
            path: "/rest/api/2/search".into(),
            body: json!({ "jql": jql }),
            issue_key: None,
        });
        Ok(self
            .issues
            .values()
            .filter(|i| {
                let p = i["fields"]["project"]["key"].as_str().unwrap_or("");
                let t = i["fields"]["issuetype"]["name"].as_str().unwrap_or("");
                jql.contains(&format!("project = \"{p}\""))
                    && jql.contains(&format!("issuetype = \"{t}\""))
            })
            .cloned()
            .collect())
    }
}

/// The value of query parameter `name` in `query` (empty when absent).
fn query_value(query: &[(&str, String)], name: &str) -> String {
    query
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}
