//! A mock Jira HTTP server built on `std::net::TcpListener`, for tests (feature `mock`).
//!
//! It serves the Cloud layout (`/rest/api/3`, token-paginated `/search/jql`, ADF bodies) or the
//! Data Center layout (`/rest/api/2`, `startAt` paging, plain text bodies); requests under the
//! other version's prefix get 404. It checks the `Authorization` header, can answer scripted
//! responses (for example 429 then 200) and records every request, so tests can prove that no
//! request was sent. This is a model of Jira built from its public documentation, not a
//! recording of a real instance.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Cloud,
    DataCenter,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    /// Path with query string
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    pub fn body_json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

#[derive(Debug, Clone)]
struct Scripted {
    method: String,
    path_contains: String,
    status: u16,
    body: String,
    headers: Vec<(String, String)>,
}

#[derive(Default)]
struct State {
    requests: Vec<RecordedRequest>,
    scripted: Vec<Scripted>,
    issues: BTreeMap<String, Value>,
    projects: BTreeMap<String, String>,
    transitions: Vec<(String, String)>,
    /// Fix Versions by id (`POST /version`)
    versions: BTreeMap<String, Value>,
    /// Xray executions (raven API): execution key to test key to status name
    raven: BTreeMap<String, BTreeMap<String, String>>,
    /// Zephyr Scale (`/v2`): test cases by key, cycles by key, executions by id
    zephyr_cases: BTreeMap<String, Value>,
    zephyr_cycles: BTreeMap<String, Value>,
    zephyr_execs: BTreeMap<String, Value>,
    /// Zephyr Squad (`/rest/zapi/latest`): cycles and executions by id
    zapi_cycles: BTreeMap<String, Value>,
    zapi_execs: BTreeMap<String, Value>,
    zephyr_next: u32,
    next_key: u32,
    next_version: u32,
}

