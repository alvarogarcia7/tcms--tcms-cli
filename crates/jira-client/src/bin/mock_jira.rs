//! `mock-jira`: a standalone mock Jira for acceptance tests (TCMS-42.9). It serves the
//! in-process mock of `jira_client::mock` on a random local port, seeded with the features PAY-1
//! and PAY-2 (Stories) in project PAY, until it is killed.
//!
//! Usage: `MOCK_JIRA_TOKEN=<token> mock-jira [--layout cloud|datacenter]`. It prints
//! `MOCK_JIRA_URL=<base url>` on its first line. The bearer token is checked on every request;
//! it is read from the environment, never from the command line. Verified against the mock
//! only: this models Jira from public documentation, it is not a Jira.

use std::io::Write;
use std::thread;

use jira_client::mock::{Layout, MockJira};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let layout = match args
        .iter()
        .position(|a| a == "--layout")
        .and_then(|i| args.get(i + 1))
    {
        None => Layout::Cloud,
        Some(v) if v == "cloud" => Layout::Cloud,
        Some(v) if v == "datacenter" => Layout::DataCenter,
        Some(v) => {
            eprintln!("mock-jira: unknown layout '{v}' (cloud or datacenter)");
            std::process::exit(2);
        }
    };
    let token = std::env::var("MOCK_JIRA_TOKEN").unwrap_or_else(|_| "mock".to_string());
    let mock = MockJira::start(layout, &format!("Bearer {token}"));
    for key in ["PAY-1", "PAY-2"] {
        mock.add_issue(
            key,
            "Story",
            &format!("Feature {key}"),
            "Feature text from the mock",
            "To Do",
            "2026-10-01T00:00:00.000+0000",
            &[],
        );
    }
    println!("MOCK_JIRA_URL={}", mock.base_url());
    let _ = std::io::stdout().flush();
    // serve until the process is killed
    loop {
        thread::park();
    }
}
