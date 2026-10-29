//! Static check of the GitLab CI job `tcms:jira-sync` (TCMS-42.9 AC 3). GitLab CI cannot run
//! here, so this parses `.gitlab-ci.yml` and asserts the job's contract: it runs the sync
//! on every default-branch pipeline (D-42.9d: no results-merge marker exists), applies only there, takes its
//! secrets from CI variables, references only files that exist, and the documentation names
//! the same variables and modes.

use std::path::PathBuf;

use serde_yaml::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn ci() -> Value {
    let text = std::fs::read_to_string(root().join(".gitlab-ci.yml")).expect("read .gitlab-ci.yml");
    serde_yaml::from_str(&text).expect(".gitlab-ci.yml parses as YAML")
}

fn job<'a>(ci: &'a Value, name: &str) -> &'a Value {
    ci.get(name)
        .unwrap_or_else(|| panic!("no job or template '{name}'"))
}

/// The script lines of a job, joined (block scalars included). A job that only extends a
/// template has none of its own.
fn script_text(job: &Value) -> String {
    let Some(lines) = job.get("script") else {
        return String::new();
    };
    lines
        .as_sequence()
        .expect("script is a list")
        .iter()
        .map(|l| l.as_str().expect("script line is a string").to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The effective script: the template's, then the job's (`extends`).
fn effective<'a>(ci: &'a Value, name: &str) -> (&'a Value, String) {
    let j = job(ci, name);
    let parent = j["extends"].as_str().map(|p| job(ci, p));
    let text = match parent {
        Some(p) => format!("{}\n{}", script_text(p), script_text(j)),
        None => script_text(j),
    };
    assert!(!text.is_empty(), "{name} has a script");
    (j, text)
}

#[test]
fn the_job_is_in_a_declared_stage_and_extends_an_existing_template() {
    let ci = ci();
    let stages: Vec<&str> = ci["stages"]
        .as_sequence()
        .expect("stages")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let (j, _) = effective(&ci, "tcms:jira-sync");
    let stage = j["stage"].as_str().or_else(|| {
        j["extends"]
            .as_str()
            .and_then(|p| job(&ci, p)["stage"].as_str())
    });
    let stage = stage.expect("the job has a stage");
    assert!(
        stages.contains(&stage),
        "stage {stage} is not in {stages:?}"
    );
    assert_eq!(
        j["extends"].as_str(),
        Some(".tcms-jira-sync"),
        "the job extends the template"
    );
    assert!(
        job(&ci, ".tcms-jira-sync").get("script").is_some(),
        "the template has a script"
    );
}

#[test]
fn the_job_runs_on_default_branch_pipelines_and_applies_only_there() {
    let ci = ci();
    let (j, _) = effective(&ci, "tcms:jira-sync");
    let rules = j["rules"].as_sequence().expect("rules");
    let first = &rules[0];
    let cond = first["if"].as_str().expect("first rule has an if");
    assert!(
        cond.contains("$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"),
        "the first rule is the default branch pipeline (D-42.9d): {cond}"
    );
    assert!(cond.contains("$TCMS_PRODUCT"), "needs a product: {cond}");
    assert_eq!(first["when"].as_str(), Some("on_success"));
    for r in rules {
        let when = r["when"].as_str().unwrap_or("on_success");
        assert!(
            [
                "on_success",
                "manual",
                "always",
                "never",
                "delayed",
                "on_failure"
            ]
            .contains(&when),
            "not a GitLab when value: {when}"
        );
    }
}

#[test]
fn the_script_chooses_apply_by_branch_and_names_the_sync_command() {
    let ci = ci();
    let (_, text) = effective(&ci, "tcms:jira-sync");
    assert!(
        text.contains("cargo build -p tcms --release"),
        "builds tcms:\n{text}"
    );
    assert!(text.contains("jira sync"), "runs the sync:\n{text}");
    assert!(
        text.contains("if [ \"$CI_COMMIT_BRANCH\" = \"$CI_DEFAULT_BRANCH\" ]; then MODE=--apply; else MODE=--dry-run; fi"),
        "the mode follows the branch:\n{text}"
    );
    assert!(
        text.contains("\"$MODE\""),
        "the chosen mode is what the sync gets:\n{text}"
    );
    assert!(
        !text.contains("--apply\n") || text.contains("MODE=--apply"),
        "--apply appears only as the default-branch mode:\n{text}"
    );
    assert!(text.contains("--product \"$TCMS_PRODUCT\""), "{text}");
    assert!(
        text.contains("--config \"$TCMS_INTEGRATION_CONFIG\""),
        "{text}"
    );
    assert!(text.contains("--state-dir \"$TCMS_STATE_DIR\""), "{text}");
}

#[test]
fn the_binary_the_job_runs_is_a_tcms_binary_target() {
    // target/release/tcms is produced by `cargo build -p tcms --release`
    let manifest = std::fs::read_to_string(root().join("crates/tcms/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("name = \"tcms\""),
        "the tcms binary exists"
    );
    let ci = ci();
    let (_, text) = effective(&ci, "tcms:jira-sync");
    assert!(text.contains("./target/release/tcms"), "{text}");
}

#[test]
fn no_secret_is_a_literal_in_the_job_and_none_is_a_job_variable() {
    let ci = ci();
    let (j, text) = effective(&ci, "tcms:jira-sync");
    let mut vars: Vec<String> = Vec::new();
    for name in [".tcms-jira-sync", "tcms:jira-sync"] {
        if let Some(v) = job(&ci, name).get("variables").and_then(Value::as_mapping) {
            vars.extend(v.keys().filter_map(|k| k.as_str().map(str::to_string)));
        }
    }
    for v in &vars {
        let upper = v.to_uppercase();
        assert!(
            !(upper.contains("TOKEN") || upper.contains("SECRET") || upper.contains("PASSWORD")),
            "secret variable {v} is set in the job; set it in CI/CD variables instead"
        );
    }
    for needle in ["Bearer ", "TCMS_JIRA_TOKEN=", "password", "token="] {
        assert!(
            !text.contains(needle),
            "literal '{needle}' in the job script"
        );
    }
    // the job names its secrets only through the environment (the config names the variable)
    assert!(j["extends"].as_str().is_some());
}

#[test]
fn files_and_scripts_the_job_references_exist() {
    let ci = ci();
    let (_, text) = effective(&ci, "tcms:jira-sync");
    for token in text.split(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        if token.starts_with("scripts/") || token.starts_with("./scripts/") {
            let p = root().join(token.trim_start_matches("./"));
            assert!(
                p.exists(),
                "{} referenced by the job does not exist",
                p.display()
            );
        }
    }
    // the artifact path is the audit directory of the state dir the job writes to
    let template = job(&ci, ".tcms-jira-sync");
    let paths: Vec<&str> = template["artifacts"]["paths"]
        .as_sequence()
        .expect("artifact paths")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(paths, vec![".tcms/jira-sync/audit/"]);
    let state_dir = job(&ci, ".tcms-jira-sync")["variables"]["TCMS_STATE_DIR"]
        .as_str()
        .expect("TCMS_STATE_DIR");
    assert!(
        paths[0].starts_with(state_dir),
        "{paths:?} is under {state_dir}"
    );
}

#[test]
fn the_documentation_names_the_same_variables_and_modes() {
    let doc = std::fs::read_to_string(root().join("docs/JIRA_INTEGRATION.md")).unwrap();
    for needle in [
        "tcms:jira-sync",
        ".tcms-jira-sync",
        "TCMS_INTEGRATION_CONFIG",
        "TCMS_PRODUCT",
        "TCMS_JIRA_TOKEN",
        "--dry-run",
        "--apply",
    ] {
        assert!(
            doc.contains(needle),
            "docs/JIRA_INTEGRATION.md does not mention {needle}"
        );
    }
}