pub struct MockJira {
    layout: Layout,
    addr: String,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockJira {
    /// Start a server; `expected_auth` is the full `Authorization` header value it accepts.
    pub fn start(layout: Layout, expected_auth: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock jira");
        let addr = listener.local_addr().unwrap().to_string();
        listener.set_nonblocking(true).unwrap();
        let state = Arc::new(Mutex::new(State {
            next_key: 100,
            next_version: 10100,
            ..State::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (st, sp, auth) = (state.clone(), stop.clone(), expected_auth.to_string());
        let handle = std::thread::spawn(move || {
            while !sp.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        handle_connection(stream, layout, &st, &auth);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(2)),
                }
            }
        });
        let mock = Self {
            layout,
            addr,
            state,
            stop,
            handle: Some(handle),
        };
        mock.add_project("PAY", "Payments");
        mock.add_project("AUTH", "Authentication");
        mock
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
    pub fn add_project(&self, key: &str, name: &str) {
        self.state
            .lock()
            .unwrap()
            .projects
            .insert(key.into(), name.into());
    }
    pub fn remove_project(&self, key: &str) {
        self.state.lock().unwrap().projects.remove(key);
    }
    /// Workflow transition offered on every issue.
    pub fn add_transition(&self, id: &str, name: &str) {
        self.state
            .lock()
            .unwrap()
            .transitions
            .push((id.into(), name.into()));
    }

    /// Add an issue shaped like the real API: description is an ADF document on Cloud and a
    /// string on Data Center.
    #[allow(clippy::too_many_arguments)]
    pub fn add_issue(
        &self,
        key: &str,
        issue_type: &str,
        summary: &str,
        description: &str,
        status: &str,
        updated: &str,
        labels: &[&str],
    ) {
        let desc = match self.layout {
            Layout::Cloud => json!({"type":"doc","version":1,"content":[
                {"type":"paragraph","content":[{"type":"text","text":description}]}]}),
            Layout::DataCenter => json!(description),
        };
        let issue = json!({
            "id": key.replace('-', ""),
            "key": key,
            "fields": {
                "summary": summary,
                "description": desc,
                "issuetype": {"name": issue_type},
                "status": {"name": status, "statusCategory": {"key": if matches!(status, "Done" | "Closed" | "Resolved") {"done"} else {"new"}}},
                "updated": updated,
                "labels": labels,
                "project": {"key": key.rsplit_once('-').map(|x| x.0).unwrap_or("")}
            }
        });
        self.state.lock().unwrap().issues.insert(key.into(), issue);
    }

    pub fn issue(&self, key: &str) -> Option<Value> {
        self.state.lock().unwrap().issues.get(key).cloned()
    }

    /// A Fix Version as the mock holds it (`POST /version` creates them).
    pub fn version(&self, id: &str) -> Option<Value> {
        self.state.lock().unwrap().versions.get(id).cloned()
    }

    /// Answer the next request whose method matches and whose path contains `path_contains`
    /// with this response, once.
    pub fn script(
        &self,
        method: &str,
        path_contains: &str,
        status: u16,
        body: &str,
        headers: &[(&str, &str)],
    ) {
        self.state.lock().unwrap().scripted.push(Scripted {
            method: method.into(),
            path_contains: path_contains.into(),
            status,
            body: body.into(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state.lock().unwrap().requests.clone()
    }
    pub fn request_count(&self) -> usize {
        self.state.lock().unwrap().requests.len()
    }
    pub fn clear_requests(&self) {
        self.state.lock().unwrap().requests.clear();
    }
    /// Requests that change data (not GET)
    pub fn write_requests(&self) -> Vec<RecordedRequest> {
        self.requests()
            .into_iter()
            .filter(|r| r.method != "GET")
            .collect()
    }
}

impl Drop for MockJira {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<RecordedRequest> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let header_end = loop {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| {
            l.split_once(':')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect();
    let len: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = buf[header_end + 4..].to_vec();
    while body.len() < len {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    Some(RecordedRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn respond(stream: &mut TcpStream, status: u16, body: &str, extra: &[(String, String)]) {
    let reason = match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        _ => "Status",
    };
    let mut out = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extra {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    out.push_str(body);
    let _ = stream.write_all(out.as_bytes());
    let _ = stream.flush();
}

fn query_param(path: &str, name: &str) -> Option<String> {
    let q = path.split_once('?')?.1;
    q.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let h = std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("00");
                out.push(u8::from_str_radix(h, 16).unwrap_or(b'?'));
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn handle_connection(
    mut stream: TcpStream,
    layout: Layout,
    state: &Arc<Mutex<State>>,
    expected_auth: &str,
) {
    let Some(req) = read_request(&mut stream) else {
        return;
    };
    let mut st = state.lock().unwrap();
    st.requests.push(req.clone());

    // scripted responses first
    if let Some(i) = st
        .scripted
        .iter()
        .position(|s| s.method == req.method && req.path.contains(&s.path_contains))
    {
        let s = st.scripted.remove(i);
        drop(st);
        respond(&mut stream, s.status, &s.body, &s.headers);
        return;
    }

    if req.header("authorization") != Some(expected_auth) {
        drop(st);
        respond(
            &mut stream,
            401,
            r#"{"errorMessages":["Unauthorized"]}"#,
            &[],
        );
        return;
    }

    // Zephyr Scale's API (served under /v2 by this mock) and Zephyr Squad's zapi (under
    // /rest/zapi/latest) (ASSUMPTION: the shapes in route_zephyr and route_zapi are from the
    // public documentation as remembered)
    let base_path = req.path.split('?').next().unwrap_or("").to_string();
    if let Some(rest) = base_path.strip_prefix("/v2") {
        let seg: Vec<&str> = rest.trim_start_matches('/').split('/').collect();
        let (status, body) = route_zephyr(&mut st, &req, &seg);
        drop(st);
        respond(&mut stream, status, &body, &[]);
        return;
    }
    if let Some(rest) = base_path.strip_prefix("/rest/zapi/latest") {
        let seg: Vec<&str> = rest.trim_start_matches('/').split('/').collect();
        let (status, body) = route_zapi(&mut st, &req, &seg);
        drop(st);
        respond(&mut stream, status, &body, &[]);
        return;
    }

    // Xray's REST API lives at the site root, outside the Jira REST prefix (ASSUMPTION: the
    // shapes in route_raven are from Xray's public documentation as remembered)
    if let Some(raven) = req
        .path
        .split('?')
        .next()
        .and_then(|p| p.strip_prefix("/rest/raven/1.0"))
    {
        let seg: Vec<&str> = raven.trim_start_matches('/').split('/').collect();
        let (status, body) = route_raven(&mut st, &req, &seg);
        drop(st);
        respond(&mut stream, status, &body, &[]);
        return;
    }

    let api = match layout {
        Layout::Cloud => "/rest/api/3",
        Layout::DataCenter => "/rest/api/2",
    };
    let path_only = req.path.split('?').next().unwrap_or("");
    let Some(rest) = path_only.strip_prefix(api) else {
        drop(st);
        respond(
            &mut stream,
            404,
            r#"{"errorMessages":["Not found: wrong REST API version for this deployment"]}"#,
            &[],
        );
        return;
    };
    let seg: Vec<&str> = rest.trim_start_matches('/').split('/').collect();
    let (status, body) = route(&mut st, layout, &req, &seg);
    drop(st);
    respond(&mut stream, status, &body, &[]);
}

fn route(st: &mut State, layout: Layout, req: &RecordedRequest, seg: &[&str]) -> (u16, String) {
    let m = req.method.as_str();
    match (m, seg) {
        ("GET", ["myself"]) => (
            200,
            json!({"displayName":"TCMS Bot","emailAddress":"tcms-bot@example.com","name":"tcms-bot","active":true}).to_string(),
        ),
        ("GET", ["serverInfo"]) => (
            200,
            json!({"version":"1000.0.0","serverTitle":"Mock Jira","deploymentType": match layout {Layout::Cloud=>"Cloud", Layout::DataCenter=>"Server"}}).to_string(),
        ),
        ("GET", ["project", key]) => match st.projects.get(*key) {
            Some(name) => (200, json!({"id": "10000", "key": key, "name": name}).to_string()),
            None => (404, r#"{"errorMessages":["No project could be found"]}"#.into()),
        },
        ("GET", ["search", "jql"]) if layout == Layout::Cloud => {
            let jql = query_param(&req.path, "jql").unwrap_or_default();
            let max: usize = query_param(&req.path, "maxResults").and_then(|v| v.parse().ok()).unwrap_or(50);
            let start: usize = query_param(&req.path, "nextPageToken").and_then(|v| v.parse().ok()).unwrap_or(0);
            let all = matching_issues(st, &jql);
            let page: Vec<Value> = all.iter().skip(start).take(max).cloned().collect();
            let next = start + page.len();
            let mut v = json!({"issues": page});
            if next < all.len() {
                v["nextPageToken"] = json!(next.to_string());
                v["isLast"] = json!(false);
            } else {
                v["isLast"] = json!(true);
            }
            (200, v.to_string())
        }
        ("GET", ["search"]) if layout == Layout::DataCenter => {
            let jql = query_param(&req.path, "jql").unwrap_or_default();
            let max: usize = query_param(&req.path, "maxResults").and_then(|v| v.parse().ok()).unwrap_or(50);
            let start: usize = query_param(&req.path, "startAt").and_then(|v| v.parse().ok()).unwrap_or(0);
            let all = matching_issues(st, &jql);
            let page: Vec<Value> = all.iter().skip(start).take(max).cloned().collect();
            (200, json!({"startAt": start, "maxResults": max, "total": all.len(), "issues": page}).to_string())
        }
        ("PUT", ["issue", key]) => match st.issues.get_mut(*key) {
            Some(issue) => {
                if let Some(fields) = req.body_json()["fields"].as_object() {
                    for (k, v) in fields {
                        issue["fields"][k] = v.clone();
                    }
                }
                (204, String::new())
            }
            None => (404, r#"{"errorMessages":["Issue does not exist"]}"#.into()),
        },
        ("POST", ["issue", key, "comment"]) => {
            if st.issues.contains_key(*key) {
                (201, json!({"id":"10001","body": req.body_json()["body"].clone()}).to_string())
            } else {
                (404, r#"{"errorMessages":["Issue does not exist"]}"#.into())
            }
        }
        ("GET", ["issue", key]) => match st.issues.get(*key) {
            Some(issue) => (200, issue.to_string()),
            None => (404, r#"{"errorMessages":["Issue does not exist"]}"#.into()),
        },
        ("GET", ["issue", key, "transitions"]) => {
            if st.issues.contains_key(*key) {
                let t: Vec<Value> = st.transitions.iter().map(|(id, n)| json!({"id": id, "name": n})).collect();
                (200, json!({"transitions": t}).to_string())
            } else {
                (404, r#"{"errorMessages":["Issue does not exist"]}"#.into())
            }
        }
        ("POST", ["issue", key, "transitions"]) => {
            if st.issues.contains_key(*key) {
                (204, String::new())
            } else {
                (404, r#"{"errorMessages":["Issue does not exist"]}"#.into())
            }
        }
        ("POST", ["issueLink"]) => (201, String::new()),
        ("POST", ["version"]) => {
            let body = req.body_json();
            st.next_version += 1;
            let id = st.next_version.to_string();
            let mut v = json!({"id": id, "released": false});
            for k in ["name", "description", "projectId"] {
                v[k] = body[k].clone();
            }
            st.versions.insert(id.clone(), v);
            (201, json!({"id": id, "name": body["name"]}).to_string())
        }
        ("GET", ["version", id]) => match st.versions.get(*id) {
            Some(v) => (200, v.to_string()),
            None => (404, r#"{"errorMessages":["Version does not exist"]}"#.into()),
        },
        ("PUT", ["version", id]) => match st.versions.get_mut(*id) {
            Some(v) => {
                if let Some(fields) = req.body_json().as_object() {
                    for (k, val) in fields {
                        v[k] = val.clone();
                    }
                }
                (204, String::new())
            }
            None => (404, r#"{"errorMessages":["Version does not exist"]}"#.into()),
        },
        ("POST", ["issue"]) => {
            let mut fields = req.body_json()["fields"].clone();
            // Jira gives every new issue its initial status
            if fields.get("status").is_none() {
                fields["status"] = json!({"name": "To Do", "statusCategory": {"key": "new"}});
            }
            let project = fields["project"]["key"].as_str().unwrap_or("PAY").to_string();
            st.next_key += 1;
            let key = format!("{project}-{}", st.next_key);
            // every new issue has its own id, as in Jira (the id of an issue is read back by GET)
            let id = (20000 + st.next_key).to_string();
            st.issues.insert(key.clone(), json!({"id": id, "key": key, "fields": fields}));
            (201, json!({"id": id, "key": key}).to_string())
        }
        _ => (404, json!({"errorMessages":[format!("Unknown mock route {m} {seg:?}")]}).to_string()),
    }
}

/// A query or body value as text (a JSON string as is, a number as its digits).
fn text_of(v: &Value) -> String {
    v.as_str()
        .map(str::to_string)
        .unwrap_or_else(|| v.to_string())
}

/// Merge the top-level fields of `body` into `target`; `customFields` is merged field by field.
fn merge_fields(target: &mut Value, body: &Value) {
    let (Some(t), Some(b)) = (target.as_object_mut(), body.as_object()) else {
        return;
    };
    for (k, v) in b {
        match (k.as_str(), t.get_mut(k), v.as_object()) {
            ("customFields", Some(Value::Object(tm)), Some(bm)) => {
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

/// Zephyr Scale (`/v2`): test cases, cycles and executions. Listings filter on `projectKey`, on
/// `query` (`labels = "..."` for cases, `name = "..."` for cycles) and on `testCycleKey`.
fn route_zephyr(st: &mut State, req: &RecordedRequest, seg: &[&str]) -> (u16, String) {
    let not_found = |what: &str| {
        (
            404,
            json!({"errorMessages":[format!("{what} not found")]}).to_string(),
        )
    };
    let body = req.body_json();
    let project = query_param(&req.path, "projectKey");
    let query = query_param(&req.path, "query").unwrap_or_default();
    let in_project = |o: &Value| project.as_deref().is_none_or(|p| o["projectKey"] == p);
    match (req.method.as_str(), seg) {
        ("GET", ["testcases"]) => {
            let labels = jql_values(&query, "labels");
            let values: Vec<Value> = st
                .zephyr_cases
                .values()
                .filter(|c| in_project(c))
                .filter(|c| {
                    labels.as_ref().is_none_or(|ls| {
                        c["labels"].as_array().is_some_and(|cl| {
                            cl.iter()
                                .any(|l| ls.iter().any(|x| l.as_str() == Some(x.as_str())))
                        })
                    })
                })
                .cloned()
                .collect();
            let total = values.len();
            (200, json!({"values": values, "total": total}).to_string())
        }
        ("POST", ["testcases"]) => {
            st.zephyr_next += 1;
            let n = st.zephyr_next;
            let p = body["projectKey"].as_str().unwrap_or("PAY").to_string();
            let key = format!("{p}-T{n}");
            let case = json!({"id": n, "key": key, "projectKey": p, "name": body["name"],
                "labels": body["labels"], "statusName": body["statusName"]});
            st.zephyr_cases.insert(key, case.clone());
            (201, case.to_string())
        }
        ("GET", ["testcases", key]) => match st.zephyr_cases.get(*key) {
            Some(c) => (200, c.to_string()),
            None => not_found("Test case"),
        },
        ("PUT", ["testcases", key]) => match st.zephyr_cases.get_mut(*key) {
            Some(c) => {
                merge_fields(c, &body);
                (200, c.to_string())
            }
            None => not_found("Test case"),
        },
        ("GET", ["testcycles"]) => {
            let names = jql_values(&query, "name");
            let values: Vec<Value> = st
                .zephyr_cycles
                .values()
                .filter(|c| in_project(c))
                .filter(|c| {
                    names
                        .as_ref()
                        .is_none_or(|ns| ns.iter().any(|n| c["name"] == n.as_str()))
                })
                .cloned()
                .collect();
            let total = values.len();
            (200, json!({"values": values, "total": total}).to_string())
        }
        ("POST", ["testcycles"]) => {
            st.zephyr_next += 1;
            let n = st.zephyr_next;
            let p = body["projectKey"].as_str().unwrap_or("PAY").to_string();
            let key = format!("{p}-C{n}");
            let cycle = json!({"id": n, "key": key, "projectKey": p, "name": body["name"]});
            st.zephyr_cycles.insert(key, cycle.clone());
            (201, cycle.to_string())
        }
        ("GET", ["testexecutions"]) => {
            let cycle = query_param(&req.path, "testCycleKey").unwrap_or_default();
            let values: Vec<Value> = st
                .zephyr_execs
                .values()
                .filter(|e| in_project(e) && e["testCycleKey"] == cycle.as_str())
                .map(|e| json!({"id": e["id"], "testCaseKey": e["testCaseKey"], "testCycleKey": e["testCycleKey"], "statusName": e["statusName"]}))
                .collect();
            let total = values.len();
            (200, json!({"values": values, "total": total}).to_string())
        }
        ("POST", ["testexecutions"]) => {
            st.zephyr_next += 1;
            let n = st.zephyr_next;
            let mut exec = body.clone();
            exec["id"] = json!(n);
            st.zephyr_execs.insert(n.to_string(), exec.clone());
            (201, json!({"id": n}).to_string())
        }
        ("PUT", ["testexecutions", id]) => match st.zephyr_execs.get_mut(*id) {
            Some(e) => {
                merge_fields(e, &body);
                (200, e.to_string())
            }
            None => not_found("Test execution"),
        },
        _ => (
            404,
            json!({"errorMessages":[format!("Unknown Zephyr Scale route {} {seg:?}", req.method)]})
                .to_string(),
        ),
    }
}

/// Zephyr Squad (zapi): cycles of a project (an object keyed by cycle id, as assumed by the
/// backend), executions of a cycle (`executions` list) and their status updates.
fn route_zapi(st: &mut State, req: &RecordedRequest, seg: &[&str]) -> (u16, String) {
    let not_found = |what: &str| {
        (
            404,
            json!({"errorMessages":[format!("{what} not found")]}).to_string(),
        )
    };
    let body = req.body_json();
    match (req.method.as_str(), seg) {
        ("GET", ["cycle"]) => {
            let project = query_param(&req.path, "projectId").unwrap_or_default();
            let mut cycles = serde_json::Map::new();
            for (id, c) in st
                .zapi_cycles
                .iter()
                .filter(|(_, c)| text_of(&c["projectId"]) == project)
            {
                cycles.insert(id.clone(), json!({"id": c["id"], "name": c["name"]}));
            }
            (200, Value::Object(cycles).to_string())
        }
        ("POST", ["cycle"]) => {
            st.zephyr_next += 1;
            let n = st.zephyr_next;
            let cycle =
                json!({"id": n, "name": body["name"], "projectId": text_of(&body["projectId"])});
            st.zapi_cycles.insert(n.to_string(), cycle);
            (200, json!({"id": n}).to_string())
        }
        ("GET", ["execution"]) => {
            let cycle = query_param(&req.path, "cycleId").unwrap_or_default();
            let executions: Vec<Value> = st
                .zapi_execs
                .values()
                .filter(|e| text_of(&e["cycleId"]) == cycle)
                .map(|e| json!({"id": e["id"], "issueId": e["issueId"], "cycleId": e["cycleId"], "executionStatus": e["status"]}))
                .collect();
            (200, json!({ "executions": executions }).to_string())
        }
        ("POST", ["execution"]) => {
            st.zephyr_next += 1;
            let n = st.zephyr_next;
            let exec = json!({"id": n, "issueId": text_of(&body["issueId"]),
                "cycleId": text_of(&body["cycleId"]), "status": body["status"]});
            st.zapi_execs.insert(n.to_string(), exec);
            (200, json!({"id": n}).to_string())
        }
        ("PUT", ["execution", id, "execute"]) => match st.zapi_execs.get_mut(*id) {
            Some(e) => {
                e["status"] = body["status"].clone();
                (200, e.to_string())
            }
            None => not_found("Execution"),
        },
        _ => (
            404,
            json!({"errorMessages":[format!("Unknown zapi route {} {seg:?}", req.method)]})
                .to_string(),
        ),
    }
}

/// Xray (raven) routes: add tests to an execution, set a test's status in it, read an execution.
/// The execution must be a Jira issue of the mock; a test must be added before its status is set.
fn route_raven(st: &mut State, req: &RecordedRequest, seg: &[&str]) -> (u16, String) {
    let not_found = |what: &str| {
        (
            404,
            json!({"errorMessages":[format!("{what} not found")]}).to_string(),
        )
    };
    match (req.method.as_str(), seg) {
        ("POST", ["api", "testexec", exec, "test"]) => {
            if !st.issues.contains_key(*exec) {
                return not_found("Test execution");
            }
            let add = req.body_json()["add"].clone();
            let tests = st.raven.entry(exec.to_string()).or_default();
            for k in add
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                tests
                    .entry(k.to_string())
                    .or_insert_with(|| "TODO".to_string());
            }
            (200, "[]".into())
        }
        ("PUT", ["api", "testrun", "status"]) => {
            let body = req.body_json();
            let exec = body["testExecKey"].as_str().unwrap_or("").to_string();
            let test = body["testKey"].as_str().unwrap_or("").to_string();
            match st.raven.get_mut(&exec).and_then(|t| t.get_mut(&test)) {
                Some(status) => {
                    *status = body["status"].as_str().unwrap_or("").to_string();
                    (200, "{}".into())
                }
                None => not_found("Test in execution"),
            }
        }
        ("GET", ["api", "testexec", exec]) => match st.raven.get(*exec) {
            Some(tests) => {
                let list: Vec<Value> = tests
                    .iter()
                    .map(|(k, s)| json!({"key": k, "status": s}))
                    .collect();
                (200, json!({"key": exec, "tests": list}).to_string())
            }
            None => not_found("Test execution"),
        },
        _ => (
            404,
            json!({"errorMessages":[format!("Unknown raven route {} {seg:?}", req.method)]})
                .to_string(),
        ),
    }
}

/// The values of a `field = "value"` or `field in (a, "b")` clause of a JQL text (exact, case as
/// written). `None` when the clause is absent.
fn jql_values(jql: &str, field: &str) -> Option<Vec<String>> {
    let eq = format!("{field} = \"");
    if let Some(i) = jql.find(&eq) {
        let start = i + eq.len();
        let end = jql[start..].find('"')?;
        return Some(vec![jql[start..start + end].to_string()]);
    }
    let list = format!("{field} in (");
    let i = jql.find(&list)? + list.len();
    let end = jql[i..].find(')')?;
    Some(
        jql[i..i + end]
            .split(',')
            .map(|v| v.trim().trim_matches('"').to_string())
            .collect(),
    )
}

/// Issues matching the `project`, `issuetype` and `labels` clauses of the JQL text. A clause that
/// is absent does not filter. Values are compared exactly: `Test` does not match `Test Execution`,
/// and `labels = "tcms-pr-PR-1"` does not match a label `tcms-pr-PR-10`.
fn matching_issues(st: &State, jql: &str) -> Vec<Value> {
    let projects = jql_values(jql, "project");
    let types = jql_values(jql, "issuetype");
    let labels = jql_values(jql, "labels");
    st.issues
        .values()
        .filter(|i| {
            let p = i["fields"]["project"]["key"].as_str().unwrap_or("");
            let t = i["fields"]["issuetype"]["name"].as_str().unwrap_or("");
            let has_label = |wanted: &String| {
                i["fields"]["labels"]
                    .as_array()
                    .is_some_and(|ls| ls.iter().any(|l| l.as_str() == Some(wanted.as_str())))
            };
            projects.as_ref().is_none_or(|v| v.iter().any(|x| x == p))
                && types.as_ref().is_none_or(|v| v.iter().any(|x| x == t))
                && labels.as_ref().is_none_or(|v| v.iter().any(has_label))
        })
        .cloned()
        .collect()
}
